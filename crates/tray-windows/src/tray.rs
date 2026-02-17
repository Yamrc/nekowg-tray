//! Windows tray implementation
//!
//! Internal implementation for Windows platform.
//! This module is not part of the public API.

use gpui::{App, MenuItem as GpuiMenuItem, SharedString};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{FALSE, HWND, TRUE};
use windows::Win32::UI::Shell::{
    NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyMenu, DestroyWindow, GWLP_USERDATA, GetWindowLongPtrW, HMENU, SetWindowLongPtrW,
};

use crate::util::encode_wide;
use crate::window::{WM_USER_TRAYICON, build_menu, create_tray_window};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Platform tray trait - internal use only
///
/// This trait is implemented by all platform-specific tray implementations.
pub trait PlatformTray: 'static {
    /// Create or update the tray
    fn set_tray(&mut self, app: &mut App, config: WindowsTrayConfig);
    /// Hide the tray icon
    fn hide(&mut self);
    /// Show the tray icon
    fn show(&mut self, app: &mut App);
    /// Destroy the tray and release resources
    fn destroy(&mut self);
}

/// Windows tray configuration
pub struct WindowsTrayConfig {
    pub tooltip: Option<SharedString>,
    pub visible: bool,
    pub menu_items: Option<Vec<GpuiMenuItem>>,
    /// Event callback - receives raw event data (button, x, y, menu_id)
    /// Format: (event_type, button_or_menu_id, x, y)
    /// event_type: 0=click, 1=menu_select
    pub event_callback: Option<EventCallback>,
}

/// Event callback wrapper - uses raw types to avoid duplication with gpui_tray::TrayEvent
#[derive(Clone)]
pub struct EventCallback {
    inner: Rc<RefCell<dyn FnMut(u32, u32, i32, i32) + 'static>>,
}

impl EventCallback {
    pub fn new<F>(callback: F) -> Self
    where
        F: FnMut(u32, u32, i32, i32) + 'static,
    {
        Self {
            inner: Rc::new(RefCell::new(callback)),
        }
    }

    /// Invoke the callback
    ///
    /// Parameters:
    /// - event_type: 0=click, 1=menu_select, 2=scroll
    /// - button_or_id: mouse button (0=left, 1=right, 2=middle) or menu item id
    /// - x: x coordinate (for click events)
    /// - y: y coordinate (for click events)
    pub fn invoke(&self, event_type: u32, button_or_id: u32, x: i32, y: i32) {
        (self.inner.borrow_mut())(event_type, button_or_id, x, y);
    }
}

/// Windows-specific tray implementation
pub struct WindowsTray {
    tray_id: u32,
    hwnd: HWND,
    visible: bool,
    registered: bool,
    hmenu: Option<HMENU>,
    event_callback: Option<EventCallback>,
}

impl WindowsTray {
    /// Create a new Windows tray instance
    pub fn new() -> Self {
        Self {
            tray_id: 0,
            hwnd: HWND(std::ptr::null_mut()),
            visible: false,
            registered: false,
            hmenu: None,
            event_callback: None,
        }
    }

    /// Ensure the tray window exists, creating it if necessary
    fn ensure_window(&mut self) -> Option<HWND> {
        if self.hwnd.is_invalid() {
            self.hwnd = create_tray_window();
            if self.hwnd.is_invalid() {
                log::error!("Failed to create tray window");
                return None;
            }
            log::debug!("Created tray window: {:?}", self.hwnd);
        }
        Some(self.hwnd)
    }

