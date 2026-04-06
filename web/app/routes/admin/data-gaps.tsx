import { useState, useEffect, useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { useAdminStore, useAuthStore } from "~/stores/admin";
import { AdminLayout } from "~/components/admin-layout";
import { Button } from "~/components/ui/button";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "~/components/ui/table";
import { Input } from "~/components/ui/input";
import { Badge } from "~/components/ui/badge";
import { Loader2, Search, Play } from "lucide-react";
import { adminStartJob } from "~/server/admin";

export const Route = createFileRoute("/admin/data-gaps")({
  component: AdminDataGaps,
});

function timeAgo(dateStr: string | null): string {
  if (!dateStr) return "never";
  const diff = Date.now() - new Date(dateStr).getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days === 0) return "today";
  if (days === 1) return "1 day ago";
  return `${days} days ago`;
}

function statusBadge(status: string) {
  if (status === "never_fetched")
    return <Badge variant="secondary">never fetched</Badge>;
  if (status === "failed") return <Badge variant="destructive">failed</Badge>;
  if (status === "missing")
    return (
      <Badge
        variant="outline"
        className="border-orange-500/30 text-orange-500 bg-orange-500/10"
      >
        missing
      </Badge>
    );
  if (status === "success")
    return (
      <Badge
        variant="outline"
        className="border-yellow-500/30 text-yellow-500 bg-yellow-500/10"
      >
        stale
      </Badge>
    );
  return <Badge variant="secondary">{status}</Badge>;
}

function AdminDataGaps() {
  const { authenticated } = useAdminAuth();
  const { dataGaps: gaps, loading, fetchDataGaps } = useAdminStore();
  const [filter, setFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState<
    "all" | "failed" | "stale" | "never"
  >("all");
  const [fetchingKey, setFetchingKey] = useState<string | null>(null);
  const [fixingAll, setFixingAll] = useState(false);

  useEffect(() => {
    if (authenticated) fetchDataGaps();
  }, [authenticated, fetchDataGaps]);

  const handleFetch = async (iata: string, source: string) => {
    const key = `${iata}:${source}`;
    setFetchingKey(key);
    try {
      const password = useAuthStore.getState().password || "";
      await adminStartJob({
        data: {
          password,
          body: {
            airports: [iata],
            sources: source === "none" ? null : [source],
            score: true,
          },
        },
      });
    } catch (err) {
      console.error("Failed to start job", err);
    } finally {
      setFetchingKey(null);
    }
  };

  const handleFixAll = async () => {
    setFixingAll(true);
    try {
      const password = useAuthStore.getState().password || "";
      // Collect unique airports from the current filtered view (exclude operator gaps)
      const fetchable = filtered.filter((g) => g.source !== "operator");
      const airports = [...new Set(fetchable.map((g) => g.iataCode))];
      // Collect unique sources (excluding "none" which means no sources at all)
      const sources = [
        ...new Set(
          fetchable.map((g) => g.source).filter((s) => s !== "none"),
        ),
      ];
      await adminStartJob({
        data: {
          password,
          body: {
            airports,
            sources: sources.length > 0 ? sources : null,
            score: true,
          },
        },
      });
    } catch (err) {
      console.error("Failed to start fix-all job", err);
    } finally {
      setFixingAll(false);
    }
  };

  const filtered = useMemo(() => {
    let result = gaps;

    if (statusFilter === "failed") {
      result = result.filter((g) => g.lastStatus === "failed");
    } else if (statusFilter === "stale") {
      result = result.filter((g) => g.lastStatus === "success");
    } else if (statusFilter === "never") {
      result = result.filter((g) => g.lastStatus === "never_fetched");
    }

    if (filter) {
      const q = filter.toLowerCase();
      result = result.filter(
        (g) =>
          g.iataCode.toLowerCase().includes(q) ||
          g.name.toLowerCase().includes(q) ||
          g.source.toLowerCase().includes(q),
      );
    }

    return result;
  }, [gaps, filter, statusFilter]);

  const counts = useMemo(
    () => ({
      all: gaps.length,
      failed: gaps.filter((g) => g.lastStatus === "failed").length,
      stale: gaps.filter((g) => g.lastStatus === "success").length,
      never: gaps.filter((g) => g.lastStatus === "never_fetched").length,
      airports: new Set(gaps.map((g) => g.iataCode)).size,
    }),
    [gaps],
  );

  if (!authenticated) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <p className="text-sm text-muted-foreground">Not authenticated</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Loader2 className="size-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <AdminLayout title="Data Gaps">
      <div className="flex items-center gap-3 mb-4 flex-wrap">
        <div className="relative">
          <Search className="absolute left-2.5 top-2 size-3.5 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by airport or source..."
            className="pl-8 h-8 w-64"
          />
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant={statusFilter === "all" ? "secondary" : "ghost"}
            size="xs"
            onClick={() => setStatusFilter("all")}
          >
            All ({counts.all})
          </Button>
          <Button
            variant={statusFilter === "failed" ? "secondary" : "ghost"}
            size="xs"
            onClick={() => setStatusFilter("failed")}
            className={statusFilter === "failed" ? "" : "text-destructive"}
          >
            Failed ({counts.failed})
          </Button>
          <Button
            variant={statusFilter === "stale" ? "secondary" : "ghost"}
            size="xs"
            onClick={() => setStatusFilter("stale")}
            className={statusFilter === "stale" ? "" : "text-yellow-500"}
          >
            Stale ({counts.stale})
          </Button>
          <Button
            variant={statusFilter === "never" ? "secondary" : "ghost"}
            size="xs"
            onClick={() => setStatusFilter("never")}
          >
            Never fetched ({counts.never})
          </Button>
        </div>
        <span className="text-xs text-muted-foreground">
          {filtered.length} gaps across{" "}
          {new Set(filtered.map((g) => g.iataCode)).size} airports
        </span>
        {filtered.length > 0 && (
          <Button
            size="sm"
            onClick={handleFixAll}
            disabled={fixingAll}
            className="ml-auto bg-green-600 hover:bg-green-700 text-white"
          >
            {fixingAll ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Play className="size-3.5" />
            )}
            Fix {new Set(filtered.map((g) => g.iataCode)).size} airports + score
          </Button>
        )}
      </div>

      {gaps.length === 0 ? (
        <div className="py-12 text-center">
          <p className="text-sm text-muted-foreground">
            No data gaps — all sources are up to date.
          </p>
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Airport</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Source</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Last Fetched</TableHead>
              <TableHead className="w-20">Action</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((gap) => {
              const key = `${gap.iataCode}:${gap.source}`;
              return (
                <TableRow key={key}>
                  <TableCell className="font-mono text-xs font-bold">
                    {gap.iataCode}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {gap.name}
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {gap.source}
                  </TableCell>
                  <TableCell>{statusBadge(gap.lastStatus)}</TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    {timeAgo(gap.lastFetchedAt)}
                  </TableCell>
                  <TableCell>
                    {gap.source === "operator" ? (
                      <span className="text-xs text-muted-foreground italic">manual</span>
                    ) : (
                      <Button
                        variant="ghost"
                        size="xs"
                        onClick={() => handleFetch(gap.iataCode, gap.source)}
                        disabled={fetchingKey === key}
                      >
                        {fetchingKey === key ? (
                          <Loader2 className="size-3 animate-spin" />
                        ) : (
                          <Play className="size-3" />
                        )}
                        {gap.source === "none" ? "Fetch all" : `Fetch ${gap.source}`}
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}
    </AdminLayout>
  );
}
