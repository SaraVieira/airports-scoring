import { useState, useEffect, useCallback } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { AdminLayout } from "~/components/admin-layout";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "~/components/ui/dialog";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "~/components/ui/table";
import { Checkbox } from "~/components/ui/checkbox";
import { Input } from "~/components/ui/input";
import { JobStatusBadge } from "~/components/admin/job-status-badge";
import {
  adminStartJob,
  adminListJobs,
  adminListAirports,
  adminCancelJob,
} from "~/server/admin";
import { ALL_SOURCES } from "~/utils/constants";
import type { components } from "~/api/types";
import { Plus, Loader2, X } from "lucide-react";

type JobInfo = components["schemas"]["JobInfo"];
type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

const SOURCE_DESCRIPTIONS: Record<string, string> = {
  ourairports:
    "Runways, frequencies, navaids, basic airport info from OurAirports database",
  wikipedia:
    "Passenger stats, opened year, operator, terminals, Skytrax history, ACI awards",
  eurocontrol: "Monthly flight counts, ATFM delays, delay cause breakdown",
  eurostat: "Historical passenger traffic from EU statistics",
  routes: "Route network from OPDI + FlightRadar24 fallback",
  metar: "Daily weather — temperature, wind, visibility, fog flags",
  reviews:
    "Skytrax + Google Maps reviews (both scrapers). Longest running source",
  skytrax: "Skytrax reviews only",
  google_reviews: "Google Maps reviews only — requires scraper service",
  sentiment:
    "RoBERTa + NLI ML sentiment analysis on unprocessed reviews. Requires HF_TOKEN",
  opensky: "Flight movements from OpenSky Network",
  caa: "UK CAA passenger statistics",
  aena: "Spanish AENA passenger statistics",
  carbon_accreditation: "ACI carbon accreditation levels",
  priority_pass: "Priority Pass lounge data",
};

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

