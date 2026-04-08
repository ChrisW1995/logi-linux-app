use hidpp::device;
use hidpp::features::BatteryInfo;
#[cfg(test)]
use hidpp::features::{ChargingStatus, BatteryLevel};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DeviceDto {
    pub path: String,
    pub product_id: String,
    pub product_name: String,
    pub device_index: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceBatteryDto {
    pub path: String,
    pub battery: Option<BatteryInfo>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn list_devices() -> Result<Vec<DeviceDto>, String> {
    device::find_logitech_devices()
        .map(|devs| {
            devs.into_iter()
                .map(|d| DeviceDto {
                    path: d.path.clone(),
                    product_id: format!("0x{:04X}", d.product_id),
                    product_name: d.product_name.clone(),
                    device_index: d.device_index,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_device_battery(path: String, device_index: u8) -> DeviceBatteryDto {
    let info = hidpp::device::LogitechDeviceInfo {
        path: path.clone(),
        product_id: 0,
        product_name: String::new(),
        device_index,
    };

    match device::open_device(&info) {
        Ok(access) => match access.get_battery() {
            Ok(battery) => DeviceBatteryDto { path, battery, error: None },
            Err(e) => DeviceBatteryDto { path, battery: None, error: Some(e.to_string()) },
        },
        Err(e) => DeviceBatteryDto { path, battery: None, error: Some(e.to_string()) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_dto_serializes_product_id() {
        let dto = DeviceDto {
            path: "/dev/hidraw0".to_string(),
            product_id: "0xC548".to_string(),
            product_name: "Bolt Receiver".to_string(),
            device_index: 1,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("0xC548"));
    }

    #[test]
    fn battery_dto_with_none() {
        let dto = DeviceBatteryDto {
            path: "/dev/hidraw0".to_string(),
            battery: None,
            error: Some("not found".to_string()),
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"battery\":null"));
    }

    #[test]
    fn battery_dto_with_info() {
        let dto = DeviceBatteryDto {
            path: "/dev/hidraw0".to_string(),
            battery: Some(BatteryInfo {
                percentage: Some(75),
                level: BatteryLevel::Good,
                status: ChargingStatus::Discharging,
            }),
            error: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"percentage\":75"));
        assert!(json.contains("\"Good\""));
    }
}
