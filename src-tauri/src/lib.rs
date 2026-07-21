mod commands;
pub mod config;

fn check_solaar() {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pgrep").arg("-x").arg("solaar").output() {
            if output.status.success() {
                eprintln!(
                    "\x1b[33m[WARN]\x1b[0m Solaar is running. \
                     Both apps access HID++ devices via hidraw, \
                     which can cause response conflicts. \
                     Consider stopping Solaar: killall solaar"
                );
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hidpp=debug,logi_linux_app=debug".parse().unwrap()),
        )
        .init();

    check_solaar();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::devices::list_devices,
            commands::devices::get_device_battery,
            commands::settings::get_device_capabilities,
            commands::settings::get_device_firmware,
            commands::settings::get_dpi_settings,
            commands::settings::set_dpi,
            commands::settings::get_smart_shift,
            commands::settings::set_smart_shift,
            commands::settings::get_hires_wheel,
            commands::settings::set_hires_wheel_mode,
            commands::settings::set_hires_wheel_ratchet,
            commands::settings::get_reprog_controls,
            commands::settings::set_reprog_control,
            commands::settings::save_device_settings,
            commands::settings::load_device_settings,
            commands::settings::apply_device_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
