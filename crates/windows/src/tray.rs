use crate::icon::{DecodedIcon, OwnedIcon, create_hicon, decode_icon};
use gpui::{Action, MenuItem, MouseButton, Point};
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{
    BackendError, ClickEvent, DoubleClickEvent, Error, Result, RuntimeEvent, Tray,
};
use log::debug;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::os::windows::ffi::OsStrExt;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, TRUE, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HMENU, HWND_MESSAGE,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, PM_REMOVE, PeekMessageW, PostMessageW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, SetWindowLongPtrW, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TrackPopupMenu, TranslateMessage, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
    WM_COMMAND, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_MBUTTONUP, WM_NCCREATE, WM_NULL, WM_RBUTTONUP,
    WNDCLASSW,
};
use windows::core::PCWSTR;

const WM_TRAYICON: u32 = WM_APP + 71;
const TRAY_CLASS_NAME: &str = "GPUI::Tray::VNext";
const TRAY_ID: u32 = 1;

enum BackendCommand {
    SetTray {
        tray: Tray,
        response: Sender<Result<()>>,
    },
    RemoveTray {
        response: Sender<Result<()>>,
    },
    IconDecoded {
        revision: u64,
        icon_key: u64,
        decoded: Result<DecodedIcon>,
    },
    Shutdown,
}

struct OwnedMenu(HMENU);

impl Drop for OwnedMenu {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DestroyMenu(self.0);
            }
        }
    }
}

struct TrayWindowState {
    event_tx: Sender<RuntimeEvent>,
    command_tx: Sender<BackendCommand>,
    current_tray: Option<Tray>,
    current_icon: Option<OwnedIcon>,
    current_menu: Option<OwnedMenu>,
    menu_actions: HashMap<u16, Box<dyn Action>>,
    registered: bool,
    requested_icon_revision: u64,
    current_icon_key: Option<u64>,
    taskbar_restart_msg: u32,
}

impl TrayWindowState {
    fn new(event_tx: Sender<RuntimeEvent>, command_tx: Sender<BackendCommand>) -> Self {
        Self {
            event_tx,
            command_tx,
            current_tray: None,
            current_icon: None,
            current_menu: None,
            menu_actions: HashMap::new(),
            registered: false,
            requested_icon_revision: 0,
            current_icon_key: None,
            taskbar_restart_msg: unsafe {
                RegisterWindowMessageW(windows::core::w!("TaskbarCreated"))
            },
        }
    }

    fn clear_menu(&mut self) {
        self.current_menu.take();
        self.menu_actions.clear();
    }
}

pub(crate) struct WindowsBackend {
    command_tx: Sender<BackendCommand>,
    event_rx: Mutex<Receiver<RuntimeEvent>>,
}

impl WindowsBackend {
    fn send_and_wait(&self, cmd: impl FnOnce(Sender<Result<()>>) -> BackendCommand) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.command_tx
            .send(cmd(tx))
            .map_err(|_| Error::Backend(BackendError::ChannelSend))?;
        rx.recv()
            .map_err(|_| Error::Backend(BackendError::ChannelReceive))?
    }
}

impl PlatformTray for WindowsBackend {
    fn set_tray(&self, tray: Tray) -> Result<()> {
        debug!(
            "set_tray requested, visible={}, tooltip={:?}, has_icon={}, has_menu={}",
            tray.visible,
            tray.tooltip,
            tray.icon.is_some(),
            tray.menu_builder.is_some()
        );
        self.send_and_wait(|response| BackendCommand::SetTray { tray, response })
    }

    fn remove_tray(&self) -> Result<()> {
        self.send_and_wait(|response| BackendCommand::RemoveTray { response })
    }

    fn try_recv_event(&self) -> Result<Option<RuntimeEvent>> {
        let rx = self.event_rx.lock().map_err(|_| Error::RuntimeClosed)?;
        match rx.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(Error::RuntimeClosed),
        }
    }

    fn shutdown(&self) -> Result<()> {
        if self.command_tx.send(BackendCommand::Shutdown).is_err() {
            return Err(Error::RuntimeClosed);
        }
        Ok(())
    }
}

