use hidapi::{HidApi, HidDevice, DeviceInfo};
use crate::error::HidppError;
use crate::features::{HidTransport, FeatureAccess};
use tracing::{debug, info};

/// Logitech vendor ID.
pub const LOGITECH_VID: u16 = 0x046d;

/// HID++ usage page for vendor-specific reports.
const HIDPP_USAGE_PAGE: u16 = 0xFF00;

/// Device index for USB-connected devices.
pub const USB_DEVICE_INDEX: u8 = 0xFF;
/// Device index for the first device on a Unifying/Bolt receiver.
pub const RECEIVER_DEVICE_INDEX_1: u8 = 0x01;

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

    for dev in api.device_list() {
        if dev.vendor_id() != LOGITECH_VID {
            continue;
        }

        // Look for HID++ interfaces (usage page 0xFF00 or interface 2)
        if dev.usage_page() == HIDPP_USAGE_PAGE || dev.interface_number() == 2 {
            let path = dev.path().to_string_lossy().to_string();
            let product_name = dev.product_string().unwrap_or("Unknown").to_string();
            let product_id = dev.product_id();

            // Determine device index based on product type
            let device_index = classify_device_index(dev);

            debug!(
                "Found Logitech device: {} (PID: 0x{:04X}) at {}",
                product_name, product_id, path
            );

            devices.push(LogitechDeviceInfo {
                path,
                product_id,
                product_name,
                device_index,
            });
        }
    }

    Ok(devices)
}

/// Classify the device index based on product type.
/// USB-direct devices use 0xFF, receiver-connected use 0x01+.
fn classify_device_index(dev: &DeviceInfo) -> u8 {
    let pid = dev.product_id();
    // Unifying receivers: 0xC52B (nano), 0xC532 (full-size)
    // Bolt receivers: 0xC548
    if matches!(pid, 0xC52B | 0xC532 | 0xC548) {
        RECEIVER_DEVICE_INDEX_1
    } else {
        USB_DEVICE_INDEX
    }
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
