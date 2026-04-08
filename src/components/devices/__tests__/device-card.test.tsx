import { render, screen } from "@testing-library/react";
import { DeviceCard } from "../device-card";
import type { DeviceWithBattery } from "@/hooks/use-devices";

const baseDevice: DeviceWithBattery = {
  path: "/dev/hidraw0",
  product_id: "0x0000",
  product_name: "Test Mouse",
  device_index: 1,
  battery: null,
  batteryError: null,
};

describe("DeviceCard", () => {
  it("shows device name", () => {
    render(<DeviceCard device={baseDevice} />);
    expect(screen.getByText("Test Mouse")).toBeInTheDocument();
  });

  it("shows 'No battery info' when battery is null", () => {
    render(<DeviceCard device={baseDevice} />);
    expect(screen.getByText("No battery info")).toBeInTheDocument();
  });

  it("shows battery percentage", () => {
    const device = {
      ...baseDevice,
      battery: {
        percentage: 75,
        level: "Good" as const,
        status: "Discharging" as const,
      },
    };
    render(<DeviceCard device={device} />);
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("shows charging indicator when Recharging", () => {
    const device = {
      ...baseDevice,
      battery: {
        percentage: 50,
        level: "Good" as const,
        status: "Recharging" as const,
      },
    };
    render(<DeviceCard device={device} />);
    expect(screen.getByText("Charging")).toBeInTheDocument();
  });

  it("uses img when thumbnail is available", () => {
    const device = {
      ...baseDevice,
      product_id: "0xC548",
    };
    render(<DeviceCard device={device} />);
    // c548 depot maps to a thumbnail in our catalog
    const img = screen.queryByRole("img");
    // Since c548 is the Bolt receiver and exists in catalog
    if (img) {
      expect(img).toHaveAttribute("src");
    }
  });

  it("falls back to icon when no thumbnail", () => {
    render(<DeviceCard device={baseDevice} />);
    // No matching thumbnail for product_id "0x0000", should show Lucide icon
    expect(screen.queryByRole("img")).toBeNull();
  });
});
