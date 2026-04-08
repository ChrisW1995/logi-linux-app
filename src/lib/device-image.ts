import { DEVICE_CATALOG } from "@/data/device-catalog";

export function getDeviceThumbnail(nameOrId: string): string | null {
  const normalized = nameOrId.toLowerCase().replace("0x", "");

  for (const entry of Object.values(DEVICE_CATALOG)) {
    // Match by modelId or depot
    if (
      entry.modelId.toLowerCase() === normalized ||
      entry.depot.toLowerCase() === normalized
    ) {
      return entry.thumbnail;
    }
    // Match by displayName (e.g. "MX Master 4")
    if (entry.displayName.toLowerCase() === normalized) {
      return entry.thumbnail;
    }
  }
  return null;
}
