use gpui::App;
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{Error, Result, Tray};
use log::{debug, error, info};
use std::cell::RefCell;

use crate::dbus::DbusService;
use crate::icon::Icon;
use crate::menu::DBusMenu;

pub struct LinuxTray {
    dbus_service: RefCell<Option<DbusService>>,
    current_tray: RefCell<Option<Tray>>,
    registered: RefCell<bool>,
}

impl LinuxTray {
    pub(crate) fn new() -> Self {
        Self {
            dbus_service: RefCell::new(None),
            current_tray: RefCell::new(None),
            registered: RefCell::new(false),
        }
    }

    fn create_tray(&self, tray: &Tray) -> Result<()> {
        let tooltip = tray.tooltip.as_ref().map(|t| t.to_string());
        let icon = tray.icon.as_ref().and_then(|img| {
            match Icon::from_image(img) {
                Ok(icon) => {
                    debug!("Icon created with {} pixmaps", icon.as_pixmaps().len());
                    Some(icon)
                }
                Err(e) => {
                    error!("Failed to create icon: {:?}", e);
                    None
                }
            }
        });

        let menu = DBusMenu::new();

        match DbusService::new("GPUI Tray", tooltip, icon, menu) {
            Ok(service) => {
                *self.dbus_service.borrow_mut() = Some(service);
                debug!("D-Bus service created with initial icon");
            }
            Err(e) => {
                error!("Failed to create D-Bus service: {}", e);
                return Err(Error::Platform(e.to_string()));
            }
        }

        Ok(())
    }

    fn build_menu_from_tray(&self, _cx: &mut App, tray: &Tray) {
        if let Some(builder) = &tray.menu_builder {
            let _items = builder(_cx);
        }
    }
}

impl PlatformTray for LinuxTray {
    fn set_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        debug!("set_tray: visible={}", tray.visible);

        if !tray.visible {
            *self.dbus_service.borrow_mut() = None;
            *self.registered.borrow_mut() = false;
            *self.current_tray.borrow_mut() = None;
            info!("Tray hidden");
            return Ok(());
        }

        if self.dbus_service.borrow().is_none() {
            self.create_tray(tray)?;
            self.build_menu_from_tray(cx, tray);
        }

        *self.registered.borrow_mut() = true;
        *self.current_tray.borrow_mut() = Some(tray.clone());

        info!("Tray set successfully");
        Ok(())
    }

    fn update_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()> {
        debug!("update_tray");
        self.set_tray(cx, tray)
    }

    fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
        debug!("remove_tray");

        *self.dbus_service.borrow_mut() = None;
        *self.registered.borrow_mut() = false;
        *self.current_tray.borrow_mut() = None;

        info!("Tray removed");
        Ok(())
    }
}

impl Drop for LinuxTray {
    fn drop(&mut self) {
        debug!("Dropping LinuxTray");
        *self.dbus_service.borrow_mut() = None;
    }
}

pub fn create() -> Result<Box<dyn PlatformTray>> {
    debug!("Creating Linux tray implementation");
    Ok(Box::new(LinuxTray::new()))
}
