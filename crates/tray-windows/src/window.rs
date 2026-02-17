//! Window creation and message handling for tray

use crate::tray::TrayUserData;
use gpui::MenuItem as GpuiMenuItem;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    GetCursorPos, HMENU, MF_POPUP, MF_SEPARATOR, MF_STRING, RegisterClassW, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu, WM_LBUTTONUP, WM_MBUTTONUP, WM_RBUTTONUP,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

/// Custom window message for tray icon notifications
pub const WM_USER_TRAYICON: u32 = 6002;

/// Window class name for tray window
const PLATFORM_TRAY_CLASS_NAME: PCWSTR = w!("GPUI::Tray");

/// Register the window class for tray window
fn register_platform_tray_class() {
    static REGISTERED: std::sync::Once = std::sync::Once::new();

    REGISTERED.call_once(|| {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_procedure),
            lpszClassName: PCWSTR(PLATFORM_TRAY_CLASS_NAME.as_ptr()),
            ..Default::default()
        };

        unsafe {
            let result = RegisterClassW(&wc);
            log::debug!("RegisterClassW result: {}", result);
        }
    });
}

/// Create the hidden window for tray message handling
pub fn create_tray_window() -> HWND {
    register_platform_tray_class();

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            PLATFORM_TRAY_CLASS_NAME,
            None,
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            0,
            CW_USEDEFAULT,
            0,
            None,
            None,
            None,
            None,
        )
    };

    match hwnd {
        Ok(h) => {
            log::debug!("window created: {:?}", h);
            h
        }
        Err(e) => {
            log::error!("Failed to create tray window: {:?}", e);
            HWND(std::ptr::null_mut())
        }
    }
}

/// Build Windows HMENU from GPUI MenuItems
pub fn build_menu(items: &[GpuiMenuItem]) -> Option<HMENU> {
    unsafe {
        let hmenu = CreatePopupMenu().ok()?;
        build_menu_items(hmenu, items, 0);
        Some(hmenu)
    }
}

/// Recursively build menu items and return the next available ID
unsafe fn build_menu_items(hmenu: HMENU, items: &[GpuiMenuItem], start_id: u32) -> u32 {
    let mut current_id = start_id;

    for item in items {
        match item {
            GpuiMenuItem::Separator => unsafe {
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            },
            GpuiMenuItem::Action { name, .. } => {
                current_id += 1;
                let wide_name: Vec<u16> = OsStr::new(name.as_ref())
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                unsafe {
                    let result = AppendMenuW(
                        hmenu,
                        MF_STRING,
                        current_id as usize,
                        PCWSTR(wide_name.as_ptr()),
                    );
                    if result.is_err() {
                        log::error!("Failed to append menu item: {}", name);
                    }
                }
            }
            GpuiMenuItem::Submenu(submenu) => {
                unsafe {
                    if let Ok(submenu_handle) = CreatePopupMenu() {
                        // Recursively build submenu items
                        let next_id = build_menu_items(submenu_handle, &submenu.items, current_id);
                        current_id = next_id;

                        // Add the submenu to the parent
                        let wide_name: Vec<u16> = OsStr::new(submenu.name.as_ref())
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();

                        // MF_POPUP requires the HMENU as the parameter (cast to usize)
                        let result = AppendMenuW(
                            hmenu,
                            MF_POPUP,
                            submenu_handle.0 as usize,
                            PCWSTR(wide_name.as_ptr()),
                        );

                        if result.is_err() {
                            log::error!("Failed to append submenu: {}", submenu.name);
                            // Clean up the submenu if we failed to add it
                            let _ = DestroyMenu(submenu_handle);
                        }
                        // Note: submenu_handle is now owned by parent menu, don't destroy on success
                    } else {
                        log::error!("Failed to create submenu: {}", submenu.name);
                    }
                }
            }
            _ => {
                log::warn!("Unsupported menu item type");
            }
        }
    }

    current_id
}

/// Show tray context menu at cursor position
pub fn show_tray_menu(hwnd: HWND, hmenu: HMENU) {
    unsafe {
        let mut cursor_pos = windows::Win32::Foundation::POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor_pos).is_ok() {
            let _ = SetForegroundWindow(hwnd);
            let _ = TrackPopupMenu(
                hmenu,
                TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                cursor_pos.x,
                cursor_pos.y,
                Some(0),
                hwnd,
                None,
            );
        }
    }
}

/// Window procedure for tray window
unsafe extern "system" fn tray_procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_USER_TRAYICON {
        let event = lparam.0 as u32;

        let cursor_pos = unsafe {
            let mut pos = windows::Win32::Foundation::POINT { x: 0, y: 0 };
            let _ = GetCursorPos(&mut pos);
            (pos.x, pos.y)
        };

        match event {
            WM_LBUTTONUP => {
                log::debug!(
                    "WM_LBUTTONUP detected at ({}, {})",
                    cursor_pos.0,
                    cursor_pos.1
                );

                // event_type=0 (click), button=0 (left), x, y
                dispatch_raw_event(hwnd, 0, 0, cursor_pos.0, cursor_pos.1);
            }
            WM_RBUTTONUP => {
                log::debug!(
                    "WM_RBUTTONUP detected at ({}, {})",
                    cursor_pos.0,
                    cursor_pos.1
                );

                // event_type=0 (click), button=1 (right), x, y
                dispatch_raw_event(hwnd, 0, 1, cursor_pos.0, cursor_pos.1);

                unsafe {
                    let user_data_ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    );
                    if user_data_ptr != 0 {
                        let user_data = &*(user_data_ptr as *const TrayUserData);
                        if let Some(hmenu) = user_data.hmenu {
                            show_tray_menu(hwnd, hmenu);
                        }
                    }
                }
            }
            WM_MBUTTONUP => {
                log::debug!(
                    "WM_MBUTTONUP detected at ({}, {})",
                    cursor_pos.0,
                    cursor_pos.1
                );

                // event_type=0 (click), button=2 (middle), x, y
                dispatch_raw_event(hwnd, 0, 2, cursor_pos.0, cursor_pos.1);
            }
            _ => {}
        }
    } else if msg == windows::Win32::UI::WindowsAndMessaging::WM_COMMAND {
        let menu_id = wparam.0 as u16;
        if menu_id > 0 {
            log::debug!("WM_COMMAND detected with menu_id {}", menu_id);

            // event_type=1 (menu_select), menu_id, x=0, y=0
            dispatch_raw_event(hwnd, 1, menu_id as u32, 0, 0);
        }
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Dispatch raw event data to the callback
///
/// Parameters:
/// - hwnd: Window handle
/// - event_type: 0=click, 1=menu_select
/// - button_or_id: mouse button (0=left, 1=right, 2=middle) or menu item id
/// - x: x coordinate (for click events)
/// - y: y coordinate (for click events)
fn dispatch_raw_event(hwnd: HWND, event_type: u32, button_or_id: u32, x: i32, y: i32) {
    unsafe {
        let user_data_ptr = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        );
        if user_data_ptr != 0 {
            let user_data = &*(user_data_ptr as *const TrayUserData);
            user_data.dispatch_event(event_type, button_or_id, x, y);
        }
    }
}
