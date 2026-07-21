import { useCallback, useRef } from "react";
import { Card } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { DeviceSettings } from "@/hooks/use-device-settings";

function SettingRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <Label className="text-sm shrink-0">{label}</Label>
      <div className="flex items-center gap-2">{children}</div>
    </div>
  );
}

function DpiSection({ settings }: { settings: DeviceSettings }) {
  const { dpi } = settings;
  if (!dpi) return null;

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleDpiChange = useCallback(
    (value: number[]) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        settings.setDpi(value[0]);
      }, 100);
    },
    [settings],
  );

  // Find nearest valid DPI in list for slider
  const min = dpi.dpi_list[0] ?? 200;
  const max = dpi.dpi_list[dpi.dpi_list.length - 1] ?? 8000;
  const step = dpi.dpi_list.length > 1 ? dpi.dpi_list[1] - dpi.dpi_list[0] : 50;

  return (
    <Card className="p-5 space-y-4">
      <h3 className="text-sm font-medium">DPI / Sensitivity</h3>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-2xl font-semibold tabular-nums">{dpi.current_dpi}</span>
          <span className="text-xs text-muted-foreground">
            {min} – {max}
          </span>
        </div>
        <Slider
          value={[dpi.current_dpi]}
          min={min}
          max={max}
          step={step > 0 ? step : 50}
          onValueChange={handleDpiChange}
        />
      </div>
    </Card>
  );
}

function SmartShiftSection({ settings }: { settings: DeviceSettings }) {
  const { smartShift } = settings;
  if (!smartShift) return null;

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleThresholdChange = useCallback(
    (value: number[]) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        settings.setSmartShift(smartShift.mode, value[0]);
      }, 100);
    },
    [settings, smartShift.mode],
  );

  return (
    <Card className="p-5 space-y-4">
      <h3 className="text-sm font-medium">SmartShift</h3>
      <SettingRow label="Scroll Mode">
        <Select
          value={String(smartShift.mode)}
          onValueChange={(v) => settings.setSmartShift(Number(v), smartShift.threshold)}
        >
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="0">Ratchet</SelectItem>
            <SelectItem value="1">Free Spin</SelectItem>
            <SelectItem value="2">SmartShift</SelectItem>
          </SelectContent>
        </Select>
      </SettingRow>

      {smartShift.mode === 2 && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-sm">Sensitivity</Label>
            <span className="text-sm tabular-nums text-muted-foreground">
              {smartShift.threshold}
            </span>
          </div>
          <Slider
            value={[smartShift.threshold]}
            min={1}
            max={50}
            step={1}
            onValueChange={handleThresholdChange}
          />
        </div>
      )}
    </Card>
  );
}

function HiResWheelSection({ settings }: { settings: DeviceSettings }) {
  const { hiResWheel } = settings;
  if (!hiResWheel) return null;

  return (
    <Card className="p-5 space-y-4">
      <h3 className="text-sm font-medium">Scroll Wheel</h3>

      <SettingRow label="Hi-Res Scrolling">
        <Switch
          checked={hiResWheel.hi_res}
          onCheckedChange={(checked) =>
            settings.setHiResWheelMode(checked, hiResWheel.invert)
          }
        />
      </SettingRow>

      {hiResWheel.has_invert && (
        <SettingRow label="Invert Direction">
          <Switch
            checked={hiResWheel.invert}
            onCheckedChange={(checked) =>
              settings.setHiResWheelMode(hiResWheel.hi_res, checked)
            }
          />
        </SettingRow>
      )}

      {hiResWheel.has_ratchet && (
        <SettingRow label="Ratchet Mode">
          <Switch
            checked={hiResWheel.ratchet}
            onCheckedChange={settings.setHiResWheelRatchet}
          />
        </SettingRow>
      )}
    </Card>
  );
}

export function PointScrollTab({ settings }: { settings: DeviceSettings }) {
  return (
    <>
      {settings.capabilities?.has_dpi && <DpiSection settings={settings} />}
      {settings.capabilities?.has_smart_shift && <SmartShiftSection settings={settings} />}
      {settings.capabilities?.has_hires_wheel && <HiResWheelSection settings={settings} />}
    </>
  );
}
