use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Format(serde_json::Error),
    InvalidDocument(String),
    BuildEnvironment(String),
    InvalidSelection,
    ServiceUnavailable,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Storage: {error}"),
            Self::Format(error) => write!(f, "Prototype JSON: {error}"),
            Self::InvalidDocument(reason) => write!(f, "Invalid document: {reason}"),
            Self::BuildEnvironment(reason) => write!(f, "Local build setup: {reason}"),
            Self::InvalidSelection => f.write_str("Select an existing layer and key"),
            Self::ServiceUnavailable => f.write_str("Storage worker is unavailable"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Format(value)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
