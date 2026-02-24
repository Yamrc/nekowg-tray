use std::sync::{Arc, Mutex};
use zbus::{interface, zvariant::ObjectPath};

use crate::icon::Icon;

const STATUS_NOTIFIER_ITEM_PATH: &str = "/StatusNotifierItem";
const DBUS_MENU_PATH: &str = "/MenuBar";

#[derive(Clone)]
pub struct StatusNotifierItem {
    inner: Arc<Mutex<StatusNotifierItemInner>>,
}

struct StatusNotifierItemInner {
    title: String,
    tooltip: String,
    icon: Option<Icon>,
}

impl StatusNotifierItem {
    pub fn new(title: impl Into<String>, tooltip: Option<String>, icon: Option<Icon>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StatusNotifierItemInner {
                title: title.into(),
                tooltip: tooltip.unwrap_or_default(),
                icon,
            })),
        }
    }

    pub fn set_tooltip(&self, tooltip: impl Into<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.tooltip = tooltip.into();
        }
    }

    pub fn set_icon(&self, icon: Icon) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.icon = Some(icon);
        }
    }
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[zbus(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }

    #[zbus(property)]
    fn id(&self) -> String {
        self.inner
            .lock()
            .map(|inner| inner.title.clone())
            .unwrap_or_default()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        self.inner
            .lock()
            .map(|inner| inner.title.clone())
            .unwrap_or_default()
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }

    #[zbus(property, name = "IconName")]
    fn icon_name(&self) -> &str {
        ""
    }

    #[zbus(property, name = "IconPixmap")]
    fn icon_pixmap(&self) -> Vec<crate::icon::Pixmap> {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return Vec::new(),
        };

        match &inner.icon {
            Some(icon) => icon.as_pixmaps().to_vec(),
            None => Vec::new(),
        }
    }

    #[zbus(property)]
    fn tooltip(&self) -> (String, Vec<(String, String, String)>, String, String) {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return (String::new(), Vec::new(), String::new(), String::new()),
        };

        (
            inner.tooltip.clone(),
            Vec::new(),
            String::new(),
            String::new(),
        )
    }

    #[zbus(property)]
    fn menu(&self) -> ObjectPath<'_> {
        ObjectPath::from_static_str(DBUS_MENU_PATH)
            .unwrap_or_else(|_| ObjectPath::from_static_str("/").expect("fallback path is valid"))
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property, name = "WindowId")]
    fn window_id(&self) -> i32 {
        0
    }

    fn activate(&self, _x: i32, _y: i32) {}

    fn secondary_activate(&self, _x: i32, _y: i32) {}

    fn context_menu(&self, _x: i32, _y: i32) {}

    fn scroll(&self, _delta: i32, _orientation: &str) {}
}

pub fn item_path() -> &'static str {
    STATUS_NOTIFIER_ITEM_PATH
}
