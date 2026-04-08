import { useState, useEffect, useCallback, useRef } from "react";
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
  const refreshingRef = useRef(false);

  const refresh = useCallback(async () => {
    // Prevent concurrent calls (StrictMode double-mount + polling)
    if (refreshingRef.current) return;
    refreshingRef.current = true;

    try {
      setLoading(true);
      setError(null);
      const devs = await listDevices();

      // Query battery sequentially to avoid hidraw contention
      const withBattery: DeviceWithBattery[] = [];
      for (const dev of devs) {
        const bat = await getDeviceBattery(dev.path, dev.device_index);
        withBattery.push({
          ...dev,
          battery: bat.battery,
          batteryError: bat.error,
        });
      }

      setDevices(withBattery);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
      refreshingRef.current = false;
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 30_000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { devices, loading, error, refresh };
}
