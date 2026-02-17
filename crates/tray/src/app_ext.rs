//! App extension for system tray support
//!
//! This module implements the platform-agnostic AppTrayExt trait
//! by delegating to platform-specific implementations.

use gpui::{App, BorrowAppContext, Global};

use crate::events::TrayEvent;
use crate::platform::PlatformTrayConfig;
use crate::types::Tray;

/// Global state for tray management
pub struct TrayGlobal {
    /// Platform-specific tray implementation
    platform_tray: Option<Box<dyn PlatformTray>>,
    /// Global event handler
    event_handler: Option<Box<dyn FnMut(TrayEvent, &mut App) + 'static>>,
}

impl TrayGlobal {
    fn new() -> Self {
        Self {
            platform_tray: None,
            event_handler: None,
        }
    }

    /// Get or create the platform tray implementation
    fn get_or_create_tray(&mut self) -> &mut Box<dyn PlatformTray> {
        if self.platform_tray.is_none() {
            self.platform_tray = Some(create_platform_tray());
        }
        self.platform_tray.as_mut().unwrap()
    }
}

impl Global for TrayGlobal {}

/// Extension trait for App to manage system tray
///
/// This trait provides a unified, platform-agnostic API for setting the system tray.
/// Simply call `cx.set_tray(tray)` from your application.
///
/// # Example
/// ```rust,ignore
/// use gpui::*;
/// use gpui_tray::{Tray, AppTrayExt};
///
/// fn main() {
///     Application::new().run(|cx: &mut App| {
///         let tray = Tray::new()
///             .tooltip("My App")
///             .visible(true)
///             .menu(|_cx| vec![
///                 MenuItem::action("Show", ShowAction),
///                 MenuItem::separator(),
///                 MenuItem::action("Quit", QuitAction),
///             ]);
///         
///         cx.set_tray(tray);
///     });
/// }
/// ```
pub trait AppTrayExt {
    /// Set or update the system tray.
    ///
    /// This method will create the tray if it doesn't exist, or update it if it does.
    /// The platform-specific implementation (Windows/Linux/macOS) is automatically selected
    /// at compile time.
    fn set_tray(&mut self, tray: Tray);

    /// Set a global event handler for all tray events.
    ///
    /// This handler will be called in addition to any tray-specific handler set via
    /// `Tray::on_event()`. It is useful for application-wide event handling.
    ///
    /// # Example
    /// ```rust,ignore
    /// cx.on_tray_event(|event, cx| {
    ///     match event {
    ///         TrayEvent::Click { .. } => {
    ///             cx.dispatch_action(&ShowWindow.boxed_clone());
    ///         }
    ///         TrayEvent::MenuSelect { id } => {
    ///             log::info!("Menu item selected: {}", id);
    ///         }
    ///         _ => {}
    ///     }
    /// });
    /// ```
    fn on_tray_event<F>(&mut self, handler: F)
    where
        F: FnMut(TrayEvent, &mut App) + 'static;
}

impl AppTrayExt for App {
    fn set_tray(&mut self, tray: Tray) {
        if !self.has_global::<TrayGlobal>() {
            self.set_global(TrayGlobal::new());
        }

        let menu_items = tray.menu_builder.as_ref().map(|builder| builder(self));

        let event_callback = tray.event_handler.clone().map(|handler| {
            crate::platform::PlatformEventCallback::new(move |event| {
                handler.borrow_mut().handle(event);
            })
        });

        let config = PlatformTrayConfig {
            tooltip: tray.tooltip,
            visible: tray.visible,
            menu_items,
            event_callback,
        };

        self.update_global::<TrayGlobal, _>(|global, cx| {
            let platform = global.get_or_create_tray();
            platform.set_tray(cx, config);
        });
    }

    fn on_tray_event<F>(&mut self, handler: F)
    where
        F: FnMut(TrayEvent, &mut App) + 'static,
    {
        if !self.has_global::<TrayGlobal>() {
            self.set_global(TrayGlobal::new());
        }

        self.update_global::<TrayGlobal, _>(|global, _cx| {
            global.event_handler = Some(Box::new(handler));
        });
    }
}

