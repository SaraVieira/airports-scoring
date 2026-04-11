import { useAuthStore } from "~/stores/admin";
import { useState, useEffect } from "react";
import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Checkbox } from "~/components/ui/checkbox";
import { Loader2 } from "lucide-react";
import { adminUpdateAirport } from "~/server/admin";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];

export function EditAirportDialog({
  open,
  onOpenChange,
  airport,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  airport: SupportedAirport | null;
  onSaved: () => void;
}) {
  const [form, setForm] = useState({
    enabled: true,
    name: "",
    skytrax_review_slug: "",
    skytrax_rating_slug: "",
    google_maps_url: "",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (airport) {
      setForm({
        enabled: airport.enabled,
        name: airport.name,
        skytrax_review_slug: airport.skytraxReviewSlug || "",
        skytrax_rating_slug: airport.skytraxRatingSlug || "",
        google_maps_url: airport.googleMapsUrl || "",
      });
      setError("");
    }
  }, [airport]);

  if (!airport) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
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
      onOpenChange(false);
      onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            Edit {airport.iataCode}{" "}
            <span className="text-muted-foreground font-normal">
              — {airport.countryCode}
            </span>
          </DialogTitle>
          <DialogDescription>
            Update airport name, review links, and enabled status.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="text-sm font-medium mb-1 block">Name</label>
            <Input
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              required
            />
          </div>

          <div>
            <label className="text-sm font-medium mb-1 block">
              Skytrax Review Slug
            </label>
            <Input
              value={form.skytrax_review_slug}
              onChange={(e) =>
                setForm({ ...form, skytrax_review_slug: e.target.value })
              }
              placeholder="london-heathrow-airport"
              className="font-mono text-xs"
            />
            <p className="text-[10px] text-muted-foreground mt-1">
              The slug in airlinequality.com/airport-reviews/[slug]
            </p>
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
              placeholder="london-heathrow-airport"
              className="font-mono text-xs"
            />
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
              placeholder="https://www.google.com/maps/place/..."
              className="font-mono text-xs"
            />
          </div>

          <div className="flex items-center gap-2">
            <Checkbox
              id="edit-enabled"
              checked={form.enabled}
              onCheckedChange={(v) =>
                setForm({ ...form, enabled: v === true })
              }
            />
            <label
              htmlFor="edit-enabled"
              className="text-sm font-medium cursor-pointer"
            >
              Enabled
            </label>
          </div>

          {error && (
            <p className="text-xs text-destructive">{error}</p>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => onOpenChange(false)}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={loading}>
              {loading && <Loader2 className="size-3 animate-spin" />}
              Save
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
