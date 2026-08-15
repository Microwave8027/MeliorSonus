#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RustError {
    Custom(String),
}

impl std::fmt::Display for RustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustError::Custom(msg) => write!(f, "{msg}"),
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
