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
import { Input } from "~/components/ui/input";
import { Badge } from "~/components/ui/badge";
import { BatchImportModal } from "~/components/admin/batch-import-modal";
import { EditAirportDialog } from "~/components/admin/edit-airport-dialog";
import { SourceIndicators } from "~/components/admin/source-indicators";
import {
  FileText,
  Pencil,
  Trash2,
  Play,
  Loader2,
  Search,
} from "lucide-react";
import { adminDeleteAirport, adminStartJob } from "~/server/admin";
import { useAdminStore, useAuthStore } from "~/stores/admin";

export const Route = createFileRoute("/admin/airports")({
  component: AdminAirports,
});

function AdminAirports() {
  const { authenticated } = useAdminAuth();
  const { airports, loading, fetchAirports } = useAdminStore();
  const [editingAirport, setEditingAirport] = useState<
    import("~/api/types").components["schemas"]["SupportedAirportWithStatus"] | null
  >(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [fetchingIata, setFetchingIata] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [countryFilter, setCountryFilter] = useState<string>("all");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [sortBy, setSortBy] = useState<string>("name");
  const [batchOpen, setBatchOpen] = useState(false);

  useEffect(() => {
    if (authenticated) fetchAirports();
  }, [authenticated, fetchAirports]);

  const handleDelete = async (iata: string) => {
    try {
      const password = useAuthStore.getState().password || "";
      await adminDeleteAirport({ data: { password, iata } });
      setDeleteConfirm(null);
      await fetchAirports();
    } catch (err) {
      console.error("Failed to delete", err);
    }
  };

  const handleFetchNow = async (iata: string) => {
    setFetchingIata(iata);
    try {
      const password = useAuthStore.getState().password || "";
      await adminStartJob({
        data: { password, body: { airports: [iata] } },
      });
    } catch (err) {
      console.error("Failed to start fetch", err);
    } finally {
      setFetchingIata(null);
    }
  };

  const [scoringIata, setScoringIata] = useState<string | null>(null);
  const handleScore = async (iata: string) => {
    setScoringIata(iata);
    try {
      const password = useAuthStore.getState().password || "";
      await adminStartJob({
        data: {
          password,
          body: { airports: [iata], sources: ["sentiment"], score: true },
        },
      });
    } catch (err) {
      console.error("Failed to start scoring", err);
    } finally {
      setScoringIata(null);
    }
  };

  const countries = useMemo(() => {
    const codes = [...new Set(airports.map((a) => a.countryCode))].sort();
    return codes;
  }, [airports]);

  const filtered = useMemo(() => {
    const lf = filter.toLowerCase();
    return airports
      .filter((a) => {
        if (
          lf &&
          !a.iataCode.toLowerCase().includes(lf) &&
          !a.name.toLowerCase().includes(lf) &&
          !a.countryCode.toLowerCase().includes(lf)
        )
          return false;
        if (countryFilter !== "all" && a.countryCode !== countryFilter) return false;
        if (statusFilter === "enabled" && !a.enabled) return false;
        if (statusFilter === "disabled" && a.enabled) return false;
        if (statusFilter === "scored" && !a.hasScore) return false;
        if (statusFilter === "unscored" && a.hasScore) return false;
        return true;
      })
      .sort((a, b) => {
        switch (sortBy) {
          case "iata": return a.iataCode.localeCompare(b.iataCode);
          case "country": return a.countryCode.localeCompare(b.countryCode) || a.name.localeCompare(b.name);
          case "newest": return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
          case "updated": return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
          default: return a.name.localeCompare(b.name);
        }
      });
  }, [airports, filter, countryFilter, statusFilter, sortBy]);

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
        <Button
          size="sm"
          onClick={() => setBatchOpen(true)}
          className="bg-green-600 hover:bg-green-700 text-white"
        >
          <FileText className="size-3.5" />
          Batch Import
        </Button>
      }
    >
      <BatchImportModal
        open={batchOpen}
        onOpenChange={setBatchOpen}
        onComplete={fetchAirports}
      />

      {/* Filters */}
      <div className="flex items-center gap-3 mb-4 flex-wrap">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by IATA, name..."
            className="pl-8 w-56"
          />
        </div>

        <select
          value={countryFilter}
          onChange={(e) => setCountryFilter(e.target.value)}
          className="h-9 rounded-md border border-border bg-muted px-3 text-xs text-foreground"
        >
          <option value="all">All countries</option>
          {countries.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>

        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          className="h-9 rounded-md border border-border bg-muted px-3 text-xs text-foreground"
        >
          <option value="all">All statuses</option>
          <option value="enabled">Enabled</option>
          <option value="disabled">Disabled</option>
          <option value="scored">Scored</option>
          <option value="unscored">Unscored</option>
        </select>

        <select
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value)}
          className="h-9 rounded-md border border-border bg-muted px-3 text-xs text-foreground"
        >
          <option value="name">Sort: Name</option>
          <option value="iata">Sort: IATA</option>
          <option value="country">Sort: Country</option>
          <option value="newest">Sort: Newest first</option>
          <option value="updated">Sort: Recently updated</option>
        </select>

        <span className="text-xs text-muted-foreground">
          {filtered.length} of {airports.length} airports
        </span>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 mb-3">
        <span className="text-xs text-muted-foreground">Sources:</span>
        {[
          { color: "bg-green-400", label: "recent", shape: "rounded-full" },
          { color: "bg-yellow-400", label: "stale", shape: "rounded-full" },
          { color: "bg-red-400", label: "failed", shape: "rounded-full" },
          { color: "bg-zinc-600", label: "never ran", shape: "rounded-full" },
          { color: "bg-blue-400", label: "scored", shape: "rounded-sm" },
        ].map((l) => (
          <span key={l.label} className="flex items-center gap-1">
            <span
              className={`inline-block w-2 h-2 ${l.shape} ${l.color}`}
            />
            <span className="text-xs text-muted-foreground">{l.label}</span>
          </span>
        ))}
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
          {filtered.map((airport) => (
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
                      onClick={() => setEditingAirport(airport)}
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
                    {airport.scoreStatus !== "scored" && (
                      <span
                        className="relative group"
                        title={
                          airport.scoreStatus === "too_small"
                            ? "Too few routes to score"
                            : undefined
                        }
                      >
                        <Button
                          variant="ghost"
                          size="xs"
                          onClick={() => handleScore(airport.iataCode)}
                          disabled={
                            airport.scoreStatus === "too_small" ||
                            scoringIata === airport.iataCode
                          }
                          className={
                            airport.scoreStatus === "too_small"
                              ? "opacity-40 cursor-not-allowed"
                              : ""
                          }
                        >
                          {scoringIata === airport.iataCode ? (
                            <Loader2 className="size-3 animate-spin" />
                          ) : (
                            <Play className="size-3" />
                          )}
                          Score
                        </Button>
                      </span>
                    )}
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
          ))}
        </TableBody>
      </Table>

      <EditAirportDialog
        open={editingAirport !== null}
        onOpenChange={(open) => !open && setEditingAirport(null)}
        airport={editingAirport}
        onSaved={() => {
          setEditingAirport(null);
          fetchAirports();
        }}
      />
    </AdminLayout>
  );
}
