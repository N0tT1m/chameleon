// src/error.rs
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum MacError {
    ValidationFailed(String),
    PermissionDenied(String),
    SystemError(String),
    InvalidFormat(String),
    NetworkError(String),
    DatabaseError(String),
    VendorNotFound(String),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
    ParseError(String),
    UnsupportedPlatform(String),  // Added this variant
}

impl fmt::Display for MacError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MacError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            MacError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            MacError::SystemError(msg) => write!(f, "System error: {}", msg),
            MacError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            MacError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            MacError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            MacError::VendorNotFound(msg) => write!(f, "Vendor not found: {}", msg),
            MacError::IoError(e) => write!(f, "IO error: {}", e),
            MacError::SerdeError(e) => write!(f, "Serialization error: {}", e),
            MacError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            MacError::UnsupportedPlatform(msg) => write!(f, "Unsupported platform: {}", msg),
        }
    }
}

impl Error for MacError {}

impl From<ParseIntError> for MacError {
    fn from(err: ParseIntError) -> Self {
        MacError::ParseError(err.to_string())
    }
}

impl From<std::io::Error> for MacError {
    fn from(err: std::io::Error) -> Self {
        MacError::IoError(err)
    }
}

impl From<serde_json::Error> for MacError {
    fn from(err: serde_json::Error) -> Self {
        MacError::SerdeError(err)
    }
}

impl From<&str> for MacError {
    fn from(s: &str) -> Self {
        MacError::ValidationFailed(s.to_string())
    }
}

impl From<String> for MacError {
    fn from(s: String) -> Self {
        MacError::ValidationFailed(s)
    }
}

impl From<Box<dyn Error>> for MacError {
    fn from(err: Box<dyn Error>) -> Self {
        MacError::SystemError(err.to_string())
    }
}