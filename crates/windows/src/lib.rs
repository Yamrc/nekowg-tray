#![cfg(target_os = "windows")]

//! Windows system tray implementation for GPUI.
//!
//! This crate provides native Windows system tray functionality using the
//! Windows Shell API (Shell_NotifyIconW).

mod icon;
mod tray;

use gpui_tray_core::Result;
use gpui_tray_core::platform_trait::PlatformTray;

/// Creates a new Windows platform tray implementation.
pub fn create() -> Result<Box<dyn PlatformTray>> {
    tray::create()
}
