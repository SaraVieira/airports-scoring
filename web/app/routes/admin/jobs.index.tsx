import { useState, useEffect, useMemo } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useAdminAuth } from "~/hooks/use-admin-auth";
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
import { JobStatusBadge } from "~/components/admin/job-status-badge";
import { NewJobDialog } from "~/components/admin/new-job-dialog";
import { adminCancelJob } from "~/server/admin";
import { useAdminStore, useAuthStore } from "~/stores/admin";
import type { components } from "~/api/types";
import { Plus, X, Eye, EyeOff } from "lucide-react";

type JobInfo = components["schemas"]["JobInfo"];

export const Route = createFileRoute("/admin/jobs/")({
  component: AdminJobs,
});

function ProgressBar({ progress }: { progress: JobInfo["progress"] }) {
  const pct =
    progress.airportsTotal > 0
      ? (progress.airportsCompleted / progress.airportsTotal) * 100
      : 0;
  return (
    <div className="flex items-center gap-2">
      <div className="w-24 h-1.5 bg-muted rounded-full overflow-hidden">
        <div
          className="h-full bg-yellow-400 transition-all rounded-full"
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="font-mono text-xs text-muted-foreground">
        {progress.airportsCompleted}/{progress.airportsTotal}
      </span>
      {progress.currentAirport && (
        <span className="font-mono text-xs text-muted-foreground/60">
          {progress.currentAirport}
          {progress.currentSource && ` / ${progress.currentSource}`}
        </span>
      )}
    </div>
  );
}

function AdminJobs() {
  const { authenticated } = useAdminAuth();
  const { airports, jobs, fetchJobs, fetchAirports } = useAdminStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [hiddenJobIds, setHiddenJobIds] = useState<Set<string>>(() => {
    try {
      const stored = sessionStorage.getItem("hidden-job-ids");
      return stored ? new Set(JSON.parse(stored) as string[]) : new Set();
    } catch {
      return new Set();
    }
  });
  const [showFilter, setShowFilter] = useState<"all" | "active" | "completed">("all");

  useEffect(() => {
    if (authenticated) {
      fetchJobs();
      fetchAirports();
    }
  }, [authenticated, fetchJobs, fetchAirports]);

  const visibleJobs = useMemo(() => {
    return jobs
      .filter((j) => !hiddenJobIds.has(j.id))
      .filter((j) => {
        if (showFilter === "active") return j.status === "running" || j.status === "queued";
        if (showFilter === "completed") return j.status !== "running" && j.status !== "queued";
        return true;
      });
  }, [jobs, hiddenJobIds, showFilter]);

  const isActive = (j: JobInfo) => j.status === "running" || j.status === "queued";

  const activeCount = useMemo(() => jobs.filter(isActive).length, [jobs]);
  const completedCount = useMemo(() => jobs.filter((j) => !isActive(j)).length, [jobs]);

  const updateHidden = (next: Set<string>) => {
    setHiddenJobIds(next);
    sessionStorage.setItem("hidden-job-ids", JSON.stringify([...next]));
  };

  const hideJob = (id: string) => {
    updateHidden(new Set([...hiddenJobIds, id]));
  };

  const hideAllCompleted = () => {
    const completedIds = jobs.filter((j) => !isActive(j)).map((j) => j.id);
    updateHidden(new Set([...hiddenJobIds, ...completedIds]));
  };

  // Auto-refresh every 5s if active jobs
  useEffect(() => {
    if (!authenticated) return;
    const hasActive = jobs.some(
      (j) => j.status === "running" || j.status === "queued",
    );
    if (!hasActive) return;
    const interval = setInterval(fetchJobs, 5000);
    return () => clearInterval(interval);
  }, [jobs, authenticated, fetchJobs]);

  const handleCancel = async (jobId: string) => {
    try {
      const password = useAuthStore.getState().password || "";
      await adminCancelJob({ data: { password, id: jobId } });
      await fetchJobs();
    } catch (err) {
      console.error("Failed to cancel job", err);
    }
  };

  if (!authenticated) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <p className="text-sm text-muted-foreground">Not authenticated</p>
      </div>
    );
  }

  return (
    <AdminLayout
      title="Jobs"
      actions={
        <Button
          size="sm"
          onClick={() => setDialogOpen(true)}
          className="bg-green-600 hover:bg-green-700 text-white"
        >
          <Plus className="size-3.5" />
          New Job
        </Button>
      }
    >
      <NewJobDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        airports={airports}
        onStarted={fetchJobs}
      />

      <div className="flex items-center gap-2 mb-4">
        {(["all", "active", "completed"] as const).map((f) => (
          <Button
            key={f}
            variant={showFilter === f ? "default" : "outline"}
            size="xs"
            onClick={() => setShowFilter(f)}
          >
            {f === "all" && `All (${jobs.length})`}
            {f === "active" && `Active (${activeCount})`}
            {f === "completed" && `Completed (${completedCount})`}
          </Button>
        ))}
        {completedCount > 0 && (
          <Button
            variant="ghost"
            size="xs"
            onClick={hideAllCompleted}
            className="ml-auto text-muted-foreground"
          >
            <EyeOff className="size-3" />
            Hide all completed
          </Button>
        )}
        {hiddenJobIds.size > 0 && (
          <Button
            variant="ghost"
            size="xs"
            onClick={() => updateHidden(new Set())}
            className="text-muted-foreground"
          >
            <Eye className="size-3" />
            Show {hiddenJobIds.size} hidden
          </Button>
        )}
        <span className="text-xs text-muted-foreground ml-2">
          {visibleJobs.length} jobs shown
        </span>
      </div>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>ID</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Airports</TableHead>
            <TableHead>Sources</TableHead>
            <TableHead>Progress</TableHead>
            <TableHead>Started</TableHead>
            <TableHead>Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {visibleJobs.map((job) => (
            <TableRow key={job.id}>
              <TableCell className="font-mono text-xs">
                <Link
                  to="/admin/jobs/$id"
                  params={{ id: job.id }}
                  className="hover:text-foreground underline underline-offset-2"
                >
                  {job.id.slice(0, 8)}
                </Link>
              </TableCell>
              <TableCell>
                <JobStatusBadge status={job.status} />
              </TableCell>
              <TableCell className="text-xs text-muted-foreground max-w-48 truncate">
                {job.airports.join(", ")}
              </TableCell>
              <TableCell className="text-xs text-muted-foreground max-w-48 truncate">
                {job.sources.join(", ")}
              </TableCell>
              <TableCell>
                <ProgressBar progress={job.progress} />
              </TableCell>
              <TableCell className="text-xs text-muted-foreground">
                {job.startedAt
                  ? new Date(job.startedAt).toLocaleString()
                  : "—"}
              </TableCell>
              <TableCell>
                <div className="flex items-center gap-1">
                  {(job.status === "running" || job.status === "queued") && (
                    <Button
                      variant="destructive"
                      size="xs"
                      onClick={() => handleCancel(job.id)}
                    >
                      <X className="size-3" />
                      Cancel
                    </Button>
                  )}
                  {job.status !== "running" && job.status !== "queued" && (
                    <Button
                      variant="ghost"
                      size="xs"
                      onClick={() => hideJob(job.id)}
                      className="text-muted-foreground"
                    >
                      <EyeOff className="size-3" />
                      Hide
                    </Button>
                  )}
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </AdminLayout>
  );
}
