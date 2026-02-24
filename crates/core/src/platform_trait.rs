use crate::{Result, Tray};
use gpui::App;

/// Platform-specific tray implementation trait.
///
/// Each platform (Windows, macOS, Linux) implements this trait to provide
/// native system tray functionality.
pub trait PlatformTray {
    /// Creates or updates the tray icon with the specified configuration.
    fn set_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()>;

    /// Updates an existing tray icon.
    fn update_tray(&mut self, cx: &mut App, tray: &Tray) -> Result<()>;

    /// Removes the tray icon from the system tray.
    fn remove_tray(&mut self, cx: &mut App) -> Result<()>;
}
