import { useParams, useSearchParams, Link } from "react-router-dom";
import { ArrowLeft, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useDeviceSettings } from "@/hooks/use-device-settings";
import { PointScrollTab } from "@/components/device-settings/point-scroll-tab";
import { ButtonsTab } from "@/components/device-settings/buttons-tab";
import { BatteryTab } from "@/components/device-settings/battery-tab";

export function DeviceDetailPage() {
  const { path, deviceIndex } = useParams<{ path: string; deviceIndex: string }>();
  const [searchParams] = useSearchParams();
  const decodedPath = decodeURIComponent(path ?? "");
  const idx = Number(deviceIndex ?? "0");
  const deviceName = searchParams.get("name") ?? undefined;

  const settings = useDeviceSettings(decodedPath, idx, deviceName);

  const hasPointScroll =
    settings.capabilities?.has_dpi ||
    settings.capabilities?.has_smart_shift ||
    settings.capabilities?.has_hires_wheel;

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/">
            <ArrowLeft className="h-5 w-5" />
          </Link>
        </Button>
        <div>
          <h1 className="text-2xl font-semibold">{deviceName ?? "Device Settings"}</h1>
          <p className="text-sm text-muted-foreground">
            {decodedPath} &middot; Index {idx}
          </p>
        </div>
      </div>

      {settings.loading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : settings.error ? (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4 text-sm text-destructive">
          {settings.error}
        </div>
      ) : (
        <Tabs defaultValue={hasPointScroll ? "point-scroll" : "battery"}>
          <TabsList>
            {hasPointScroll && (
              <TabsTrigger value="point-scroll">Point & Scroll</TabsTrigger>
            )}
            {settings.capabilities?.has_reprog_controls && (
              <TabsTrigger value="buttons">Buttons</TabsTrigger>
            )}
            <TabsTrigger value="battery">Battery</TabsTrigger>
          </TabsList>

          {hasPointScroll && (
            <TabsContent value="point-scroll" className="space-y-6 pt-4">
              <PointScrollTab settings={settings} />
            </TabsContent>
          )}

          {settings.capabilities?.has_reprog_controls && (
            <TabsContent value="buttons" className="space-y-4 pt-4">
              <ButtonsTab settings={settings} />
            </TabsContent>
          )}

          <TabsContent value="battery" className="space-y-4 pt-4">
            <BatteryTab
              path={decodedPath}
              deviceIndex={idx}
              firmware={settings.firmware}
            />
          </TabsContent>
        </Tabs>
      )}
    </div>
  );
}
