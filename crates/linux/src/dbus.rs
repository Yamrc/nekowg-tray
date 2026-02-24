use gpui::{Action, MouseButton, Point};
use std::cell::Cell;
use std::sync::Arc;
use zbus::blocking::Connection;

use crate::icon::Icon;
use crate::menu::{DBusMenu, menu_path};
use crate::sni::{StatusNotifierItem, item_path};

const STATUS_NOTIFIER_WATCHER: &str = "org.kde.StatusNotifierWatcher";
const STATUS_NOTIFIER_WATCHER_PATH: &str = "/StatusNotifierWatcher";

pub trait TrayEventDispatcher: Send + Sync + 'static {
    fn dispatch_click(&self, button: MouseButton, position: Point<f32>);
    fn dispatch_double_click(&self);
    fn dispatch_menu_action(&self, action: Box<dyn Action>);
}

thread_local! {
    static DISPATCHER: Cell<Option<&'static dyn TrayEventDispatcher>> = Cell::new(None);
}

pub fn set_dispatcher(dispatcher: Option<&'static dyn TrayEventDispatcher>) {
    DISPATCHER.set(dispatcher);
}

fn dispatch_click(button: MouseButton, position: Point<f32>) {
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

pub struct DbusService {
    _connection: Arc<Connection>,
}

impl DbusService {
    pub fn new(
        title: impl Into<String>,
        tooltip: Option<String>,
        icon: Option<Icon>,
        menu: DBusMenu,
    ) -> Result<Self, zbus::Error> {
        let service_name = format!(
            "org.freedesktop.StatusNotifierItem-{}-0",
            std::process::id()
        );

        let connection = Arc::new(Connection::session()?);
        connection.request_name(service_name.as_str())?;

        let item = StatusNotifierItem::new(title, tooltip, icon);

        let item_path = item_path();
        let menu_path = menu_path();

        connection.object_server().at(item_path, item)?;
        connection.object_server().at(menu_path, menu)?;

        Self::register_with_watcher(&connection, &service_name)?;

        Ok(Self { _connection: connection })
    }

    fn register_with_watcher(
        connection: &Connection,
        service_name: &str,
    ) -> Result<(), zbus::Error> {
        let proxy = zbus::blocking::Proxy::new(
            connection,
            STATUS_NOTIFIER_WATCHER,
            STATUS_NOTIFIER_WATCHER_PATH,
            STATUS_NOTIFIER_WATCHER,
        )?;

        proxy.call_method("RegisterStatusNotifierItem", &(service_name,))?;

        Ok(())
    }
}

pub struct EventLoop;

impl EventLoop {
    pub fn new() -> Self {
        Self
    }

    pub fn process_events(&self) {
        // Event processing is handled by zbus in the background
        // This method is called periodically to dispatch any pending events
    }
}
