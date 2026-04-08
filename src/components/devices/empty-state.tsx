import { MousePointerClick } from "lucide-react";

export function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center">
      <MousePointerClick className="h-16 w-16 text-muted-foreground/30 mb-4" strokeWidth={1} />
      <h3 className="text-lg font-medium text-foreground">
        No Logitech devices found
      </h3>
      <p className="text-sm text-muted-foreground mt-1">
        Connect a device to get started
      </p>
    </div>
  );
}