pub fn create() -> Result<Box<dyn PlatformTray>> {
    let (command_tx, command_rx) = mpsc::channel::<BackendCommand>();
    let (event_tx, event_rx) = mpsc::channel::<RuntimeEvent>();
    let (boot_tx, boot_rx) = mpsc::channel::<Result<()>>();

    let thread_command_tx = command_tx.clone();
    thread::Builder::new()
        .name("gpui-tray-windows".to_string())
        .spawn(move || {
            backend_thread_main(command_rx, thread_command_tx, event_tx, boot_tx);
        })
        .map_err(|err| Error::Backend(BackendError::platform("spawn", err.to_string())))?;

    boot_rx
        .recv()
        .map_err(|_| Error::Backend(BackendError::ChannelReceive))??;

    Ok(Box::new(WindowsBackend {
        command_tx,
        event_rx: Mutex::new(event_rx),
    }))
}

fn backend_thread_main(
    command_rx: Receiver<BackendCommand>,
    command_tx: Sender<BackendCommand>,
    event_tx: Sender<RuntimeEvent>,
    boot_tx: Sender<Result<()>>,
) {
    let class_name = encode_wide(TRAY_CLASS_NAME);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        let _ = boot_tx.send(Err(BackendError::platform(
            "RegisterClassW",
            "returned atom=0",
        )
        .into()));
        return;
    }

    let mut state = Box::new(TrayWindowState::new(event_tx, command_tx));
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            None,
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            Some(state.as_mut() as *mut TrayWindowState as *const _),
        )
    };

    let hwnd = match hwnd {
        Ok(hwnd) => hwnd,
        Err(err) => {
            debug!("CreateWindowExW failed: {err:?}");
            let _ = boot_tx.send(Err(BackendError::platform(
                "CreateWindowExW",
                format!("{err:?}"),
            )
            .into()));
            unsafe {
                let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), None);
            }
            return;
        }
    };

    let _ = boot_tx.send(Ok(()));

    let mut running = true;
    while running {
        process_window_messages();

        match command_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(cmd) => {
                running = handle_command(hwnd, state.as_mut(), cmd);
                while let Ok(cmd) = command_rx.try_recv() {
                    if !handle_command(hwnd, state.as_mut(), cmd) {
                        running = false;
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                running = false;
            }
        }
    }

    cleanup(hwnd, state.as_mut());
    unsafe {
        let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), None);
    }
}

