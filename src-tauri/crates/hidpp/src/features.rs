use crate::error::HidppError;
use crate::report::HidppReport;
use tracing::debug;

/// Well-known HID++ 2.0 feature IDs.
pub const FEATURE_ROOT: u16 = 0x0000;
pub const FEATURE_CRYPTO_IDENTIFIER: u16 = 0x0021;
pub const FEATURE_DEVICE_FW_VERSION: u16 = 0x0003;
pub const FEATURE_DEVICE_NAME: u16 = 0x0005;
pub const FEATURE_BATTERY_STATUS: u16 = 0x1000;
pub const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
pub const FEATURE_CHANGE_HOST: u16 = 0x1814;
pub const FEATURE_REPROG_CONTROLS_V4: u16 = 0x1B04;
pub const FEATURE_SMART_SHIFT: u16 = 0x2110;
pub const FEATURE_SMART_SHIFT_ENHANCED: u16 = 0x2111;
pub const FEATURE_HIRES_WHEEL: u16 = 0x2121;
pub const FEATURE_ADJUSTABLE_DPI: u16 = 0x2201;

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

/// Firmware entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum FirmwareKind {
    Main,
    Bootloader,
    Hardware,
    Other,
}

/// Firmware version info for a single entity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FirmwareInfo {
    pub kind: FirmwareKind,
    pub prefix: String,
    pub version: String,
    pub build: u16,
}

/// DPI settings read from device.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DpiInfo {
    pub current_dpi: u16,
    pub default_dpi: u16,
    pub dpi_list: Vec<u16>,
}

/// SmartShift settings.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SmartShiftInfo {
    pub mode: u8,
    pub threshold: u8,
    pub is_enhanced: bool,
}

/// Hi-res scroll wheel settings.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HiResWheelInfo {
    pub multiplier: u8,
    pub has_invert: bool,
    pub has_ratchet: bool,
    pub hi_res: bool,
    pub invert: bool,
    pub ratchet: bool,
}

/// A programmable control (button) on the device.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReprogControl {
    pub cid: u16,
    pub task_id: u16,
    pub flags: u16,
    pub position: u8,
    pub group: u8,
    pub group_mask: u8,
    pub mapped_to: u16,
    pub name: String,
    pub is_remappable: bool,
}

