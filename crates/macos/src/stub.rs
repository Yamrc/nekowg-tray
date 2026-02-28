//! Stub implementation for macOS tray.
//!
//! This is a placeholder implementation that logs warnings.

use gpui::App;
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{Result, Tray};
use log::warn;

pub struct MacosTrayStub;

impl MacosTrayStub {
    pub fn new() -> Self {
        warn!("macOS tray is not fully implemented. Using stub implementation.");
        Self
    }
}

impl PlatformTray for MacosTrayStub {
    fn set_tray(&mut self, _cx: &mut App, _tray: &Tray) -> Result<()> {
            warn!("Tray::set_tray called but macOS tray is not implemented");
        Ok(())
    }

    fn update_tray(&mut self, _cx: &mut App, _tray: &Tray) -> Result<()> {
        warn!("Tray::update_tray called but macOS tray is not implemented");
        Ok(())
    }

    fn remove_tray(&mut self, _cx: &mut App) -> Result<()> {
        warn!("Tray::remove_tray called but macOS tray is not implemented");
        Ok(())
    }
}
