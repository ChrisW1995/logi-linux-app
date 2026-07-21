use crate::config::{self, ButtonRemap, DeviceSettingsConfig};
use hidpp::device;
use hidpp::features::{
    DeviceCapabilities, DpiInfo, FirmwareInfo, HiResWheelInfo, ReprogControl, SmartShiftInfo,
};

/// Helper: open device and run a closure on the FeatureAccess.
fn with_device<T: Send + 'static>(
    path: String,
    device_index: u8,
    f: impl FnOnce(
            hidpp::features::FeatureAccess<hidpp::device::HidApiTransport>,
        ) -> Result<T, hidpp::error::HidppError>
        + Send
        + 'static,
) -> Result<T, String> {
    let info = device::LogitechDeviceInfo {
        path,
        product_id: 0,
        product_name: String::new(),
        device_index,
    };
    let access = device::open_device(&info).map_err(|e| e.to_string())?;
    f(access).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_device_capabilities(
    path: String,
    device_index: u8,
) -> Result<DeviceCapabilities, String> {
    tokio::task::spawn_blocking(move || {
        let info = device::LogitechDeviceInfo {
            path,
            product_id: 0,
            product_name: String::new(),
            device_index,
        };
        let access = device::open_device(&info).map_err(|e| e.to_string())?;
        Ok(access.discover_capabilities())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_device_firmware(
    path: String,
    device_index: u8,
) -> Result<Vec<FirmwareInfo>, String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, |a| a.get_firmware_version())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_dpi_settings(path: String, device_index: u8) -> Result<DpiInfo, String> {
    tokio::task::spawn_blocking(move || with_device(path, device_index, |a| a.get_dpi_settings()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_dpi(path: String, device_index: u8, dpi: u16) -> Result<(), String> {
    tokio::task::spawn_blocking(move || with_device(path, device_index, move |a| a.set_dpi(dpi)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_smart_shift(path: String, device_index: u8) -> Result<SmartShiftInfo, String> {
    tokio::task::spawn_blocking(move || with_device(path, device_index, |a| a.get_smart_shift()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_smart_shift(
    path: String,
    device_index: u8,
    mode: u8,
    threshold: u8,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, move |a| {
            a.set_smart_shift(mode, threshold)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_hires_wheel(path: String, device_index: u8) -> Result<HiResWheelInfo, String> {
    tokio::task::spawn_blocking(move || with_device(path, device_index, |a| a.get_hires_wheel()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_hires_wheel_mode(
    path: String,
    device_index: u8,
    hi_res: bool,
    invert: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, move |a| {
            a.set_hires_wheel_mode(hi_res, invert)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_hires_wheel_ratchet(
    path: String,
    device_index: u8,
    ratchet: bool,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, move |a| {
            a.set_hires_wheel_ratchet(ratchet)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_reprog_controls(
    path: String,
    device_index: u8,
) -> Result<Vec<ReprogControl>, String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, |a| a.get_reprog_controls())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_reprog_control(
    path: String,
    device_index: u8,
    cid: u16,
    remap_to: u16,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        with_device(path, device_index, move |a| {
            a.set_reprog_control_mapping(cid, remap_to)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Persistence commands ──

#[tauri::command]
pub async fn save_device_settings(
    device_name: String,
    dpi: Option<u16>,
    smart_shift_mode: Option<u8>,
    smart_shift_threshold: Option<u8>,
    hires_hi_res: Option<bool>,
    hires_invert: Option<bool>,
    hires_ratchet: Option<bool>,
    button_remaps: Vec<ButtonRemap>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let mut cfg = config::load_config();
        cfg.device.insert(
            device_name,
            DeviceSettingsConfig {
                dpi,
                smart_shift_mode,
                smart_shift_threshold,
                hires_hi_res,
                hires_invert,
                hires_ratchet,
                button_remaps,
            },
        );
        config::save_config(&cfg)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_device_settings(device_name: String) -> Result<DeviceSettingsConfig, String> {
    tokio::task::spawn_blocking(move || {
        let cfg = config::load_config();
        Ok(cfg.device.get(&device_name).cloned().unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_device_settings(
    path: String,
    device_index: u8,
    device_name: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        config::apply_saved_settings(&path, device_index, &device_name)
    })
    .await
    .map_err(|e| e.to_string())?
}
