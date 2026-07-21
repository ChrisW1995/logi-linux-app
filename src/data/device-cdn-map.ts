// Maps HID++ device names to Logitech CDN image paths.
// Base URL: https://resource.logitech.com/{transforms}/d_transparent.gif/content/dam/logitech/{path}
// Transforms for thumbnails: w_160,h_160,c_pad,q_auto,f_png,dpr_2.0
// Transforms for detail: w_320,h_320,c_pad,q_auto,f_png,dpr_2.0

export interface CdnImageEntry {
  /** Path after /content/dam/logitech/ */
  path: string;
}

const CDN_BASE = "https://resource.logitech.com";

export const DEVICE_CDN_MAP: Record<string, CdnImageEntry> = {
  // Mice
  "MX Master 4": {
    path: "en/products/mice/mx-master-4/gallery/mx-master-4-graphite-top-angle-gallery-1.png",
  },
  "MX Master 3S": {
    path: "en/products/mice/mx-master-3s/2025-update/mx-master-3s-bluetooth-edition-top-view-graphite-new-1.png",
  },
  "MX Master 3": {
    path: "en/products/mice/mx-master-3s/2025-update/mx-master-3s-bluetooth-edition-top-view-graphite-new-1.png",
  },
  "MX Anywhere 3S": {
    path: "en/products/mice/mx-anywhere-3s/product-gallery/graphite/mx-anywhere-3s-mouse-top-view-graphite.png",
  },
  "MX Anywhere 3": {
    path: "en/products/mice/mx-anywhere-3s/product-gallery/graphite/mx-anywhere-3s-mouse-top-view-graphite.png",
  },

  // Keyboards
  "MX Keys S": {
    path: "en/products/keyboards/mx-keys-s/migration-assets-for-delorean-2025/gallery/mx-keys-s-top-view-graphite-us.png",
  },
  "MX Keys": {
    path: "en/products/keyboards/mx-keys-s/migration-assets-for-delorean-2025/gallery/mx-keys-s-top-view-graphite-us.png",
  },
  "MX Mechanical Mini": {
    path: "en/products/keyboards/mx-mechanical/gallery/mx-mechanical-mini/mx-mechanical-mini-mini-keyboard-top-view-graphite-us.png",
  },
};

function buildCdnUrl(imagePath: string, size: "thumbnail" | "detail"): string {
  const transforms =
    size === "thumbnail"
      ? "w_160,h_160,c_pad,q_auto,f_png,dpr_2.0"
      : "w_320,h_320,c_pad,q_auto,f_png,dpr_2.0";
  return `${CDN_BASE}/${transforms}/d_transparent.gif/content/dam/logitech/${imagePath}`;
}

/**
 * Get CDN image URL for a device by its HID++ name.
 * Returns null if no mapping exists.
 */
export function getCdnDeviceImage(
  deviceName: string,
  size: "thumbnail" | "detail" = "thumbnail",
): string | null {
  // Exact match
  const entry = DEVICE_CDN_MAP[deviceName];
  if (entry) return buildCdnUrl(entry.path, size);

  // Fuzzy: try matching by substring (e.g. "MX Master 4" matches "MX Master 4 Wireless")
  const lower = deviceName.toLowerCase();
  for (const [key, value] of Object.entries(DEVICE_CDN_MAP)) {
    if (lower.includes(key.toLowerCase()) || key.toLowerCase().includes(lower)) {
      return buildCdnUrl(value.path, size);
    }
  }

  return null;
}
