use thiserror::Error;

/// Errors that can occur when working with system tray.
#[derive(Error, Debug)]
pub enum Error {
    /// The requested tray was not found.
    #[error("Tray not found")]
    NotFound,

    /// The tray manager has already been initialized.
    #[error("Tray runtime already initialized")]
    AlreadyInitialized,

    /// The platform does not currently support tray integration.
    #[error("Current platform is not supported yet")]
    UnsupportedPlatform,

    /// The backend runtime is closed.
    #[error("Tray runtime is closed")]
    RuntimeClosed,

    /// Backend-specific error.
    #[error(transparent)]
    Backend(#[from] BackendError),

    /// The provided icon data is invalid or unsupported.
    #[error("Invalid icon data")]
    InvalidIcon,
}

/// Errors raised from platform backend implementations.
#[derive(Error, Debug)]
pub enum BackendError {
    /// Failed to send a command to the backend worker.
    #[error("Failed to send command to backend worker")]
    ChannelSend,

    /// Failed to receive a response from the backend worker.
    #[error("Failed to receive response from backend worker")]
    ChannelReceive,

    /// A native platform API call failed.
    #[error("Platform call `{operation}` failed: {message}")]
    Platform {
        operation: &'static str,
        message: String,
    },
}

impl BackendError {
    pub fn platform(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Platform {
            operation,
            message: message.into(),
        }
    }
}

/// A specialized Result type for tray operations.
pub type Result<T> = std::result::Result<T, Error>;