fn process_window_messages() {
    let mut msg = MSG::default();
    while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() } {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn handle_command(hwnd: HWND, state: &mut TrayWindowState, cmd: BackendCommand) -> bool {
    match cmd {
        BackendCommand::SetTray { tray, response } => {
            debug!(
                "SetTray visible={}, has_icon={}, has_menu={}",
                tray.visible,
                tray.icon.is_some(),
                tray.menu_builder.is_some()
            );
            let result = apply_tray_snapshot(hwnd, state, tray.clone());
            if result.is_ok() {
                schedule_icon_decode(state, tray);
            }
            let _ = response.send(result);
            true
        }
        BackendCommand::RemoveTray { response } => {
            state.current_tray = None;
            state.requested_icon_revision = state.requested_icon_revision.saturating_add(1);
            remove_tray_icon(hwnd, state);
            state.current_icon = None;
            state.clear_menu();
            let _ = response.send(Ok(()));
            true
        }
        BackendCommand::IconDecoded {
            revision,
            icon_key,
            decoded,
        } => {
            debug!(
                "IconDecoded revision={} key={} (requested={})",
                revision, icon_key, state.requested_icon_revision
            );
            if revision != state.requested_icon_revision {
                debug!(
                    "decoded icon ignored: stale revision={} requested={}",
                    revision, state.requested_icon_revision
                );
                return true;
            }

            let Some(tray) = state.current_tray.as_ref() else {
                return true;
            };

            if !tray.visible {
                debug!(
                    "decoded icon ignored: tray hidden revision={} key={}",
                    revision, icon_key
                );
                return true;
            }

            match decoded {
                Ok(decoded) => match create_hicon(&decoded) {
                    Ok(icon) => {
                        debug!(
                            "applying decoded icon revision={} key={}",
                            revision, icon_key
                        );
                        state.current_icon = Some(icon);
                        state.current_icon_key = Some(icon_key);
                        if let Err(err) = add_or_update_icon(hwnd, state, false) {
                            log::error!("failed to apply decoded icon: {err}");
                        }
                    }
                    Err(err) => {
                        log::error!("failed to create icon handle: {err}");
                    }
                },
                Err(err) => {
                    log::error!("failed to decode tray icon: {err}");
                }
            }
            true
        }
        BackendCommand::Shutdown => false,
    }
}

fn schedule_icon_decode(state: &mut TrayWindowState, tray: Tray) {
    if let Some(image) = tray.icon {
        state.requested_icon_revision = state.requested_icon_revision.saturating_add(1);
        let revision = state.requested_icon_revision;
        let icon_key = image_key(&image);
        debug!(
            "schedule icon decode revision={} key={}",
            revision, icon_key
        );

        if state.current_icon_key == Some(icon_key) && state.current_icon.is_some() {
            debug!("icon decode skipped: unchanged key={}", icon_key);
            return;
        }

        let tx = state.command_tx.clone();
        thread::spawn(move || {
            let start = std::time::Instant::now();
            let decoded = decode_icon(&image);
            debug!(
                "windows backend decode thread: revision={} key={} done in {:?}",
                revision,
                icon_key,
                start.elapsed()
            );
            let _ = tx.send(BackendCommand::IconDecoded {
                revision,
                icon_key,
                decoded,
            });
        });
    } else {
        state.requested_icon_revision = state.requested_icon_revision.saturating_add(1);
        state.current_icon_key = None;
    }
}

fn apply_tray_snapshot(hwnd: HWND, state: &mut TrayWindowState, tray: Tray) -> Result<()> {
    debug!(
        "apply snapshot visible={}, tooltip={:?}",
        tray.visible, tray.tooltip
    );
    state.current_tray = Some(tray.clone());
    state.clear_menu();

    if !tray.visible {
        remove_tray_icon(hwnd, state);
        state.current_icon = None;
        state.current_icon_key = None;
        return Ok(());
    }

    if tray.icon.is_none() {
        state.current_icon = None;
        state.current_icon_key = None;
    }

    add_or_update_icon(hwnd, state, false)?;
    Ok(())
}

fn add_or_update_icon(hwnd: HWND, state: &mut TrayWindowState, force_add: bool) -> Result<()> {
    let Some(tray) = state.current_tray.as_ref() else {
        return Err(Error::NotFound);
    };

    let mut tip = [0u16; 128];
    if let Some(tooltip) = &tray.tooltip {
        for (index, ch) in encode_wide(tooltip.as_ref())
            .into_iter()
            .take(127)
            .enumerate()
        {
            tip[index] = ch;
        }
    }

    let hicon = state
        .current_icon
        .as_ref()
        .map(|icon| icon.0)
        .unwrap_or_default();
    let flags = NIF_MESSAGE | NIF_TIP | NIF_ICON;
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        uFlags: flags,
        uCallbackMessage: WM_TRAYICON,
        hIcon: hicon,
        szTip: tip,
        ..unsafe { std::mem::zeroed() }
    };

    let op = if force_add || !state.registered {
        NIM_ADD
    } else {
        NIM_MODIFY
    };
    debug!(
        "Shell_NotifyIconW op={:?}, force_add={}, registered={}, has_hicon={}",
        op,
        force_add,
        state.registered,
        !hicon.is_invalid()
    );

    let result = unsafe { Shell_NotifyIconW(op, &nid) };
    if result != TRUE {
        return Err(BackendError::platform(
            "Shell_NotifyIconW",
            format!("operation {op:?} failed"),
        )
        .into());
    }

    state.registered = true;
    Ok(())
}

fn remove_tray_icon(hwnd: HWND, state: &mut TrayWindowState) {
    if !state.registered {
        return;
    }

    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        ..unsafe { std::mem::zeroed() }
    };
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
    state.registered = false;
}

