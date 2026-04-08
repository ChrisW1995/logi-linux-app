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

/// HID++ 1.0 sub-IDs for register operations.
const SUB_ID_SET_REGISTER: u8 = 0x80;
const SUB_ID_GET_REGISTER: u8 = 0x81;
const SUB_ID_ERROR: u8 = 0x8F;

/// Receiver register addresses.
const REG_RECEIVER_INFO: u8 = 0xB5;

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

// ─── HID++ 1.0 Register Protocol ───────────────────────────────────────────

/// Read a register from the receiver using HID++ 1.0 short report.
/// Returns the 3 parameter bytes from the response, or None on error/timeout.
fn register_read(device: &HidDevice, register: u8, params: &[u8]) -> Option<Vec<u8>> {
    let mut report = [0u8; 7];
    report[0] = 0x10; // short report
    report[1] = 0xFF; // device index = receiver itself
    report[2] = SUB_ID_GET_REGISTER;
    report[3] = register;
    for (i, &p) in params.iter().enumerate() {
        if 4 + i < 7 {
            report[4 + i] = p;
        }
    }

    device.write(&report).ok()?;

    let mut buf = [0u8; 64];
    for _ in 0..10 {
        let n = device.read_timeout(&mut buf, 500).ok()?;
        if n == 0 {
            return None;
        }

        // Match response: same sub_id and register
        if buf[1] == 0xFF && buf[2] == SUB_ID_GET_REGISTER && buf[3] == register {
            // Short report response: params at [4..7]
            if n >= 7 {
                return Some(buf[4..7].to_vec());
            }
        }

        // Long report response (0x11) with same register
        if buf[0] == 0x11 && buf[1] == 0xFF && buf[2] == SUB_ID_GET_REGISTER && buf[3] == register {
            if n >= 20 {
                return Some(buf[4..20].to_vec());
            }
        }

        // Error response
        if buf[2] == SUB_ID_ERROR {
            debug!("Register read error: register=0x{:02X}, error=0x{:02X}", register, buf[5]);
            return None;
        }

        // Non-matching response (notification etc.), skip and read next
        debug!("Skipping non-matching response: sub_id=0x{:02X}", buf[2]);
    }

    None
}

/// Read a long register from the receiver (request via 0x11 long report).
fn register_read_long(device: &HidDevice, register: u8, params: &[u8]) -> Option<Vec<u8>> {
    let mut report = [0u8; 20];
    report[0] = 0x11; // long report
    report[1] = 0xFF; // device index = receiver itself
    report[2] = SUB_ID_GET_REGISTER;
    report[3] = register;
    for (i, &p) in params.iter().enumerate() {
        if 4 + i < 20 {
            report[4 + i] = p;
        }
    }

    device.write(&report).ok()?;

    let mut buf = [0u8; 64];
    for _ in 0..10 {
        let n = device.read_timeout(&mut buf, 500).ok()?;
        if n == 0 {
            return None;
        }

        // Long report response
        if buf[0] == 0x11 && buf[1] == 0xFF && buf[2] == SUB_ID_GET_REGISTER && buf[3] == register {
            return Some(buf[4..n.min(20)].to_vec());
        }

        // Short report response (fallback)
        if buf[0] == 0x10 && buf[1] == 0xFF && buf[2] == SUB_ID_GET_REGISTER && buf[3] == register {
            return Some(buf[4..n.min(7)].to_vec());
        }

        // Error
        if buf[2] == SUB_ID_ERROR {
            return None;
        }
    }

    None
}

// ─── Device Probing ────────────────────────────────────────────────────────

/// Probe a receiver for paired devices using HID++ 1.0 register reads.
fn probe_paired_devices(
    api: &HidApi,
    path: &str,
    receiver_pid: u16,
) -> Result<Vec<LogitechDeviceInfo>, HidppError> {
    let device = api
        .open_path(&std::ffi::CString::new(path).unwrap())
        .map_err(|e| HidppError::Io(e.to_string()))?;

    let is_bolt = receiver_pid == 0xC548;
    let mut paired = Vec::new();

    for idx in 1u8..=6 {
        // Step 1: Check if a device is paired at this index via receiver pairing info register
        let pairing_sub = if is_bolt {
            0x50 + idx
        } else {
            0x20 + (idx - 1)
        };

        let pair_info = register_read(&device, REG_RECEIVER_INFO, &[pairing_sub]);
        if pair_info.is_none() {
            debug!("No device paired at index {} (no pairing info)", idx);
            continue;
        }

        // Step 2: Read device codename from receiver
        let codename = read_codename(&device, idx, is_bolt);

        // Step 3: Try HID++ 2.0 DeviceName for full name (device must be online)
        let full_name = read_device_name_hidpp20(&device, idx);

        let display_name = full_name
            .or(codename)
            .unwrap_or_else(|| format!("Logitech Device #{}", idx));

        debug!("Paired device at index {}: {}", idx, display_name);

        paired.push(LogitechDeviceInfo {
            path: path.to_string(),
            product_id: receiver_pid,
            product_name: display_name,
            device_index: idx,
        });
    }

    Ok(paired)
}

/// Read device codename from receiver register 0xB5.
fn read_codename(device: &HidDevice, idx: u8, is_bolt: bool) -> Option<String> {
    let params = if is_bolt {
        vec![0x60 + idx, 0x01]
    } else {
        vec![0x40 + (idx - 1)]
    };

    let resp = register_read(&device, REG_RECEIVER_INFO, &params)?;

    // Extract ASCII name bytes (skip leading zero/length bytes, stop at null or non-ASCII)
    let name_bytes: Vec<u8> = if is_bolt {
        // Bolt: name starts after some header bytes
        resp.iter()
            .copied()
            .filter(|&b| b >= 0x20 && b < 0x7F)
            .collect()
    } else {
        // Unifying: name is in the response payload
        resp.iter()
            .copied()
            .filter(|&b| b >= 0x20 && b < 0x7F)
            .collect()
    };

    if name_bytes.is_empty() {
        return None;
    }

    String::from_utf8(name_bytes).ok()
}

/// Try to get device name via HID++ 2.0 feature 0x0005 (DeviceName).
/// This requires the device to be online and support HID++ 2.0.
fn read_device_name_hidpp20(device: &HidDevice, device_index: u8) -> Option<String> {
    // IRoot.GetFeatureIndex(0x0005) on the device
    let mut report = HidppReport::new_long(device_index, 0x00, 0x00, 0x01);
    report.set_param(0, 0x00);
    report.set_param(1, 0x05);
    device.write(report.as_bytes()).ok()?;

    let feat_idx = read_matching_response(device, device_index, 0x00, 0x00)?;
    let feat_idx = feat_idx.params().first().copied().unwrap_or(0);
    if feat_idx == 0 {
        return None;
    }

    // GetDeviceNameCount (function 0)
    let report = HidppReport::new_long(device_index, feat_idx, 0x00, 0x01);
    device.write(report.as_bytes()).ok()?;
    let resp = read_matching_response(device, device_index, feat_idx, 0x00)?;
    let name_len = resp.params().first().copied().unwrap_or(0) as usize;
    if name_len == 0 {
        return None;
    }

    // GetDeviceName (function 1) — read in chunks
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
        let resp = HidppReport::from_bytes(&buf[..n]).ok()?;

        // Error response for this device
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
