//! Windows system tray implementation
//!
//! This crate provides Windows-specific implementation for system tray.
//! It is used internally by gpui-tray and should not be used directly by
//! application code.

pub mod tray;
mod util;
mod window;

pub use tray::{EventCallback, PlatformTray, WindowsTray, WindowsTrayConfig};
