import { useState, useEffect, useCallback } from "react";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { createFileRoute, Link } from "@tanstack/react-router";
import type { components } from "~/api/types";
import { AdminLayout } from "~/components/admin-layout";
import { LogTerminal } from "~/components/admin/log-terminal";
import { JobStatusBadge } from "~/components/admin/job-status-badge";
import { Card, CardContent } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "~/components/ui/table";
import {
  RefreshCw,
  Calculator,
  Plane,
  AlertTriangle,
  CheckCircle,
  Activity,
} from "lucide-react";
import {
  adminListAirports,
  adminDataGaps,
  adminListJobs,
  adminRefresh,
  adminTriggerScoring,
} from "~/server/admin";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];
type DataGap = components["schemas"]["DataGapResponse"];
type JobInfo = components["schemas"]["JobInfo"];

export const Route = createFileRoute("/admin/")({
  component: AdminDashboard,
});

function LoginForm({ onLogin }: { onLogin: () => void }) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
    localStorage.setItem("admin_password", password);
    try {
      await adminListAirports({ data: password });
      onLogin();
    } catch {
      localStorage.removeItem("admin_password");
      setError("Invalid password");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-background flex items-center justify-center">
      <Card className="w-full max-w-sm">
        <CardContent className="pt-6">
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <h1 className="font-grotesk text-lg font-bold">Admin Login</h1>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              className="w-full bg-muted border border-border text-foreground text-sm px-3 py-2 rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
              autoFocus
            />
            {error && <p className="text-destructive text-xs">{error}</p>}
            <Button type="submit" disabled={loading} className="w-full">
              {loading ? "Verifying..." : "Login"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function StatCard({
  label,
  value,
  sub,
  icon: Icon,
  color,
}: {
  label: string;
  value: string | number;
  sub?: string;
  icon: React.FC<{ className?: string }>;
  color?: string;
}) {
  return (
    <Card>
      <CardContent className="pt-4 pb-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
            {label}
          </span>
          <Icon className="size-3.5 text-muted-foreground/50" />
        </div>
        <span
          className={`font-grotesk text-2xl font-bold ${color || "text-foreground"}`}
        >
          {value}
        </span>
        {sub && <p className="text-xs text-muted-foreground mt-0.5">{sub}</p>}
      </CardContent>
    </Card>
  );
}

function Dashboard() {
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [dataGaps, setDataGaps] = useState<DataGap[]>([]);
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    const password = localStorage.getItem("admin_password") || "";
    try {
      const [a, g, j] = await Promise.all([
        adminListAirports({ data: password }),
        adminDataGaps({ data: password }),
        adminListJobs({ data: password }),
      ]);
      setAirports(a);
      setDataGaps(g);
      setJobs(j);
    } catch (err) {
      console.error("Failed to fetch admin data", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    const hasActive = jobs.some(
      (j) => j.status === "running" || j.status === "queued",
    );
    if (!hasActive) return;
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [jobs, fetchData]);

  const handleRefreshAll = async () => {
    const password = localStorage.getItem("admin_password") || "";
    setActionLoading("refresh");
    try {
      await adminRefresh({ data: password });
      await fetchData();
    } finally {
      setActionLoading(null);
    }
  };

  const handleScoring = async () => {
    const password = localStorage.getItem("admin_password") || "";
    setActionLoading("scoring");
    try {
      await adminTriggerScoring({ data: password });
      await fetchData();
    } finally {
      setActionLoading(null);
    }
  };

  const enabledCount = airports.filter((a) => a.enabled).length;
  const activeJobs = jobs.filter(
    (j) => j.status === "running" || j.status === "queued",
  ).length;
  const scoredCount = airports.filter((a) => a.hasScore).length;
  const recentJobs = jobs.slice(0, 5);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    );
  }

  return (
    <AdminLayout
      title="Dashboard"
      actions={
        <>
          <Button
            variant="outline"
            size="sm"
            onClick={handleRefreshAll}
            disabled={actionLoading != null}
          >
            <RefreshCw
              className={`size-3.5 ${actionLoading === "refresh" ? "animate-spin" : ""}`}
            />
            Refresh All
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleScoring}
            disabled={actionLoading != null}
          >
            <Calculator className="size-3.5" />
            Run Scoring
          </Button>
          <Link to="/admin/jobs">
            <Button
              size="sm"
              className="bg-green-600 hover:bg-green-700 text-white"
            >
              New Job
            </Button>
          </Link>
        </>
      }
    >
      {/* Stats row */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <StatCard
          label="Airports"
          value={enabledCount}
          sub="tracked"
          icon={Plane}
        />
        <StatCard
          label="Active Jobs"
          value={activeJobs}
          sub={activeJobs > 0 ? "running" : "idle"}
          icon={Activity}
          color={activeJobs > 0 ? "text-yellow-500" : undefined}
        />
        <StatCard
          label="Data Gaps"
          value={dataGaps.length}
          sub={`across ${new Set(dataGaps.map((g) => g.iataCode)).size} airports`}
          icon={AlertTriangle}
          color={dataGaps.length > 0 ? "text-destructive" : undefined}
        />
        <StatCard
          label="Scored"
          value={`${scoredCount}/${enabledCount}`}
          icon={CheckCircle}
          color={
            scoredCount === enabledCount ? "text-green-500" : "text-yellow-500"
          }
        />
      </div>

      {/* Main content: jobs table + log panel */}
      <div
        className="grid sm:grid-cols-[1fr_420px] gap-4"
        style={{ height: "calc(100vh - 320px)" }}
      >
        <Card className="overflow-hidden flex flex-col">
          <div className="px-4 py-3 border-b border-border flex items-center justify-between">
            <span className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
              Recent Jobs
            </span>
            <Link to="/admin/jobs">
              <Button variant="ghost" size="xs">
                View all
              </Button>
            </Link>
          </div>
          <div className="flex-1 overflow-y-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-20">Status</TableHead>
                  <TableHead className="w-20">Airport</TableHead>
                  <TableHead>Source</TableHead>
                  <TableHead className="w-28">Progress</TableHead>
                  <TableHead className="w-36">Started</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {recentJobs.length === 0 && (
                  <TableRow>
                    <TableCell
                      colSpan={5}
                      className="text-center text-muted-foreground py-8"
                    >
                      No jobs yet
                    </TableCell>
                  </TableRow>
                )}
                {recentJobs.map((job) => (
                  <TableRow key={job.id}>
                    <TableCell>
                      <JobStatusBadge status={job.status} />
                    </TableCell>
                    <TableCell className="font-mono text-xs">
                      {job.progress.currentAirport || job.airports[0] || "—"}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {job.progress.currentSource ||
                        job.sources.slice(0, 3).join(", ") ||
                        "all"}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <div className="w-16 h-1 rounded-full bg-muted overflow-hidden">
                          <div
                            className="h-full rounded-full bg-yellow-500 transition-all"
                            style={{
                              width: `${
                                job.progress.airportsTotal > 0
                                  ? (job.progress.airportsCompleted /
                                      job.progress.airportsTotal) *
                                    100
                                  : 0
                              }%`,
                            }}
                          />
                        </div>
                        <span className="text-xs text-muted-foreground tabular-nums">
                          {job.progress.airportsCompleted}/
                          {job.progress.airportsTotal}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {job.startedAt
                        ? new Date(job.startedAt).toLocaleString()
                        : "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </Card>

        {/* Log terminal */}
        <LogTerminal />
      </div>
    </AdminLayout>
  );
}

function AdminDashboard() {
  const { authenticated, setAuthenticated } = useAdminAuth();

  if (authenticated === null) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    );
  }

  if (!authenticated) {
    return <LoginForm onLogin={() => setAuthenticated(true)} />;
  }

  return <Dashboard />;
}