/// Device capability flags (which features are supported).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceCapabilities {
    pub has_firmware_version: bool,
    pub has_dpi: bool,
    pub has_smart_shift: bool,
    pub has_smart_shift_enhanced: bool,
    pub has_hires_wheel: bool,
    pub has_reprog_controls: bool,
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
        Self {
            transport,
            device_index,
        }
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

            debug!(
                "Skipping non-matching report: feature={:#04X}",
                response.feature_index()
            );
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

    /// CryptoIdentifier.GetIdentifier (feature 0x0021, functions 0 and 1).
    ///
    /// Flow uses this stable 32-byte device value when deriving the shared
    /// discovery key. Logitech exposes it as two 16-byte HID++ responses.
    pub fn get_crypto_identifier(&self) -> Result<[u8; 32], HidppError> {
        let feature_index = self.get_feature_index(FEATURE_CRYPTO_IDENTIFIER)?;
        let mut identifier = [0u8; 32];

        for chunk_index in 0..2u8 {
            let report =
                HidppReport::new_long(self.device_index, feature_index, chunk_index, SW_ID);
            let response = self.request(&report, 2000)?;
            let params = response.params();
            if params.len() < 16 {
                return Err(HidppError::InvalidLength {
                    expected: 16,
                    got: params.len(),
                });
            }

            let offset = chunk_index as usize * 16;
            identifier[offset..offset + 16].copy_from_slice(&params[..16]);
        }

        Ok(identifier)
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

    /// UnifiedBattery.GetStatus (feature 0x1004, function 1)
    /// Returns battery info from modern Logitech devices.
    pub fn get_unified_battery(&self) -> Result<BatteryInfo, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_UNIFIED_BATTERY)?;
        let report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
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
        Ok(BatteryInfo {
            percentage: Some(percentage),
            level,
            status,
        })
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
        Ok(BatteryInfo {
            percentage: Some(discharge_level),
            level,
            status,
        })
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

    /// DeviceFwVersion.getEntityCount (feature 0x0003, function 0)
    /// then DeviceFwVersion.getFwInfo (function 1) for each entity.
    /// Returns firmware info for all entities (main fw, bootloader, hardware).
    pub fn get_firmware_version(&self) -> Result<Vec<FirmwareInfo>, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_DEVICE_FW_VERSION)?;

        // Function 0: getEntityCount
        let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        let response = self.request(&report, 2000)?;
        let count = response.params().first().copied().unwrap_or(0);
        debug!("FwVersion entity count: {count}");

        let mut firmware = Vec::new();
        for index in 0..count {
            // Function 1: getFwInfo(index)
            let mut report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
            report.set_param(0, index);
            let response = self.request(&report, 2000)?;
            let params = response.params();

            let level = params.first().copied().unwrap_or(0) & 0x0F;
            let kind = match level {
                0 => FirmwareKind::Main,
                1 => FirmwareKind::Bootloader,
                2 => FirmwareKind::Hardware,
                _ => FirmwareKind::Other,
            };

            let (prefix, version, build) = if level == 0 || level == 1 {
                // Bytes 1-3: 3-char ASCII name, 4: major, 5: minor, 6-7: build (BE u16)
                let name_bytes = &params[1..4.min(params.len())];
                let prefix = String::from_utf8_lossy(name_bytes).to_string();
                let major = params.get(4).copied().unwrap_or(0);
                let minor = params.get(5).copied().unwrap_or(0);
                let build_hi = params.get(6).copied().unwrap_or(0) as u16;
                let build_lo = params.get(7).copied().unwrap_or(0) as u16;
                let build = (build_hi << 8) | build_lo;
                let version = if build > 0 {
                    format!("{major:02X}.{minor:02X}.B{build:04X}")
                } else {
                    format!("{major:02X}.{minor:02X}")
                };
                (prefix, version, build)
            } else if level == 2 {
                // Hardware: byte 1 = revision
                let rev = params.get(1).copied().unwrap_or(0);
                (String::new(), format!("{rev}"), 0)
            } else {
                (String::new(), String::new(), 0)
            };

            debug!("FwInfo[{index}]: kind={kind:?}, prefix={prefix:?}, version={version}");
            firmware.push(FirmwareInfo {
                kind,
                prefix,
                version,
                build,
            });
        }

        Ok(firmware)
    }

    // ── AdjustableDPI (0x2201) ──

    /// Reads the list of DPI values supported by the device sensor.
    /// Handles both individual values and range-encoded entries.
    pub fn get_dpi_list(&self) -> Result<Vec<u16>, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_ADJUSTABLE_DPI)?;

        let mut raw_bytes = Vec::new();
        for page in 0..0x10u8 {
            // Function 1: getSensorDpiList(sensor=0, direction=0, page)
            let mut report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
            report.set_param(0, 0x00); // sensor index
            report.set_param(1, 0x00); // direction
            report.set_param(2, page);
            let response = self.request(&report, 2000)?;
            let params = response.params();

            // Skip first byte (sensor echo), collect the rest
            if params.len() > 1 {
                raw_bytes.extend_from_slice(&params[1..]);
            }
            // Stop if last two bytes are 0x0000 (terminator)
            if raw_bytes.len() >= 2
                && raw_bytes[raw_bytes.len() - 2] == 0
                && raw_bytes[raw_bytes.len() - 1] == 0
            {
                break;
            }
        }

        // Decode DPI list: individual values (2B) or ranges (4B with flag)
        let mut dpi_list = Vec::new();
        let mut i = 0;
        while i + 1 < raw_bytes.len() {
            let val = ((raw_bytes[i] as u16) << 8) | (raw_bytes[i + 1] as u16);
            if val == 0 {
                break;
            }
            if val >> 13 == 0b111 {
                // Range: step = val & 0x1FFF, next 2 bytes = end value
                let step = val & 0x1FFF;
                if i + 3 < raw_bytes.len() && step > 0 {
                    let last = ((raw_bytes[i + 2] as u16) << 8) | (raw_bytes[i + 3] as u16);
                    if let Some(&prev) = dpi_list.last() {
                        let mut v = prev + step;
                        while v <= last {
                            dpi_list.push(v);
                            v += step;
                        }
                    }
                    i += 4;
                } else {
                    break;
                }
            } else {
                dpi_list.push(val);
                i += 2;
            }
        }

        debug!(
            "DPI list ({} values): {:?}",
            dpi_list.len(),
            &dpi_list[..dpi_list.len().min(5)]
        );
        Ok(dpi_list)
    }

    /// Reads the current and default DPI for sensor 0.
    pub fn get_dpi(&self) -> Result<(u16, u16), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_ADJUSTABLE_DPI)?;

        // Function 2: getSensorDpi(sensor=0)
        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x02, SW_ID);
        report.set_param(0, 0x00); // sensor index
        let response = self.request(&report, 2000)?;
        let params = response.params();

        // Bytes: [sensor], [current_hi, current_lo], [default_hi, default_lo]
        let current = ((params.get(1).copied().unwrap_or(0) as u16) << 8)
            | (params.get(2).copied().unwrap_or(0) as u16);
        let default = ((params.get(3).copied().unwrap_or(0) as u16) << 8)
            | (params.get(4).copied().unwrap_or(0) as u16);

        // If current is 0, use default
        let current = if current == 0 { default } else { current };

        debug!("DPI: current={current}, default={default}");
        Ok((current, default))
    }

    /// Sets the DPI for sensor 0.
    pub fn set_dpi(&self, dpi: u16) -> Result<(), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_ADJUSTABLE_DPI)?;

        // Function 3: setSensorDpi(sensor=0, dpi_hi, dpi_lo)
        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x03, SW_ID);
        report.set_param(0, 0x00); // sensor index
        report.set_param(1, (dpi >> 8) as u8);
        report.set_param(2, (dpi & 0xFF) as u8);

        debug!("SetDPI: {dpi}");
        let _response = self.request(&report, 2000)?;
        Ok(())
    }

    /// Convenience: reads DPI list + current/default into DpiInfo.
    pub fn get_dpi_settings(&self) -> Result<DpiInfo, HidppError> {
        let dpi_list = self.get_dpi_list()?;
        let (current_dpi, default_dpi) = self.get_dpi()?;
        Ok(DpiInfo {
            current_dpi,
            default_dpi,
            dpi_list,
        })
    }

    // ── SmartShift (0x2110 / 0x2111) ──

    /// Reads SmartShift settings. Tries enhanced (0x2111) first, falls back to standard (0x2110).
    pub fn get_smart_shift(&self) -> Result<SmartShiftInfo, HidppError> {
        // Try enhanced first
        match self.get_feature_index(FEATURE_SMART_SHIFT_ENHANCED) {
            Ok(feature_index) => {
                // Enhanced: read function = 0x01
                let report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
                let response = self.request(&report, 2000)?;
                let params = response.params();
                let mode = params.first().copied().unwrap_or(0);
                let threshold = params.get(1).copied().unwrap_or(0);
                debug!("SmartShiftEnhanced: mode={mode}, threshold={threshold}");
                Ok(SmartShiftInfo {
                    mode,
                    threshold,
                    is_enhanced: true,
                })
            }
            Err(HidppError::FeatureNotFound { .. }) => {
                // Try standard
                let feature_index = self.get_feature_index(FEATURE_SMART_SHIFT)?;
                // Standard: read function = 0x00
                let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
                let response = self.request(&report, 2000)?;
                let params = response.params();
                let mode = params.first().copied().unwrap_or(0);
                let threshold = params.get(1).copied().unwrap_or(0);
                debug!("SmartShift: mode={mode}, threshold={threshold}");
                Ok(SmartShiftInfo {
                    mode,
                    threshold,
                    is_enhanced: false,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Sets SmartShift mode and threshold.
    pub fn set_smart_shift(&self, mode: u8, threshold: u8) -> Result<(), HidppError> {
        // Try enhanced first
        match self.get_feature_index(FEATURE_SMART_SHIFT_ENHANCED) {
            Ok(feature_index) => {
                // Enhanced: write function = 0x02
                let mut report =
                    HidppReport::new_long(self.device_index, feature_index, 0x02, SW_ID);
                report.set_param(0, mode);
                report.set_param(1, threshold);
                debug!("SetSmartShiftEnhanced: mode={mode}, threshold={threshold}");
                let _response = self.request(&report, 2000)?;
                Ok(())
            }
            Err(HidppError::FeatureNotFound { .. }) => {
                let feature_index = self.get_feature_index(FEATURE_SMART_SHIFT)?;
                // Standard: write function = 0x01
                let mut report =
                    HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
                report.set_param(0, mode);
                report.set_param(1, threshold);
                debug!("SetSmartShift: mode={mode}, threshold={threshold}");
                let _response = self.request(&report, 2000)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    // ── HiResWheel (0x2121) ──

    /// Reads hi-res wheel capabilities and current mode.
    pub fn get_hires_wheel(&self) -> Result<HiResWheelInfo, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_HIRES_WHEEL)?;

        // Function 0: getCapabilities
        let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        let response = self.request(&report, 2000)?;
        let params = response.params();
        let multiplier = params.first().copied().unwrap_or(0);
        let cap_flags = params.get(1).copied().unwrap_or(0);
        let has_ratchet = cap_flags & 0x04 != 0;
        let has_invert = cap_flags & 0x08 != 0;

        // Function 1: getMode
        let report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
        let response = self.request(&report, 2000)?;
        let mode_byte = response.params().first().copied().unwrap_or(0);
        let hi_res = mode_byte & 0x02 != 0;
        let invert = mode_byte & 0x04 != 0;

        // Function 3: getRatchet
        let report = HidppReport::new_long(self.device_index, feature_index, 0x03, SW_ID);
        let response = self.request(&report, 2000)?;
        let ratchet = response.params().first().copied().unwrap_or(0) & 0x01 != 0;

        debug!("HiResWheel: multiplier={multiplier}, hi_res={hi_res}, invert={invert}, ratchet={ratchet}");
        Ok(HiResWheelInfo {
            multiplier,
            has_invert,
            has_ratchet,
            hi_res,
            invert,
            ratchet,
        })
    }

    /// Sets hi-res wheel mode (resolution + invert).
    pub fn set_hires_wheel_mode(&self, hi_res: bool, invert: bool) -> Result<(), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_HIRES_WHEEL)?;

        let mode_byte = (if hi_res { 0x02 } else { 0 }) | (if invert { 0x04 } else { 0 });
        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x02, SW_ID);
        report.set_param(0, mode_byte);

        debug!("SetHiResWheelMode: hi_res={hi_res}, invert={invert}");
        let _response = self.request(&report, 2000)?;
        Ok(())
    }

    /// Sets hi-res wheel ratchet mode.
    pub fn set_hires_wheel_ratchet(&self, ratchet: bool) -> Result<(), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_HIRES_WHEEL)?;

        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x04, SW_ID);
        report.set_param(0, if ratchet { 0x01 } else { 0x00 });

        debug!("SetHiResWheelRatchet: ratchet={ratchet}");
        let _response = self.request(&report, 2000)?;
        Ok(())
    }

    // ── ReprogControls v4 (0x1B04) ──

    /// Enumerates all programmable controls and their current mappings.
    pub fn get_reprog_controls(&self) -> Result<Vec<ReprogControl>, HidppError> {
        let feature_index = self.get_feature_index(FEATURE_REPROG_CONTROLS_V4)?;

        // Function 0: getControlCount
        let report = HidppReport::new_long(self.device_index, feature_index, 0x00, SW_ID);
        let response = self.request(&report, 2000)?;
        let count = response.params().first().copied().unwrap_or(0);
        debug!("ReprogControls: {count} controls");

        let mut controls = Vec::new();
        for index in 0..count {
            // Function 1: getControlInfo(index)
            let mut report = HidppReport::new_long(self.device_index, feature_index, 0x01, SW_ID);
            report.set_param(0, index);
            let response = self.request(&report, 2000)?;
            let p = response.params();

            let cid = ((p.get(0).copied().unwrap_or(0) as u16) << 8)
                | (p.get(1).copied().unwrap_or(0) as u16);
            let task_id = ((p.get(2).copied().unwrap_or(0) as u16) << 8)
                | (p.get(3).copied().unwrap_or(0) as u16);
            let flags1 = p.get(4).copied().unwrap_or(0) as u16;
            let position = p.get(5).copied().unwrap_or(0);
            let group = p.get(6).copied().unwrap_or(0);
            let group_mask = p.get(7).copied().unwrap_or(0);
            let flags2 = p.get(8).copied().unwrap_or(0) as u16;
            let flags = (flags2 << 8) | flags1;
            // Divertable flag (bit 6 of flags1) indicates remappable
            let is_remappable = flags1 & 0x40 != 0;

            // Function 2: getControlMapping(cid)
            let mut report = HidppReport::new_long(self.device_index, feature_index, 0x02, SW_ID);
            report.set_param(0, (cid >> 8) as u8);
            report.set_param(1, (cid & 0xFF) as u8);
            let response = self.request(&report, 2000)?;
            let mp = response.params();
            // Response: CID(2B) + MappingFlags(1B) + MappedTo(2B)
            let mapped_to = ((mp.get(3).copied().unwrap_or(0) as u16) << 8)
                | (mp.get(4).copied().unwrap_or(0) as u16);

            let name = crate::cid_names::cid_name(cid).to_string();

            controls.push(ReprogControl {
                cid,
                task_id,
                flags,
                position,
                group,
                group_mask,
                mapped_to,
                name,
                is_remappable,
            });
        }

        Ok(controls)
    }

    /// Remaps a control (button) to a different function.
    pub fn set_reprog_control_mapping(&self, cid: u16, remap_to: u16) -> Result<(), HidppError> {
        let feature_index = self.get_feature_index(FEATURE_REPROG_CONTROLS_V4)?;

        // Function 3: setControlMapping(cid, flags=0, remap_to)
        let mut report = HidppReport::new_long(self.device_index, feature_index, 0x03, SW_ID);
        report.set_param(0, (cid >> 8) as u8);
        report.set_param(1, (cid & 0xFF) as u8);
        report.set_param(2, 0x00); // mapping flags
        report.set_param(3, (remap_to >> 8) as u8);
        report.set_param(4, (remap_to & 0xFF) as u8);

        debug!("SetReprogControl: CID=0x{cid:04X} -> 0x{remap_to:04X}");
        let _response = self.request(&report, 2000)?;
        Ok(())
    }

    // ── Feature Discovery ──

    /// Probes the device for all known features and returns capability flags.
    pub fn discover_capabilities(&self) -> DeviceCapabilities {
        let check = |feature_id: u16| -> bool {
            matches!(self.get_feature_index(feature_id), Ok(idx) if idx > 0)
        };

        DeviceCapabilities {
            has_firmware_version: check(FEATURE_DEVICE_FW_VERSION),
            has_dpi: check(FEATURE_ADJUSTABLE_DPI),
            has_smart_shift: check(FEATURE_SMART_SHIFT) || check(FEATURE_SMART_SHIFT_ENHANCED),
            has_smart_shift_enhanced: check(FEATURE_SMART_SHIFT_ENHANCED),
            has_hires_wheel: check(FEATURE_HIRES_WHEEL),
            has_reprog_controls: check(FEATURE_REPROG_CONTROLS_V4),
        }
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
        assert!(matches!(
            result,
            Err(HidppError::FeatureNotFound { feature_id: 0x9999 })
        ));
    }

    #[test]
    fn crypto_identifier_reads_both_chunks() {
        let feature_index = 0x0c;
        let get_feature = make_get_feature_response(0x01, feature_index);
        let first: Vec<u8> = (0u8..16).collect();
        let second: Vec<u8> = (16u8..32).collect();
        let first_response = make_feature_response(0x01, feature_index, 0, &first);
        let second_response = make_feature_response(0x01, feature_index, 1, &second);
        let transport = MockTransport::new(vec![get_feature, first_response, second_response]);
        let access = FeatureAccess::new(transport, 0x01);

        let identifier = access.get_crypto_identifier().unwrap();

        assert_eq!(identifier, std::array::from_fn(|index| index as u8));
        let writes = access.transport.written();
        assert_eq!(writes.len(), 3);
        assert_eq!(&writes[0][4..6], &[0x00, 0x21]);
        assert_eq!(writes[1][3], 0x01);
        assert_eq!(writes[2][3], 0x11);
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
        host_count_resp[4] = 3; // 3 hosts

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
        assert!(matches!(
            result,
            Err(HidppError::ProtocolError {
                function: 0,
                error_code: 2
            })
        ));
    }

    fn make_battery_response(
        device_index: u8,
        feature_index: u8,
        function_id: u8,
        params: &[u8],
    ) -> Vec<u8> {
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
        let bat_resp = make_battery_response(0x01, 0x05, 0x01, &[100, 0x08, 2]); // 100%, Full, Full
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
        let bat_resp = make_battery_response(0x01, 0x05, 0x01, &[15, 0x02, 0]); // 15%, Low, Discharging
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
        let bs_found = make_get_feature_response(0x01, 0x06); // BatteryStatus at index 6
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

    fn make_feature_response(
        device_index: u8,
        feature_index: u8,
        function_id: u8,
        params: &[u8],
    ) -> Vec<u8> {
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
    fn firmware_version_main_and_bootloader() {
        let feat_idx = 0x04;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // getEntityCount response: count=2, rest is unit/model ID (ignored here)
        let count_resp = make_feature_response(0x01, feat_idx, 0x00, &[2]);

        // getFwInfo(0): Main firmware - level=0, name="MPM", major=0x0A, minor=0x02, build=0x0005
        let fw0_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x00, // level=0 (Main)
                b'M', b'P', b'M', // 3-char name
                0x0A, 0x02, // major, minor
                0x00, 0x05, // build BE = 5
            ],
        );
        // getFwInfo(1): Bootloader - level=1, name="BOT", major=0x01, minor=0x00, build=0
        let fw1_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x01, // level=1 (Bootloader)
                b'B', b'O', b'T', 0x01, 0x00, 0x00, 0x00, // no build
            ],
        );

        let transport = MockTransport::new(vec![get_feat, count_resp, fw0_resp, fw1_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let fw = access.get_firmware_version().unwrap();
        assert_eq!(fw.len(), 2);

        assert_eq!(fw[0].kind, FirmwareKind::Main);
        assert_eq!(fw[0].prefix, "MPM");
        assert_eq!(fw[0].version, "0A.02.B0005");
        assert_eq!(fw[0].build, 5);

        assert_eq!(fw[1].kind, FirmwareKind::Bootloader);
        assert_eq!(fw[1].prefix, "BOT");
        assert_eq!(fw[1].version, "01.00");
        assert_eq!(fw[1].build, 0);
    }

    #[test]
    fn firmware_version_hardware_entity() {
        let feat_idx = 0x04;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let count_resp = make_feature_response(0x01, feat_idx, 0x00, &[1]);
        // Hardware entity: level=2, revision=5
        let fw_resp = make_feature_response(0x01, feat_idx, 0x01, &[0x02, 0x05]);

        let transport = MockTransport::new(vec![get_feat, count_resp, fw_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let fw = access.get_firmware_version().unwrap();
        assert_eq!(fw.len(), 1);
        assert_eq!(fw[0].kind, FirmwareKind::Hardware);
        assert_eq!(fw[0].version, "5");
    }

    #[test]
    fn firmware_version_not_supported() {
        let not_found = make_get_feature_response(0x01, 0x00);
        let transport = MockTransport::new(vec![not_found]);
        let access = FeatureAccess::new(transport, 0x01);

        let result = access.get_firmware_version();
        assert!(matches!(
            result,
            Err(HidppError::FeatureNotFound { feature_id: 0x0003 })
        ));
    }

    // ── DPI tests ──

    #[test]
    fn dpi_list_individual_values() {
        let feat_idx = 0x08;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // getSensorDpiList response: [sensor_echo, DPI values..., 0x0000 terminator]
        // DPI values: 400 (0x0190), 800 (0x0320), 1600 (0x0640)
        let list_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x00, // sensor echo
                0x01, 0x90, // 400
                0x03, 0x20, // 800
                0x06, 0x40, // 1600
                0x00, 0x00, // terminator
            ],
        );
        let transport = MockTransport::new(vec![get_feat, list_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let list = access.get_dpi_list().unwrap();
        assert_eq!(list, vec![400, 800, 1600]);
    }

    #[test]
    fn dpi_list_with_range() {
        let feat_idx = 0x08;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // DPI values: 200 (individual), then range step=200 up to 4000
        // Range marker: top 3 bits = 111, step = 200 (0xC8)
        // 0b111_00000_11001000 = 0xE0C8
        let list_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x00, // sensor echo
                0x00, 0xC8, // 200 (individual)
                0xE0, 0xC8, // range: step=200
                0x03, 0xE8, // end=1000
                0x00, 0x00, // terminator
            ],
        );
        let transport = MockTransport::new(vec![get_feat, list_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let list = access.get_dpi_list().unwrap();
        // 200 (individual), then range from 200+200=400 to 1000 step 200: 400, 600, 800, 1000
        assert_eq!(list, vec![200, 400, 600, 800, 1000]);
    }

    #[test]
    fn get_dpi_current_and_default() {
        let feat_idx = 0x08;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // getSensorDpi response: [sensor, current_hi, current_lo, default_hi, default_lo]
        let dpi_resp = make_feature_response(
            0x01,
            feat_idx,
            0x02,
            &[
                0x00, // sensor
                0x06, 0x40, // current = 1600
                0x03, 0x20, // default = 800
            ],
        );
        let transport = MockTransport::new(vec![get_feat, dpi_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let (current, default) = access.get_dpi().unwrap();
        assert_eq!(current, 1600);
        assert_eq!(default, 800);
    }

    #[test]
    fn get_dpi_zero_uses_default() {
        let feat_idx = 0x08;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let dpi_resp = make_feature_response(
            0x01,
            feat_idx,
            0x02,
            &[
                0x00, // sensor
                0x00, 0x00, // current = 0 (use default)
                0x03, 0x20, // default = 800
            ],
        );
        let transport = MockTransport::new(vec![get_feat, dpi_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let (current, _default) = access.get_dpi().unwrap();
        assert_eq!(current, 800); // falls back to default
    }

    #[test]
    fn set_dpi_sends_correct_bytes() {
        let feat_idx = 0x08;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let set_resp = make_feature_response(0x01, feat_idx, 0x03, &[]);
        let transport = MockTransport::new(vec![get_feat, set_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.set_dpi(1600).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes.len(), 2);
        // Second write: setSensorDpi
        assert_eq!(writes[1][2], feat_idx); // feature index
        assert_eq!(writes[1][3], 0x31); // function_id=3, sw_id=1
        assert_eq!(writes[1][4], 0x00); // sensor
        assert_eq!(writes[1][5], 0x06); // dpi hi (1600 = 0x0640)
        assert_eq!(writes[1][6], 0x40); // dpi lo
    }

    // ── SmartShift tests ──

    #[test]
    fn smart_shift_enhanced_read() {
        let feat_idx = 0x09;
        let get_feat = make_get_feature_response(0x01, feat_idx); // 0x2111 found
        let read_resp = make_feature_response(0x01, feat_idx, 0x01, &[2, 15]); // mode=2 (smart), threshold=15
        let transport = MockTransport::new(vec![get_feat, read_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_smart_shift().unwrap();
        assert_eq!(info.mode, 2);
        assert_eq!(info.threshold, 15);
        assert!(info.is_enhanced);
    }

    #[test]
    fn smart_shift_standard_fallback() {
        let enhanced_not_found = make_get_feature_response(0x01, 0x00); // 0x2111 not found
        let feat_idx = 0x09;
        let standard_found = make_get_feature_response(0x01, feat_idx); // 0x2110 found
        let read_resp = make_feature_response(0x01, feat_idx, 0x00, &[1, 30]); // mode=1 (freespin), threshold=30
        let transport = MockTransport::new(vec![enhanced_not_found, standard_found, read_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_smart_shift().unwrap();
        assert_eq!(info.mode, 1);
        assert_eq!(info.threshold, 30);
        assert!(!info.is_enhanced);
    }

    #[test]
    fn set_smart_shift_enhanced() {
        let feat_idx = 0x09;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let set_resp = make_feature_response(0x01, feat_idx, 0x02, &[]);
        let transport = MockTransport::new(vec![get_feat, set_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.set_smart_shift(2, 20).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes[1][3], 0x21); // function_id=2 (enhanced write), sw_id=1
        assert_eq!(writes[1][4], 2); // mode
        assert_eq!(writes[1][5], 20); // threshold
    }

    // ── HiResWheel tests ──

    #[test]
    fn hires_wheel_read() {
        let feat_idx = 0x0A;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // getCapabilities: multiplier=8, flags: has_ratchet(0x04) + has_invert(0x08) = 0x0C
        let caps_resp = make_feature_response(0x01, feat_idx, 0x00, &[8, 0x0C]);
        // getMode: hi_res(0x02) + invert(0x04) = 0x06
        let mode_resp = make_feature_response(0x01, feat_idx, 0x01, &[0x06]);
        // getRatchet: ratcheted = 1
        let ratchet_resp = make_feature_response(0x01, feat_idx, 0x03, &[0x01]);
        let transport = MockTransport::new(vec![get_feat, caps_resp, mode_resp, ratchet_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        let info = access.get_hires_wheel().unwrap();
        assert_eq!(info.multiplier, 8);
        assert!(info.has_invert);
        assert!(info.has_ratchet);
        assert!(info.hi_res);
        assert!(info.invert);
        assert!(info.ratchet);
    }

    #[test]
    fn set_hires_wheel_mode_compose_bits() {
        let feat_idx = 0x0A;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let set_resp = make_feature_response(0x01, feat_idx, 0x02, &[]);
        let transport = MockTransport::new(vec![get_feat, set_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.set_hires_wheel_mode(true, false).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes[1][4], 0x02); // hi_res bit only
    }

    #[test]
    fn set_hires_wheel_ratchet() {
        let feat_idx = 0x0A;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let set_resp = make_feature_response(0x01, feat_idx, 0x04, &[]);
        let transport = MockTransport::new(vec![get_feat, set_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.set_hires_wheel_ratchet(false).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes[1][4], 0x00); // freespin
    }

    // ── discover_capabilities test ──

    #[test]
    fn discover_capabilities_mixed() {
        // FwVersion found at idx 4, DPI found at idx 8, SmartShift not found, HiResWheel found at idx 10
        let responses = vec![
            make_get_feature_response(0x01, 0x04), // DEVICE_FW_VERSION -> found
            make_get_feature_response(0x01, 0x08), // ADJUSTABLE_DPI -> found
            make_get_feature_response(0x01, 0x00), // SMART_SHIFT -> not found
            make_get_feature_response(0x01, 0x00), // SMART_SHIFT_ENHANCED -> not found
            make_get_feature_response(0x01, 0x00), // (smart_shift_enhanced again for has_smart_shift_enhanced)
            make_get_feature_response(0x01, 0x0A), // HIRES_WHEEL -> found
            make_get_feature_response(0x01, 0x00), // REPROG_CONTROLS_V4 -> not found
        ];
        let transport = MockTransport::new(responses);
        let access = FeatureAccess::new(transport, 0x01);

        let caps = access.discover_capabilities();
        assert!(caps.has_firmware_version);
        assert!(caps.has_dpi);
        assert!(!caps.has_smart_shift);
        assert!(!caps.has_smart_shift_enhanced);
        assert!(caps.has_hires_wheel);
        assert!(!caps.has_reprog_controls);
    }

    // ── ReprogControls tests ──

    #[test]
    fn reprog_controls_enumerate() {
        let feat_idx = 0x0B;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        // getControlCount: 2 controls
        let count_resp = make_feature_response(0x01, feat_idx, 0x00, &[2]);

        // Control 0: Left Button (CID=0x0050, TaskID=0x0038, flags1=0x40 divertable, pos=0, group=1, gmask=1, flags2=0)
        let info0_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x00, 0x50, // CID
                0x00, 0x38, // Task ID
                0x40, // flags1 (divertable)
                0x00, // position
                0x01, // group
                0x01, // group_mask
                0x00, // flags2
            ],
        );
        // getControlMapping for CID 0x0050: mapped to itself (0x0050)
        let map0_resp = make_feature_response(
            0x01,
            feat_idx,
            0x02,
            &[
                0x00, 0x50, // CID echo
                0x00, // mapping flags
                0x00, 0x50, // mapped to
            ],
        );

        // Control 1: Smart Shift (CID=0x00C4, TaskID=0x009C, flags1=0x00 not divertable)
        let info1_resp = make_feature_response(
            0x01,
            feat_idx,
            0x01,
            &[
                0x00, 0xC4, 0x00, 0x9C, 0x00, // not divertable
                0x00, 0x00, 0x00, 0x00,
            ],
        );
        let map1_resp =
            make_feature_response(0x01, feat_idx, 0x02, &[0x00, 0xC4, 0x00, 0x00, 0xC4]);

        let transport = MockTransport::new(vec![
            get_feat, count_resp, info0_resp, map0_resp, info1_resp, map1_resp,
        ]);
        let access = FeatureAccess::new(transport, 0x01);

        let controls = access.get_reprog_controls().unwrap();
        assert_eq!(controls.len(), 2);

        assert_eq!(controls[0].cid, 0x0050);
        assert_eq!(controls[0].name, "Left Button");
        assert!(controls[0].is_remappable);
        assert_eq!(controls[0].mapped_to, 0x0050);

        assert_eq!(controls[1].cid, 0x00C4);
        assert_eq!(controls[1].name, "Smart Shift");
        assert!(!controls[1].is_remappable);
    }

    #[test]
    fn set_reprog_control_sends_correct_bytes() {
        let feat_idx = 0x0B;
        let get_feat = make_get_feature_response(0x01, feat_idx);
        let set_resp = make_feature_response(0x01, feat_idx, 0x03, &[]);
        let transport = MockTransport::new(vec![get_feat, set_resp]);
        let access = FeatureAccess::new(transport, 0x01);

        access.set_reprog_control_mapping(0x0050, 0x0051).unwrap();

        let writes = access.transport.written();
        assert_eq!(writes[1][4], 0x00); // CID hi
        assert_eq!(writes[1][5], 0x50); // CID lo
        assert_eq!(writes[1][6], 0x00); // flags
        assert_eq!(writes[1][7], 0x00); // remap hi
        assert_eq!(writes[1][8], 0x51); // remap lo
    }
}
