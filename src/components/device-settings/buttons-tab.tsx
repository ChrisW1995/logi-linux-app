import { Card } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { DeviceSettings } from "@/hooks/use-device-settings";

export function ButtonsTab({ settings }: { settings: DeviceSettings }) {
  const { controls } = settings;

  if (controls.length === 0) {
    return (
      <p className="text-sm text-muted-foreground py-4">
        No programmable buttons found.
      </p>
    );
  }

  // Build list of remap targets (all controls the device has)
  const remapTargets = controls.map((c) => ({
    cid: c.cid,
    name: c.name !== "Unknown" ? c.name : `CID 0x${c.cid.toString(16).toUpperCase().padStart(4, "0")}`,
  }));

  return (
    <Card className="divide-y">
      {controls.map((control) => {
        const displayName =
          control.name !== "Unknown"
            ? control.name
            : `CID 0x${control.cid.toString(16).toUpperCase().padStart(4, "0")}`;
        const mappedName =
          remapTargets.find((t) => t.cid === control.mapped_to)?.name ??
          `0x${control.mapped_to.toString(16).toUpperCase().padStart(4, "0")}`;

        return (
          <div
            key={control.cid}
            className="flex items-center justify-between gap-4 px-5 py-3"
          >
            <div className="min-w-0">
              <p className="text-sm font-medium truncate">{displayName}</p>
              {!control.is_remappable && (
                <p className="text-xs text-muted-foreground">Not remappable</p>
              )}
            </div>

            {control.is_remappable ? (
              <Select
                value={String(control.mapped_to)}
                onValueChange={(v) =>
                  settings.setReprogControl(control.cid, Number(v))
                }
              >
                <SelectTrigger className="w-44">
                  <SelectValue>{mappedName}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {remapTargets.map((target) => (
                    <SelectItem key={target.cid} value={String(target.cid)}>
                      {target.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <span className="text-sm text-muted-foreground">{mappedName}</span>
            )}
          </div>
        );
      })}
    </Card>
  );
}
