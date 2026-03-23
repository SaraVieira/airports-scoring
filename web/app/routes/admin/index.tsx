import { useState, useEffect, useCallback } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { api } from "~/api/client";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];
type DataGap = components["schemas"]["DataGapResponse"];
type JobInfo = components["schemas"]["JobInfo"];

export const Route = createFileRoute("/admin/")({
  component: AdminDashboard,
});

function useAdmin() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);

  const verify = useCallback(() => {
    const password = localStorage.getItem("admin_password");
    if (!password) {
      setAuthenticated(false);
      return;
    }
    api.admin
      .listSupportedAirports()
      .then(() => setAuthenticated(true))
      .catch(() => {
        localStorage.removeItem("admin_password");
        setAuthenticated(false);
      });
  }, []);

  useEffect(() => {
    verify();
  }, [verify]);

  return { authenticated, setAuthenticated };
}

function LoginForm({
  onLogin,
}: {
  onLogin: () => void;
}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
    localStorage.setItem("admin_password", password);
    try {
      await api.admin.listSupportedAirports();
      onLogin();
    } catch {
      localStorage.removeItem("admin_password");
      setError("Invalid password");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
      <form
        onSubmit={handleSubmit}
        className="bg-zinc-900 border border-zinc-800 p-8 w-full max-w-sm"
      >
        <h1 className="font-grotesk text-lg font-bold text-zinc-100 mb-6">
          Admin Login
        </h1>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="Password"
          className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-sm px-3 py-2 mb-4 focus:outline-none focus:border-zinc-500"
          autoFocus
        />
        {error && (
          <p className="text-red-400 font-mono text-xs mb-4">{error}</p>
        )}
        <button
          type="submit"
          disabled={loading}
          className="w-full bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-2 disabled:opacity-50"
        >
          {loading ? "Verifying..." : "Login"}
        </button>
      </form>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    completed: "text-green-400 bg-green-400/10",
    running: "text-yellow-400 bg-yellow-400/10",
    queued: "text-zinc-400 bg-zinc-400/10",
    failed: "text-red-400 bg-red-400/10",
    cancelled: "text-zinc-500 bg-zinc-500/10",
  };
  const cls = colors[status] || "text-zinc-400 bg-zinc-400/10";
  return (
    <span className={`font-mono text-xs px-2 py-0.5 ${cls}`}>{status}</span>
  );
}