function NewJobDialog({
  open,
  onOpenChange,
  airports,
  onStarted,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
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

  const handleSubmit = async () => {
    setLoading(true);
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminStartJob({
        data: {
          password,
          body: {
            airports: selectedAirports.length > 0 ? selectedAirports : null,
            sources: selectedSources.length > 0 ? selectedSources : null,
            fullRefresh: fullRefresh || null,
            score: score || null,
          },
        },
      });
      setSelectedAirports([]);
      setSelectedSources([]);
      setFullRefresh(false);
      setScore(false);
      setAirportFilter("");
      onOpenChange(false);
      onStarted();
    } catch (err) {
      console.error("Failed to start job", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-5xl flex flex-col" style={{ height: "min(85vh, 700px)" }}>
        <DialogHeader>
          <DialogTitle>New Job</DialogTitle>
          <DialogDescription>
            Configure and start a new pipeline job. Leave airports and sources
            empty to run all.
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 min-h-0 grid grid-cols-[1fr_1.2fr] gap-6 py-2">
          {/* Left: Airport selection */}
          <div className="flex flex-col min-h-0">
            <label className="text-sm font-medium mb-2 block">
              Airports{" "}
              <span className="text-muted-foreground font-normal">
                (empty = all enabled)
              </span>
            </label>
            <Input
              value={airportFilter}
              onChange={(e) => setAirportFilter(e.target.value)}
              placeholder="Filter airports..."
              className="mb-2"
            />
            <div className="flex-1 overflow-y-auto border rounded-md bg-muted/20 p-1 min-h-0">
              {filteredAirports.map((a) => (
                <label
                  key={a.iataCode}
                  className={`flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors ${
                    selectedAirports.includes(a.iataCode)
                      ? "bg-green-500/10"
                      : "hover:bg-muted"
                  }`}
                >
                  <Checkbox
                    checked={selectedAirports.includes(a.iataCode)}
                    onCheckedChange={() => toggleAirport(a.iataCode)}
                  />
                  <span className="text-sm font-mono font-medium">
                    {a.iataCode}
                  </span>
                  <span className="text-sm text-muted-foreground truncate">
                    {a.name}
                  </span>
                </label>
              ))}
            </div>
            {selectedAirports.length > 0 && (
              <p className="text-xs text-muted-foreground mt-2">
                {selectedAirports.length} selected:{" "}
                {selectedAirports.join(", ")}
              </p>
            )}
          </div>

          {/* Right: Source selection */}
          <div className="flex flex-col min-h-0">
            <label className="text-sm font-medium mb-2 block">
              Sources{" "}
              <span className="text-muted-foreground font-normal">
                (empty = all sources)
              </span>
            </label>
            <div className="flex-1 overflow-y-auto border rounded-md bg-muted/20 p-1 space-y-0.5 min-h-0">
              {ALL_SOURCES.map((s) => (
                <label
                  key={s}
                  className={`flex items-start gap-2 px-2.5 py-2 rounded cursor-pointer transition-colors ${
                    selectedSources.includes(s)
                      ? "bg-green-500/10"
                      : "hover:bg-muted"
                  }`}
                >
                  <Checkbox
                    checked={selectedSources.includes(s)}
                    onCheckedChange={() => toggleSource(s)}
                    className="mt-0.5"
                  />
                  <div className="min-w-0">
                    <span className="text-sm font-medium block">{s}</span>
                    {SOURCE_DESCRIPTIONS[s] && (
                      <span className="text-xs text-muted-foreground block leading-relaxed">
                        {SOURCE_DESCRIPTIONS[s]}
                      </span>
                    )}
                  </div>
                </label>
              ))}
            </div>
          </div>
        </div>

        {/* Options */}
        <div className="flex items-center gap-6 pt-2">
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={fullRefresh}
              onCheckedChange={(v) => setFullRefresh(v === true)}
            />
            Full Refresh
          </label>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={score}
              onCheckedChange={(v) => setScore(v === true)}
            />
            Score After
          </label>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={loading}
          >
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading}>
            {loading && <Loader2 className="size-3 animate-spin" />}
            {loading ? "Starting..." : "Start Job"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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

  const handleCancel = async (id: string) => {
    const password = localStorage.getItem("admin_password") || "";
    try {
      await adminCancelJob({ data: { password, id } });
      await fetchData();
    } catch (err) {
      console.error("Failed to cancel job", err);
    }
  };

  if (authenticated === false) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-center">
          <p className="text-sm text-muted-foreground mb-4">
            Not authenticated
          </p>
          <Link
            to="/admin"
            className="text-xs text-muted-foreground hover:text-foreground"
          >
            Go to login
          </Link>
        </div>
      </div>
    );
  }

  if (loading || authenticated === null) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <Loader2 className="size-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <AdminLayout
      title="Jobs"
      actions={
        <Button
          onClick={() => setDialogOpen(true)}
          className="bg-green-600 hover:bg-green-700 text-white"
        >
          <Plus className="size-3" />
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

      {jobs.length === 0 ? (
        <p className="text-sm text-muted-foreground py-8 text-center">
          No jobs yet. Start one with the button above.
        </p>
      ) : (
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
                <TableCell className="font-mono text-xs max-w-32 truncate">
                  {job.airports.length > 0 ? job.airports.join(", ") : "all"}
                </TableCell>
                <TableCell className="font-mono text-xs max-w-32 truncate">
                  {job.sources.length > 0 ? job.sources.join(", ") : "all"}
                </TableCell>
                <TableCell>
                  <ProgressBar progress={job.progress} />
                </TableCell>
                <TableCell className="font-mono text-xs text-muted-foreground">
                  {job.startedAt
                    ? new Date(job.startedAt).toLocaleString()
                    : "-"}
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
                  {job.error && (
                    <span
                      className="text-xs text-destructive cursor-help"
                      title={job.error}
                    >
                      error
                    </span>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </AdminLayout>
  );
}
