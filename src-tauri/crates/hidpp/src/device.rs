use hidapi::{HidApi, HidDevice, DeviceInfo};
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

/// Maximum paired devices to probe per receiver.
const MAX_PAIRED_DEVICES: u8 = 6;

/// Software ID for probing.
const SW_ID: u8 = 0x01;

/// Information about a discovered Logitech HID++ device.
#[derive(Debug, Clone)]
pub struct LogitechDeviceInfo {
    pub path: String,
    pub product_id: u16,
    pub product_name: String,
    pub device_index: u8,
}

/// Find all Logitech HID++ capable devices on the system.
/// For receivers, enumerates paired devices (device index 1-6).
/// For direct USB devices, uses device index 0xFF.
pub fn find_logitech_devices() -> Result<Vec<LogitechDeviceInfo>, HidppError> {
    let api = HidApi::new().map_err(|e| HidppError::Io(e.to_string()))?;
    let mut devices = Vec::new();
    let mut seen_paths = HashSet::new();

    for dev in api.device_list() {
        if dev.vendor_id() != LOGITECH_VID {
            continue;
        }

        // Only look for HID++ interfaces (usage page 0xFF00)
        if dev.usage_page() != HIDPP_USAGE_PAGE {
            continue;
        }

        let path = dev.path().to_string_lossy().to_string();

        // Deduplicate by path
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        let product_id = dev.product_id();
        let product_name = dev.product_string().unwrap_or("Unknown").to_string();

        if is_receiver(product_id) {
            debug!("Found receiver: {} (PID: 0x{:04X}) at {}", product_name, product_id, path);
            match probe_paired_devices(&api, dev, &path, product_id) {
                Ok(paired) => devices.extend(paired),
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

    Ok(devices)
}

fn is_receiver(product_id: u16) -> bool {
    RECEIVER_PIDS.contains(&product_id)
}

/// Probe a receiver for paired devices by sending HID++ requests to each device index.
fn probe_paired_devices(
    api: &HidApi,
    _dev_info: &DeviceInfo,
    path: &str,
    receiver_pid: u16,
) -> Result<Vec<LogitechDeviceInfo>, HidppError> {
    let device = api
        .open_path(&std::ffi::CString::new(path).unwrap())
        .map_err(|e| HidppError::Io(e.to_string()))?;

    let mut paired = Vec::new();

    for idx in 1..=MAX_PAIRED_DEVICES {
        // Try IRoot.GetFeatureIndex(0x0005 = DeviceName) on this device index
        let mut report = HidppReport::new_long(idx, 0x00, 0x00, SW_ID);
        report.set_param(0, 0x00); // Feature 0x0005 high byte
        report.set_param(1, 0x05); // Feature 0x0005 low byte

        if device.write(report.as_bytes()).is_err() {
            continue;
        }

        let mut buf = [0u8; 64];
        // Short timeout — device should respond quickly if online
        match device.read_timeout(&mut buf, 500) {
            Ok(n) if n > 0 => {
                let resp = match HidppReport::from_bytes(&buf[..n]) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                // Error response with the device index means no device or error
                if resp.is_error() && resp.device_index() == idx {
                    debug!("No device at index {} (error response)", idx);
                    continue;
                }

                // If we got a valid response for this device index, it's online
                if resp.device_index() == idx && !resp.is_error() {
                    let feat_idx = resp.params().first().copied().unwrap_or(0);
                    let name = if feat_idx > 0 {
                        read_device_name(&device, idx, feat_idx)
                    } else {
                        None
                    };

                    let display_name = name.unwrap_or_else(|| format!("Logitech Device #{}", idx));
                    debug!("Paired device at index {}: {}", idx, display_name);

                    paired.push(LogitechDeviceInfo {
                        path: path.to_string(),
                        product_id: receiver_pid,
                        product_name: display_name,
                        device_index: idx,
                    });
                }
            }
            _ => {
                debug!("No response from index {} (timeout)", idx);
            }
        }
    }

    Ok(paired)
}

/// Read device name via HID++ feature 0x0005.
fn read_device_name(device: &HidDevice, device_index: u8, feat_idx: u8) -> Option<String> {
    // GetDeviceNameCount (function 0)
    let report = HidppReport::new_long(device_index, feat_idx, 0x00, SW_ID);
    device.write(report.as_bytes()).ok()?;

    let mut buf = [0u8; 64];
    let n = device.read_timeout(&mut buf, 500).ok()?;
    if n == 0 { return None; }
    let resp = HidppReport::from_bytes(&buf[..n]).ok()?;
    if resp.device_index() != device_index || resp.is_error() { return None; }

    let name_len = resp.params().first().copied().unwrap_or(0) as usize;
    if name_len == 0 { return None; }

    // GetDeviceName (function 1) — read in chunks
    let mut name_bytes = Vec::new();
    let mut offset: u8 = 0;
    while name_bytes.len() < name_len {
        let mut req = HidppReport::new_long(device_index, feat_idx, 0x01, SW_ID);
        req.set_param(0, offset);
        device.write(req.as_bytes()).ok()?;

        let n = device.read_timeout(&mut buf, 500).ok()?;
        if n == 0 { break; }
        let resp = HidppReport::from_bytes(&buf[..n]).ok()?;
        if resp.device_index() != device_index || resp.is_error() { break; }

        let chunk = resp.params();
        let remaining = name_len - name_bytes.len();
        let take = remaining.min(chunk.len());
        name_bytes.extend_from_slice(&chunk[..take]);
        offset += take as u8;
    }

    String::from_utf8(name_bytes).ok()
}

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
