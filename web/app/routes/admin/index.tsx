import { useState, useEffect } from "react";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { createFileRoute, Link } from "@tanstack/react-router";
import { AdminLayout } from "~/components/admin-layout";
import { LogTerminal } from "~/components/admin/log-terminal";
import { Button } from "~/components/ui/button";
import {
  Calculator,
  Plane,
  AlertTriangle,
  CheckCircle,
  Activity,
} from "lucide-react";
import { adminTriggerScoring } from "~/server/admin";
import { useAdminStore, useAuthStore } from "~/stores/admin";
import { StatCard } from "~/components/admin/stat-card";
import { LoginForm } from "~/components/admin/login-form";
import { RecentJobs } from "~/components/admin/recent-jobs";

export const Route = createFileRoute("/admin/")({
  component: AdminDashboard,
});

function Dashboard() {
  const { airports, jobs, dataGaps, loading, fetchAll, fetchJobs } = useAdminStore();
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  // Auto-refresh jobs every 5s if active
  useEffect(() => {
    const hasActive = jobs.some(
      (j) => j.status === "running" || j.status === "queued",
    );
    if (!hasActive) return;
    const interval = setInterval(fetchJobs, 5000);
    return () => clearInterval(interval);
  }, [jobs, fetchJobs]);

  const handleScoring = async () => {
    const password = useAuthStore.getState().password || "";
    setActionLoading("scoring");
    try {
      await adminTriggerScoring({ data: password });
      await fetchAll();
    } finally {
      setActionLoading(null);
    }
  };

  const enabledCount = airports.filter((a) => a.enabled).length;
  const activeJobs = jobs.filter(
    (j) => j.status === "running" || j.status === "queued",
  ).length;
  const scoredCount = airports.filter((a) => a.hasScore).length;

  if (loading && airports.length === 0) {
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
        <Link to="/admin/data-gaps" className="hover:opacity-80 transition-opacity cursor-pointer">
          <StatCard
            label="Data Gaps"
            value={dataGaps.length}
            sub={`across ${new Set(dataGaps.map((g) => g.iataCode)).size} airports`}
            icon={AlertTriangle}
            color={dataGaps.length > 0 ? "text-destructive" : undefined}
          />
        </Link>
        <StatCard
          label="Scored"
          value={`${scoredCount}/${enabledCount}`}
          icon={CheckCircle}
          color={
            scoredCount === enabledCount ? "text-green-500" : "text-yellow-500"
          }
        />
      </div>

      <div
        className="grid sm:grid-cols-[1fr_420px] gap-4"
        style={{ height: "calc(100vh - 320px)" }}
      >
        <RecentJobs recentJobs={jobs.slice(0, 5)} />
        <LogTerminal />
      </div>
    </AdminLayout>
  );
}

function AdminDashboard() {
  const { authenticated, setPassword } = useAdminAuth();

  if (authenticated === null) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    );
  }

  if (!authenticated) {
    return <LoginForm onLogin={(pw: string) => setPassword(pw)} />;
  }

  return <Dashboard />;
}
