use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RustError {
    AudioEngineError(AudioEngineError),
    NotImplementedError(String),
    FeatureExtactorBuildError(String),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum AudioEngineError {
    StreamBuildError(String),
    BufferOverfill,
}

impl fmt::Display for RustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(msg) => write!(f, "{msg}"),
            Self::AudioEngineError(err) => match err {
                AudioEngineError::BufferOverfill => {
                    write!(f, "Audio Buffer Overfilled, please reset")
                }
                AudioEngineError::StreamBuildError(msg) => {
                    write!(f, "Stream build failed: {msg}")
                }
            },
            Self::FeatureExtactorBuildError(msg) => {
                write!(f, "Extractor Build Exception: {msg}")
            }
            Self::NotImplementedError(msg) => {
                write!(f, "Feature is not implemented: {msg}")
            }
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
