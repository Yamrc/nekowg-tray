//! macOS system tray implementation
//!
//! This crate provides macOS-specific implementation for system tray.
//! It is used internally by gpui-tray and should not be used directly.

pub mod tray;

pub use tray::{MacosTray, MacosTrayConfig, PlatformTray};