/// Create platform-specific tray implementation
#[cfg(target_os = "windows")]
fn create_platform_tray() -> Box<dyn PlatformTray> {
    use tray_windows::WindowsTray;
    Box::new(WindowsTray::new())
}

#[cfg(target_os = "linux")]
fn create_platform_tray() -> Box<dyn PlatformTray> {
    use tray_linux::{LinuxTray, PlatformTray};
    Box::new(LinuxTray::new())
}

#[cfg(target_os = "macos")]
fn create_platform_tray() -> Box<dyn PlatformTray> {
    use tray_macos::{MacosTray, PlatformTray};
    Box::new(MacosTray::new())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn create_platform_tray() -> Box<dyn PlatformTray> {
    compile_error!("Unsupported platform. Only Windows, Linux, and macOS are supported.");
}

/// Re-export PlatformTray trait for platform implementations
trait PlatformTray: 'static {
    fn set_tray(&mut self, app: &mut App, config: PlatformTrayConfig);
    fn hide(&mut self);
    fn show(&mut self, app: &mut App);
    fn destroy(&mut self);
}

#[cfg(target_os = "windows")]
impl PlatformTray for tray_windows::WindowsTray {
    fn set_tray(&mut self, app: &mut App, config: PlatformTrayConfig) {
        use gpui::Point;

        let windows_config = tray_windows::WindowsTrayConfig {
            tooltip: config.tooltip,
            visible: config.visible,
            menu_items: config.menu_items,
            event_callback: config.event_callback.map(|cb| {
                tray_windows::EventCallback::new(
                    move |event_type: u32, button_or_id: u32, x: i32, y: i32| {
                        let event = match event_type {
                            0 => {
                                // Click event
                                let button = match button_or_id {
                                    0 => crate::events::MouseButton::Left,
                                    1 => crate::events::MouseButton::Right,
                                    2 => crate::events::MouseButton::Middle,
                                    _ => crate::events::MouseButton::Left,
                                };
                                TrayEvent::Click {
                                    button,
                                    position: Point::new(x, y),
                                }
                            }
                            1 => {
                                // Menu select event
                                TrayEvent::MenuSelect {
                                    id: button_or_id.to_string(),
                                }
                            }
                            _ => TrayEvent::Click {
                                button: crate::events::MouseButton::Left,
                                position: Point::new(x, y),
                            },
                        };
                        cb.invoke(event);
                    },
                )
            }),
        };
        tray_windows::PlatformTray::set_tray(self, app, windows_config);
    }

    fn hide(&mut self) {
        tray_windows::PlatformTray::hide(self);
    }

    fn show(&mut self, app: &mut App) {
        tray_windows::PlatformTray::show(self, app);
    }

    fn destroy(&mut self) {
        tray_windows::PlatformTray::destroy(self);
    }
}

#[cfg(target_os = "linux")]
impl PlatformTray for tray_linux::LinuxTray {
    fn set_tray(&mut self, app: &mut App, config: PlatformTrayConfig) {
        let linux_config = tray_linux::LinuxTrayConfig {
            tooltip: config.tooltip,
            visible: config.visible,
            menu_items: config.menu_items,
        };
        tray_linux::PlatformTray::set_tray(self, app, linux_config);
    }

    fn hide(&mut self) {
        tray_linux::PlatformTray::hide(self);
    }

    fn show(&mut self, app: &mut App) {
        tray_linux::PlatformTray::show(self, app);
    }

    fn destroy(&mut self) {
        tray_linux::PlatformTray::destroy(self);
    }
}

#[cfg(target_os = "macos")]
impl PlatformTray for tray_macos::MacosTray {
    fn set_tray(&mut self, app: &mut App, config: PlatformTrayConfig) {
        let macos_config = tray_macos::MacosTrayConfig {
            tooltip: config.tooltip,
            visible: config.visible,
            menu_items: config.menu_items,
        };
        tray_macos::PlatformTray::set_tray(self, app, macos_config);
    }

    fn hide(&mut self) {
        tray_macos::PlatformTray::hide(self);
    }

    fn show(&mut self, app: &mut App) {
        tray_macos::PlatformTray::show(self, app);
    }

    fn destroy(&mut self) {
        tray_macos::PlatformTray::destroy(self);
    }
}
