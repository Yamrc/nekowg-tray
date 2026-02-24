//! GPUI System Tray - Cross-platform system tray integration for GPUI.
//!
//! This crate provides a unified API for creating and managing system tray icons
//! across Windows, macOS, and Linux platforms.
//!
//! # Quick Start
//!
//! ```rust
//! use gpui_tray::{Tray, TrayAppContext};
//!
//! // Set a tray icon
//! cx.set_tray(
//!     Tray::new()
//!         .tooltip("My Application")
//!         .icon(image)
//! );
//!
//! // Update the tray
//! cx.update_tray(|tray| {
//!     tray.tooltip = Some("Updated".into());
//! });
//!
//! // Remove the tray
//! cx.remove_tray();
//! ```

pub use gpui_tray_core::*;

mod manager;

pub use manager::TrayAppContext;
