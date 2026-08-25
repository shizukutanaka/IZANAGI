//! Errors.
//!
//! One error type. No `Box<dyn Error>`, no error hierarchy trees.
//! When something fails, there is one path out.

use std::fmt;

/// Anything that can go wrong in IZANAGI.
#[derive(Debug)]
pub enum Error {
    /// A backend failed to initialize or present.
    Backend(String),
    /// An asset could not be loaded.
    Asset(String),
    /// I/O error (saves, asset files).
    Io(std::io::Error),
    /// Audio initialization or playback error.
    Audio(String),
    /// A configuration value was out of range or malformed.
    Config(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Backend(msg) => write!(f, "backend: {msg}"),
            Error::Asset(msg) => write!(f, "asset: {msg}"),
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Audio(msg) => write!(f, "audio: {msg}"),
            Error::Config(msg) => write!(f, "config: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Short alias for engine results.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_contains_category() {
        let e = Error::Backend("no device".into());
        assert!(e.to_string().contains("backend"));
        assert!(e.to_string().contains("no device"));
    }

    #[test]
    fn io_error_conversion_preserves_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let e: Error = io.into();
        assert!(std::error::Error::source(&e).is_some());
    }
}
