import { useState, useEffect, useCallback, useRef } from "react";
import {
  getDeviceCapabilities,
  getDeviceFirmware,
  getDpiSettings,
  setDpi as setDpiCmd,
  getSmartShift,
  setSmartShift as setSmartShiftCmd,
  getHiResWheel,
  setHiResWheelMode as setHiResWheelModeCmd,
  setHiResWheelRatchet as setHiResWheelRatchetCmd,
  getReprogControls,
  setReprogControl as setReprogControlCmd,
  saveDeviceSettings,
  type DeviceCapabilities,
  type FirmwareInfo,
  type DpiInfo,
  type SmartShiftInfo,
  type HiResWheelInfo,
  type ReprogControl,
} from "@/lib/tauri";

export interface DeviceSettings {
  capabilities: DeviceCapabilities | null;
  firmware: FirmwareInfo[];
  dpi: DpiInfo | null;
  smartShift: SmartShiftInfo | null;
  hiResWheel: HiResWheelInfo | null;
  controls: ReprogControl[];
  loading: boolean;
  error: string | null;
  setDpi: (dpi: number) => Promise<void>;
  setSmartShift: (mode: number, threshold: number) => Promise<void>;
  setHiResWheelMode: (hiRes: boolean, invert: boolean) => Promise<void>;
  setHiResWheelRatchet: (ratchet: boolean) => Promise<void>;
  setReprogControl: (cid: number, remapTo: number) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useDeviceSettings(path: string, deviceIndex: number, deviceName?: string): DeviceSettings {
  const [capabilities, setCapabilities] = useState<DeviceCapabilities | null>(null);
  const [firmware, setFirmware] = useState<FirmwareInfo[]>([]);
  const [dpi, setDpiState] = useState<DpiInfo | null>(null);
  const [smartShift, setSmartShiftState] = useState<SmartShiftInfo | null>(null);
  const [hiResWheel, setHiResWheelState] = useState<HiResWheelInfo | null>(null);
  const [controls, setControls] = useState<ReprogControl[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const refreshingRef = useRef(false);

  const refresh = useCallback(async () => {
    if (refreshingRef.current) return;
    refreshingRef.current = true;

    try {
      setLoading(true);
      setError(null);

      const caps = await getDeviceCapabilities(path, deviceIndex);
      setCapabilities(caps);

      // Load features conditionally based on capabilities
      const promises: Promise<void>[] = [];

      if (caps.has_firmware_version) {
        promises.push(
          getDeviceFirmware(path, deviceIndex).then(setFirmware).catch(() => {}),
        );
      }
      if (caps.has_dpi) {
        promises.push(
          getDpiSettings(path, deviceIndex).then(setDpiState).catch(() => {}),
        );
      }
      if (caps.has_smart_shift) {
        promises.push(
          getSmartShift(path, deviceIndex).then(setSmartShiftState).catch(() => {}),
        );
      }
      if (caps.has_hires_wheel) {
        promises.push(
          getHiResWheel(path, deviceIndex).then(setHiResWheelState).catch(() => {}),
        );
      }
      if (caps.has_reprog_controls) {
        promises.push(
          getReprogControls(path, deviceIndex).then(setControls).catch(() => {}),
        );
      }

      await Promise.all(promises);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
      refreshingRef.current = false;
    }
  }, [path, deviceIndex]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Persist current state to TOML after any setter change
  const persistRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const persist = useCallback(() => {
    if (!deviceName) return;
    if (persistRef.current) clearTimeout(persistRef.current);
    persistRef.current = setTimeout(() => {
      // Read latest state via functional refs — we capture the closure's scope
      saveDeviceSettings(
        deviceName,
        dpi?.current_dpi ?? null,
        smartShift?.mode ?? null,
        smartShift?.threshold ?? null,
        hiResWheel?.hi_res ?? null,
        hiResWheel?.invert ?? null,
        hiResWheel?.ratchet ?? null,
        controls.filter((c) => c.mapped_to !== c.cid).map((c) => ({
          cid: c.cid,
          remap_to: c.mapped_to,
        })),
      ).catch(() => {}); // best-effort persist
    }, 500);
  }, [deviceName, dpi, smartShift, hiResWheel, controls]);

  const setDpi = useCallback(async (newDpi: number) => {
    await setDpiCmd(path, deviceIndex, newDpi);
    setDpiState((prev) => prev ? { ...prev, current_dpi: newDpi } : prev);
  }, [path, deviceIndex]);

  const setSmartShift = useCallback(async (mode: number, threshold: number) => {
    await setSmartShiftCmd(path, deviceIndex, mode, threshold);
    setSmartShiftState((prev) => prev ? { ...prev, mode, threshold } : prev);
  }, [path, deviceIndex]);

  const setHiResWheelMode = useCallback(async (hiRes: boolean, invert: boolean) => {
    await setHiResWheelModeCmd(path, deviceIndex, hiRes, invert);
    setHiResWheelState((prev) => prev ? { ...prev, hi_res: hiRes, invert } : prev);
  }, [path, deviceIndex]);

  const setHiResWheelRatchet = useCallback(async (ratchet: boolean) => {
    await setHiResWheelRatchetCmd(path, deviceIndex, ratchet);
    setHiResWheelState((prev) => prev ? { ...prev, ratchet } : prev);
  }, [path, deviceIndex]);

  const setReprogControl = useCallback(async (cid: number, remapTo: number) => {
    await setReprogControlCmd(path, deviceIndex, cid, remapTo);
    setControls((prev) =>
      prev.map((c) => (c.cid === cid ? { ...c, mapped_to: remapTo } : c)),
    );
  }, [path, deviceIndex]);

  // Auto-persist whenever settings state changes (debounced)
  useEffect(() => {
    if (!loading) persist();
  }, [dpi, smartShift, hiResWheel, controls, loading, persist]);

  return {
    capabilities,
    firmware,
    dpi,
    smartShift,
    hiResWheel,
    controls,
    loading,
    error,
    setDpi,
    setSmartShift,
    setHiResWheelMode,
    setHiResWheelRatchet,
    setReprogControl,
    refresh,
  };
}