    /// Destroy the tray window and clean up associated resources
    fn destroy_window(&mut self) {
        if !self.hwnd.is_invalid() {
            unsafe {
                let user_data = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA);
                if user_data != 0 {
                    let _ = Box::from_raw(user_data as *mut TrayUserData);
                    SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                }
                let _ = DestroyWindow(self.hwnd);
            }
            self.hwnd = HWND(std::ptr::null_mut());
            log::debug!("Destroyed tray window");
        }
    }

    /// Build menu from GPUI menu items
    fn build_menu(&mut self, items: Option<&[GpuiMenuItem]>) {
        self.destroy_menu();
        if let Some(items) = items {
            self.hmenu = build_menu(items);
        }
    }

    /// Destroy the menu resource
    fn destroy_menu(&mut self) {
        if let Some(hmenu) = self.hmenu.take() {
            unsafe {
                let _ = DestroyMenu(hmenu);
            }
            log::debug!("Destroyed menu");
        }
    }

    /// Add the tray icon to the system tray
    fn add_tray_icon(&mut self, tooltip: Option<&str>) -> bool {
        if self.hwnd.is_invalid() {
            log::error!("Cannot add tray icon: no window");
            return false;
        }

        let mut flags = NIF_MESSAGE;
        let mut sz_tip: [u16; 128] = [0; 128];

        if let Some(tip) = tooltip {
            flags |= NIF_TIP;
            let wide_tip = encode_wide(tip);
            for (i, &ch) in wide_tip.iter().take(128).enumerate() {
                sz_tip[i] = ch;
            }
        }

        unsafe {
            let mut nid = NOTIFYICONDATAW {
                uFlags: flags,
                hWnd: self.hwnd,
                uID: self.tray_id,
                uCallbackMessage: WM_USER_TRAYICON,
                szTip: sz_tip,
                ..std::mem::zeroed()
            };

            let result = Shell_NotifyIconW(NIM_ADD, &mut nid);
            log::debug!("Shell_NotifyIconW(NIM_ADD) result: {:?}", result);
            result == TRUE
        }
    }

    /// Modify the existing tray icon
    fn modify_tray_icon(&mut self, tooltip: Option<&str>) -> bool {
        if self.hwnd.is_invalid() {
            log::error!("Cannot modify tray icon: no window");
            return false;
        }

        let mut flags = NIF_MESSAGE;
        let mut sz_tip: [u16; 128] = [0; 128];

        if let Some(tip) = tooltip {
            flags |= NIF_TIP;
            let wide_tip = encode_wide(tip);
            for (i, &ch) in wide_tip.iter().take(128).enumerate() {
                sz_tip[i] = ch;
            }
        }

        unsafe {
            let mut nid = NOTIFYICONDATAW {
                uFlags: flags,
                hWnd: self.hwnd,
                uID: self.tray_id,
                uCallbackMessage: WM_USER_TRAYICON,
                szTip: sz_tip,
                ..std::mem::zeroed()
            };

            let result = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
            log::debug!("Shell_NotifyIconW(NIM_MODIFY) result: {:?}", result);
            result == TRUE
        }
    }

    /// Remove the tray icon from the system tray
    fn remove_tray_icon(&mut self) {
        if self.hwnd.is_invalid() {
            return;
        }

        unsafe {
            let mut nid = NOTIFYICONDATAW {
                uFlags: NIF_MESSAGE,
                hWnd: self.hwnd,
                uID: self.tray_id,
                ..std::mem::zeroed()
            };

            if Shell_NotifyIconW(NIM_DELETE, &mut nid) == FALSE {
                log::error!("Failed to remove system tray icon");
            } else {
                log::debug!("Removed tray icon");
            }
        }
    }

    /// Update user data attached to the window
    fn update_user_data(&mut self) {
        if self.hwnd.is_invalid() {
            return;
        }

        unsafe {
            let old_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA);
            if old_ptr != 0 {
                let _ = Box::from_raw(old_ptr as *mut TrayUserData);
            }

            let user_data = Box::new(TrayUserData::new(self.hmenu, self.event_callback.clone()));
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, Box::into_raw(user_data) as isize);
        }
        log::debug!("Updated window user data");
    }
}

impl PlatformTray for WindowsTray {
    fn set_tray(&mut self, _app: &mut App, config: WindowsTrayConfig) {
        if self.tray_id == 0 {
            self.tray_id = COUNTER.fetch_add(1, Ordering::Relaxed);
            log::debug!("Assigned tray ID: {}", self.tray_id);
        }

        if let Some(callback) = config.event_callback {
            self.event_callback = Some(callback);
        }

        if !config.visible {
            self.hide();
            return;
        }

        if self.ensure_window().is_none() {
            return;
        }

        self.build_menu(config.menu_items.as_deref());

        let tooltip = config.tooltip.as_ref().map(|s| s.as_ref());
        if !self.registered {
            if self.add_tray_icon(tooltip) {
                self.registered = true;
                self.visible = true;
                log::info!("Tray icon created successfully");
            } else {
                log::error!("Failed to create tray icon");
            }
        } else {
            if self.modify_tray_icon(tooltip) {
                log::info!("Tray icon updated successfully");
            } else {
                log::error!("Failed to update tray icon");
            }
        }

        self.update_user_data();
    }

    fn hide(&mut self) {
        if self.registered {
            self.remove_tray_icon();
            self.registered = false;
            self.visible = false;
            log::info!("Tray icon hidden");
        }
    }

    fn show(&mut self, app: &mut App) {
        if !self.registered {
            let config = WindowsTrayConfig {
                tooltip: None,
                visible: true,
                menu_items: None,
                event_callback: None,
            };
            self.set_tray(app, config);
        }
    }

    fn destroy(&mut self) {
        self.hide();
        self.destroy_menu();
        self.destroy_window();
        self.tray_id = 0;
        log::info!("Tray destroyed");
    }
}

impl Drop for WindowsTray {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// User data stored in the tray window
pub(crate) struct TrayUserData {
    pub(crate) hmenu: Option<HMENU>,
    pub(crate) event_callback: Option<EventCallback>,
}

impl TrayUserData {
    pub(crate) fn new(hmenu: Option<HMENU>, event_callback: Option<EventCallback>) -> Self {
        Self {
            hmenu,
            event_callback,
        }
    }

    pub(crate) fn dispatch_event(&self, event_type: u32, button_or_id: u32, x: i32, y: i32) {
        if let Some(callback) = &self.event_callback {
            callback.invoke(event_type, button_or_id, x, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_tray_new() {
        let tray = WindowsTray::new();
        assert_eq!(tray.tray_id, 0);
        assert!(!tray.visible);
        assert!(!tray.registered);
    }

    #[test]
    fn test_event_callback() {
        use std::cell::Cell;
        use std::rc::Rc;

        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        let callback = EventCallback::new(move |_etype, _bid, _x, _y| {
            called_clone.set(true);
        });

        callback.invoke(0, 0, 0, 0);
        assert!(called.get());
    }
}
