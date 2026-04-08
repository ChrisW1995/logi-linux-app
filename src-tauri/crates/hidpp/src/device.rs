use hidapi::{HidApi, HidDevice};
use crate::error::HidppError;
use crate::report::HidppReport;
use crate::features::{HidTransport, FeatureAccess};
use tracing::{debug, info, warn};
use std::collections::HashSet;

/// Logitech vendor ID.
pub const LOGITECH_VID: u16 = 0x046d;

/// HID++ usage page for vendor-specific reports.
const HIDPP_USAGE_PAGE: u16 = 0xFF00;

/// Device index for USB-connected devices.
pub const USB_DEVICE_INDEX: u8 = 0xFF;

/// Well-known receiver product IDs.
const RECEIVER_PIDS: &[u16] = &[
    0xC52B, // Unifying nano
    0xC532, // Unifying full-size
    0xC534, // Nano receiver
    0xC548, // Bolt
];

/// Information about a discovered Logitech HID++ device.
#[derive(Debug, Clone)]
pub struct LogitechDeviceInfo {
    pub path: String,
    pub product_id: u16,
    pub product_name: String,
    pub device_index: u8,
}

/// Find all Logitech HID++ capable devices on the system.
pub fn find_logitech_devices() -> Result<Vec<LogitechDeviceInfo>, HidppError> {
    let api = HidApi::new().map_err(|e| HidppError::Io(e.to_string()))?;
    let mut devices = Vec::new();
    let mut seen_paths = HashSet::new();

    for dev in api.device_list() {
        if dev.vendor_id() != LOGITECH_VID {
            continue;
        }

        if dev.usage_page() != HIDPP_USAGE_PAGE {
            continue;
        }

        let path = dev.path().to_string_lossy().to_string();
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        let product_id = dev.product_id();
        let product_name = dev.product_string().unwrap_or("Unknown").to_string();

        if is_receiver(product_id) {
            debug!("Found receiver: {} (PID: 0x{:04X}) at {}", product_name, product_id, path);
            match probe_paired_devices(&api, &path, product_id) {
                Ok(paired) => {
                    info!("Receiver at {} has {} paired device(s)", path, paired.len());
                    devices.extend(paired);
                }
                Err(e) => warn!("Failed to probe receiver at {}: {}", path, e),
            }
        } else {
            debug!("Found USB device: {} (PID: 0x{:04X}) at {}", product_name, product_id, path);
            devices.push(LogitechDeviceInfo {
                path,
                product_id,
                product_name,
                device_index: USB_DEVICE_INDEX,
            });
        }
    }

    info!("Total devices found: {}", devices.len());
    Ok(devices)
}

fn is_receiver(product_id: u16) -> bool {
    RECEIVER_PIDS.contains(&product_id)
}

/// Probe a receiver for paired devices using HID++ 2.0 IRoot requests.
/// For each device index (1-6), sends IRoot.GetFeatureIndex and checks for response.
fn probe_paired_devices(
    api: &HidApi,
    path: &str,
    receiver_pid: u16,
) -> Result<Vec<LogitechDeviceInfo>, HidppError> {
    let device = api
        .open_path(&std::ffi::CString::new(path).unwrap())
        .map_err(|e| HidppError::Io(e.to_string()))?;

    let mut paired = Vec::new();

    for idx in 1u8..=6 {
        debug!("Probing device index {}...", idx);

        // Send IRoot.GetFeatureIndex(0x0001 = FeatureSet) to check if device exists
        let mut report = HidppReport::new_long(idx, 0x00, 0x00, 0x01);
        report.set_param(0, 0x00);
        report.set_param(1, 0x01); // Feature 0x0001 = FeatureSet

        if let Err(e) = device.write(report.as_bytes()) {
            debug!("Write failed for index {}: {}", idx, e);
            continue;
        }

        // Read responses with matching loop
        match read_probe_response(&device, idx) {
            ProbeResult::DeviceFound => {
                debug!("Device found at index {}, reading name...", idx);

                // Try to get device name via HID++ 2.0 feature 0x0005
                let name = read_device_name(&device, idx);
                let display_name = name.unwrap_or_else(|| format!("Logitech Device #{}", idx));

                info!("Paired device at index {}: {}", idx, display_name);
                paired.push(LogitechDeviceInfo {
                    path: path.to_string(),
                    product_id: receiver_pid,
                    product_name: display_name,
                    device_index: idx,
                });
            }
            ProbeResult::NoDevice => {
                debug!("No device at index {}", idx);
            }
            ProbeResult::Timeout => {
                debug!("Timeout probing index {}", idx);
            }
        }
    }

    Ok(paired)
}

enum ProbeResult {
    DeviceFound,
    NoDevice,
    Timeout,
}

