use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MacError {
    #[error("Invalid MAC format: {0}")]
    InvalidFormat(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Vendor not found: {0}")]
    VendorNotFound(String),
}

impl From<ParseIntError> for MacError {
    fn from(err: ParseIntError) -> Self {
        MacError::InvalidFormat(format!("Invalid hexadecimal value: {}", err))
    }
}

impl From<std::io::Error> for MacError {
    fn from(err: std::io::Error) -> Self {
        MacError::SystemError(err.to_string())
    }
}