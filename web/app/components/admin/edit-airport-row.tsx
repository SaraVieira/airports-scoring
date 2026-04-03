import { useAuthStore } from "~/stores/admin";
import { useState } from "react";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Checkbox } from "~/components/ui/checkbox";
import { TableRow, TableCell } from "~/components/ui/table";
import { Loader2, Check, X } from "lucide-react";
import { SourceIndicators } from "./source-indicators";
import { adminUpdateAirport } from "~/server/admin";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

export function EditRow({
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
      const password = useAuthStore.getState().password || "";
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
