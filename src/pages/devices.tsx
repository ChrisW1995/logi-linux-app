import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DeviceList } from "@/components/devices/device-list";
import { useDevices } from "@/hooks/use-devices";

export function DevicesPage() {
  const { devices, loading, error, refresh } = useDevices();

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Devices</h1>
          <p className="text-sm text-muted-foreground">
            Manage your Logitech devices
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={refresh}>
          <RefreshCw className="h-4 w-4 mr-1.5" />
          Refresh
        </Button>
      </div>
      <DeviceList
        devices={devices}
        loading={loading}
        error={error}
        onRetry={refresh}
      />
    </div>
  );
}
