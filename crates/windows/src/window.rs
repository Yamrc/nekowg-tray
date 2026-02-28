use gpui::{MenuItem as GpuiMenuItem, MouseButton};
use log::{debug, error};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

use crate::util::encode_wide;

pub(crate) const WM_USER_TRAYICON: u32 = 6002;
pub(crate) const WM_USER_SET_MENU: u32 = WM_USER + 1;
pub(crate) const WM_USER_DESTROY_MENU: u32 = WM_USER + 2;

const PLATFORM_TRAY_CLASS_NAME: PCWSTR = w!("GPUI::Tray");

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);
static mut CLASS_ATOM: u16 = 0;

/// Trait for dispatching tray events to the application.
pub trait TrayEventDispatcher: Send + Sync + 'static {
    fn dispatch_click(&self, button: MouseButton, position: gpui::Point<f32>);
    fn dispatch_double_click(&self);
    fn dispatch_menu_action(&self, action: Box<dyn gpui::Action>);
}

/// Type alias for menu actions map.
pub(crate) type MenuActionsMap = Rc<HashMap<u32, Box<dyn gpui::Action>>>;

thread_local! {
    static DISPATCHER: Cell<Option<&'static dyn TrayEventDispatcher>> = Cell::new(None);
    static MENU_ACTIONS: RefCell<Option<MenuActionsMap>> = RefCell::new(None);
}

#[doc(hidden)]
pub fn set_dispatcher(dispatcher: Option<&'static dyn TrayEventDispatcher>) {
    DISPATCHER.set(dispatcher);
}

pub(crate) fn set_menu_actions(actions: Option<MenuActionsMap>) {
    MENU_ACTIONS.with(|cell| *cell.borrow_mut() = actions);
}

fn get_menu_action(id: u32) -> Option<Box<dyn gpui::Action>> {
    MENU_ACTIONS.with(|cell| cell.borrow().as_ref()?.get(&id).map(|a| a.boxed_clone()))
}

fn dispatch_click(button: MouseButton, position: gpui::Point<f32>) {
    DISPATCHER.with(|cell| {
        if let Some(dispatcher) = cell.get() {
            dispatcher.dispatch_click(button, position);
        }
    });
}

fn dispatch_double_click() {
    DISPATCHER.with(|cell| {
        if let Some(dispatcher) = cell.get() {
            dispatcher.dispatch_double_click();
        }
    });
}

fn dispatch_menu_action(action_id: u32) {
    if let Some(action) = get_menu_action(action_id) {
        DISPATCHER.with(|dispatcher_cell| {
            if let Some(dispatcher) = dispatcher_cell.get() {
                dispatcher.dispatch_menu_action(action);
            }
        });
    }
}

struct TrayUserData {
    hmenu: Option<HMENU>,
}

