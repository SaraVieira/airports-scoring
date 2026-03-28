import { useState, useEffect, useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
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
import { adminListJobs, adminListAirports, adminCancelJob } from "~/server/admin";
import type { components } from "~/api/types";
import { Plus, X } from "lucide-react";

type JobInfo = components["schemas"]["JobInfo"];
type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

export const Route = createFileRoute("/admin/jobs")({
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
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const password = localStorage.getItem("admin_password") || "";
      const [j, a] = await Promise.all([
        adminListJobs({ data: password }),
        adminListAirports({ data: password }),
      ]);
      setJobs(j);
      setAirports(a);
    } catch (err) {
      console.error("Failed to fetch jobs data", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (authenticated) fetchData();
  }, [authenticated, fetchData]);

  // Auto-refresh every 5s if active jobs
  useEffect(() => {
    if (!authenticated) return;
    const hasActive = jobs.some(
      (j) => j.status === "running" || j.status === "queued",
    );
    if (!hasActive) return;
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [jobs, authenticated, fetchData]);

  const handleCancel = async (jobId: string) => {
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminCancelJob({ data: { password, id: jobId } });
      await fetchData();
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
        onStarted={fetchData}
      />

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
          {jobs.map((job) => (
            <TableRow key={job.id}>
              <TableCell className="font-mono text-xs">
                {job.id.slice(0, 8)}
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
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </AdminLayout>
  );
}
