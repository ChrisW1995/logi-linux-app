import { useState, useEffect, useRef, useCallback } from "react";
import { Card } from "@/components/ui/card";
import { getDeviceBattery, type BatteryInfo, type FirmwareInfo } from "@/lib/tauri";

interface BatteryReading {
  time: number;
  percentage: number;
}

function useBatteryHistory(path: string, deviceIndex: number) {
  const [history, setHistory] = useState<BatteryReading[]>([]);
  const [current, setCurrent] = useState<BatteryInfo | null>(null);
  const refreshingRef = useRef(false);

  const poll = useCallback(async () => {
    if (refreshingRef.current) return;
    refreshingRef.current = true;
    try {
      const result = await getDeviceBattery(path, deviceIndex);
      if (result.battery) {
        setCurrent(result.battery);
        if (result.battery.percentage != null) {
          setHistory((prev) => {
            const next = [...prev, { time: Date.now(), percentage: result.battery!.percentage! }];
            return next.slice(-48); // Keep last 48 readings
          });
        }
      }
    } finally {
      refreshingRef.current = false;
    }
  }, [path, deviceIndex]);

  useEffect(() => {
    poll();
    const interval = setInterval(poll, 30_000);
    return () => clearInterval(interval);
  }, [poll]);

  return { history, current };
}

function BatteryDisplay({ current }: { current: BatteryInfo | null }) {
  if (!current) {
    return <p className="text-sm text-muted-foreground">No battery information available.</p>;
  }

  const percentage = current.percentage ?? 0;
  const color =
    percentage > 50
      ? "text-emerald-500"
      : percentage > 20
        ? "text-amber-500"
        : "text-red-500";

  return (
    <Card className="p-5">
      <div className="flex items-baseline gap-3">
        <span className={`text-4xl font-bold tabular-nums ${color}`}>
          {current.percentage != null ? `${current.percentage}%` : "—"}
        </span>
        <span className="text-sm text-muted-foreground capitalize">
          {current.status.replace(/([A-Z])/g, " $1").trim()}
        </span>
      </div>
      <div className="mt-3 h-3 w-full rounded-full bg-secondary">
        <div
          className={`h-full rounded-full transition-all ${
            percentage > 50 ? "bg-emerald-500" : percentage > 20 ? "bg-amber-500" : "bg-red-500"
          }`}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />
      </div>
    </Card>
  );
}

function FirmwareDisplay({ firmware }: { firmware: FirmwareInfo[] }) {
  if (firmware.length === 0) return null;

  return (
    <Card className="p-5 space-y-2">
      <h3 className="text-sm font-medium">Firmware</h3>
      {firmware.map((fw, i) => (
        <div key={i} className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground capitalize">{fw.kind}</span>
          <span className="font-mono text-xs">
            {fw.prefix && `${fw.prefix} `}{fw.version}
          </span>
        </div>
      ))}
    </Card>
  );
}

export function BatteryTab({
  path,
  deviceIndex,
  firmware,
}: {
  path: string;
  deviceIndex: number;
  firmware: FirmwareInfo[];
}) {
  const { current } = useBatteryHistory(path, deviceIndex);

  return (
    <>
      <BatteryDisplay current={current} />
      <FirmwareDisplay firmware={firmware} />
    </>
  );
}
