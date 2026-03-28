import { useState, useEffect, useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
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
import { OwnershipBadge } from "~/components/ownership-badge";
import {
  EditOperatorDialog,
  CreateOperatorDialog,
  AirportMappingDialog,
  type Operator,
} from "~/components/admin/operator-dialogs";
import { Plus, Pencil, Trash2, Link2, Loader2, Search } from "lucide-react";
import { adminListOperators, adminDeleteOperator } from "~/server/admin";

export const Route = createFileRoute("/admin/operators")({
  component: AdminOperators,
});

function AdminOperators() {
  const { authenticated } = useAdminAuth();
  const [operators, setOperators] = useState<Operator[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("");
  const [editOp, setEditOp] = useState<Operator | null>(null);
  const [airportsOp, setAirportsOp] = useState<Operator | null>(null);
  const [createOpen, setCreateOpen] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const password = localStorage.getItem("admin_password") || "";
      const data = await adminListOperators({ data: password });
      setOperators(data);
    } catch (err) {
      console.error("Failed to fetch operators", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (authenticated) fetchData();
  }, [authenticated, fetchData]);

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

  const filtered = filter
    ? operators.filter(
        (o) =>
          o.name.toLowerCase().includes(filter.toLowerCase()) ||
          (o.shortName?.toLowerCase().includes(filter.toLowerCase()) ??
            false) ||
          (o.countryCode?.toLowerCase().includes(filter.toLowerCase()) ??
            false),
      )
    : operators;

  return (
    <AdminLayout
      title="Operators"
      actions={
        <Button
          size="sm"
          onClick={() => setCreateOpen(true)}
          className="bg-green-600 hover:bg-green-700 text-white"
        >
          <Plus className="size-3.5" />
          Add Operator
        </Button>
      }
    >
      <div className="flex items-center gap-3 mb-4">
        <div className="relative">
          <Search className="absolute left-2.5 top-2 size-3.5 text-muted-foreground" />
          <Input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter operators..."
            className="pl-8 h-8 w-64"
          />
        </div>
        <span className="text-xs text-muted-foreground">
          {filtered.length} operators
        </span>
      </div>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead className="w-16">Country</TableHead>
            <TableHead className="w-20">Model</TableHead>
            <TableHead className="w-16 text-right">Public</TableHead>
            <TableHead className="w-16 text-right">Airports</TableHead>
            <TableHead className="w-32">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {filtered.map((op) => (
            <TableRow key={op.id}>
              <TableCell>
                <div>
                  <span className="font-medium">
                    {op.shortName || op.name}
                  </span>
                  {op.shortName && op.shortName !== op.name && (
                    <span className="text-xs text-muted-foreground ml-2 truncate">
                      {op.name}
                    </span>
                  )}
                </div>
              </TableCell>
              <TableCell className="text-muted-foreground">
                {op.countryCode || "—"}
              </TableCell>
              <TableCell>
                <OwnershipBadge model={op.ownershipModel} />
              </TableCell>
              <TableCell className="text-right text-muted-foreground">
                {op.publicSharePct != null
                  ? `${op.publicSharePct.toFixed(0)}%`
                  : "—"}
              </TableCell>
              <TableCell className="text-right">
                <Badge variant="secondary">{op.airportCount}</Badge>
              </TableCell>
              <TableCell>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={() => setEditOp(op)}
                  >
                    <Pencil className="size-3" />
                    Edit
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={() => setAirportsOp(op)}
                  >
                    <Link2 className="size-3" />
                    Airports
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    className="text-destructive"
                    onClick={async () => {
                      if (!confirm(`Delete ${op.shortName || op.name}?`))
                        return;
                      const password =
                        localStorage.getItem("admin_password") || "";
                      await adminDeleteOperator({
                        data: { password, id: op.id },
                      });
                      fetchData();
                    }}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {editOp && (
        <EditOperatorDialog
          operator={editOp}
          onClose={() => setEditOp(null)}
          onSaved={fetchData}
        />
      )}
      {airportsOp && (
        <AirportMappingDialog
          operator={airportsOp}
          onClose={() => setAirportsOp(null)}
          onSaved={fetchData}
        />
      )}
      {createOpen && (
        <CreateOperatorDialog
          onClose={() => setCreateOpen(false)}
          onSaved={fetchData}
        />
      )}
    </AdminLayout>
  );
}