fn cleanup(hwnd: HWND, state: &mut TrayWindowState) {
    remove_tray_icon(hwnd, state);
    state.current_icon = None;
    state.clear_menu();

    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = unsafe {
            &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW)
        };
        let ptr = create.lpCreateParams as *mut TrayWindowState;
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize) };
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut TrayWindowState;
    if ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let state = unsafe { &mut *ptr };

    match msg {
        WM_TRAYICON => {
            let event = lparam.0 as u32;
            match event {
                WM_LBUTTONUP => {
                    debug!("WM_TRAYICON event=WM_LBUTTONUP");
                    dispatch_click(state, MouseButton::Left)
                }
                WM_MBUTTONUP => {
                    debug!("WM_TRAYICON event=WM_MBUTTONUP");
                    dispatch_click(state, MouseButton::Middle)
                }
                WM_RBUTTONUP => {
                    debug!("WM_TRAYICON event=WM_RBUTTONUP");
                    dispatch_click(state, MouseButton::Right);
                    show_context_menu(hwnd, state);
                }
                WM_LBUTTONDBLCLK => {
                    debug!("WM_TRAYICON event=WM_LBUTTONDBLCLK");
                    let _ = state
                        .event_tx
                        .send(RuntimeEvent::Action(Box::new(DoubleClickEvent)));
                }
                _ => {}
            }
            return LRESULT(0);
        }
        WM_COMMAND => {
            let action_id = (wparam.0 & 0xFFFF) as u16;
            debug!("WM_COMMAND action_id={action_id}");
            if let Some(action) = state.menu_actions.get(&action_id) {
                let _ = state
                    .event_tx
                    .send(RuntimeEvent::Action(action.boxed_clone()));
            }
            return LRESULT(0);
        }
        _ => {
            if msg == state.taskbar_restart_msg && state.current_tray.is_some() {
                debug!("taskbar restart detected, re-registering tray");
                let _ = add_or_update_icon(hwnd, state, true);
                return LRESULT(0);
            }
        }
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn dispatch_click(state: &TrayWindowState, button: MouseButton) {
    let mut pos = POINT::default();
    let _ = unsafe { GetCursorPos(&mut pos) };
    let event = ClickEvent {
        button,
        position: Point::new(pos.x as f32, pos.y as f32),
    };
    debug!(
        "dispatch click button={:?} pos=({}, {})",
        button, pos.x, pos.y
    );
    let _ = state.event_tx.send(RuntimeEvent::Action(Box::new(event)));
}

fn show_context_menu(hwnd: HWND, state: &mut TrayWindowState) {
    let Some(tray) = state.current_tray.as_ref() else {
        return;
    };
    let Some(builder) = tray.menu_builder.as_ref() else {
        return;
    };

    let items = builder();
    debug!("rebuild menu lazily, items={}", items.len());
    if items.is_empty() {
        return;
    }

    let mut next_id: u16 = 0;
    let mut actions = HashMap::new();
    let Some(menu) = build_menu(&items, &mut next_id, &mut actions) else {
        return;
    };

    state.current_menu = Some(OwnedMenu(menu));
    state.menu_actions = actions;
    debug!("popup menu ready, actions={}", state.menu_actions.len());

    let mut cursor = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor) };
    unsafe {
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            cursor.x,
            cursor.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
}

fn build_menu(
    items: &[MenuItem],
    next_id: &mut u16,
    actions: &mut HashMap<u16, Box<dyn Action>>,
) -> Option<HMENU> {
    let menu = unsafe { CreatePopupMenu().ok()? };

    for item in items {
        match item {
            MenuItem::Separator => unsafe {
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            },
            MenuItem::Action { name, action, .. } => {
                *next_id = next_id.saturating_add(1);
                let id = *next_id;
                let wide = encode_wide(name.as_ref());
                let result =
                    unsafe { AppendMenuW(menu, MF_STRING, id as usize, PCWSTR(wide.as_ptr())) };
                if result.is_ok() {
                    actions.insert(id, action.boxed_clone());
                }
            }
            MenuItem::Submenu(submenu) => {
                if let Some(sub) = build_menu(&submenu.items, next_id, actions) {
                    let wide = encode_wide(submenu.name.as_ref());
                    let _ = unsafe {
                        AppendMenuW(menu, MF_POPUP, sub.0 as usize, PCWSTR(wide.as_ptr()))
                    };
                }
            }
            _ => {}
        }
    }

    Some(menu)
}

fn encode_wide<S: AsRef<OsStr>>(s: S) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

fn image_key(image: &gpui::Image) -> u64 {
    let mut hasher = DefaultHasher::new();
    image.bytes.hash(&mut hasher);
    hasher.finish()
}
