//! Platform abstraction layer for system tray implementations
//!
//! This module defines configuration types that are passed to platform
//! implementations. The actual PlatformTray trait is defined in app_ext.rs
//! to avoid circular dependencies.

use crate::events::TrayEvent;
use gpui::{MenuItem, SharedString};

/// Configuration for platform tray implementations
///
/// This structure contains all the information needed to create or update
/// a system tray icon across all platforms.
pub struct PlatformTrayConfig {
    /// Tooltip text shown when hovering over the tray icon
    pub tooltip: Option<SharedString>,

    /// Whether the tray icon should be visible
    pub visible: bool,

    /// Menu items for the context menu
    pub menu_items: Option<Vec<MenuItem>>,

    /// Event callback for tray interactions
    ///
    /// This callback will be invoked for all tray events (clicks, menu selections, etc.)
    pub event_callback: Option<PlatformEventCallback>,
}

impl PlatformTrayConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self {
            tooltip: None,
            visible: true,
            menu_items: None,
            event_callback: None,
        }
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

    /// Set menu items
    pub fn menu_items(mut self, items: Vec<MenuItem>) -> Self {
        self.menu_items = Some(items);
        self
    }

    /// Set event callback
    pub fn on_event<F>(mut self, callback: F) -> Self
    where
        F: FnMut(TrayEvent) + 'static,
    {
        self.event_callback = Some(PlatformEventCallback::new(callback));
        self
    }
}

impl Default for PlatformTrayConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Event callback wrapper for platform implementations
///
/// This type provides a cloneable wrapper around the event callback,
/// allowing platform implementations to store and invoke the callback.
#[derive(Clone)]
pub struct PlatformEventCallback {
    pub(crate) inner: std::rc::Rc<std::cell::RefCell<dyn FnMut(TrayEvent) + 'static>>,
}

impl PlatformEventCallback {
    /// Create a new event callback wrapper
    pub fn new<F>(callback: F) -> Self
    where
        F: FnMut(TrayEvent) + 'static,
    {
        Self {
            inner: std::rc::Rc::new(std::cell::RefCell::new(callback)),
        }
    }

    /// Invoke the callback with the given event
    pub fn invoke(&self, event: TrayEvent) {
        (self.inner.borrow_mut())(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_config_builder() {
        let config = PlatformTrayConfig::new()
            .tooltip("Test Tooltip")
            .visible(true)
            .on_event(|_| {});

        assert_eq!(
            config.tooltip.as_ref().map(|s| s.to_string()),
            Some("Test Tooltip".to_string())
        );
        assert!(config.visible);
        assert!(config.event_callback.is_some());
    }

    #[test]
    fn test_event_callback_invoke() {
        use crate::events::{MouseButton, Point};
        use std::cell::RefCell;
        use std::rc::Rc;

        let received = Rc::new(RefCell::new(false));
        let received_clone = received.clone();

        let callback = PlatformEventCallback::new(move |event| {
            if let TrayEvent::Click { .. } = event {
                *received_clone.borrow_mut() = true;
            }
        });

        callback.invoke(TrayEvent::Click {
            button: MouseButton::Left,
            position: Point::new(0, 0),
        });

        assert!(*received.borrow());
    }
}