unsafe extern "system" fn tray_procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let user_data = Box::new(TrayUserData { hmenu: None });
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(user_data) as isize);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
    }

    let user_data_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if user_data_ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let user_data = unsafe { &mut *(user_data_ptr as *mut TrayUserData) };

    match msg {
        WM_DESTROY => {
            debug!("Window WM_DESTROY received, cleaning up");
            if let Some(hmenu) = user_data.hmenu.take()
                && unsafe { DestroyMenu(hmenu).is_err() }
            {
                error!("Failed to destroy menu during window cleanup");
            }
            let _ = unsafe { Box::from_raw(user_data_ptr as *mut TrayUserData) };
            LRESULT(0)
        }
        WM_USER_SET_MENU => {
            let menu_ptr = lparam.0;
            debug!("Received WM_USER_SET_MENU with menu_ptr={:?}", menu_ptr);
            user_data.hmenu = if menu_ptr == 0 {
                None
            } else {
                Some(HMENU(menu_ptr as *mut _))
            };
            LRESULT(0)
        }
        WM_USER_DESTROY_MENU => {
            debug!("Received WM_USER_DESTROY_MENU");
            if let Some(hmenu) = user_data.hmenu.take() {
                if unsafe { DestroyMenu(hmenu).is_err() } {
                    error!("Failed to destroy menu");
                } else {
                    debug!("Menu destroyed successfully");
                }
            }
            LRESULT(0)
        }
        WM_USER_TRAYICON => {
            let event = lparam.0 as u32;
            let mut pos = POINT { x: 0, y: 0 };
            let has_pos = unsafe { GetCursorPos(&mut pos).is_ok() };
            let position = gpui::Point::new(pos.x as f32, pos.y as f32);
            match event {
                WM_LBUTTONDOWN => {
                    debug!(
                        "Received WM_LBUTTONDOWN with position=({}, {})",
                        pos.x, pos.y
                    );
                    if has_pos {
                        dispatch_click(MouseButton::Left, position);
                    }
                }
                WM_LBUTTONDBLCLK => {
                    debug!(
                        "Received WM_LBUTTONDBLCLK with position=({}, {})",
                        pos.x, pos.y
                    );
                    dispatch_double_click();
                }
                WM_MBUTTONUP => {
                    debug!("Received WM_MBUTTONUP with position=({}, {})", pos.x, pos.y);
                    if has_pos {
                        dispatch_click(MouseButton::Middle, position);
                    }
                }
                WM_RBUTTONUP => {
                    debug!("Received WM_RBUTTONUP with position=({}, {})", pos.x, pos.y);
                    if has_pos {
                        dispatch_click(MouseButton::Right, position);
                    }
                    if let Some(hmenu) = user_data.hmenu {
                        show_tray_menu(hwnd, hmenu);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let command_id = wparam.0 as u32;
            debug!("Received WM_COMMAND with id={}", command_id);
            dispatch_menu_action(command_id);
            LRESULT(0)
        }
        _ => {
            let taskbar_restart = crate::tray::taskbar_restart_message();
            if msg == taskbar_restart {
                debug!("Received TaskbarCreated message, re-registering tray icon");
                unsafe { SendMessageW(hwnd, WM_USER_TRAYICON, Some(WPARAM(0)), Some(LPARAM(0))) };
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

fn register_platform_tray_class() -> Result<(), &'static str> {
    if CLASS_REGISTERED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_procedure),
            lpszClassName: PLATFORM_TRAY_CLASS_NAME,
            ..Default::default()
        };

        let result = unsafe { RegisterClassW(&wc) };
        if result == 0 {
            error!("Failed to register window class");
            CLASS_REGISTERED.store(false, Ordering::SeqCst);
            return Err("Failed to register window class");
        }
        unsafe { CLASS_ATOM = result };
        debug!("Window class registered successfully, atom: {}", result);
    }
    Ok(())
}

pub(crate) fn unregister_tray_class() {
    if unsafe { CLASS_ATOM } != 0 {
        let result = unsafe { UnregisterClassW(PCWSTR(CLASS_ATOM as usize as *const u16), None) };
        if result.is_ok() {
            debug!("Window class unregistered successfully");
            unsafe { CLASS_ATOM = 0 };
            CLASS_REGISTERED.store(false, Ordering::SeqCst);
        } else {
            error!("Failed to unregister window class");
        }
    }
}

pub(crate) fn create_tray_window() -> Result<HWND, &'static str> {
    register_platform_tray_class()?;

    let hwnd_result = unsafe {
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

    match hwnd_result {
        Ok(h) => {
            debug!("Created tray window successfully: {:?}", h);
            Ok(h)
        }
        Err(e) => {
            error!("Failed to create tray window: {:?}", e);
            Err("Failed to create tray window")
        }
    }
}

struct MenuHandle(HMENU);

impl Drop for MenuHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DestroyMenu(self.0);
            }
        }
    }
}

/// Type alias for menu build result.
pub(crate) type MenuBuildResult = Option<(HMENU, Vec<(u32, Box<dyn gpui::Action>)>)>;

