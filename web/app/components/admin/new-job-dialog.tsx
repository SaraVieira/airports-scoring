import { useState, useMemo } from "react";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "~/components/ui/dialog";
import { Checkbox } from "~/components/ui/checkbox";
import { Input } from "~/components/ui/input";
import { Loader2 } from "lucide-react";
import { adminStartJob } from "~/server/admin";
import { ALL_SOURCES } from "~/utils/constants";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

type AirportGapStatus = "ok" | "failed" | "stale" | "never";

function getAirportStatus(a: SupportedAirport): AirportGapStatus {
  if (a.sources.length === 0) return "never";
  if (a.sources.some((s) => s.lastStatus === "failed")) return "failed";
  const now = Date.now();
  if (a.sources.some((s) => {
    if (!s.lastFetchedAt) return true;
    return (now - new Date(s.lastFetchedAt).getTime()) / (1000 * 60 * 60 * 24) > 7;
  })) return "stale";
  return "ok";
}

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

export function NewJobDialog({
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
  const [statusFilter, setStatusFilter] = useState<"all" | "gaps" | "failed" | "never">("all");
  const [sortBy, setSortBy] = useState<"name" | "newest">("name");

  const filteredAirports = useMemo(() => {
    let result = airports.filter((a) => a.enabled);

    if (statusFilter === "gaps") {
      result = result.filter((a) => getAirportStatus(a) !== "ok");
    } else if (statusFilter === "failed") {
      result = result.filter((a) => getAirportStatus(a) === "failed");
    } else if (statusFilter === "never") {
      result = result.filter((a) => getAirportStatus(a) === "never");
    }

    if (airportFilter) {
      const q = airportFilter.toLowerCase();
      result = result.filter(
        (a) =>
          a.iataCode.toLowerCase().includes(q) ||
          a.name.toLowerCase().includes(q),
      );
    }

    if (sortBy === "newest") {
      result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
    } else {
      result.sort((a, b) => a.iataCode.localeCompare(b.iataCode));
    }

    return result;
  }, [airports, airportFilter, statusFilter, sortBy]);

  const gapCounts = useMemo(() => {
    const enabled = airports.filter((a) => a.enabled);
    return {
      all: enabled.length,
      gaps: enabled.filter((a) => getAirportStatus(a) !== "ok").length,
      failed: enabled.filter((a) => getAirportStatus(a) === "failed").length,
      never: enabled.filter((a) => getAirportStatus(a) === "never").length,
    };
  }, [airports]);

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
      <DialogContent
        className="sm:max-w-5xl flex flex-col"
        style={{ height: "min(85vh, 700px)" }}
      >
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
            <label className="text-sm font-medium mb-1 block">
              Airports{" "}
              <span className="text-muted-foreground font-normal">
                (empty = all enabled)
              </span>
            </label>
            <div className="flex items-center gap-1 mb-2">
              <Button variant={statusFilter === "all" ? "secondary" : "ghost"} size="xs" onClick={() => setStatusFilter("all")}>
                All ({gapCounts.all})
              </Button>
              <Button variant={statusFilter === "gaps" ? "secondary" : "ghost"} size="xs" onClick={() => setStatusFilter("gaps")} className={statusFilter !== "gaps" ? "text-yellow-500" : ""}>
                Gaps ({gapCounts.gaps})
              </Button>
              <Button variant={statusFilter === "failed" ? "secondary" : "ghost"} size="xs" onClick={() => setStatusFilter("failed")} className={statusFilter !== "failed" ? "text-destructive" : ""}>
                Failed ({gapCounts.failed})
              </Button>
              <Button variant={statusFilter === "never" ? "secondary" : "ghost"} size="xs" onClick={() => setStatusFilter("never")}>
                New ({gapCounts.never})
              </Button>
              <div className="w-px h-4 bg-border mx-1" />
              <Button variant={sortBy === "name" ? "secondary" : "ghost"} size="xs" onClick={() => setSortBy("name")}>
                A-Z
              </Button>
              <Button variant={sortBy === "newest" ? "secondary" : "ghost"} size="xs" onClick={() => setSortBy("newest")}>
                Newest
              </Button>
            </div>
            <Input
              value={airportFilter}
              onChange={(e) => setAirportFilter(e.target.value)}
              placeholder="Filter airports..."
              className="mb-2"
            />
            <div className="flex-1 overflow-y-auto border rounded-md bg-muted/20 p-1 min-h-0">
              {filteredAirports.map((a) => {
                const status = getAirportStatus(a);
                const failedCount = a.sources.filter((s) => s.lastStatus === "failed").length;
                return (
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
                    <span className="text-sm text-muted-foreground truncate flex-1">
                      {a.name}
                    </span>
                    {status === "never" && (
                      <Badge variant="secondary" className="text-[10px] shrink-0">new</Badge>
                    )}
                    {status === "failed" && (
                      <Badge variant="destructive" className="text-[10px] shrink-0">{failedCount} failed</Badge>
                    )}
                    {status === "stale" && (
                      <Badge variant="outline" className="text-[10px] shrink-0 border-yellow-500/30 text-yellow-500 bg-yellow-500/10">stale</Badge>
                    )}
                  </label>
                );
              })}
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
