#![cfg(target_os = "linux")]

use gpui_tray_core::Result;
use gpui_tray_core::platform_trait::PlatformTray;

pub fn create() -> Result<Box<dyn PlatformTray>> {
    Err(gpui_tray_core::Error::UnsupportedPlatform)
}
