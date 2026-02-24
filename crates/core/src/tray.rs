use gpui::*;
use std::fmt;
use std::sync::Arc;

/// Builder function type for constructing context menus.
pub type MenuBuilder = Arc<dyn Fn(&mut App) -> Vec<MenuItem> + Send + Sync>;

/// Configuration for a system tray icon.
///
/// Use the builder pattern to construct a tray configuration:
///
/// ```rust
/// let tray = Tray::new()
///     .tooltip("My Application")
///     .icon(image)
///     .menu(|cx| vec![MenuItem::action("Quit", Quit)]);
/// ```
pub struct Tray {
    /// Tooltip text displayed when hovering over the tray icon.
    pub tooltip: Option<SharedString>,
    /// Title text for the tray item (platform-dependent).
    pub title: Option<SharedString>,
    /// Icon image displayed in the system tray.
    pub icon: Option<Image>,
    /// Whether the tray icon is currently visible.
    pub visible: bool,
    /// Optional menu builder for context menu.
    pub menu_builder: Option<MenuBuilder>,
}

impl Tray {
    /// Creates a new tray configuration with default settings.
    pub fn new() -> Self {
        Self {
            tooltip: None,
            title: None,
            icon: None,
            visible: true,
            menu_builder: None,
        }
    }

    /// Sets the tooltip text.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Sets the title text.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the icon image.
    pub fn icon(mut self, icon: Image) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the visibility state.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets the context menu builder.
    pub fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(&mut App) -> Vec<MenuItem> + Send + Sync + 'static,
    {
        self.menu_builder = Some(Arc::new(builder));
        self
    }
}

impl Default for Tray {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Tray {
    fn clone(&self) -> Self {
        Self {
            tooltip: self.tooltip.clone(),
            title: self.title.clone(),
            icon: self.icon.clone(),
            visible: self.visible,
            menu_builder: self.menu_builder.clone(),
        }
    }
}

impl fmt::Debug for Tray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tray")
            .field("tooltip", &self.tooltip)
            .field("title", &self.title)
            .field("visible", &self.visible)
            .field("menu_builder", &self.menu_builder.is_some())
            .finish()
    }
}
