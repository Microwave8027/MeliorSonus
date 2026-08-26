use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RustError {
    StreamBuildError(String),
    BufferOverfill,
    Custom(String),
}

impl fmt::Display for RustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(msg) => write!(f, "{msg}"),
            Self::BufferOverfill => write!(f, "Audio Buffer Overfilled!"),
            Self::StreamBuildError(msg) => write!(f, "Stream Build Error: {msg}"),
        }
    }
}

impl std::error::Error for RustError {}

impl From<String> for RustError {
    fn from(msg: String) -> Self {
        RustError::Custom(msg)
    }
}

impl From<&str> for RustError {
    fn from(msg: &str) -> Self {
        RustError::Custom(msg.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for RustError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        RustError::Custom(err.to_string())
    }
}