/// Read responses after a probe request, properly matching by device_index.
fn read_probe_response(device: &HidDevice, device_index: u8) -> ProbeResult {
    let mut buf = [0u8; 64];

    for attempt in 0..10 {
        let n = match device.read_timeout(&mut buf, 500) {
            Ok(n) => n,
            Err(e) => {
                debug!("Read error on attempt {}: {}", attempt, e);
                return ProbeResult::Timeout;
            }
        };

        if n == 0 {
            return ProbeResult::Timeout;
        }

        // Parse response
        let resp = match HidppReport::from_bytes(&buf[..n]) {
            Ok(r) => r,
            Err(_) => continue,
        };

        debug!(
            "  Response: dev={} feat=0x{:02X} fn={} err={}",
            resp.device_index(),
            resp.feature_index(),
            resp.function_id(),
            resp.is_error()
        );

        // Must match our target device index
        if resp.device_index() != device_index {
            // Response for a different device index — skip
            debug!("  Skipping response for device {}", resp.device_index());
            continue;
        }

        // Error response for our device → no device or error
        if resp.is_error() {
            debug!("  Error response for device {}", device_index);
            return ProbeResult::NoDevice;
        }

        // Valid response matching our device → device exists!
        return ProbeResult::DeviceFound;
    }

    ProbeResult::Timeout
}

/// Read device name via HID++ 2.0 feature 0x0005 (DeviceName).
fn read_device_name(device: &HidDevice, device_index: u8) -> Option<String> {
    // Step 1: IRoot.GetFeatureIndex(0x0005)
    let mut report = HidppReport::new_long(device_index, 0x00, 0x00, 0x01);
    report.set_param(0, 0x00);
    report.set_param(1, 0x05);
    device.write(report.as_bytes()).ok()?;

    let resp = read_matching_response(device, device_index, 0x00, 0x00)?;
    let feat_idx = resp.params().first().copied().unwrap_or(0);
    if feat_idx == 0 {
        debug!("DeviceName feature not supported on device {}", device_index);
        return None;
    }

    // Step 2: GetDeviceNameCount (function 0)
    let report = HidppReport::new_long(device_index, feat_idx, 0x00, 0x01);
    device.write(report.as_bytes()).ok()?;
    let resp = read_matching_response(device, device_index, feat_idx, 0x00)?;
    let name_len = resp.params().first().copied().unwrap_or(0) as usize;
    if name_len == 0 {
        return None;
    }

    // Step 3: GetDeviceName (function 1) — read in chunks
    let mut name_bytes = Vec::new();
    let mut offset: u8 = 0;
    while name_bytes.len() < name_len {
        let mut req = HidppReport::new_long(device_index, feat_idx, 0x01, 0x01);
        req.set_param(0, offset);
        device.write(req.as_bytes()).ok()?;

        let resp = read_matching_response(device, device_index, feat_idx, 0x01)?;
        let chunk = resp.params();
        let remaining = name_len - name_bytes.len();
        let take = remaining.min(chunk.len());
        name_bytes.extend_from_slice(&chunk[..take]);
        offset += take as u8;
    }

    String::from_utf8(name_bytes).ok()
}

/// Read HID++ 2.0 response matching device_index, feature_index, and function_id.
/// Skips non-matching responses (notifications, other devices).
fn read_matching_response(
    device: &HidDevice,
    device_index: u8,
    feature_index: u8,
    function_id: u8,
) -> Option<HidppReport> {
    let mut buf = [0u8; 64];
    for _ in 0..10 {
        let n = device.read_timeout(&mut buf, 1000).ok()?;
        if n == 0 {
            return None;
        }
        let resp = match HidppReport::from_bytes(&buf[..n]) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if resp.is_error() && resp.device_index() == device_index {
            return None;
        }

        if resp.device_index() == device_index
            && resp.feature_index() == feature_index
            && resp.function_id() == function_id
        {
            return Some(resp);
        }
    }
    None
}

// ─── Transport ─────────────────────────────────────────────────────────────

/// Wrapper around hidapi::HidDevice that implements HidTransport.
pub struct HidApiTransport {
    device: HidDevice,
}

impl HidApiTransport {
    pub fn open(path: &str) -> Result<Self, HidppError> {
        let api = HidApi::new().map_err(|e| HidppError::Io(e.to_string()))?;
        let device = api
            .open_path(&std::ffi::CString::new(path).unwrap())
            .map_err(|e| HidppError::Io(e.to_string()))?;
        info!("Opened HID device at {path}");
        Ok(Self { device })
    }
}

impl HidTransport for HidApiTransport {
    fn write(&self, data: &[u8]) -> Result<usize, HidppError> {
        self.device.write(data).map_err(|e| HidppError::Io(e.to_string()))
    }

    fn read_timeout(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, HidppError> {
        self.device
            .read_timeout(buf, timeout_ms)
            .map_err(|e| HidppError::Io(e.to_string()))
    }
}

/// Open a Logitech device and return a FeatureAccess for HID++ communication.
pub fn open_device(info: &LogitechDeviceInfo) -> Result<FeatureAccess<HidApiTransport>, HidppError> {
    let transport = HidApiTransport::open(&info.path)?;
    Ok(FeatureAccess::new(transport, info.device_index))
}
