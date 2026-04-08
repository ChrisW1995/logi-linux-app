#[derive(Debug, thiserror::Error)]
pub enum HidppError {
    #[error("device not found")]
    DeviceNotFound,
    #[error("HID I/O error: {0}")]
    Io(String),
    #[error("timeout reading from device")]
    Timeout,
    #[error("feature 0x{feature_id:04X} not found on device")]
    FeatureNotFound { feature_id: u16 },
    #[error("HID++ error: function {function}, error code {error_code}")]
    ProtocolError { function: u8, error_code: u8 },
    #[error("unexpected response: expected function {expected}, got {got}")]
    UnexpectedResponse { expected: u8, got: u8 },
    #[error("invalid report length: expected {expected}, got {got}")]
    InvalidLength { expected: usize, got: usize },
}

/// HID++ 2.0 error codes returned in ERR_MSG (0xFF).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HidppErrorCode {
    NoError = 0x00,
    Unknown = 0x01,
    InvalidArgument = 0x02,
    OutOfRange = 0x03,
    HardwareError = 0x04,
    LogitechInternal = 0x05,
    InvalidFeatureIndex = 0x06,
    InvalidFunctionId = 0x07,
    Busy = 0x08,
    Unsupported = 0x09,
}

impl HidppErrorCode {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::NoError,
            0x01 => Self::Unknown,
            0x02 => Self::InvalidArgument,
            0x03 => Self::OutOfRange,
            0x04 => Self::HardwareError,
            0x05 => Self::LogitechInternal,
            0x06 => Self::InvalidFeatureIndex,
            0x07 => Self::InvalidFunctionId,
            0x08 => Self::Busy,
            0x09 => Self::Unsupported,
            _ => Self::Unknown,
        }
    }
}
