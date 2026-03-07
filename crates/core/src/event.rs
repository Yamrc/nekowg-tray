use gpui::*;

#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = gpui_tray, no_json)]
pub struct ClickEvent {
    pub button: MouseButton,
    pub position: Point<f32>,
}

/// Left mouse button double-click event for tray icon.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = gpui_tray, no_json)]
pub struct DoubleClickEvent;

/// Internal runtime event emitted by platform backends.
#[derive(Debug)]
pub enum RuntimeEvent {
    Action(Box<dyn Action>),
}
