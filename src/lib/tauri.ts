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

// ── Device Settings Types ──

export interface DeviceCapabilities {
  has_firmware_version: boolean;
  has_dpi: boolean;
  has_smart_shift: boolean;
  has_smart_shift_enhanced: boolean;
  has_hires_wheel: boolean;
  has_reprog_controls: boolean;
}

export interface FirmwareInfo {
  kind: "Main" | "Bootloader" | "Hardware" | "Other";
  prefix: string;
  version: string;
  build: number;
}

export interface DpiInfo {
  current_dpi: number;
  default_dpi: number;
  dpi_list: number[];
}

export interface SmartShiftInfo {
  mode: number;
  threshold: number;
  is_enhanced: boolean;
}

export interface HiResWheelInfo {
  multiplier: number;
  has_invert: boolean;
  has_ratchet: boolean;
  hi_res: boolean;
  invert: boolean;
  ratchet: boolean;
}

export interface ReprogControl {
  cid: number;
  task_id: number;
  flags: number;
  position: number;
  group: number;
  group_mask: number;
  mapped_to: number;
  name: string;
  is_remappable: boolean;
}

// ── Device Settings Commands ──

export async function getDeviceCapabilities(path: string, deviceIndex: number): Promise<DeviceCapabilities> {
  return invoke<DeviceCapabilities>("get_device_capabilities", { path, deviceIndex });
}

export async function getDeviceFirmware(path: string, deviceIndex: number): Promise<FirmwareInfo[]> {
  return invoke<FirmwareInfo[]>("get_device_firmware", { path, deviceIndex });
}

export async function getDpiSettings(path: string, deviceIndex: number): Promise<DpiInfo> {
  return invoke<DpiInfo>("get_dpi_settings", { path, deviceIndex });
}

export async function setDpi(path: string, deviceIndex: number, dpi: number): Promise<void> {
  return invoke<void>("set_dpi", { path, deviceIndex, dpi });
}

export async function getSmartShift(path: string, deviceIndex: number): Promise<SmartShiftInfo> {
  return invoke<SmartShiftInfo>("get_smart_shift", { path, deviceIndex });
}

export async function setSmartShift(path: string, deviceIndex: number, mode: number, threshold: number): Promise<void> {
  return invoke<void>("set_smart_shift", { path, deviceIndex, mode, threshold });
}

export async function getHiResWheel(path: string, deviceIndex: number): Promise<HiResWheelInfo> {
  return invoke<HiResWheelInfo>("get_hires_wheel", { path, deviceIndex });
}

export async function setHiResWheelMode(path: string, deviceIndex: number, hiRes: boolean, invert: boolean): Promise<void> {
  return invoke<void>("set_hires_wheel_mode", { path, deviceIndex, hiRes, invert });
}

export async function setHiResWheelRatchet(path: string, deviceIndex: number, ratchet: boolean): Promise<void> {
  return invoke<void>("set_hires_wheel_ratchet", { path, deviceIndex, ratchet });
}

export async function getReprogControls(path: string, deviceIndex: number): Promise<ReprogControl[]> {
  return invoke<ReprogControl[]>("get_reprog_controls", { path, deviceIndex });
}

export async function setReprogControl(path: string, deviceIndex: number, cid: number, remapTo: number): Promise<void> {
  return invoke<void>("set_reprog_control", { path, deviceIndex, cid, remapTo });
}

// ── Persistence ──

export interface ButtonRemap {
  cid: number;
  remap_to: number;
}

export interface DeviceSettingsConfig {
  dpi: number | null;
  smart_shift_mode: number | null;
  smart_shift_threshold: number | null;
  hires_hi_res: boolean | null;
  hires_invert: boolean | null;
  hires_ratchet: boolean | null;
  button_remaps: ButtonRemap[];
}

export async function saveDeviceSettings(
  deviceName: string,
  dpi: number | null,
  smartShiftMode: number | null,
  smartShiftThreshold: number | null,
  hiresHiRes: boolean | null,
  hiresInvert: boolean | null,
  hiresRatchet: boolean | null,
  buttonRemaps: ButtonRemap[],
): Promise<void> {
  return invoke<void>("save_device_settings", {
    deviceName, dpi, smartShiftMode, smartShiftThreshold,
    hiresHiRes, hiresInvert, hiresRatchet, buttonRemaps,
  });
}

export async function loadDeviceSettings(deviceName: string): Promise<DeviceSettingsConfig> {
  return invoke<DeviceSettingsConfig>("load_device_settings", { deviceName });
}

export async function applyDeviceSettings(path: string, deviceIndex: number, deviceName: string): Promise<void> {
  return invoke<void>("apply_device_settings", { path, deviceIndex, deviceName });
}
