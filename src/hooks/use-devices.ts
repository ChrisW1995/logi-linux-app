import { useState, useEffect, useCallback } from "react";
import {
  listDevices,
  getDeviceBattery,
  type Device,
  type BatteryInfo,
} from "@/lib/tauri";

export interface DeviceWithBattery extends Device {
  battery: BatteryInfo | null;
  batteryError: string | null;
}

export function useDevices() {
  const [devices, setDevices] = useState<DeviceWithBattery[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const devs = await listDevices();

      const withBattery = await Promise.all(
        devs.map(async (dev) => {
          const bat = await getDeviceBattery(dev.path, dev.device_index);
          return {
            ...dev,
            battery: bat.battery,
            batteryError: bat.error,
          };
        }),
      );

      setDevices(withBattery);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 30_000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { devices, loading, error, refresh };
}
