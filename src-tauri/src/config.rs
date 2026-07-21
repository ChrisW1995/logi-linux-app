use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Per-device saved settings, keyed by device product_name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceSettingsConfig {
    pub dpi: Option<u16>,
    pub smart_shift_mode: Option<u8>,
    pub smart_shift_threshold: Option<u8>,
    pub hires_hi_res: Option<bool>,
    pub hires_invert: Option<bool>,
    pub hires_ratchet: Option<bool>,
    #[serde(default)]
    pub button_remaps: Vec<ButtonRemap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonRemap {
    pub cid: u16,
    pub remap_to: u16,
}

/// Top-level config file structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub device: HashMap<String, DeviceSettingsConfig>,
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("logi-linux-app");
    config_dir.join("devices.toml")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                debug!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                warn!("Failed to parse config {}: {e}", path.display());
                AppConfig::default()
            }
        },
        Err(_) => {
            debug!("No config file at {}, using defaults", path.display());
            AppConfig::default()
        }
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let contents = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    debug!("Saved config to {}", path.display());
    Ok(())
}

/// Apply saved settings to a connected device.
pub fn apply_saved_settings(path: &str, device_index: u8, device_name: &str) -> Result<(), String> {
    let config = load_config();
    let Some(settings) = config.device.get(device_name) else {
        debug!("No saved settings for device '{device_name}'");
        return Ok(());
    };

    let info = hidpp::device::LogitechDeviceInfo {
        path: path.to_string(),
        product_id: 0,
        product_name: device_name.to_string(),
        device_index,
    };
    let access = hidpp::device::open_device(&info).map_err(|e| e.to_string())?;

    if let Some(dpi) = settings.dpi {
        if let Err(e) = access.set_dpi(dpi) {
            warn!("Failed to apply DPI {dpi}: {e}");
        } else {
            debug!("Applied DPI={dpi} to '{device_name}'");
        }
    }

    if let (Some(mode), Some(threshold)) =
        (settings.smart_shift_mode, settings.smart_shift_threshold)
    {
        if let Err(e) = access.set_smart_shift(mode, threshold) {
            warn!("Failed to apply SmartShift mode={mode} threshold={threshold}: {e}");
        } else {
            debug!("Applied SmartShift mode={mode} threshold={threshold} to '{device_name}'");
        }
    }

    if let (Some(hi_res), Some(invert)) = (settings.hires_hi_res, settings.hires_invert) {
        if let Err(e) = access.set_hires_wheel_mode(hi_res, invert) {
            warn!("Failed to apply HiResWheel mode: {e}");
        }
    }

    if let Some(ratchet) = settings.hires_ratchet {
        if let Err(e) = access.set_hires_wheel_ratchet(ratchet) {
            warn!("Failed to apply HiResWheel ratchet: {e}");
        }
    }

    for remap in &settings.button_remaps {
        if let Err(e) = access.set_reprog_control_mapping(remap.cid, remap.remap_to) {
            warn!("Failed to apply button remap CID=0x{:04X}: {e}", remap.cid);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let mut config = AppConfig::default();
        config.device.insert(
            "MX Master 4".to_string(),
            DeviceSettingsConfig {
                dpi: Some(1600),
                smart_shift_mode: Some(2),
                smart_shift_threshold: Some(15),
                hires_hi_res: Some(true),
                hires_invert: Some(false),
                hires_ratchet: Some(true),
                button_remaps: vec![ButtonRemap {
                    cid: 0x0050,
                    remap_to: 0x0051,
                }],
            },
        );

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        let dev = parsed.device.get("MX Master 4").unwrap();
        assert_eq!(dev.dpi, Some(1600));
        assert_eq!(dev.smart_shift_mode, Some(2));
        assert_eq!(dev.button_remaps.len(), 1);
        assert_eq!(dev.button_remaps[0].cid, 0x0050);
    }

    #[test]
    fn empty_config_parses() {
        let parsed: AppConfig = toml::from_str("").unwrap();
        assert!(parsed.device.is_empty());
    }
}
