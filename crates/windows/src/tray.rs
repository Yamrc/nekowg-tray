use gpui::App;
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{Error, Result, Tray};
use log::{debug, error};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{FALSE, HWND, TRUE};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

use crate::icon::Icon;
use crate::util::encode_wide;
use crate::window::{
    WM_USER_TRAYICON, build_menu, create_tray_window, destroy_window, destroy_window_menu,
    set_menu_actions, set_window_menu, unregister_tray_class,
};

static TRAY_COUNTER: AtomicU32 = AtomicU32::new(0);
static WM_TASKBAR_RESTART: AtomicU32 = AtomicU32::new(0);

/// Returns the TaskbarCreated message ID, registering it if necessary.
///
/// This message is broadcast by Windows when the taskbar is recreated
/// (e.g., after explorer.exe restart). Applications should re-register
/// their tray icons when receiving this message.
pub(crate) fn taskbar_restart_message() -> u32 {
    let msg = WM_TASKBAR_RESTART.load(Ordering::Relaxed);
    if msg == 0 {
        let new_msg = unsafe { RegisterWindowMessageW(windows::core::w!("TaskbarCreated")) };
        WM_TASKBAR_RESTART.store(new_msg, Ordering::Relaxed);
        new_msg
    } else {
        msg
    }
}

/// Windows system tray implementation.
///
/// Manages a tray icon with support for:
/// - Custom icons (decoded from various image formats)
/// - Tooltip text
/// - Context menus with action dispatching
/// - Click event handling
///
/// # Example
///
/// ```rust
/// let tray = WindowsTray::new();
/// tray.set_tray(cx, &Tray::new()
///     .tooltip("My App")
///     .icon(image)
///     .menu(|cx| vec![MenuItem::action("Quit", Quit)]))?;
/// ```
pub(crate) struct WindowsTray {
    hwnd: HWND,
    tray_id: u32,
    registered: bool,
    visible: bool,
    current_tray: Option<Tray>,
    icon: Option<Icon>,
}

impl WindowsTray {
    pub(crate) fn new() -> Self {
        let tray_id = TRAY_COUNTER.fetch_add(1, Ordering::Relaxed);

        taskbar_restart_message();

        Self {
            hwnd: HWND(std::ptr::null_mut()),
            tray_id,
            registered: false,
            visible: false,
            current_tray: None,
            icon: None,
        }
    }

    fn ensure_window(&mut self) -> Result<()> {
        if self.hwnd.is_invalid() {
            self.hwnd = match create_tray_window() {
                Ok(hwnd) => hwnd,
                Err(e) => {
                    error!("Failed to create tray window: {}", e);
                    return Err(Error::Platform(e.to_string()));
                }
            };
        }
        Ok(())
    }

    fn build_menu_from_tray(&mut self, cx: &mut App, tray: &Tray) {
        destroy_window_menu(self.hwnd);
        set_menu_actions(None);

        if let Some(builder) = &tray.menu_builder {
            let items = builder(cx);
            if !items.is_empty()
                && let Some((hmenu, actions)) = unsafe { build_menu(&items) }
            {
                let actions_map: HashMap<u32, Box<dyn gpui::Action>> =
                    actions.into_iter().collect();
                set_menu_actions(Some(Rc::new(actions_map)));
                set_window_menu(self.hwnd, Some(hmenu));
            }
        }
    }

    fn add_or_update_tray_icon(&mut self, tray: &Tray, is_update: bool) -> Result<()> {
        let mut flags = NIF_MESSAGE;
        let mut sz_tip: [u16; 128] = [0; 128];
        let mut hicon = windows::Win32::UI::WindowsAndMessaging::HICON(std::ptr::null_mut());

        if let Some(tooltip) = &tray.tooltip {
            flags |= NIF_TIP;
            let wide_tip = encode_wide(tooltip.as_ref());
            for (i, &ch) in wide_tip.iter().take(127).enumerate() {
                sz_tip[i] = ch;
            }
        }

        if let Some(image) = &tray.icon {
            match Icon::from_image(image) {
                Ok(icon) => {
                    hicon = icon.as_hicon();
                    flags |= NIF_ICON;
                    self.icon = Some(icon);
                    debug!("Icon created and set successfully");
                }
                Err(e) => {
                    error!("Failed to create icon: {}", e);
                }
            }
        }

        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            uFlags: flags,
            hWnd: self.hwnd,
            uID: self.tray_id,
            uCallbackMessage: WM_USER_TRAYICON,
            hIcon: hicon,
            szTip: sz_tip,
            ..unsafe { std::mem::zeroed() }
        };

        let action = if is_update { NIM_MODIFY } else { NIM_ADD };
        let result = unsafe { Shell_NotifyIconW(action, &nid) };

        if result != TRUE {
            let err_msg = format!("Shell_NotifyIconW failed for action {:?}", action);
            error!("{}", err_msg);
            return Err(Error::Platform(err_msg));
        }

        Ok(())
    }

    fn remove_tray_icon(&mut self) {
        if !self.hwnd.is_invalid() && self.registered {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                uFlags: NIF_MESSAGE,
                hWnd: self.hwnd,
                uID: self.tray_id,
                ..unsafe { std::mem::zeroed() }
            };

            if unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) } == FALSE {
                error!("Failed to remove tray icon");
            } else {
                debug!("Tray icon removed successfully");
            }
        }
        self.icon = None;
    }
}

impl PlatformTray for WindowsTray {
    fn set_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        self.ensure_window()?;

        if !tray.visible {
            self.remove_tray_icon();
            destroy_window_menu(self.hwnd);
            set_menu_actions(None);
            self.registered = false;
            self.visible = false;
            self.current_tray = None;
            return Ok(());
        }

        self.build_menu_from_tray(cx, tray);

        if !self.registered {
            self.add_or_update_tray_icon(tray, false)?;
            self.registered = true;
            self.visible = true;
        } else {
            self.add_or_update_tray_icon(tray, true)?;
        }

        self.current_tray = Some(tray.clone());
        Ok(())
    }

    fn update_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        self.set_tray(cx, tray)
    }

    fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
        self.remove_tray_icon();
        destroy_window_menu(self.hwnd);
        set_menu_actions(None);

        self.registered = false;
        self.visible = false;
        self.current_tray = None;

        Ok(())
    }
}

impl Drop for WindowsTray {
    fn drop(&mut self) {
        if !self.hwnd.is_invalid() {
            self.remove_tray_icon();
            destroy_window_menu(self.hwnd);
            set_menu_actions(None);
            destroy_window(self.hwnd);
            self.hwnd = HWND(std::ptr::null_mut());
        }
        unregister_tray_class();
    }
}

pub fn create() -> Result<Box<dyn PlatformTray>> {
    debug!("Creating Windows tray implementation");
    Ok(Box::new(WindowsTray::new()))
}
