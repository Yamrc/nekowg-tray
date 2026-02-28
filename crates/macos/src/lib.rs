#![cfg(target_os = "macos")]

use gpui_tray_core::Result;
use gpui_tray_core::platform_trait::PlatformTray;
use log::warn;

use crate::stub::MacosTrayStub;

mod stub;

pub fn create() -> Result<Box<dyn PlatformTray>> {
    warn!("Creating macOS tray stub implementation.");
    Ok(Box::new(MacosTrayStub::new()))
}
