import { useState, useEffect, useCallback } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { api } from "~/api/client";
import type { components } from "~/api/types";

type JobInfo = components["schemas"]["JobInfo"];
type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

const SOURCES = [
  "ourairports",
  "wikipedia",
  "eurocontrol",
  "eurostat",
  "routes",
  "metar",
  "reviews",
  "skytrax",
  "google_reviews",
  "sentiment",
  "opensky",
  "caa",
  "aena",
];

export const Route = createFileRoute("/admin/jobs")({
  component: AdminJobs,
});

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

function ProgressBar({ progress }: { progress: JobInfo["progress"] }) {
  const pct =
    progress.airportsTotal > 0
      ? (progress.airportsCompleted / progress.airportsTotal) * 100
      : 0;
  return (
    <div className="flex items-center gap-2">
      <div className="w-24 h-1.5 bg-zinc-800 overflow-hidden">
        <div
          className="h-full bg-yellow-400 transition-all"
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="font-mono text-xs text-zinc-400">
        {progress.airportsCompleted}/{progress.airportsTotal}
      </span>
      {progress.currentAirport && (
        <span className="font-mono text-xs text-zinc-500">
          {progress.currentAirport}
          {progress.currentSource && ` / ${progress.currentSource}`}
        </span>
      )}
    </div>
  );
}

