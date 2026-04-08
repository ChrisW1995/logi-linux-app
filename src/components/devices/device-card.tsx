import { Mouse, Keyboard, BatteryCharging } from "lucide-react";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { getDeviceThumbnail, getDeviceDisplayName } from "@/lib/device-image";
import type { DeviceWithBattery } from "@/hooks/use-devices";

function BatteryBar({ percentage }: { percentage: number }) {
  const color =
    percentage > 50
      ? "bg-emerald-500"
      : percentage > 20
        ? "bg-amber-500"
        : "bg-red-500";

  return (
    <div className="h-2 w-full rounded-full bg-secondary">
      <div
        className={cn("h-full rounded-full transition-all", color)}
        style={{ width: `${Math.min(percentage, 100)}%` }}
      />
    </div>
  );
}

function DeviceIcon({ productName }: { productName: string }) {
  const isKeyboard = /keyboard|keys|k[0-9]{3}/i.test(productName);
  const Icon = isKeyboard ? Keyboard : Mouse;
  return (
    <Icon className="h-20 w-20 text-muted-foreground/40" strokeWidth={1} />
  );
}

export function DeviceCard({ device }: { device: DeviceWithBattery }) {
  const thumbnail = getDeviceThumbnail(device.product_id);
  const displayName = getDeviceDisplayName(
    device.product_id,
    device.product_name,
  );

  return (
    <Card className="group w-full overflow-hidden transition-all hover:shadow-md hover:-translate-y-0.5 cursor-pointer">
      <div className="flex items-center justify-center bg-secondary/50 p-6 h-40">
        {thumbnail ? (
          <img
            src={thumbnail}
            alt={displayName}
            className="max-h-full max-w-full object-contain"
          />
        ) : (
          <DeviceIcon productName={device.product_name} />
        )}
      </div>
      <div className="space-y-3 p-4">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-emerald-500 shrink-0" />
          <span className="text-sm font-medium truncate">{displayName}</span>
        </div>

        {device.battery ? (
          <div className="space-y-1.5">
            <BatteryBar percentage={device.battery.percentage ?? 0} />
            <div className="flex items-center justify-between text-xs text-muted-foreground">
              <span>
                {device.battery.percentage != null
                  ? `${device.battery.percentage}%`
                  : "—"}
              </span>
              {device.battery.status === "Recharging" && (
                <span className="flex items-center gap-1 text-emerald-500">
                  <BatteryCharging className="h-3 w-3" />
                  Charging
                </span>
              )}
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">No battery info</p>
        )}
      </div>
    </Card>
  );
}
