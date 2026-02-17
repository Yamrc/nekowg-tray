//! GPUI Cross-Platform System Tray Library
//!
//! This crate provides a unified API for system tray functionality
//! across Windows, Linux, and macOS platforms.
//!
//! # Example
//! ```rust,ignore
//! use gpui::*;
//! use gpui_tray::{Tray, AppTrayExt};
//!
//! fn main() {
//!     Application::new().run(|cx: &mut App| {
//!         let tray = Tray::new()
//!             .tooltip("My App")
//!             .visible(true)
//!             .menu(|_cx| vec![
//!                 MenuItem::action("Show", ShowAction),
//!                 MenuItem::separator(),
//!                 MenuItem::action("Quit", QuitAction),
//!             ]);
//!
//!         cx.set_tray(tray);
//!     });
//! }
//! ```

pub use gpui;

mod app_ext;
mod events;
mod platform;
mod types;

pub use app_ext::AppTrayExt;
pub use events::{EventHandler, MouseButton, TrayEvent};
pub use platform::{PlatformEventCallback, PlatformTrayConfig};
pub use types::{Tray, TrayIcon, TrayIconData};
