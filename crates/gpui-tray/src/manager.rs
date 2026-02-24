use gpui::{Action, App, Global, MouseButton, Point};
use gpui_tray_core::Result;
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{self as core, *};
use log::debug;
use std::cell::Cell;

#[cfg(target_os = "windows")]
use gpui_tray_windows as platform_impl;

#[cfg(target_os = "macos")]
use gpui_tray_macos as platform_impl;

#[cfg(target_os = "linux")]
use gpui_tray_linux as platform_impl;

/// Internal tray manager stored as a GPUI global.
struct TrayManager {
    current_tray: Option<Tray>,
    platform_impl: Box<dyn PlatformTray>,
}

impl Global for TrayManager {}

#[cfg(target_os = "windows")]
struct GlobalDispatcher;

#[cfg(target_os = "windows")]
impl platform_impl::TrayEventDispatcher for GlobalDispatcher {
    fn dispatch_click(&self, button: MouseButton, position: Point<f32>) {
        debug!(
            "Dispatching click event: button={:?}, position={:?}",
            button, position
        );
        let event = core::ClickEvent { button, position };
        DISPATCHER_APP.with(|cell| {
            if let Some(app_ptr) = cell.get() {
                unsafe {
                    let app = &mut *app_ptr;
                    app.dispatch_action(&event);
                }
            }
        });
    }

    fn dispatch_double_click(&self) {
        debug!("Dispatching double click event");
        let event = core::DoubleClickEvent;
        DISPATCHER_APP.with(|cell| {
            if let Some(app_ptr) = cell.get() {
                unsafe {
                    let app = &mut *app_ptr;
                    app.dispatch_action(&event);
                }
            }
        });
    }

    fn dispatch_menu_action(&self, action: Box<dyn Action>) {
        debug!("Dispatching menu action");
        DISPATCHER_APP.with(|cell| {
            if let Some(app_ptr) = cell.get() {
                unsafe {
                    let app = &mut *app_ptr;
                    app.dispatch_action(action.as_ref());
                }
            }
        });
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    static DISPATCHER_APP: Cell<Option<*mut App>> = const { Cell::new(None) };
}

impl TrayManager {
    /// Creates a new TrayManager with platform-specific implementation.
    fn new() -> Result<Self> {
        debug!("Creating new TrayManager");
        Ok(Self {
            current_tray: None,
            platform_impl: platform_impl::create()?,
        })
    }

    /// Sets the current tray configuration.
    fn set_tray(&mut self, cx: &mut App, tray: Tray) -> Result<()> {
        debug!(
            "Setting tray: tooltip={:?}, visible={}",
            tray.tooltip, tray.visible
        );
        self.platform_impl.set_tray(cx, &tray)?;
        self.current_tray = Some(tray);
        Ok(())
    }

    /// Returns the current tray configuration if any.
    fn tray(&self) -> Option<&Tray> {
        self.current_tray.as_ref()
    }

    /// Updates the current tray using the provided closure.
    fn update_tray(&mut self, cx: &mut App, f: impl FnOnce(&mut Tray)) -> Result<Tray> {
        let Some(tray) = self.current_tray.as_mut() else {
            debug!("Tray not found for update");
            return Err(Error::NotFound);
        };
        f(tray);
        debug!("Updating tray");
        self.platform_impl.update_tray(cx, tray)?;
        Ok(self.current_tray.clone().unwrap())
    }

    /// Removes the tray icon from the system tray.
    fn remove_tray(&mut self, cx: &mut App) -> Result<()> {
        debug!("Removing tray");
        self.platform_impl.remove_tray(cx)?;
        self.current_tray = None;
        Ok(())
    }
}

/// Extension trait for GPUI's `App` to manage system tray icons.
///
/// This trait provides a high-level API for tray management that works
/// consistently across all supported platforms.
///
/// # Example
///
/// ```rust
/// use gpui_tray::{Tray, TrayAppContext};
///
/// // In your app initialization
/// cx.set_tray(
///     Tray::new()
///         .tooltip("My App")
///         .icon(icon_image)
///         .menu(|cx| vec![
///             MenuItem::action("Settings", OpenSettings),
///             MenuItem::separator(),
///             MenuItem::action("Quit", Quit),
///         ])
/// );
///
/// // Later, update the tooltip
/// cx.update_tray(|tray| {
///     tray.tooltip = Some("Updated Status".into());
/// });
///
/// // Remove when done
/// cx.remove_tray();
/// ```
pub trait TrayAppContext {
    /// Sets or replaces the current tray icon.
    fn set_tray(&mut self, tray: Tray) -> Result<()>;

    /// Returns the current tray configuration if set.
    fn tray(&self) -> Option<&Tray>;

    /// Updates the current tray configuration.
    fn update_tray(&mut self, f: impl FnOnce(&mut Tray)) -> Result<Tray>;

    /// Removes the tray icon from the system tray.
    fn remove_tray(&mut self) -> Result<()>;
}

impl TrayAppContext for App {
    fn set_tray(&mut self, tray: Tray) -> Result<()> {
        #[cfg(target_os = "windows")]
        set_dispatcher_app(self);

        let mut manager = if self.has_global::<TrayManager>() {
            self.remove_global::<TrayManager>()
        } else {
            TrayManager::new()?
        };

        manager.set_tray(self, tray)?;

        self.set_global(manager);
        Ok(())
    }

    fn tray(&self) -> Option<&Tray> {
        self.try_global::<TrayManager>()
            .and_then(|manager| manager.tray())
    }

    fn update_tray(&mut self, f: impl FnOnce(&mut Tray)) -> Result<Tray> {
        if !self.has_global::<TrayManager>() {
            return Err(Error::NotFound);
        }

        let mut manager = self.remove_global::<TrayManager>();
        let result = manager.update_tray(self, f)?;

        self.set_global(manager);
        Ok(result)
    }

    fn remove_tray(&mut self) -> Result<()> {
        if !self.has_global::<TrayManager>() {
            return Err(Error::NotFound);
        }

        let mut manager = self.remove_global::<TrayManager>();
        manager.remove_tray(self)?;

        #[cfg(target_os = "windows")]
        clear_dispatcher_app();

        self.set_global(manager);
        Ok(())
    }
}

/// Sets up the event dispatcher for Windows platform.
#[cfg(target_os = "windows")]
fn set_dispatcher_app(app: &mut App) {
    DISPATCHER_APP.set(Some(app as *mut App));
    let dispatcher: &'static GlobalDispatcher = Box::leak(Box::new(GlobalDispatcher));
    platform_impl::set_dispatcher(Some(dispatcher));
}

/// Clears the event dispatcher for Windows platform.
#[cfg(target_os = "windows")]
fn clear_dispatcher_app() {
    DISPATCHER_APP.set(None);
    platform_impl::set_dispatcher(None);
}
