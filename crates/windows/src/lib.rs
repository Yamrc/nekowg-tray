mod tray;
mod util;
mod window;

use gpui_tray_core::{PlatformTray, Result};
use log::debug;

pub use tray::{WindowsTray, taskbar_restart_message};
pub use window::{TrayEventDispatcher, set_dispatcher, set_menu_actions, unregister_tray_class};

pub fn create() -> Result<Box<dyn PlatformTray>> {
    debug!("Creating Windows tray implementation");
    tray::create()
}
