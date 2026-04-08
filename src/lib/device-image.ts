import { DEVICE_CATALOG } from "@/data/device-catalog";

export function getDeviceThumbnail(productId: string): string | null {
  // product_id comes as "0xC548" from Tauri, catalog uses lowercase model IDs
  const normalized = productId.toLowerCase().replace("0x", "");

  // Try direct lookup
  for (const entry of Object.values(DEVICE_CATALOG)) {
    if (entry.modelId.toLowerCase() === normalized) {
      return entry.thumbnail;
    }
    // Also check depot name which sometimes matches product_id
    if (entry.depot.toLowerCase() === normalized) {
      return entry.thumbnail;
    }
  }
  return null;
}

export function getDeviceDisplayName(
  productId: string,
  fallbackName: string,
): string {
  const normalized = productId.toLowerCase().replace("0x", "");
  for (const entry of Object.values(DEVICE_CATALOG)) {
    if (
      entry.modelId.toLowerCase() === normalized ||
      entry.depot.toLowerCase() === normalized
    ) {
      return entry.displayName;
    }
  }
  return fallbackName;
}
