import { invoke } from "@tauri-apps/api/core";

export interface Device {
  path: string;
  product_id: string;
  product_name: string;
  device_index: number;
}

export interface BatteryInfo {
  percentage: number | null;
  level: "Full" | "Good" | "Low" | "Critical" | "Empty";
  status:
    | "Discharging"
    | "Recharging"
    | "Full"
    | "SlowRecharge"
    | "InvalidBattery"
    | "ThermalError"
    | "Unknown";
}

export interface DeviceBattery {
  path: string;
  battery: BatteryInfo | null;
  error: string | null;
}

export async function listDevices(): Promise<Device[]> {
  return invoke<Device[]>("list_devices");
}

export async function getDeviceBattery(
  path: string,
  deviceIndex: number,
): Promise<DeviceBattery> {
  return invoke<DeviceBattery>("get_device_battery", {
    path,
    deviceIndex,
  });
}
