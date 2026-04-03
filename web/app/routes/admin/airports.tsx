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
import { AddAirportDialog } from "~/components/admin/add-airport-dialog";
import { EditRow } from "~/components/admin/edit-airport-row";
import { SourceIndicators } from "~/components/admin/source-indicators";
import {
  Plus,
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
  const [editingIata, setEditingIata] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [fetchingIata, setFetchingIata] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [batchOpen, setBatchOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);

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
        onCreated={fetchAirports}
      />
      <BatchImportModal
        open={batchOpen}
        onOpenChange={setBatchOpen}
        onComplete={fetchAirports}
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
          {filtered.map((airport) =>
            editingIata === airport.iataCode ? (
              <EditRow
                key={airport.iataCode}
                airport={airport}
                onSaved={() => {
                  setEditingIata(null);
                  fetchAirports();
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
