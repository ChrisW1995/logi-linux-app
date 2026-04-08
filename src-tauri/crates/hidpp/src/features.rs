use crate::error::HidppError;
use crate::report::HidppReport;
use tracing::debug;

/// Well-known HID++ 2.0 feature IDs.
pub const FEATURE_ROOT: u16 = 0x0000;
pub const FEATURE_DEVICE_NAME: u16 = 0x0005;
pub const FEATURE_BATTERY_STATUS: u16 = 0x1000;
pub const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
pub const FEATURE_CHANGE_HOST: u16 = 0x1814;

/// Battery charge level (coarse classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum BatteryLevel {
    Full,
    Good,
    Low,
    Critical,
    Empty,
}

/// Battery charging status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ChargingStatus {
    Discharging,
    Recharging,
    Full,
    SlowRecharge,
    InvalidBattery,
    ThermalError,
    Unknown,
}

/// Combined battery information from either UnifiedBattery or BatteryStatus features.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatteryInfo {
    pub percentage: Option<u8>,
    pub level: BatteryLevel,
    pub status: ChargingStatus,
}

/// Software ID used in our requests (arbitrary, for matching responses).
const SW_ID: u8 = 0x01;

/// Trait for sending/receiving HID++ reports.
/// This abstraction allows testing without a physical device.
pub trait HidTransport: Send {
    fn write(&self, data: &[u8]) -> Result<usize, HidppError>;
    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, HidppError>;
}

/// HID++ 2.0 feature access.
pub struct FeatureAccess<T: HidTransport> {
    transport: T,
    device_index: u8,
}

impl<T: HidTransport> FeatureAccess<T> {
    pub fn new(transport: T, device_index: u8) -> Self {
        Self { transport, device_index }
    }

    /// Send a request and wait for the matching response.
    /// Skips non-matching reports (notifications, other responses).
    fn request(&self, report: &HidppReport, timeout_ms: i32) -> Result<HidppReport, HidppError> {
        self.transport.write(report.as_bytes())?;

        let expected_feature = report.feature_index();
        let expected_function = report.function_id();

        // Read responses, filtering for the one that matches our request
        let mut buf = [0u8; 64];
        for _ in 0..10 {
            let n = self.transport.read_timeout(&mut buf, timeout_ms)?;
            if n == 0 {
                return Err(HidppError::Timeout);
            }

            let response = HidppReport::from_bytes(&buf[..n])?;

            // Check for error response
            if response.is_error() && response.device_index() == self.device_index {
                let error_feature = response.params().first().copied().unwrap_or(0);
                if error_feature == expected_feature {
                    let error_code = response.params().get(1).copied().unwrap_or(0);
                    return Err(HidppError::ProtocolError {
                        function: expected_function,
                        error_code,
                    });
                }
            }

            // Check if this is our response
            if response.device_index() == self.device_index
                && response.feature_index() == expected_feature
                && response.function_id() == expected_function
            {
                return Ok(response);
            }

            debug!("Skipping non-matching report: feature={:#04X}", response.feature_index());
        }

        Err(HidppError::Timeout)
    }

    /// IRoot.GetFeatureIndex (feature 0x0000, function 0)
    /// Returns the feature index for the given feature ID.
    pub fn get_feature_index(&self, feature_id: u16) -> Result<u8, HidppError> {
        let mut report = HidppReport::new_long(self.device_index, 0x00, 0x00, SW_ID);
        report.set_param(0, (feature_id >> 8) as u8);
        report.set_param(1, (feature_id & 0xFF) as u8);

        debug!("GetFeatureIndex: feature_id=0x{feature_id:04X}");
        let response = self.request(&report, 2000)?;

        let index = response.params().first().copied().unwrap_or(0);
        if index == 0 {
            return Err(HidppError::FeatureNotFound { feature_id });
        }

        debug!("Feature 0x{feature_id:04X} -> index {index}");
        Ok(index)
    }