function StartJobForm({
  airports,
  onStarted,
}: {
  airports: SupportedAirport[];
  onStarted: () => void;
}) {
  const [selectedAirports, setSelectedAirports] = useState<string[]>([]);
  const [selectedSources, setSelectedSources] = useState<string[]>([]);
  const [fullRefresh, setFullRefresh] = useState(false);
  const [score, setScore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [airportFilter, setAirportFilter] = useState("");

  const filteredAirports = airports
    .filter((a) => a.enabled)
    .filter(
      (a) =>
        !airportFilter ||
        a.iataCode.toLowerCase().includes(airportFilter.toLowerCase()) ||
        a.name.toLowerCase().includes(airportFilter.toLowerCase()),
    );

  const toggleAirport = (iata: string) => {
    setSelectedAirports((prev) =>
      prev.includes(iata) ? prev.filter((a) => a !== iata) : [...prev, iata],
    );
  };

  const toggleSource = (source: string) => {
    setSelectedSources((prev) =>
      prev.includes(source)
        ? prev.filter((s) => s !== source)
        : [...prev, source],
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      await api.admin.startJob({
        airports: selectedAirports.length > 0 ? selectedAirports : null,
        sources: selectedSources.length > 0 ? selectedSources : null,
        fullRefresh: fullRefresh || null,
        score: score || null,
      });
      setSelectedAirports([]);
      setSelectedSources([]);
      setFullRefresh(false);
      setScore(false);
      onStarted();
    } catch (err) {
      console.error("Failed to start job", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-zinc-900 border border-zinc-800 p-4 mb-8"
    >
      <h2 className="font-grotesk text-sm font-bold text-zinc-300 mb-4">
        Start Custom Job
      </h2>

      <div className="grid grid-cols-2 gap-4 mb-4">
        {/* Airport selection */}
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-2">
            Airports{" "}
            <span className="text-zinc-600">(empty = all enabled)</span>
          </label>
          <input
            type="text"
            value={airportFilter}
            onChange={(e) => setAirportFilter(e.target.value)}
            placeholder="Filter airports..."
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 mb-2 focus:outline-none focus:border-zinc-500"
          />
          <div className="max-h-32 overflow-y-auto border border-zinc-800 bg-zinc-950 p-1">
            {filteredAirports.map((a) => (
              <label
                key={a.iataCode}
                className="flex items-center gap-2 px-1 py-0.5 hover:bg-zinc-800 cursor-pointer"
              >
                <input
                  type="checkbox"
                  checked={selectedAirports.includes(a.iataCode)}
                  onChange={() => toggleAirport(a.iataCode)}
                  className="accent-yellow-400"
                />
                <span className="font-mono text-xs text-zinc-300">
                  {a.iataCode}
                </span>
                <span className="font-mono text-xs text-zinc-500 truncate">
                  {a.name}
                </span>
              </label>
            ))}
          </div>
          {selectedAirports.length > 0 && (
            <p className="font-mono text-xs text-zinc-500 mt-1">
              {selectedAirports.length} selected:{" "}
              {selectedAirports.join(", ")}
            </p>
          )}
        </div>

        {/* Source selection */}
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-2">
            Sources{" "}
            <span className="text-zinc-600">(empty = all sources)</span>
          </label>
          <div className="border border-zinc-800 bg-zinc-950 p-1">
            {SOURCES.map((s) => (
              <label
                key={s}
                className="flex items-center gap-2 px-1 py-0.5 hover:bg-zinc-800 cursor-pointer"
              >
                <input
                  type="checkbox"
                  checked={selectedSources.includes(s)}
                  onChange={() => toggleSource(s)}
                  className="accent-yellow-400"
                />
                <span className="font-mono text-xs text-zinc-300">{s}</span>
              </label>
            ))}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-6 mb-4">
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={fullRefresh}
            onChange={(e) => setFullRefresh(e.target.checked)}
            className="accent-yellow-400"
          />
          <span className="font-mono text-xs text-zinc-300">Full Refresh</span>
        </label>
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={score}
            onChange={(e) => setScore(e.target.checked)}
            className="accent-yellow-400"
          />
          <span className="font-mono text-xs text-zinc-300">
            Run Scoring After
          </span>
        </label>
      </div>

      <button
        type="submit"
        disabled={loading}
        className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-1.5 disabled:opacity-50"
      >
        {loading ? "Starting..." : "Start Job"}
      </button>
    </form>
  );
}

function AdminJobs() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [j, a] = await Promise.all([
        api.admin.listJobs(),
        api.admin.listSupportedAirports(),
      ]);
      setJobs(j);
      setAirports(a);
    } catch {
      // If auth fails, redirect to login
      localStorage.removeItem("admin_password");
      setAuthenticated(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const password = localStorage.getItem("admin_password");
    if (!password) {
      setAuthenticated(false);
      setLoading(false);
      return;
    }
    api.admin
      .listSupportedAirports()
      .then(() => {
        setAuthenticated(true);
        fetchData();
      })
      .catch(() => {
        localStorage.removeItem("admin_password");
        setAuthenticated(false);
        setLoading(false);
      });
  }, [fetchData]);

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

  const handleCancel = async (id: string) => {
    try {
      await api.admin.cancelJob(id);
      await fetchData();
    } catch (err) {
      console.error("Failed to cancel job", err);
    }
  };

  if (authenticated === false) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
        <div className="text-center">
          <p className="font-mono text-sm text-zinc-500 mb-4">
            Not authenticated
          </p>
          <Link
            to="/admin"
            className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
          >
            Go to login
          </Link>
        </div>
      </div>
    );
  }

  if (loading || authenticated === null) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
        <p className="font-mono text-sm text-zinc-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <h1 className="font-grotesk text-xl font-bold">Job Management</h1>
          <div className="flex items-center gap-4">
            <Link
              to="/admin"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Dashboard
            </Link>
            <Link
              to="/admin/airports"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Airports
            </Link>
          </div>
        </div>

        <StartJobForm airports={airports} onStarted={fetchData} />

        {/* Jobs List */}
        <h2 className="font-grotesk text-sm font-bold text-zinc-300 mb-3">
          All Jobs
        </h2>
        {jobs.length === 0 ? (
          <p className="font-mono text-xs text-zinc-500">No jobs</p>
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
                <th className="font-mono text-xs text-zinc-500 text-left py-2">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr key={job.id} className="border-b border-zinc-800/50">
                  <td className="font-mono text-xs text-zinc-400 py-2">
                    {job.id.slice(0, 8)}
                  </td>
                  <td className="py-2">
                    <StatusBadge status={job.status} />
                  </td>
                  <td className="font-mono text-xs text-zinc-400 py-2 max-w-32 truncate">
                    {job.airports.length > 0
                      ? job.airports.join(", ")
                      : "all"}
                  </td>
                  <td className="font-mono text-xs text-zinc-400 py-2 max-w-32 truncate">
                    {job.sources.length > 0
                      ? job.sources.join(", ")
                      : "all"}
                  </td>
                  <td className="py-2">
                    <ProgressBar progress={job.progress} />
                  </td>
                  <td className="font-mono text-xs text-zinc-500 py-2">
                    {job.startedAt
                      ? new Date(job.startedAt).toLocaleString()
                      : "-"}
                  </td>
                  <td className="py-2">
                    {(job.status === "running" || job.status === "queued") && (
                      <button
                        onClick={() => handleCancel(job.id)}
                        className="font-mono text-xs text-red-400 hover:text-red-300"
                      >
                        Cancel
                      </button>
                    )}
                    {job.error && (
                      <span
                        className="font-mono text-xs text-red-400 cursor-help"
                        title={job.error}
                      >
                        error
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