function Dashboard() {
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [dataGaps, setDataGaps] = useState<DataGap[]>([]);
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [a, g, j] = await Promise.all([
        api.admin.listSupportedAirports(),
        api.admin.dataGaps(),
        api.admin.listJobs(),
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

  // Auto-refresh every 5s if there are active jobs
  useEffect(() => {
    const hasActive = jobs.some(
      (j) => j.status === "running" || j.status === "queued",
    );
    if (!hasActive) return;
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [jobs, fetchData]);

  const handleRefreshAll = async () => {
    setActionLoading("refresh");
    try {
      await api.admin.refresh();
      await fetchData();
    } finally {
      setActionLoading(null);
    }
  };

  const handleScoring = async () => {
    setActionLoading("scoring");
    try {
      await api.admin.triggerScoring();
      await fetchData();
    } finally {
      setActionLoading(null);
    }
  };

  const handleLogout = () => {
    localStorage.removeItem("admin_password");
    window.location.reload();
  };

  const enabledCount = airports.filter((a) => a.enabled).length;
  const disabledCount = airports.length - enabledCount;
  const recentJobs = jobs.slice(0, 10);

  // Group data gaps by airport
  const gapsByAirport = dataGaps.reduce(
    (acc, gap) => {
      if (!acc[gap.iataCode]) acc[gap.iataCode] = [];
      acc[gap.iataCode].push(gap);
      return acc;
    },
    {} as Record<string, DataGap[]>,
  );
  const airportsWithGaps = Object.keys(gapsByAirport).length;

  if (loading) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] text-zinc-100 flex items-center justify-center">
        <p className="font-mono text-sm text-zinc-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <h1 className="font-grotesk text-xl font-bold">Admin Dashboard</h1>
          <div className="flex items-center gap-4">
            <Link
              to="/admin/jobs"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Jobs
            </Link>
            <Link
              to="/admin/airports"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Airports
            </Link>
            <button
              onClick={handleLogout}
              className="font-mono text-xs text-zinc-500 hover:text-zinc-300"
            >
              Logout
            </button>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-8">
          <div className="bg-zinc-900 border border-zinc-800 p-4">
            <p className="font-mono text-xs text-zinc-500 mb-1">
              Supported Airports
            </p>
            <p className="font-grotesk text-2xl font-bold">
              {enabledCount}
              <span className="text-sm text-zinc-500 ml-2">
                +{disabledCount} disabled
              </span>
            </p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 p-4">
            <p className="font-mono text-xs text-zinc-500 mb-1">Data Gaps</p>
            <p className="font-grotesk text-2xl font-bold">
              {dataGaps.length}
              <span className="text-sm text-zinc-500 ml-2">
                across {airportsWithGaps} airports
              </span>
            </p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 p-4">
            <p className="font-mono text-xs text-zinc-500 mb-1">Active Jobs</p>
            <p className="font-grotesk text-2xl font-bold">
              {
                jobs.filter(
                  (j) => j.status === "running" || j.status === "queued",
                ).length
              }
            </p>
          </div>
        </div>

        {/* Quick Actions */}
        <div className="flex gap-3 mb-8">
          <button
            onClick={handleRefreshAll}
            disabled={actionLoading != null}
            className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-1.5 disabled:opacity-50"
          >
            {actionLoading === "refresh" ? "Starting..." : "Refresh All"}
          </button>
          <button
            onClick={handleScoring}
            disabled={actionLoading != null}
            className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-1.5 disabled:opacity-50"
          >
            {actionLoading === "scoring" ? "Running..." : "Run Scoring"}
          </button>
        </div>

        {/* Recent Jobs */}
        <div className="mb-8">
          <div className="flex items-center justify-between mb-3">
            <h2 className="font-grotesk text-sm font-bold text-zinc-300">
              Recent Jobs
            </h2>
            <Link
              to="/admin/jobs"
              className="font-mono text-xs text-zinc-500 hover:text-zinc-300"
            >
              View all
            </Link>
          </div>
          {recentJobs.length === 0 ? (
            <p className="font-mono text-xs text-zinc-500">No jobs yet</p>
          ) : (
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    ID
                  </th>
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    Status
                  </th>
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    Airports
                  </th>
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    Sources
                  </th>
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    Progress
                  </th>
                  <th className="font-mono text-xs text-zinc-500 text-left py-2">
                    Started
                  </th>
                </tr>
              </thead>
              <tbody>
                {recentJobs.map((job) => (
                  <tr key={job.id} className="border-b border-zinc-800/50">
                    <td className="font-mono text-xs text-zinc-400 py-2">
                      <Link
                        to="/admin/jobs"
                        className="hover:text-zinc-100"
                      >
                        {job.id.slice(0, 8)}
                      </Link>
                    </td>
                    <td className="py-2">
                      <StatusBadge status={job.status} />
                    </td>
                    <td className="font-mono text-xs text-zinc-400 py-2">
                      {job.airports.length > 3
                        ? `${job.airports.slice(0, 3).join(", ")}...`
                        : job.airports.join(", ") || "all"}
                    </td>
                    <td className="font-mono text-xs text-zinc-400 py-2">
                      {job.sources.length > 3
                        ? `${job.sources.slice(0, 3).join(", ")}...`
                        : job.sources.join(", ") || "all"}
                    </td>
                    <td className="font-mono text-xs text-zinc-400 py-2">
                      {job.progress.airportsCompleted}/
                      {job.progress.airportsTotal}
                      {job.progress.currentAirport &&
                        ` (${job.progress.currentAirport})`}
                    </td>
                    <td className="font-mono text-xs text-zinc-500 py-2">
                      {job.startedAt
                        ? new Date(job.startedAt).toLocaleString()
                        : "-"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Data Gaps Summary */}
        {dataGaps.length > 0 && (
          <div>
            <h2 className="font-grotesk text-sm font-bold text-zinc-300 mb-3">
              Data Gaps
            </h2>
            <div className="max-h-64 overflow-y-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-800">
                    <th className="font-mono text-xs text-zinc-500 text-left py-2">
                      Airport
                    </th>
                    <th className="font-mono text-xs text-zinc-500 text-left py-2">
                      Source
                    </th>
                    <th className="font-mono text-xs text-zinc-500 text-left py-2">
                      Status
                    </th>
                    <th className="font-mono text-xs text-zinc-500 text-left py-2">
                      Last Fetched
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {dataGaps.map((gap, i) => (
                    <tr
                      key={`${gap.iataCode}-${gap.source}-${i}`}
                      className="border-b border-zinc-800/50"
                    >
                      <td className="font-mono text-xs text-zinc-300 py-2">
                        {gap.iataCode}{" "}
                        <span className="text-zinc-500">{gap.name}</span>
                      </td>
                      <td className="font-mono text-xs text-zinc-400 py-2">
                        {gap.source}
                      </td>
                      <td className="py-2">
                        <span
                          className={`font-mono text-xs px-2 py-0.5 ${
                            gap.lastStatus === "failed"
                              ? "text-red-400 bg-red-400/10"
                              : gap.lastStatus === "stale"
                                ? "text-yellow-400 bg-yellow-400/10"
                                : "text-zinc-400 bg-zinc-400/10"
                          }`}
                        >
                          {gap.lastStatus}
                        </span>
                      </td>
                      <td className="font-mono text-xs text-zinc-500 py-2">
                        {gap.lastFetchedAt
                          ? new Date(gap.lastFetchedAt).toLocaleDateString()
                          : "never"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function AdminDashboard() {
  const { authenticated, setAuthenticated } = useAdmin();

  if (authenticated === null) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
        <p className="font-mono text-sm text-zinc-500">Loading...</p>
      </div>
    );
  }

  if (!authenticated) {
    return <LoginForm onLogin={() => setAuthenticated(true)} />;
  }

  return <Dashboard />;
}
