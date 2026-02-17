//! Linux tray implementation
//!
//! Placeholder implementation for Linux platform.
//! TODO: Implement DBus StatusNotifierItem support.

use gpui::{App, MenuItem as GpuiMenuItem, SharedString};

/// Platform tray trait
pub trait PlatformTray: 'static {
    fn set_tray(&mut self, app: &mut App, config: LinuxTrayConfig);
    fn hide(&mut self);
    fn show(&mut self, app: &mut App);
    fn destroy(&mut self);
}

/// Linux tray configuration
pub struct LinuxTrayConfig {
    pub tooltip: Option<SharedString>,
    pub visible: bool,
    pub menu_items: Option<Vec<GpuiMenuItem>>,
}

/// Linux tray implementation (placeholder)
pub struct LinuxTray;

impl LinuxTray {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformTray for LinuxTray {
    fn set_tray(&mut self, _app: &mut App, _config: LinuxTrayConfig) {
        log::warn!("Linux tray not yet implemented");
    }

    fn hide(&mut self) {
        log::warn!("Linux tray hide not yet implemented");
    }

    fn show(&mut self, _app: &mut App) {
        log::warn!("Linux tray show not yet implemented");
    }

    fn destroy(&mut self) {
        log::warn!("Linux tray destroy not yet implemented");
    }
}

impl Default for LinuxTray {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_tray_new() {
        let tray = LinuxTray::new();
        // Just verify it can be created
        assert!(true);
    }
}