pub(crate) unsafe fn build_menu(items: &[GpuiMenuItem]) -> MenuBuildResult {
    let hmenu = unsafe { CreatePopupMenu().ok() }?;
    let mut actions = Vec::new();
    (unsafe { build_menu_items(hmenu, items, 0, &mut actions).ok() })?;
    Some((hmenu, actions))
}

unsafe fn build_menu_items(
    hmenu: HMENU,
    items: &[GpuiMenuItem],
    start_id: u32,
    actions: &mut Vec<(u32, Box<dyn gpui::Action>)>,
) -> Result<u32, ()> {
    let mut current_id = start_id;

    for item in items {
        match item {
            GpuiMenuItem::Separator => {
                if unsafe { AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).is_err() } {
                    error!("Failed to append separator");
                }
            }
            GpuiMenuItem::Action { name, action, .. } => {
                current_id = current_id.checked_add(1).ok_or(())?;
                let wide_name = encode_wide(name.as_ref());
                let result = unsafe {
                    AppendMenuW(
                        hmenu,
                        MF_STRING,
                        current_id as usize,
                        PCWSTR(wide_name.as_ptr()),
                    )
                };
                if result.is_err() {
                    error!("Failed to append menu item: {}", name);
                } else {
                    actions.push((current_id, action.boxed_clone()));
                }
            }
            GpuiMenuItem::Submenu(submenu) => {
                let submenu_handle = unsafe { CreatePopupMenu().map_err(|_| ()) }?;
                let _guard = MenuHandle(submenu_handle);

                let next_id = unsafe {
                    build_menu_items(submenu_handle, &submenu.items, current_id, actions)
                }?;
                current_id = next_id;

                let wide_name = encode_wide(submenu.name.as_ref());
                let result = unsafe {
                    AppendMenuW(
                        hmenu,
                        MF_POPUP,
                        submenu_handle.0 as usize,
                        PCWSTR(wide_name.as_ptr()),
                    )
                };

                if result.is_err() {
                    error!("Failed to append submenu: {}", submenu.name);
                    return Err(());
                }

                std::mem::forget(_guard);
            }
            _ => {}
        }
    }

    Ok(current_id)
}

fn show_tray_menu(hwnd: HWND, hmenu: HMENU) {
    let mut cursor_pos = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut cursor_pos).is_ok() } {
        let _ = unsafe { SetForegroundWindow(hwnd) };
        let result = unsafe {
            TrackPopupMenu(
                hmenu,
                TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                cursor_pos.x,
                cursor_pos.y,
                Some(0),
                hwnd,
                None,
            )
        };
        debug!("TrackPopupMenu result: {:?}", result);
        let _ = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };
    }
}

pub(crate) fn set_window_menu(hwnd: HWND, hmenu: Option<HMENU>) {
    if hwnd.is_invalid() {
        error!("Attempted to set menu on invalid window");
        return;
    }

    let menu_ptr = hmenu.map(|h| h.0 as isize).unwrap_or(0);
    debug!("Sending WM_USER_SET_MENU with menu_ptr: {:?}", menu_ptr);
    unsafe {
        SendMessageW(
            hwnd,
            WM_USER_SET_MENU,
            Some(WPARAM(0)),
            Some(LPARAM(menu_ptr)),
        )
    };
}

pub(crate) fn destroy_window_menu(hwnd: HWND) {
    if hwnd.is_invalid() {
        return;
    }

    debug!("Sending WM_USER_DESTROY_MENU");
    unsafe { SendMessageW(hwnd, WM_USER_DESTROY_MENU, Some(WPARAM(0)), Some(LPARAM(0))) };
}

pub(crate) fn destroy_window(hwnd: HWND) -> bool {
    if hwnd.is_invalid() {
        debug!("Window already invalid, skipping destroy");
        return true;
    }

    match unsafe { DestroyWindow(hwnd) } {
        Ok(_) => {
            debug!("Window destroyed successfully");
            true
        }
        Err(e) => {
            error!("Failed to destroy window: {:?}", e);
            false
        }
    }
}
