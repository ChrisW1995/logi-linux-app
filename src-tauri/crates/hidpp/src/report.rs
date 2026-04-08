use crate::error::HidppError;

/// HID++ report IDs.
pub const SHORT_REPORT_ID: u8 = 0x10;
pub const LONG_REPORT_ID: u8 = 0x11;

/// HID++ report sizes (including report ID).
pub const SHORT_REPORT_LEN: usize = 7;
pub const LONG_REPORT_LEN: usize = 20;

/// Special feature index indicating an error response.
pub const ERROR_MSG: u8 = 0xFF;

/// A HID++ 2.0 report (either short or long).
#[derive(Debug, Clone)]
pub struct HidppReport {
    /// Raw report bytes.
    pub data: [u8; LONG_REPORT_LEN],
    /// Actual length (7 for short, 20 for long).
    pub len: usize,
}

impl HidppReport {
    /// Create a new long report.
    pub fn new_long(device_index: u8, feature_index: u8, function_id: u8, sw_id: u8) -> Self {
        let mut data = [0u8; LONG_REPORT_LEN];
        data[0] = LONG_REPORT_ID;
        data[1] = device_index;
        data[2] = feature_index;
        data[3] = (function_id << 4) | (sw_id & 0x0F);
        Self { data, len: LONG_REPORT_LEN }
    }

    /// Create a new short report.
    pub fn new_short(device_index: u8, feature_index: u8, function_id: u8, sw_id: u8) -> Self {
        let mut data = [0u8; LONG_REPORT_LEN];
        data[0] = SHORT_REPORT_ID;
        data[1] = device_index;
        data[2] = feature_index;
        data[3] = (function_id << 4) | (sw_id & 0x0F);
        Self { data, len: SHORT_REPORT_LEN }
    }

    pub fn report_id(&self) -> u8 {
        self.data[0]
    }

    pub fn device_index(&self) -> u8 {
        self.data[1]
    }

    pub fn feature_index(&self) -> u8 {
        self.data[2]
    }

    pub fn function_id(&self) -> u8 {
        self.data[3] >> 4
    }

    pub fn sw_id(&self) -> u8 {
        self.data[3] & 0x0F
    }

    /// Get the parameter bytes (starting at offset 4).
    pub fn params(&self) -> &[u8] {
        &self.data[4..self.len]
    }

    /// Set a parameter byte at the given offset (0-based from param start).
    pub fn set_param(&mut self, offset: usize, value: u8) {
        self.data[4 + offset] = value;
    }

    /// Get the raw bytes to write to the device.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// Parse a report from raw bytes read from the device.
    pub fn from_bytes(data: &[u8]) -> Result<Self, HidppError> {
        if data.is_empty() {
            return Err(HidppError::InvalidLength { expected: SHORT_REPORT_LEN, got: 0 });
        }

        let len = match data[0] {
            SHORT_REPORT_ID => SHORT_REPORT_LEN,
            LONG_REPORT_ID => LONG_REPORT_LEN,
            _ => return Err(HidppError::InvalidLength { expected: SHORT_REPORT_LEN, got: data.len() }),
        };

        if data.len() < len {
            return Err(HidppError::InvalidLength { expected: len, got: data.len() });
        }

        let mut report_data = [0u8; LONG_REPORT_LEN];
        report_data[..len].copy_from_slice(&data[..len]);
        Ok(Self { data: report_data, len })
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        self.feature_index() == ERROR_MSG
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_report_construction() {
        let report = HidppReport::new_long(0x01, 0x05, 0x02, 0x0A);
        assert_eq!(report.report_id(), LONG_REPORT_ID);
        assert_eq!(report.device_index(), 0x01);
        assert_eq!(report.feature_index(), 0x05);
        assert_eq!(report.function_id(), 0x02);
        assert_eq!(report.sw_id(), 0x0A);
        assert_eq!(report.len, LONG_REPORT_LEN);
    }

    #[test]
    fn short_report_construction() {
        let report = HidppReport::new_short(0xFF, 0x00, 0x01, 0x01);
        assert_eq!(report.report_id(), SHORT_REPORT_ID);
        assert_eq!(report.device_index(), 0xFF);
        assert_eq!(report.feature_index(), 0x00);
        assert_eq!(report.function_id(), 0x01);
        assert_eq!(report.sw_id(), 0x01);
        assert_eq!(report.len, SHORT_REPORT_LEN);
    }

    #[test]
    fn report_params() {
        let mut report = HidppReport::new_long(0x01, 0x05, 0x00, 0x01);
        report.set_param(0, 0x18);
        report.set_param(1, 0x14);
        assert_eq!(report.params()[0], 0x18);
        assert_eq!(report.params()[1], 0x14);
    }

    #[test]
    fn from_bytes_long() {
        let mut data = [0u8; LONG_REPORT_LEN];
        data[0] = LONG_REPORT_ID;
        data[1] = 0x01;
        data[2] = 0x05;
        data[3] = 0x21; // function_id=2, sw_id=1

        let report = HidppReport::from_bytes(&data).unwrap();
        assert_eq!(report.device_index(), 0x01);
        assert_eq!(report.feature_index(), 0x05);
        assert_eq!(report.function_id(), 0x02);
        assert_eq!(report.sw_id(), 0x01);
    }

    #[test]
    fn from_bytes_short() {
        let mut data = [0u8; SHORT_REPORT_LEN];
        data[0] = SHORT_REPORT_ID;
        data[1] = 0xFF;
        data[2] = 0x00;
        data[3] = 0x11; // function_id=1, sw_id=1

        let report = HidppReport::from_bytes(&data).unwrap();
        assert_eq!(report.device_index(), 0xFF);
        assert_eq!(report.function_id(), 0x01);
    }

    #[test]
    fn error_detection() {
        let mut report = HidppReport::new_long(0x01, ERROR_MSG, 0x00, 0x00);
        assert!(report.is_error());

        report.data[2] = 0x05;
        assert!(!report.is_error());
    }

    #[test]
    fn from_bytes_too_short_fails() {
        let data = [LONG_REPORT_ID, 0x01];
        let result = HidppReport::from_bytes(&data);
        assert!(matches!(result, Err(HidppError::InvalidLength { .. })));
    }
}
