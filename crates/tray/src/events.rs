//! Tray event types and input handling

pub use gpui::Point;

/// Mouse button types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Tray events emitted by user interaction
#[derive(Clone, Debug)]
pub enum TrayEvent {
    /// Tray icon was clicked
    Click {
        button: MouseButton,
        position: Point<i32>,
    },
    /// Tray received scroll input
    Scroll { delta: Point<i32> },
    /// Menu item was selected
    MenuSelect { id: String },
    /// Menu checkbox state changed
    MenuChecked { id: String, checked: bool },
    /// Menu radio button selected
    MenuRadioSelected { group: String, id: String },
}

/// Event handler trait for tray events
pub trait EventHandler: 'static {
    /// Handle a tray event
    fn handle(&mut self, event: TrayEvent);
}

/// Blanket implementation for closures
impl<F> EventHandler for F
where
    F: FnMut(TrayEvent) + 'static,
{
    fn handle(&mut self, event: TrayEvent) {
        (self)(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_event_click() {
        let event = TrayEvent::Click {
            button: MouseButton::Left,
            position: Point::new(100, 200),
        };

        match event {
            TrayEvent::Click { button, position } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(position.x, 100);
                assert_eq!(position.y, 200);
            }
            _ => panic!("Expected Click variant"),
        }
    }

    #[test]
    fn test_tray_event_scroll() {
        let event = TrayEvent::Scroll {
            delta: Point::new(5, -3),
        };

        match event {
            TrayEvent::Scroll { delta } => {
                assert_eq!(delta.x, 5);
                assert_eq!(delta.y, -3);
            }
            _ => panic!("Expected Scroll variant"),
        }
    }

    #[test]
    fn test_tray_event_menu_select() {
        let event = TrayEvent::MenuSelect {
            id: String::from("item-1"),
        };

        match event {
            TrayEvent::MenuSelect { id } => {
                assert_eq!(id, "item-1");
            }
            _ => panic!("Expected MenuSelect variant"),
        }
    }

    #[test]
    fn test_tray_event_menu_checked() {
        let event = TrayEvent::MenuChecked {
            id: String::from("check-1"),
            checked: true,
        };

        match event {
            TrayEvent::MenuChecked { id, checked } => {
                assert_eq!(id, "check-1");
                assert!(checked);
            }
            _ => panic!("Expected MenuChecked variant"),
        }
    }

    #[test]
    fn test_tray_event_menu_radio() {
        let event = TrayEvent::MenuRadioSelected {
            group: String::from("theme-group"),
            id: String::from("dark-theme"),
        };

        match event {
            TrayEvent::MenuRadioSelected { group, id } => {
                assert_eq!(group, "theme-group");
                assert_eq!(id, "dark-theme");
            }
            _ => panic!("Expected MenuRadioSelected variant"),
        }
    }

    #[test]
    fn test_event_handler_closure() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let received = Rc::new(RefCell::new(None));
        let received_clone = received.clone();

        let mut handler: Box<dyn EventHandler> = Box::new(move |event: TrayEvent| {
            *received_clone.borrow_mut() = Some(event);
        });

        let test_event = TrayEvent::Click {
            button: MouseButton::Left,
            position: Point::new(10, 20),
        };

        handler.handle(test_event.clone());

        assert!(received.borrow().is_some());
        match received.borrow().as_ref().unwrap() {
            TrayEvent::Click { button, position } => {
                assert_eq!(*button, MouseButton::Left);
                assert_eq!(position.x, 10);
                assert_eq!(position.y, 20);
            }
            _ => panic!("Expected Click event"),
        }
    }
}
