//! Tray type definitions and builder

use crate::events::{EventHandler, TrayEvent};
use gpui::{App, MenuItem as GpuiMenuItem, SharedString};
use std::cell::RefCell;
use std::rc::Rc;

/// Tray icon types
#[derive(Clone, Debug)]
pub enum TrayIcon {
    /// Icon from raw image bytes (PNG format)
    Image { data: Vec<u8> },
    /// Icon from name (Linux uses theme icons)
    Name(String),
}

impl TrayIcon {
    /// Create icon from theme name
    pub fn from_name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }

    /// Create icon from PNG image data
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self::Image { data }
    }
}

/// Internal icon data for platform rendering
#[derive(Clone, Debug)]
pub struct TrayIconData {
    pub data: Rc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

/// System tray configuration
pub struct Tray {
    /// Icon to display
    pub icon: Option<TrayIcon>,
    /// Title text (Only macOS, displayed next to icon)
    pub title: Option<SharedString>,
    /// Tooltip text
    pub tooltip: Option<SharedString>,
    /// Whether the tray icon is visible
    pub visible: bool,
    /// Function to build the context menu
    pub menu_builder: Option<Rc<dyn Fn(&mut App) -> Vec<GpuiMenuItem> + 'static>>,
    /// Internal icon data for platform rendering
    pub(crate) icon_data: Option<TrayIconData>,
    /// Event handler for tray interactions
    pub event_handler: Option<Rc<RefCell<dyn EventHandler>>>,
}

impl Tray {
    /// Create a new tray with default settings
    pub fn new() -> Self {
        Self {
            icon: None,
            title: None,
            tooltip: None,
            visible: true,
            menu_builder: None,
            icon_data: None,
            event_handler: None,
        }
    }

    /// TODO: Set the tray icon from GPUI Image
    pub fn icon(mut self, _icon: impl Into<gpui::Image>) -> Self {
        // Store placeholder - actual rendering would happen in platform implementation
        self.icon = Some(TrayIcon::Image { data: Vec::new() });
        self
    }

    /// Set the tray title
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the tooltip text
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set the context menu builder
    pub fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(&mut App) -> Vec<GpuiMenuItem> + 'static,
    {
        self.menu_builder = Some(Rc::new(builder));
        self
    }

    /// Set event handler for tray interactions
    ///
    /// This uses a closure-based approach suitable for simple scenarios.
    /// For more complex use cases, consider using Action integration or Entity subscription.
    ///
    /// # Example
    /// ```rust,ignore
    /// let tray = Tray::new()
    ///     .on_event(|event| match event {
    ///         TrayEvent::Click { button, position } => {
    ///             log::info!("Tray clicked at {:?}", position);
    ///         }
    ///         TrayEvent::MenuSelect { id } => {
    ///             log::info!("Menu item selected: {}", id);
    ///         }
    ///         _ => {}
    ///     });
    /// ```
    pub fn on_event<F>(mut self, handler: F) -> Self
    where
        F: FnMut(TrayEvent) + 'static,
    {
        self.event_handler = Some(Rc::new(RefCell::new(handler)));
        self
    }

    /// Dispatch an event to the registered handler
    pub fn dispatch_event(&self, event: TrayEvent) {
        if let Some(handler) = &self.event_handler {
            handler.borrow_mut().handle(event);
        }
    }
}

impl Clone for Tray {
    fn clone(&self) -> Self {
        Self {
            icon: self.icon.clone(),
            title: self.title.clone(),
            tooltip: self.tooltip.clone(),
            visible: self.visible,
            menu_builder: self.menu_builder.clone(),
            icon_data: self.icon_data.clone(),
            event_handler: self.event_handler.clone(),
        }
    }
}

impl Default for Tray {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Tray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tray")
            .field("icon", &self.icon.is_some())
            .field("title", &self.title)
            .field("tooltip", &self.tooltip)
            .field("visible", &self.visible)
            .field("has_event_handler", &self.event_handler.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_builder() {
        let tray = Tray::new()
            .title("Test Title")
            .tooltip("Test Tooltip")
            .visible(true);

        assert_eq!(
            tray.title.as_ref().map(|s| s.to_string()),
            Some("Test Title".to_string())
        );
        assert_eq!(
            tray.tooltip.as_ref().map(|s| s.to_string()),
            Some("Test Tooltip".to_string())
        );
        assert!(tray.visible);
    }

    #[test]
    fn test_tray_default() {
        let tray = Tray::default();
        assert!(tray.icon.is_none());
        assert!(tray.title.is_none());
        assert!(tray.tooltip.is_none());
        assert!(tray.visible);
    }

    #[test]
    fn test_icon_from_name() {
        let icon = TrayIcon::from_name("test-icon");
        match icon {
            TrayIcon::Name(name) => assert_eq!(name, "test-icon"),
            _ => panic!("Expected Name variant"),
        }
    }

    #[test]
    fn test_icon_from_bytes() {
        let data = vec![0, 1, 2, 3];
        let icon = TrayIcon::from_bytes(data.clone());
        match icon {
            TrayIcon::Image { data: d } => assert_eq!(d, data),
            _ => panic!("Expected Image variant"),
        }
    }
}
