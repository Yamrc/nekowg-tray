use gpui::{Action, App, MouseButton, Point};
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{ClickEvent, Error, Result, Tray};
use log::{debug, error};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};

use crate::dbus::{DbusService, ItemState, MenuState, TrayEvent};
use crate::icon::{Icon, Pixmap};

static DISPATCHER_APP: AtomicPtr<App> = AtomicPtr::new(std::ptr::null_mut());
static DISPATCHER_ACTIVE: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn set_dispatcher_app(app: &mut App) {
    DISPATCHER_APP.store(app as *mut App, Ordering::SeqCst);
    DISPATCHER_ACTIVE.store(true, Ordering::SeqCst);
}

#[doc(hidden)]
pub fn clear_dispatcher_app() {
    DISPATCHER_ACTIVE.store(false, Ordering::SeqCst);
    DISPATCHER_APP.store(std::ptr::null_mut(), Ordering::SeqCst);
}

fn is_dispatcher_active() -> bool {
    DISPATCHER_ACTIVE.load(Ordering::SeqCst)
}

fn dispatch_click_linux(button: MouseButton, position: Point<f32>) {
    if !is_dispatcher_active() {
        return;
    }

    let app_ptr = DISPATCHER_APP.load(Ordering::SeqCst);
    if !app_ptr.is_null() {
        debug!(
            "Dispatching click: button={:?}, position={:?}",
            button, position
        );
        unsafe {
            let app = &mut *app_ptr;
            let event = ClickEvent { button, position };
            app.dispatch_action(&event);
        }
    }
}

fn dispatch_menu_action_linux(action: Box<dyn Action>) {
    if !is_dispatcher_active() {
        return;
    }

    let app_ptr = DISPATCHER_APP.load(Ordering::SeqCst);
    if !app_ptr.is_null() {
        debug!("Dispatching menu action");
        unsafe {
            let app = &mut *app_ptr;
            app.dispatch_action(action.as_ref());
        }
    }
}

pub(crate) struct LinuxTray {
    service: Option<DbusService>,
    item_state: Arc<Mutex<ItemState>>,
    menu_state: Arc<Mutex<MenuState>>,
    menu_actions: Arc<Mutex<HashMap<i32, Box<dyn gpui::Action>>>>,
    event_sender: Option<std::sync::mpsc::Sender<TrayEvent>>,
}

impl LinuxTray {
    pub(crate) fn new() -> Self {
        Self {
            service: None,
            item_state: Arc::new(Mutex::new(ItemState {
                title: String::new(),
                tooltip: String::new(),
                icon: None,
            })),
            menu_state: Arc::new(Mutex::new(MenuState::new())),
            menu_actions: Arc::new(Mutex::new(HashMap::new())),
            event_sender: None,
        }
    }

    fn update_tray_state(&mut self, tray: &Tray, cx: &mut App) -> Result<()> {
        if let Some(image) = &tray.icon {
            match Icon::from_image(image) {
                Ok(icon) => {
                    let pixmaps: Vec<Pixmap> = icon.as_pixmaps().to_vec();
                    self.item_state.lock().unwrap().icon = Some(pixmaps);
                }
                Err(e) => {
                    error!("Failed to create icon: {:?}", e);
                }
            }
        }

        self.item_state.lock().unwrap().tooltip = tray
            .tooltip
            .clone()
            .map(|s| s.to_string())
            .unwrap_or_default();
        self.item_state.lock().unwrap().title = tray
            .tooltip
            .clone()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "GPUI Tray".to_string());

        self.build_menu(tray, cx);

        if self.service.is_none() {
            let menu_actions = self.menu_actions.clone();
            let (event_sender, event_receiver) = std::sync::mpsc::channel();
            self.event_sender = Some(event_sender.clone());

            match DbusService::new(
                self.item_state.clone(),
                self.menu_state.clone(),
                event_sender,
            ) {
                Ok(service) => {
                    self.service = Some(service);

                    std::thread::spawn(move || {
                        loop {
                            match event_receiver.recv() {
                                Ok(event) => match event {
                                    TrayEvent::Activate { x, y } => {
                                        let position = Point::new(x as f32, y as f32);
                                        dispatch_click_linux(MouseButton::Left, position);
                                    }
                                    TrayEvent::SecondaryActivate { x, y } => {
                                        let position = Point::new(x as f32, y as f32);
                                        dispatch_click_linux(MouseButton::Middle, position);
                                    }
                                    TrayEvent::ContextMenu { x, y } => {
                                        let position = Point::new(x as f32, y as f32);
                                        dispatch_click_linux(MouseButton::Right, position);
                                    }
                                    TrayEvent::MenuClicked { id } => {
                                        if let Some(action) = menu_actions.lock().unwrap().get(&id)
                                        {
                                            dispatch_menu_action_linux(action.boxed_clone());
                                        }
                                    }
                                },
                                Err(_) => {
                                    debug!("Event receiver closed, stopping background thread");
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to create D-Bus service: {}", e);
                    return Err(Error::Platform(format!("D-Bus error: {}", e)));
                }
            }
        }

        Ok(())
    }

    fn build_menu(&mut self, tray: &Tray, cx: &mut App) {
        self.menu_state.lock().unwrap().clear();
        self.menu_actions.lock().unwrap().clear();

        if let Some(builder) = &tray.menu_builder {
            let items = builder(cx);
            for item in &items {
                self.add_menu_item(item, 0);
            }
        }

        debug!(
            "Menu built with {} actions",
            self.menu_actions.lock().unwrap().len()
        );
    }

    fn add_menu_item(&mut self, item: &gpui::MenuItem, parent_id: i32) -> i32 {
        match item {
            gpui::MenuItem::Separator => self.menu_state.lock().unwrap().add_separator(parent_id),
            gpui::MenuItem::Action { name, action, .. } => {
                let id = self
                    .menu_state
                    .lock()
                    .unwrap()
                    .add_item(name.to_string(), parent_id);
                self.menu_actions
                    .lock()
                    .unwrap()
                    .insert(id, action.boxed_clone());
                id
            }
            gpui::MenuItem::Submenu(submenu) => {
                let id = self
                    .menu_state
                    .lock()
                    .unwrap()
                    .add_item(submenu.name.to_string(), parent_id);
                for child in &submenu.items {
                    self.add_menu_item(child, id);
                }
                id
            }
            _ => 0,
        }
    }
}

impl PlatformTray for LinuxTray {
    fn set_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        if !tray.visible {
            if self.service.is_some() {
                self.service = None;
                self.event_sender = None;
            }
            self.item_state.lock().unwrap().icon = None;
            return Ok(());
        }

        self.update_tray_state(tray, cx)?;

        Ok(())
    }

    fn update_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        self.set_tray(cx, tray)
    }

    fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
        self.service = None;
        self.event_sender = None;

        self.item_state.lock().unwrap().icon = None;
        self.menu_state.lock().unwrap().clear();
        self.menu_actions.lock().unwrap().clear();

        Ok(())
    }
}

impl Drop for LinuxTray {
    fn drop(&mut self) {}
}

pub fn create() -> Result<Box<dyn PlatformTray>> {
    debug!("Creating Linux tray implementation");
    Ok(Box::new(LinuxTray::new()))
}
