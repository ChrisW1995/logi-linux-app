import { DeviceCard } from "./device-card";
import { DeviceCardSkeleton } from "./device-card-skeleton";
import { EmptyState } from "./empty-state";
import { Button } from "@/components/ui/button";
import type { DeviceWithBattery } from "@/hooks/use-devices";

interface DeviceListProps {
  devices: DeviceWithBattery[];
  loading: boolean;
  error: string | null;
  onRetry: () => void;
}

export function DeviceList({ devices, loading, error, onRetry }: DeviceListProps) {
  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-24 text-center">
        <p className="text-sm text-destructive mb-4">{error}</p>
        <Button variant="outline" size="sm" onClick={onRetry}>
          Retry
        </Button>
      </div>
    );
  }

  if (loading && devices.length === 0) {
    return (
      <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-6">
        {Array.from({ length: 3 }).map((_, i) => (
          <DeviceCardSkeleton key={i} />
        ))}
      </div>
    );
  }

  if (devices.length === 0) {
    return <EmptyState />;
  }

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-6">
      {devices.map((device) => (
        <DeviceCard key={`${device.path}-${device.device_index}`} device={device} />
      ))}
    </div>
  );
}
