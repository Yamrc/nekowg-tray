use gpui::{App, AsyncApp, Global, Task};
use gpui_tray_core::platform_trait::PlatformTray;
use gpui_tray_core::{Error, Result, RuntimeEvent, Tray};
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_os = "windows")]
use gpui_tray_windows as platform_impl;

#[cfg(target_os = "macos")]
use gpui_tray_macos as platform_impl;

#[cfg(target_os = "linux")]
use gpui_tray_linux as platform_impl;

struct TrayRuntime {
    backend: Arc<dyn PlatformTray>,
    current_tray: Option<Tray>,
    event_pump_task: Option<Task<()>>,
}

impl Global for TrayRuntime {}

impl TrayRuntime {
    fn new(cx: &mut App) -> Result<Self> {
        let backend: Arc<dyn PlatformTray> = platform_impl::create()?.into();
        let event_pump_task = spawn_event_pump(cx, backend.clone());
        Ok(Self {
            backend,
            current_tray: None,
            event_pump_task: Some(event_pump_task),
        })
    }
}

impl Drop for TrayRuntime {
    fn drop(&mut self) {
        let _ = self.backend.shutdown();
        self.event_pump_task.take();
    }
}

fn spawn_event_pump(cx: &mut App, backend: Arc<dyn PlatformTray>) -> Task<()> {
    cx.spawn(move |cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            loop {
                loop {
                    match backend.try_recv_event() {
                        Ok(Some(RuntimeEvent::Action(action))) => {
                            log::debug!("dispatching backend action {}", action.name());
                            if cx
                                .update(|app: &mut App| app.dispatch_action(action.as_ref()))
                                .is_err()
                            {
                                return;
                            }
                        }
                        Ok(None) => break,
                        Err(Error::RuntimeClosed) => return,
                        Err(err) => {
                            log::error!("tray event pump stopped: {err}");
                            return;
                        }
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(8))
                    .await;
            }
        }
    })
}

pub trait TrayAppContext {
    fn set_tray(&mut self, tray: Tray) -> Result<()>;
    fn tray(&self) -> Option<&Tray>;
    fn update_tray(&mut self, f: impl FnOnce(&mut Tray)) -> Result<Tray>;
    fn remove_tray(&mut self) -> Result<()>;
}

impl TrayAppContext for App {
    fn set_tray(&mut self, tray: Tray) -> Result<()> {
        log::debug!(
            "set_tray visible={}, has_icon={}, has_menu={}",
            tray.visible,
            tray.icon.is_some(),
            tray.menu_builder.is_some()
        );
        let mut runtime = if self.has_global::<TrayRuntime>() {
            self.remove_global::<TrayRuntime>()
        } else {
            TrayRuntime::new(self)?
        };

        runtime.backend.set_tray(tray.clone())?;
        runtime.current_tray = Some(tray);

        self.set_global(runtime);
        Ok(())
    }

    fn tray(&self) -> Option<&Tray> {
        self.try_global::<TrayRuntime>()
            .and_then(|runtime| runtime.current_tray.as_ref())
    }

    fn update_tray(&mut self, f: impl FnOnce(&mut Tray)) -> Result<Tray> {
        if !self.has_global::<TrayRuntime>() {
            return Err(Error::NotFound);
        }

        let mut runtime = self.remove_global::<TrayRuntime>();
        let Some(tray) = runtime.current_tray.as_mut() else {
            self.set_global(runtime);
            return Err(Error::NotFound);
        };

        f(tray);
        let updated = tray.clone();
        runtime.backend.set_tray(updated.clone())?;

        self.set_global(runtime);
        log::debug!(
            "update_tray done visible={}, has_icon={}, has_menu={}",
            updated.visible,
            updated.icon.is_some(),
            updated.menu_builder.is_some()
        );
        Ok(updated)
    }

    fn remove_tray(&mut self) -> Result<()> {
        if !self.has_global::<TrayRuntime>() {
            return Err(Error::NotFound);
        }

        let mut runtime = self.remove_global::<TrayRuntime>();
        if runtime.current_tray.is_none() {
            self.set_global(runtime);
            return Err(Error::NotFound);
        }

        runtime.backend.remove_tray()?;
        runtime.current_tray = None;
        self.set_global(runtime);
        Ok(())
    }
}
