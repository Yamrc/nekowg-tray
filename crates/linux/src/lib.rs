#![cfg(target_os = "linux")]

mod dbus;
mod icon;
mod menu;
mod sni;
mod tray;

use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::Result;

pub use dbus::{TrayEventDispatcher, set_dispatcher};
pub use tray::LinuxTray;

pub fn create() -> Result<Box<dyn PlatformTray>> {
    tray::create()
}
