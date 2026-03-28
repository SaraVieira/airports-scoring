import { useState, useEffect, useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useAdminAuth } from "~/hooks/use-admin-auth";
import { AdminLayout } from "~/components/admin-layout";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
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
import { Textarea } from "~/components/ui/textarea";
import { Badge } from "~/components/ui/badge";
import { OwnershipBadge } from "~/components/ownership-badge";
import { Plus, Pencil, Trash2, Link2, Loader2, Search } from "lucide-react";
import {
  adminListOperators,
  adminCreateOperator,
  adminUpdateOperator,
  adminDeleteOperator,
  adminGetOperatorAirports,
  adminSetOperatorAirports,
} from "~/server/admin";

export const Route = createFileRoute("/admin/operators")({
  component: AdminOperators,
});

interface Operator {
  id: number;
  name: string;
  shortName: string | null;
  countryCode: string | null;
  orgType: string;
  ownershipModel: string | null;
  publicSharePct: number | null;
  notes: string | null;
  airportCount: number;
}

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
          (o.shortName?.toLowerCase().includes(filter.toLowerCase()) ?? false) ||
          (o.countryCode?.toLowerCase().includes(filter.toLowerCase()) ?? false),
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
                  <span className="font-medium">{op.shortName || op.name}</span>
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
                      if (!confirm(`Delete ${op.shortName || op.name}?`)) return;
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

      {/* Edit dialog */}
      {editOp && (
        <EditOperatorDialog
          operator={editOp}
          onClose={() => setEditOp(null)}
          onSaved={fetchData}
        />
      )}

      {/* Airports mapping dialog */}
      {airportsOp && (
        <AirportMappingDialog
          operator={airportsOp}
          onClose={() => setAirportsOp(null)}
          onSaved={fetchData}
        />
      )}

      {/* Create dialog */}
      {createOpen && (
        <CreateOperatorDialog
          onClose={() => setCreateOpen(false)}
          onSaved={fetchData}
        />
      )}
    </AdminLayout>
  );
}

