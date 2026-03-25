import { Badge } from "~/components/ui/badge";

const STATUS_VARIANTS: Record<
  string,
  "default" | "secondary" | "destructive" | "outline"
> = {
  running: "outline",
  queued: "secondary",
  completed: "default",
  failed: "destructive",
  cancelled: "secondary",
};

const STATUS_CLASSES: Record<string, string> = {
  running: "border-yellow-500/30 text-yellow-500 bg-yellow-500/10",
  queued: "",
  completed: "border-green-500/30 text-green-500 bg-green-500/10",
  failed: "",
  cancelled: "text-muted-foreground",
};

export function JobStatusBadge({ status }: { status: string }) {
  return (
    <Badge
      variant={STATUS_VARIANTS[status] ?? "secondary"}
      className={STATUS_CLASSES[status] ?? ""}
    >
      {status}
    </Badge>
  );
}
