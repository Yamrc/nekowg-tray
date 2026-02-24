use thiserror::Error;

/// Errors that can occur when working with system tray.
#[derive(Error, Debug)]
pub enum Error {
    /// A platform-specific error occurred.
    #[error("Platform error: {0}")]
    Platform(String),

    /// The requested tray was not found.
    #[error("Tray not found")]
    NotFound,

    /// The provided icon data is invalid or unsupported.
    #[error("Invalid icon data")]
    InvalidIcon,

    /// The tray manager has already been initialized.
    #[error("Already initialized")]
    AlreadyInitialized,
}

/// A specialized Result type for tray operations.
pub type Result<T> = std::result::Result<T, Error>;
