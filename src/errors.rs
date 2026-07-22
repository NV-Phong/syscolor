use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid color value: {0}")]
    InvalidColorValue(String),

    #[allow(dead_code)]
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(&'static str),

    #[error("win32 call failed: {0}")]
    Win32(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;