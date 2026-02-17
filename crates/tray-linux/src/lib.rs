//! Linux system tray implementation
//!
//! This crate provides Linux-specific implementation for system tray.
//! It is used internally by gpui-tray and should not be used directly.

pub mod tray;

pub use tray::{LinuxTray, LinuxTrayConfig, PlatformTray};
