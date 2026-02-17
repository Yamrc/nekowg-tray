//! macOS tray implementation
//!
//! Placeholder implementation for macOS platform.
//! TODO: Implement NSStatusBar support.

use gpui::{App, MenuItem as GpuiMenuItem, SharedString};

/// Platform tray trait
pub trait PlatformTray: 'static {
    fn set_tray(&mut self, app: &mut App, config: MacosTrayConfig);
    fn hide(&mut self);
    fn show(&mut self, app: &mut App);
    fn destroy(&mut self);
}

/// macOS tray configuration
pub struct MacosTrayConfig {
    pub tooltip: Option<SharedString>,
    pub visible: bool,
    pub menu_items: Option<Vec<GpuiMenuItem>>,
}

/// macOS tray implementation (placeholder)
pub struct MacosTray;

impl MacosTray {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformTray for MacosTray {
    fn set_tray(&mut self, _app: &mut App, _config: MacosTrayConfig) {
        log::warn!("macOS tray not yet implemented");
    }

    fn hide(&mut self) {
        log::warn!("macOS tray hide not yet implemented");
    }

    fn show(&mut self, _app: &mut App) {
        log::warn!("macOS tray show not yet implemented");
    }

    fn destroy(&mut self) {
        log::warn!("macOS tray destroy not yet implemented");
    }
}

impl Default for MacosTray {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_tray_new() {
        let tray = MacosTray::new();
        // Just verify it can be created
        assert!(true);
    }
}
