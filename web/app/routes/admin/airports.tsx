import { useState, useEffect, useCallback, useMemo } from "react";
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
import { Input } from "~/components/ui/input";
import { Badge } from "~/components/ui/badge";
import { Checkbox } from "~/components/ui/checkbox";
import { BatchImportModal } from "~/components/admin/batch-import-modal";
import {
  Plus,
  FileText,
  Pencil,
  Trash2,
  Play,
  Loader2,
  Search,
  Check,
  X,
} from "lucide-react";
import {
  adminListAirports,
  adminCreateAirport,
  adminUpdateAirport,
  adminDeleteAirport,
  adminStartJob,
} from "~/server/admin";
import type { components } from "~/api/types";
import { PIPELINE_SOURCES } from "~/utils/constants";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];
type SourceStatus = components["schemas"]["SourceStatusResponse"];

export const Route = createFileRoute("/admin/airports")({
  component: AdminAirports,
});

function SourceDot({
  name,
  source,
}: {
  name: string;
  source?: SourceStatus;
}) {
  const now = Date.now();
  const fetched = source?.lastFetchedAt
    ? new Date(source.lastFetchedAt).getTime()
    : 0;
  const daysSince = fetched
    ? (now - fetched) / (1000 * 60 * 60 * 24)
    : Infinity;

  let color: string;
  let status: string;
  let statusColor: string;

  if (!source || !fetched) {
    color = "bg-zinc-600";
    status = "never ran";
    statusColor = "text-zinc-500";
  } else if (source.lastStatus === "success") {
    if (daysSince < 30) {
      color = "bg-green-400";
      status = `${Math.floor(daysSince)}d ago`;
      statusColor = "text-green-400";
    } else {
      color = "bg-yellow-400";
      status = `${Math.floor(daysSince)}d ago (stale)`;
      statusColor = "text-yellow-400";
    }
  } else if (source.lastStatus === "failed") {
    color = "bg-red-400";
    status = `failed${source.lastError ? `: ${source.lastError}` : ""}`;
    statusColor = "text-red-400";
  } else {
    color = "bg-zinc-600";
    status = source.lastStatus;
    statusColor = "text-zinc-500";
  }

  return (
    <span className="relative group">
      <span
        className={`inline-block w-2 h-2 rounded-full ${color} mr-1 cursor-help`}
      />
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:flex flex-col items-start bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1.5 whitespace-nowrap z-50 shadow-lg pointer-events-none">
        <span className="font-mono text-[10px] font-bold text-zinc-300">
          {name}
        </span>
        <span className={`font-mono text-[10px] ${statusColor}`}>
          {status}
        </span>
      </span>
    </span>
  );
}

function ScoreDot({ hasScore }: { hasScore: boolean }) {
  const color = hasScore ? "bg-blue-400" : "bg-zinc-600";
  const status = hasScore ? "scored" : "not scored";
  const statusColor = hasScore ? "text-blue-400" : "text-zinc-500";

  return (
    <span className="relative group ml-1">
      <span
        className={`inline-block w-2 h-2 rounded-sm ${color} cursor-help`}
      />
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:flex flex-col items-start bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1.5 whitespace-nowrap z-50 shadow-lg pointer-events-none">
        <span className="font-mono text-[10px] font-bold text-zinc-300">
          score
        </span>
        <span className={`font-mono text-[10px] ${statusColor}`}>
          {status}
        </span>
      </span>
    </span>
  );
}

function SourceIndicators({
  sources,
  hasScore,
}: {
  sources: SourceStatus[];
  hasScore: boolean;
}) {
  const byName = new Map(sources.map((s) => [s.source, s]));
  return (
    <span className="flex flex-wrap items-center gap-0">
      {PIPELINE_SOURCES.map((name) => (
        <SourceDot key={name} name={name} source={byName.get(name)} />
      ))}
      <ScoreDot hasScore={hasScore} />
    </span>
  );
}

function AddAirportDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: () => void;
}) {
  const [form, setForm] = useState({
    iata_code: "",
    name: "",
    country_code: "",
    skytrax_review_slug: "",
    skytrax_rating_slug: "",
    google_maps_url: "",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminCreateAirport({
        data: {
          password,
          body: {
            iata_code: form.iata_code.toUpperCase(),
            name: form.name,
            country_code: form.country_code.toUpperCase(),
            skytrax_review_slug: form.skytrax_review_slug || null,
            skytrax_rating_slug: form.skytrax_rating_slug || null,
            google_maps_url: form.google_maps_url || null,
          },
        },
      });
      setForm({
        iata_code: "",
        name: "",
        country_code: "",
        skytrax_review_slug: "",
        skytrax_rating_slug: "",
        google_maps_url: "",
      });
      onOpenChange(false);
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Add Airport</DialogTitle>
          <DialogDescription>
            Add a new supported airport to the pipeline.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="text-sm font-medium mb-1 block">
                IATA Code *
              </label>
              <Input
                value={form.iata_code}
                onChange={(e) =>
                  setForm({ ...form, iata_code: e.target.value })
                }
                maxLength={3}
                required
                placeholder="BER"
                className="uppercase"
              />
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">Name *</label>
              <Input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                required
                placeholder="Berlin Brandenburg"
              />
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">
                Country *
              </label>
              <Input
                value={form.country_code}
                onChange={(e) =>
                  setForm({ ...form, country_code: e.target.value })
                }
                maxLength={2}
                required
                placeholder="DE"
                className="uppercase"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-sm font-medium mb-1 block">
                Skytrax Review Slug
              </label>
              <Input
                value={form.skytrax_review_slug}
                onChange={(e) =>
                  setForm({ ...form, skytrax_review_slug: e.target.value })
                }
                placeholder="berlin-brandenburg-airport"
              />
            </div>
            <div>
              <label className="text-sm font-medium mb-1 block">
                Skytrax Rating Slug
              </label>
              <Input
                value={form.skytrax_rating_slug}
                onChange={(e) =>
                  setForm({ ...form, skytrax_rating_slug: e.target.value })
                }
                placeholder="berlin-brandenburg"
              />
            </div>
          </div>

          <div>
            <label className="text-sm font-medium mb-1 block">
              Google Maps URL
            </label>
            <Input
              value={form.google_maps_url}
              onChange={(e) =>
                setForm({ ...form, google_maps_url: e.target.value })
              }
              placeholder="https://maps.google.com/..."
            />
          </div>

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={loading}>
              {loading && <Loader2 className="size-3 animate-spin" />}
              {loading ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function EditRow({
  airport,
  onSaved,
  onCancel,
}: {
  airport: SupportedAirport;
  onSaved: () => void;
  onCancel: () => void;
}) {
  const [form, setForm] = useState({
    enabled: airport.enabled,
    name: airport.name,
    skytrax_review_slug: airport.skytraxReviewSlug || "",
    skytrax_rating_slug: airport.skytraxRatingSlug || "",
    google_maps_url: airport.googleMapsUrl || "",
  });
  const [loading, setLoading] = useState(false);

  const handleSave = async () => {
    setLoading(true);
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminUpdateAirport({
        data: {
          password,
          iata: airport.iataCode,
          body: {
            enabled: form.enabled,
            name: form.name,
            skytrax_review_slug: form.skytrax_review_slug || null,
            skytrax_rating_slug: form.skytrax_rating_slug || null,
            google_maps_url: form.google_maps_url || null,
          },
        },
      });
      onSaved();
    } catch (err) {
      console.error("Failed to update", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <TableRow className="bg-muted/30">
      <TableCell className="font-mono text-xs">{airport.iataCode}</TableCell>
      <TableCell>
        <Input
          value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })}
          className="h-7 text-xs"
        />
      </TableCell>
      <TableCell className="font-mono text-xs text-muted-foreground">
        {airport.countryCode}
      </TableCell>
      <TableCell>
        <Checkbox
          checked={form.enabled}
          onCheckedChange={(v) => setForm({ ...form, enabled: v === true })}
        />
      </TableCell>
      <TableCell>
        <SourceIndicators
          sources={airport.sources}
          hasScore={airport.hasScore}
        />
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="xs"
            onClick={handleSave}
            disabled={loading}
          >
            {loading ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              <Check className="size-3" />
            )}
            Save
          </Button>
          <Button variant="ghost" size="xs" onClick={onCancel}>
            <X className="size-3" />
            Cancel
          </Button>
        </div>
      </TableCell>
    </TableRow>
  );
}

function AdminAirports() {
  const { authenticated } = useAdminAuth();
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingIata, setEditingIata] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [fetchingIata, setFetchingIata] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [batchOpen, setBatchOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const password = localStorage.getItem("admin_password") || "";
      const a = await adminListAirports({ data: password });
      setAirports(a);
    } catch (err) {
      console.error("Failed to fetch airports", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (authenticated) fetchData();
  }, [authenticated, fetchData]);

  const handleDelete = async (iata: string) => {
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminDeleteAirport({ data: { password, iata } });
      setDeleteConfirm(null);
      await fetchData();
    } catch (err) {
      console.error("Failed to delete", err);
    }
  };

  const handleFetchNow = async (iata: string) => {
    setFetchingIata(iata);
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminStartJob({
        data: { password, body: { airports: [iata] } },
      });
    } catch (err) {
      console.error("Failed to start fetch", err);
    } finally {
      setFetchingIata(null);
    }
  };

  const filtered = useMemo(
    () =>
      airports.filter(
        (a) =>
          !filter ||
          a.iataCode.toLowerCase().includes(filter.toLowerCase()) ||
          a.name.toLowerCase().includes(filter.toLowerCase()) ||
          a.countryCode.toLowerCase().includes(filter.toLowerCase()),
      ),
    [airports, filter],
  );

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
      title="Airports"
      actions={
        <>
          <Button size="sm" onClick={() => setAddOpen(true)}>
            <Plus className="size-3.5" />
            Add Airport
          </Button>
          <Button
            size="sm"
            onClick={() => setBatchOpen(true)}
            className="bg-green-600 hover:bg-green-700 text-white"
          >
            <FileText className="size-3.5" />
            Batch Import
          </Button>
        </>
      }
    >
      <AddAirportDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onCreated={fetchData}
      />
      <BatchImportModal
        open={batchOpen}
        onOpenChange={setBatchOpen}
        onComplete={fetchData}
      />

      {/* Filter */}
      <div className="flex items-center gap-3 mb-4">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by IATA, name, or country..."
            className="pl-8 w-64"
          />
        </div>
        <span className="text-xs text-muted-foreground">
          {filtered.length} airports
        </span>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 mb-3">
        <span className="text-xs text-muted-foreground">Sources:</span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-green-400" />
          <span className="text-xs text-muted-foreground">recent</span>
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-yellow-400" />
          <span className="text-xs text-muted-foreground">stale</span>
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-red-400" />
          <span className="text-xs text-muted-foreground">failed</span>
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-full bg-zinc-600" />
          <span className="text-xs text-muted-foreground">never ran</span>
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block w-2 h-2 rounded-sm bg-blue-400" />
          <span className="text-xs text-muted-foreground">scored</span>
        </span>
      </div>

      {/* Table */}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>IATA</TableHead>
            <TableHead>Name</TableHead>
            <TableHead>Country</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Sources</TableHead>
            <TableHead>Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {filtered.map((airport) =>
            editingIata === airport.iataCode ? (
              <EditRow
                key={airport.iataCode}
                airport={airport}
                onSaved={() => {
                  setEditingIata(null);
                  fetchData();
                }}
                onCancel={() => setEditingIata(null)}
              />
            ) : (
              <TableRow key={airport.iataCode}>
                <TableCell className="font-mono text-xs">
                  {airport.iataCode}
                </TableCell>
                <TableCell className="text-sm text-muted-foreground">
                  {airport.name}
                </TableCell>
                <TableCell className="font-mono text-xs text-muted-foreground">
                  {airport.countryCode}
                </TableCell>
                <TableCell>
                  <Badge
                    variant={airport.enabled ? "default" : "secondary"}
                  >
                    {airport.enabled ? "enabled" : "disabled"}
                  </Badge>
                </TableCell>
                <TableCell>
                  <SourceIndicators
                    sources={airport.sources}
                    hasScore={airport.hasScore}
                  />
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="xs"
                      onClick={() => setEditingIata(airport.iataCode)}
                    >
                      <Pencil className="size-3" />
                      Edit
                    </Button>
                    <Button
                      variant="ghost"
                      size="xs"
                      onClick={() => handleFetchNow(airport.iataCode)}
                      disabled={fetchingIata === airport.iataCode}
                    >
                      {fetchingIata === airport.iataCode ? (
                        <Loader2 className="size-3 animate-spin" />
                      ) : (
                        <Play className="size-3" />
                      )}
                      Fetch
                    </Button>
                    {deleteConfirm === airport.iataCode ? (
                      <>
                        <Button
                          variant="destructive"
                          size="xs"
                          onClick={() => handleDelete(airport.iataCode)}
                        >
                          Confirm
                        </Button>
                        <Button
                          variant="ghost"
                          size="xs"
                          onClick={() => setDeleteConfirm(null)}
                        >
                          No
                        </Button>
                      </>
                    ) : (
                      <Button
                        variant="ghost"
                        size="xs"
                        onClick={() => setDeleteConfirm(airport.iataCode)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="size-3" />
                        Delete
                      </Button>
                    )}
                  </div>
                </TableCell>
              </TableRow>
            ),
          )}
        </TableBody>
      </Table>
    </AdminLayout>
  );
}