    /// ChangeHost.SetCurrentHost (feature 0x1814, function 0)
    /// Switches the device to the specified host channel (0-based: 0, 1, 2).
    pub fn change_host(&self, host_index: u8) -> Result<(), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_CHANGE_HOST)?;

        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        report.set_param(0, host_index);

        debug!("ChangeHost: host_index={host_index}, feature_index={feature_index}");
        let _response = self.request(&report, 2000)?;

        debug!("ChangeHost successful");
        Ok(())
    }

    /// UnifiedBattery.GetCapabilities (feature 0x1004, function 0)
    /// Returns battery info from modern Logitech devices.
    pub fn get_unified_battery(&self) -> Result<BatteryInfo, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_UNIFIED_BATTERY)?;
        let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        let response = self.request(&report, 2000)?;

        let params = response.params();
        let percentage = params.first().copied().unwrap_or(0);
        let level_mask = params.get(1).copied().unwrap_or(0);
        let status_byte = params.get(2).copied().unwrap_or(0);

        let level = match level_mask {
            m if m & 0x08 != 0 => BatteryLevel::Full,
            m if m & 0x04 != 0 => BatteryLevel::Good,
            m if m & 0x02 != 0 => BatteryLevel::Low,
            m if m & 0x01 != 0 => BatteryLevel::Critical,
            _ => BatteryLevel::Empty,
        };

        let status = match status_byte {
            0 => ChargingStatus::Discharging,
            1 => ChargingStatus::Recharging,
            2 | 3 => ChargingStatus::Full,
            6 => ChargingStatus::SlowRecharge,
            _ => ChargingStatus::Unknown,
        };

        debug!("UnifiedBattery: {percentage}%, level={level:?}, status={status:?}");
        Ok(BatteryInfo { percentage: Some(percentage), level, status })
    }

    /// BatteryStatus.GetBatteryLevelStatus (feature 0x1000, function 0)
    /// Fallback for older devices that don't support UnifiedBattery.
    pub fn get_battery_status(&self) -> Result<BatteryInfo, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_BATTERY_STATUS)?;
        let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        let response = self.request(&report, 2000)?;

        let params = response.params();
        let discharge_level = params.first().copied().unwrap_or(0);
        let _discharge_next = params.get(1).copied().unwrap_or(0);
        let status_byte = params.get(2).copied().unwrap_or(0);

        let level = match discharge_level {
            l if l >= 80 => BatteryLevel::Full,
            l if l >= 40 => BatteryLevel::Good,
            l if l >= 15 => BatteryLevel::Low,
            l if l > 0 => BatteryLevel::Critical,
            _ => BatteryLevel::Empty,
        };

        let status = match status_byte & 0x07 {
            0 => ChargingStatus::Discharging,
            1 => ChargingStatus::Recharging,
            2 => ChargingStatus::Full,
            _ => ChargingStatus::Unknown,
        };

        debug!("BatteryStatus: {discharge_level}%, status={status:?}");
        Ok(BatteryInfo { percentage: Some(discharge_level), level, status })
    }

    /// Try UnifiedBattery first, fall back to BatteryStatus.
    /// Returns Ok(None) if neither feature is supported.
    pub fn get_battery(&self) -> Result<Option<BatteryInfo>, HidppError> {
        match self.get_unified_battery() {
            Ok(info) => return Ok(Some(info)),
            Err(HidppError::FeatureNotFound { .. }) => {}
            Err(e) => return Err(e),
        }
        match self.get_battery_status() {
            Ok(info) => Ok(Some(info)),
            Err(HidppError::FeatureNotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// ChangeHost.GetHostCount (feature 0x1814, function 1)
    /// Returns the number of hosts the device supports.
    pub fn get_host_count(&self) -> Result<u8, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_CHANGE_HOST)?;

        let report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
        let response = self.request(&report, 2000)?;

        let count = response.params().first().copied().unwrap_or(0);
        debug!("HostCount: {count}");
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock HID transport that records writes and replays pre-configured responses.
    struct MockTransport {
        responses: RefCell<Vec<Vec<u8>>>,
        writes: RefCell<Vec<Vec<u8>>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                responses: RefCell::new(responses),
                writes: RefCell::new(Vec::new()),
            }
        }

        fn written(&self) -> Vec<Vec<u8>> {
            self.writes.borrow().clone()
        }
    }

    impl HidTransport for MockTransport {
        fn write(&self, data: &[u8]) -> Result<usize, HidppError> {
            self.writes.borrow_mut().push(data.to_vec());
            Ok(data.len())
        }

        fn read_timeout(&self, buf: &mut [u8], _timeout_ms: i32) -> Result<usize, HidppError> {
            let mut responses = self.responses.borrow_mut();
            if responses.is_empty() {
                return Err(HidppError::Timeout);
            }
            let response = responses.remove(0);
            let len = response.len().min(buf.len());
            buf[..len].copy_from_slice(&response[..len]);
            Ok(len)
        }
    }

    fn make_get_feature_response(device_index: u8, feature_index: u8) -> Vec<u8> {
        let mut data = vec![0u8; 20];
        data[0] = 0x11; // long report
        data[1] = device_index;
        data[2] = 0x00; // IRoot feature index
        data[3] = 0x01; // function_id=0, sw_id=1
        data[4] = feature_index;
        data
    }

    fn make_change_host_response(device_index: u8, feature_index: u8) -> Vec<u8> {
        let mut data = vec![0u8; 20];
        data[0] = 0x11;
        data[1] = device_index;
        data[2] = feature_index;
        data[3] = 0x01; // function_id=0, sw_id=1
        data
    }

    #[test]
    fn get_feature_index_success() {
        let response = make_get_feature_response(0x01, 0x07);
        let transport = MockTransport::new(vec![response]);
        let access = FeatureAccess::new(transport, 0x01);

        let index = access.get_feature_index(FEATURE_CHANGE_HOST).unwrap();
        assert_eq!(index, 0x07);
    }

    #[test]
    fn get_feature_index_not_found() {
        // index=0 means feature not found
        let response = make_get_feature_response(0x01, 0x00);
        let transport = MockTransport::new(vec![response]);
        let access = FeatureAccess::new(transport, 0x01);

        let result = access.get_feature_index(0x9999);
        assert!(matches!(result, Err(HidppError::FeatureNotFound { feature_id: 0x9999 })));
    }

    #[test]
    fn change_host_sends_correct_reports() {
        let get_feature_resp = make_get_feature_response(0x01, 0x07);
        let change_host_resp = make_change_host_response(0x01, 0x07);
        let transport = MockTransport::new(vec![get_feature_resp, change_host_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.change_host(1).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes.len(), 2);

        // First write: GetFeatureIndex(0x1814)
        assert_eq!(writes[0][0], 0x11); // long report
        assert_eq!(writes[0][1], 0x01); // device index
        assert_eq!(writes[0][2], 0x00); // IRoot feature index
        assert_eq!(writes[0][4], 0x18); // feature_id high byte
        assert_eq!(writes[0][5], 0x14); // feature_id low byte

        // Second write: ChangeHost(host_index=1)
        assert_eq!(writes[1][0], 0x11); // long report
        assert_eq!(writes[1][1], 0x01); // device index
        assert_eq!(writes[1][2], 0x07); // ChangeHost feature index
        assert_eq!(writes[1][4], 0x01); // host_index
    }

    #[test]
    fn get_host_count() {
        let get_feature_resp = make_get_feature_response(0x01, 0x07);
        let mut host_count_resp = vec![0u8; 20];
        host_count_resp[0] = 0x11;
        host_count_resp[1] = 0x01;
        host_count_resp[2] = 0x07; // ChangeHost feature index
        host_count_resp[3] = 0x11; // function_id=1, sw_id=1
        host_count_resp[4] = 3;    // 3 hosts

        let transport = MockTransport::new(vec![get_feature_resp, host_count_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let count = access.get_host_count().unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn error_response_detected() {
        let mut error_resp = vec![0u8; 20];
        error_resp[0] = 0x11;
        error_resp[1] = 0x01;
        error_resp[2] = 0xFF; // ERROR_MSG
        error_resp[3] = 0x01;
        error_resp[4] = 0x00; // feature that caused error = IRoot
        error_resp[5] = 0x02; // error code = InvalidArgument

        let transport = MockTransport::new(vec![error_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let result = access.get_feature_index(0x1814);
        assert!(matches!(result, Err(HidppError::ProtocolError { function: 0, error_code: 2 })));
    }

    fn make_battery_response(device_index: u8, feature_index: u8, function_id: u8, params: &[u8]) -> Vec<u8> {
        let mut data = vec![0u8; 20];
        data[0] = 0x11;
        data[1] = device_index;
        data[2] = feature_index;
        data[3] = (function_id << 4) | SW_ID;
        for (i, &v) in params.iter().enumerate() {
            data[4 + i] = v;
        }
        data
    }

    #[test]
    fn unified_battery_full_charged() {
        let get_feat = make_get_feature_response(0x01, 0x05); // feature index 5
        let bat_resp = make_battery_response(0x01, 0x05, 0x00, &[100, 0x08, 2]); // 100%, Full, Full
        let transport = MockTransport::new(vec![get_feat, bat_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_unified_battery().unwrap();
        assert_eq!(info.percentage, Some(100));
        assert_eq!(info.level, BatteryLevel::Full);
        assert_eq!(info.status, ChargingStatus::Full);
    }

    #[test]
    fn unified_battery_low_discharging() {
        let get_feat = make_get_feature_response(0x01, 0x05);
        let bat_resp = make_battery_response(0x01, 0x05, 0x00, &[15, 0x02, 0]); // 15%, Low, Discharging
        let transport = MockTransport::new(vec![get_feat, bat_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_unified_battery().unwrap();
        assert_eq!(info.percentage, Some(15));
        assert_eq!(info.level, BatteryLevel::Low);
        assert_eq!(info.status, ChargingStatus::Discharging);
    }

    #[test]
    fn battery_fallback_to_status() {
        // UnifiedBattery not found (index=0), then BatteryStatus found
        let ub_not_found = make_get_feature_response(0x01, 0x00); // index 0 = not found
        let bs_found = make_get_feature_response(0x01, 0x06);     // BatteryStatus at index 6
        let bs_resp = make_battery_response(0x01, 0x06, 0x00, &[85, 80, 0]); // 85%, discharging
        let transport = MockTransport::new(vec![ub_not_found, bs_found, bs_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_battery().unwrap().unwrap();
        assert_eq!(info.percentage, Some(85));
        assert_eq!(info.level, BatteryLevel::Full);
        assert_eq!(info.status, ChargingStatus::Discharging);
    }

    #[test]
    fn battery_no_feature() {
        // Neither UnifiedBattery nor BatteryStatus found
        let ub_not_found = make_get_feature_response(0x01, 0x00);
        let bs_not_found = make_get_feature_response(0x01, 0x00);
        let transport = MockTransport::new(vec![ub_not_found, bs_not_found]);
        let access = FeatureAccess::new(transport, 0x01);

        let result = access.get_battery().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn skips_non_matching_reports() {
        // First response is a notification (different feature index)
        let mut notification = vec![0u8; 20];
        notification[0] = 0x11;
        notification[1] = 0x01;
        notification[2] = 0x08; // some other feature
        notification[3] = 0x01;

        // Second response is the actual GetFeatureIndex reply
        let real_response = make_get_feature_response(0x01, 0x07);

        let transport = MockTransport::new(vec![notification, real_response]);
        let access = FeatureAccess::new(transport, 0x01);

        let index = access.get_feature_index(FEATURE_CHANGE_HOST).unwrap();
        assert_eq!(index, 0x07);
    }
}
