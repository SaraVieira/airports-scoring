import { useState, useEffect, useCallback } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { AdminLayout } from "~/components/admin-layout";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { Card } from "~/components/ui/card";
import { JobStatusBadge } from "~/components/admin/job-status-badge";
import { LogTerminal } from "~/components/admin/log-terminal";
import { adminGetJob, adminCancelJob } from "~/server/admin";
import { useAuthStore } from "~/stores/admin";
import type { components } from "~/api/types";
import { ArrowLeft, X, Loader2 } from "lucide-react";

type JobInfo = components["schemas"]["JobInfo"];

export const Route = createFileRoute("/admin/jobs/$id")({
  component: JobDetail,
});

function JobDetail() {
  const { id } = Route.useParams();
  const { authenticated } = useAdminAuth();
  const [job, setJob] = useState<JobInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchJob = useCallback(async () => {
    const password = useAuthStore.getState().password || "";
    try {
      const result = await adminGetJob({ data: { password, id } });
      setJob(result as JobInfo);
    } catch {
      setJob(null);
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    if (authenticated) fetchJob();
  }, [authenticated, fetchJob]);

  // Auto-refresh while active
  useEffect(() => {
    if (!job || (job.status !== "running" && job.status !== "queued")) return;
    const interval = setInterval(fetchJob, 3000);
    return () => clearInterval(interval);
  }, [job, fetchJob]);

  const handleCancel = async () => {
    const password = useAuthStore.getState().password || "";
    try {
      await adminCancelJob({ data: { password, id } });
      await fetchJob();
    } catch (err) {
      console.error("Failed to cancel", err);
    }
  };

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

  if (!job) {
    return (
      <AdminLayout title="Job Not Found">
        <p className="text-sm text-muted-foreground">Job {id} not found.</p>
        <Link to="/admin/jobs" className="text-xs text-muted-foreground hover:text-foreground mt-2 inline-block">
          Back to jobs
        </Link>
      </AdminLayout>
    );
  }

  const isActive = job.status === "running" || job.status === "queued";
  const pct =
    job.progress.airportsTotal > 0
      ? Math.round((job.progress.airportsCompleted / job.progress.airportsTotal) * 100)
      : 0;

  return (
    <AdminLayout
      title={`Job ${job.id.slice(0, 8)}`}
      actions={
        <div className="flex items-center gap-2">
          {isActive && (
            <Button variant="destructive" size="sm" onClick={handleCancel}>
              <X className="size-3.5" />
              Cancel
            </Button>
          )}
          <Link to="/admin/jobs">
            <Button variant="outline" size="sm">
              <ArrowLeft className="size-3.5" />
              All Jobs
            </Button>
          </Link>
        </div>
      }
    >
      {/* Status + Progress */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <Card className="p-4 space-y-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Status</span>
          <div className="flex items-center gap-2">
            <JobStatusBadge status={job.status} />
            {job.error && (
              <span className="text-xs text-destructive truncate">{job.error}</span>
            )}
          </div>
        </Card>

        <Card className="p-4 space-y-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Progress</span>
          <div className="flex items-center gap-3">
            <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-yellow-400 transition-all rounded-full"
                style={{ width: `${pct}%` }}
              />
            </div>
            <span className="font-mono text-sm font-bold">
              {job.progress.airportsCompleted}/{job.progress.airportsTotal}
            </span>
          </div>
          {isActive && job.progress.currentAirport && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Loader2 className="size-3 animate-spin" />
              <span className="font-mono font-bold">{job.progress.currentAirport}</span>
              {job.progress.currentSource && (
                <span className="text-muted-foreground/60">/ {job.progress.currentSource}</span>
              )}
            </div>
          )}
        </Card>

        <Card className="p-4 space-y-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Timing</span>
          <div className="space-y-1 text-xs text-muted-foreground">
            {job.startedAt && (
              <div>Started: <span className="font-mono text-foreground">{new Date(job.startedAt).toLocaleString()}</span></div>
            )}
            {job.completedAt && (
              <div>Completed: <span className="font-mono text-foreground">{new Date(job.completedAt).toLocaleString()}</span></div>
            )}
            {job.startedAt && job.completedAt && (
              <div>Duration: <span className="font-mono text-foreground">{formatDuration(job.startedAt, job.completedAt)}</span></div>
            )}
          </div>
        </Card>
      </div>

      {/* Airports + Sources */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        <Card className="p-4 space-y-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Airports ({job.airports.length})
          </span>
          <div className="flex flex-wrap gap-1.5">
            {job.airports.map((iata) => (
              <Badge
                key={iata}
                variant="secondary"
                className={`font-mono text-xs ${
                  job.progress.currentAirport === iata
                    ? "border-yellow-500/30 text-yellow-400 bg-yellow-500/10"
                    : ""
                }`}
              >
                {iata}
                {job.progress.currentAirport === iata && (
                  <Loader2 className="size-2.5 animate-spin ml-1" />
                )}
              </Badge>
            ))}
          </div>
        </Card>

        <Card className="p-4 space-y-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
            Sources ({job.sources.length})
          </span>
          <div className="flex flex-wrap gap-1.5">
            {job.sources.map((source) => (
              <Badge
                key={source}
                variant="secondary"
                className={`font-mono text-xs ${
                  job.progress.currentSource === source
                    ? "border-yellow-500/30 text-yellow-400 bg-yellow-500/10"
                    : ""
                }`}
              >
                {source}
              </Badge>
            ))}
          </div>
          <div className="flex items-center gap-3 pt-1 text-xs text-muted-foreground">
            {job.fullRefresh && <Badge variant="outline" className="text-[10px]">full refresh</Badge>}
            {job.score && <Badge variant="outline" className="text-[10px]">score after</Badge>}
          </div>
        </Card>
      </div>

      {/* Live Logs */}
      <div className="h-[400px]">
        <LogTerminal />
      </div>
    </AdminLayout>
  );
}

function formatDuration(start: string, end: string): string {
  const ms = new Date(end).getTime() - new Date(start).getTime();
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const remainSecs = secs % 60;
  if (mins < 60) return `${mins}m ${remainSecs}s`;
  const hours = Math.floor(mins / 60);
  const remainMins = mins % 60;
  return `${hours}h ${remainMins}m`;
}
