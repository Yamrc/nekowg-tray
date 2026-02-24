use gpui::App;
use gpui_tray_core::{Error, PlatformTray, Result, Tray};
use log::{debug, error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{FALSE, HWND, TRUE};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

use crate::util::encode_wide;
use crate::window::{
    WM_USER_TRAYICON, build_menu, create_tray_window, destroy_window, destroy_window_menu,
    set_menu_actions, set_window_menu, unregister_tray_class,
};

static TRAY_COUNTER: AtomicU32 = AtomicU32::new(0);
static WM_TASKBAR_RESTART: AtomicU32 = AtomicU32::new(0);

/// Returns the TaskbarCreated message ID, registering it if necessary.
pub fn taskbar_restart_message() -> u32 {
    let msg = WM_TASKBAR_RESTART.load(Ordering::Relaxed);
    if msg == 0 {
        let new_msg = unsafe { RegisterWindowMessageW(windows::core::w!("TaskbarCreated")) };
        WM_TASKBAR_RESTART.store(new_msg, Ordering::Relaxed);
        new_msg
    } else {
        msg
    }
}

pub struct WindowsTray {
    hwnd: HWND,
    tray_id: u32,
    registered: bool,
    visible: bool,
    current_tray: Option<Tray>,
}

impl WindowsTray {
    pub(crate) fn new() -> Self {
        let tray_id = TRAY_COUNTER.fetch_add(1, Ordering::Relaxed);
        debug!("Creating WindowsTray with ID: {}", tray_id);

        taskbar_restart_message();

        Self {
            hwnd: HWND(std::ptr::null_mut()),
            tray_id,
            registered: false,
            visible: false,
            current_tray: None,
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
            debug!("Tray window created: {:?}", self.hwnd);
        }
        Ok(())
    }

    fn build_menu_from_tray(&mut self, cx: &mut App, tray: &Tray) {
        destroy_window_menu(self.hwnd);
        set_menu_actions(None);

        if let Some(builder) = &tray.menu_builder {
            let items = builder(cx);
            if !items.is_empty() {
                unsafe {
                    if let Some((hmenu, actions)) = build_menu(&items) {
                        let actions_map: HashMap<u32, Box<dyn gpui::Action>> =
                            actions.into_iter().collect();
                        set_menu_actions(Some(Arc::new(actions_map)));
                        set_window_menu(self.hwnd, Some(hmenu));
                    }
                }
            }
        }
    }

    fn add_or_update_tray_icon(&mut self, tray: &Tray, is_update: bool) -> Result<()> {
        let mut flags = NIF_MESSAGE;
        let mut sz_tip: [u16; 128] = [0; 128];

        if let Some(tooltip) = &tray.tooltip {
            flags |= NIF_TIP;
            let wide_tip = encode_wide(tooltip.as_ref());
            for (i, &ch) in wide_tip.iter().take(127).enumerate() {
                sz_tip[i] = ch;
            }
        }

        unsafe {
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                uFlags: flags,
                hWnd: self.hwnd,
                uID: self.tray_id,
                uCallbackMessage: WM_USER_TRAYICON,
                szTip: sz_tip,
                ..std::mem::zeroed()
            };

            let action = if is_update { NIM_MODIFY } else { NIM_ADD };
            let result = Shell_NotifyIconW(action, &mut nid);

            if result != TRUE {
                let err_msg = format!("Shell_NotifyIconW failed for action {:?}", action);
                error!("{}", err_msg);
                return Err(Error::Platform(err_msg));
            }
        }

        Ok(())
    }

    fn remove_tray_icon(&mut self) {
        if !self.hwnd.is_invalid() && self.registered {
            unsafe {
                let mut nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    uFlags: NIF_MESSAGE,
                    hWnd: self.hwnd,
                    uID: self.tray_id,
                    ..std::mem::zeroed()
                };

                if Shell_NotifyIconW(NIM_DELETE, &mut nid) == FALSE {
                    error!("Failed to remove tray icon");
                } else {
                    debug!("Tray icon removed successfully");
                }
            }
        }
    }
}

impl PlatformTray for WindowsTray {
    fn set_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        debug!(
            "set_tray called: visible={}, tooltip={:?}",
            tray.visible, tray.tooltip
        );

        self.ensure_window()?;

        if !tray.visible {
            self.remove_tray_icon();
            destroy_window_menu(self.hwnd);
            set_menu_actions(None);
            self.registered = false;
            self.visible = false;
            self.current_tray = None;
            info!("Tray hidden successfully");
            return Ok(());
        }

        self.build_menu_from_tray(cx, tray);

        if !self.registered {
            self.add_or_update_tray_icon(tray, false)?;
            self.registered = true;
            self.visible = true;
            info!("Tray icon created successfully");
        } else {
            self.add_or_update_tray_icon(tray, true)?;
            info!("Tray icon updated successfully");
        }

        self.current_tray = Some(tray.clone());
        Ok(())
    }

    fn update_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        debug!("update_tray called");
        self.set_tray(cx, tray)
    }

    fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
        debug!("remove_tray called");

        self.remove_tray_icon();
        destroy_window_menu(self.hwnd);
        set_menu_actions(None);

        self.registered = false;
        self.visible = false;
        self.current_tray = None;

        info!("Tray removed successfully");
        Ok(())
    }
}

impl Drop for WindowsTray {
    fn drop(&mut self) {
        debug!("Dropping WindowsTray, cleaning up resources");
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
