import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function DeviceCardSkeleton() {
  return (
    <Card className="w-full overflow-hidden">
      <Skeleton className="h-40 w-full rounded-none" />
      <div className="space-y-3 p-4">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-2 w-full" />
        <Skeleton className="h-3 w-1/4" />
      </div>
    </Card>
  );
}
