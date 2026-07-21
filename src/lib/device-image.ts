import { DEVICE_CATALOG } from "@/data/device-catalog";
import { getCdnDeviceImage } from "@/data/device-cdn-map";

/**
 * Get device thumbnail URL.
 * Priority: CDN (resource.logitech.com) → local catalog → null (Lucide fallback in UI).
 */
export function getDeviceThumbnail(nameOrId: string): string | null {
  // 1. Try CDN first (highest quality, transparent PNGs)
  const cdnUrl = getCdnDeviceImage(nameOrId, "thumbnail");
  if (cdnUrl) return cdnUrl;

  // 2. Fall back to local catalog
  const normalized = nameOrId.toLowerCase().replace("0x", "");
  for (const entry of Object.values(DEVICE_CATALOG)) {
    if (
      entry.modelId.toLowerCase() === normalized ||
      entry.depot.toLowerCase() === normalized
    ) {
      return entry.thumbnail;
    }
    if (entry.displayName.toLowerCase() === normalized) {
      return entry.thumbnail;
    }
  }
  return null;
}

/**
 * Get device detail image (larger, for device settings page).
 */
export function getDeviceDetailImage(nameOrId: string): string | null {
  return getCdnDeviceImage(nameOrId, "detail");
}
