use crate::{Result, RuntimeEvent, Tray};

/// Platform-specific tray backend.
///
/// Backends are fully isolated from GPUI's `App` and only communicate through
/// immutable tray snapshots and runtime events.
pub trait PlatformTray: Send + Sync {
    /// Applies the latest tray snapshot.
    fn set_tray(&self, tray: Tray) -> Result<()>;

    /// Removes the tray icon.
    fn remove_tray(&self) -> Result<()>;

    /// Attempts to receive one runtime event from the backend.
    fn try_recv_event(&self) -> Result<Option<RuntimeEvent>>;

    /// Requests graceful shutdown of the backend runtime.
    fn shutdown(&self) -> Result<()>;
}