function EditOperatorDialog({
  operator,
  onClose,
  onSaved,
}: {
  operator: Operator;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(operator.name);
  const [shortName, setShortName] = useState(operator.shortName || "");
  const [countryCode, setCountryCode] = useState(operator.countryCode || "");
  const [orgType, setOrgType] = useState(operator.orgType);
  const [ownershipModel, setOwnershipModel] = useState(
    operator.ownershipModel || "",
  );
  const [publicSharePct, setPublicSharePct] = useState(
    operator.publicSharePct?.toString() || "",
  );
  const [notes, setNotes] = useState(operator.notes || "");
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminUpdateOperator({
        data: {
          password,
          id: operator.id,
          body: {
            name,
            shortName: shortName || null,
            countryCode: countryCode || null,
            orgType,
            ownershipModel: ownershipModel || null,
            publicSharePct: publicSharePct ? parseFloat(publicSharePct) : null,
            notes: notes || null,
          },
        },
      });
      onSaved();
      onClose();
    } catch (err) {
      console.error("Failed to update operator", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit {operator.shortName || operator.name}</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">Name</label>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Short Name
              </label>
              <Input
                value={shortName}
                onChange={(e) => setShortName(e.target.value)}
              />
            </div>
          </div>
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">Country</label>
              <Input
                value={countryCode}
                onChange={(e) => setCountryCode(e.target.value.toUpperCase())}
                maxLength={2}
              />
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Ownership
              </label>
              <select
                value={ownershipModel}
                onChange={(e) => setOwnershipModel(e.target.value)}
                className="w-full h-8 bg-muted border border-border text-foreground text-sm px-2 rounded-md"
              >
                <option value="">—</option>
                <option value="public">Public</option>
                <option value="private">Private</option>
                <option value="mixed">Mixed</option>
              </select>
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Public %
              </label>
              <Input
                value={publicSharePct}
                onChange={(e) => setPublicSharePct(e.target.value)}
                type="number"
                min="0"
                max="100"
              />
            </div>
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Notes</label>
            <Textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving && <Loader2 className="size-3 animate-spin" />}
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function AirportMappingDialog({
  operator,
  onClose,
  onSaved,
}: {
  operator: Operator;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [codes, setCodes] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const password = localStorage.getItem("admin_password") || "";
    adminGetOperatorAirports({ data: { password, id: operator.id } })
      .then((data: string[]) => {
        setCodes(data.join("\n"));
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [operator.id]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const password = localStorage.getItem("admin_password") || "";
      const iataCodes = codes
        .split(/[\n,\s]+/)
        .map((s) => s.trim().toUpperCase())
        .filter((s) => /^[A-Z]{3}$/.test(s));
      await adminSetOperatorAirports({
        data: { password, id: operator.id, iataCodes },
      });
      onSaved();
      onClose();
    } catch (err) {
      console.error("Failed to set airports", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            Airports for {operator.shortName || operator.name}
          </DialogTitle>
        </DialogHeader>
        {loading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="size-5 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              One IATA code per line or comma-separated. This replaces all
              existing mappings.
            </p>
            <Textarea
              value={codes}
              onChange={(e) => setCodes(e.target.value)}
              rows={10}
              className="font-mono text-sm"
              placeholder={"MAD\nBCN\nAGP"}
            />
            <p className="text-xs text-muted-foreground">
              {
                codes
                  .split(/[\n,\s]+/)
                  .map((s) => s.trim().toUpperCase())
                  .filter((s) => /^[A-Z]{3}$/.test(s)).length
              }{" "}
              airports detected
            </p>
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={saving || loading}>
            {saving && <Loader2 className="size-3 animate-spin" />}
            Save Mappings
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CreateOperatorDialog({
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState("");
  const [shortName, setShortName] = useState("");
  const [countryCode, setCountryCode] = useState("");
  const [orgType, setOrgType] = useState("both");
  const [ownershipModel, setOwnershipModel] = useState("");
  const [publicSharePct, setPublicSharePct] = useState("");
  const [notes, setNotes] = useState("");
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!name.trim()) return;
    setSaving(true);
    try {
      const password = localStorage.getItem("admin_password") || "";
      await adminCreateOperator({
        data: {
          password,
          body: {
            name,
            shortName: shortName || null,
            countryCode: countryCode || null,
            orgType,
            ownershipModel: ownershipModel || null,
            publicSharePct: publicSharePct ? parseFloat(publicSharePct) : null,
            notes: notes || null,
          },
        },
      });
      onSaved();
      onClose();
    } catch (err) {
      console.error("Failed to create operator", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Add Operator</DialogTitle>
        </DialogHeader>
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">Name *</label>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Short Name
              </label>
              <Input
                value={shortName}
                onChange={(e) => setShortName(e.target.value)}
              />
            </div>
          </div>
          <div className="grid grid-cols-3 gap-3">
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">Country</label>
              <Input
                value={countryCode}
                onChange={(e) => setCountryCode(e.target.value.toUpperCase())}
                maxLength={2}
              />
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Ownership
              </label>
              <select
                value={ownershipModel}
                onChange={(e) => setOwnershipModel(e.target.value)}
                className="w-full h-8 bg-muted border border-border text-foreground text-sm px-2 rounded-md"
              >
                <option value="">—</option>
                <option value="public">Public</option>
                <option value="private">Private</option>
                <option value="mixed">Mixed</option>
              </select>
            </div>
            <div className="space-y-1">
              <label className="text-xs text-muted-foreground">
                Public %
              </label>
              <Input
                value={publicSharePct}
                onChange={(e) => setPublicSharePct(e.target.value)}
                type="number"
                min="0"
                max="100"
              />
            </div>
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Notes</label>
            <Textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={saving || !name.trim()}>
            {saving && <Loader2 className="size-3 animate-spin" />}
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
