import { useState, useEffect } from "react";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Textarea } from "~/components/ui/textarea";
import { Loader2 } from "lucide-react";
import {
  adminCreateOperator,
  adminUpdateOperator,
  adminGetOperatorAirports,
  adminSetOperatorAirports,
} from "~/server/admin";

export interface Operator {
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

interface DialogProps {
  onClose: () => void;
  onSaved: () => void;
}

function OperatorFormFields({
  name,
  setName,
  shortName,
  setShortName,
  countryCode,
  setCountryCode,
  ownershipModel,
  setOwnershipModel,
  publicSharePct,
  setPublicSharePct,
  notes,
  setNotes,
  nameRequired,
}: {
  name: string;
  setName: (v: string) => void;
  shortName: string;
  setShortName: (v: string) => void;
  countryCode: string;
  setCountryCode: (v: string) => void;
  ownershipModel: string;
  setOwnershipModel: (v: string) => void;
  publicSharePct: string;
  setPublicSharePct: (v: string) => void;
  notes: string;
  setNotes: (v: string) => void;
  nameRequired?: boolean;
}) {
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">
            Name{nameRequired ? " *" : ""}
          </label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Short Name</label>
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
          <label className="text-xs text-muted-foreground">Ownership</label>
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
          <label className="text-xs text-muted-foreground">Public %</label>
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
  );
}

function buildBody(fields: {
  name: string;
  shortName: string;
  countryCode: string;
  orgType: string;
  ownershipModel: string;
  publicSharePct: string;
  notes: string;
}) {
  return {
    name: fields.name,
    shortName: fields.shortName || null,
    countryCode: fields.countryCode || null,
    orgType: fields.orgType,
    ownershipModel: fields.ownershipModel || null,
    publicSharePct: fields.publicSharePct
      ? parseFloat(fields.publicSharePct)
      : null,
    notes: fields.notes || null,
  };
}

export function EditOperatorDialog({
  operator,
  onClose,
  onSaved,
}: DialogProps & { operator: Operator }) {
  const [name, setName] = useState(operator.name);
  const [shortName, setShortName] = useState(operator.shortName || "");
  const [countryCode, setCountryCode] = useState(operator.countryCode || "");
  const [orgType] = useState(operator.orgType);
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
          body: buildBody({
            name,
            shortName,
            countryCode,
            orgType,
            ownershipModel,
            publicSharePct,
            notes,
          }),
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
        <OperatorFormFields
          name={name}
          setName={setName}
          shortName={shortName}
          setShortName={setShortName}
          countryCode={countryCode}
          setCountryCode={setCountryCode}
          ownershipModel={ownershipModel}
          setOwnershipModel={setOwnershipModel}
          publicSharePct={publicSharePct}
          setPublicSharePct={setPublicSharePct}
          notes={notes}
          setNotes={setNotes}
        />
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

export function CreateOperatorDialog({ onClose, onSaved }: DialogProps) {
  const [name, setName] = useState("");
  const [shortName, setShortName] = useState("");
  const [countryCode, setCountryCode] = useState("");
  const [orgType] = useState("both");
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
          body: buildBody({
            name,
            shortName,
            countryCode,
            orgType,
            ownershipModel,
            publicSharePct,
            notes,
          }),
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
        <OperatorFormFields
          name={name}
          setName={setName}
          shortName={shortName}
          setShortName={setShortName}
          countryCode={countryCode}
          setCountryCode={setCountryCode}
          ownershipModel={ownershipModel}
          setOwnershipModel={setOwnershipModel}
          publicSharePct={publicSharePct}
          setPublicSharePct={setPublicSharePct}
          notes={notes}
          setNotes={setNotes}
          nameRequired
        />
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

export function AirportMappingDialog({
  operator,
  onClose,
  onSaved,
}: DialogProps & { operator: Operator }) {
  const [codes, setCodes] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const password = localStorage.getItem("admin_password") || "";
    adminGetOperatorAirports({ data: { password, id: operator.id } })
      .then((data: string[]) => setCodes(data.join("\n")))
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
