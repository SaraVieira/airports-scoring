import { useAuthStore } from "~/stores/admin";
import { useState } from "react";
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
import { Loader2 } from "lucide-react";
import { adminCreateAirport } from "~/server/admin";

export function AddAirportDialog({
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
      const password = useAuthStore.getState().password || "";
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

          {error && <p className="text-sm text-destructive">{error}</p>}

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
