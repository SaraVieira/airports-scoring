import { useAuthStore } from "~/stores/admin";
import { useState, useMemo } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "~/components/ui/dialog";
import { Button } from "~/components/ui/button";
import { Textarea } from "~/components/ui/textarea";
import { Input } from "~/components/ui/input";
import { Checkbox } from "~/components/ui/checkbox";
import { Badge } from "~/components/ui/badge";
import { Card } from "~/components/ui/card";
import { AlertTriangle, FileText, Loader2 } from "lucide-react";
import { adminBatchImport, adminUpdateAirport } from "~/server/admin";

interface ResolvedAirport {
  iataCode: string;
  name: string;
  countryCode: string;
  icaoCode: string;
}

interface BatchResult {
  resolved: ResolvedAirport[];
  failed: string[];
  jobId?: string | null;
}

type ModalState = "paste" | "configure";

interface AirportConfig {
  iataCode: string;
  name: string;
  countryCode: string;
  skytraxReviewSlug: string;
  skytraxRatingSlug: string;
  googleMapsUrl: string;
}

export function BatchImportModal({
  open,
  onOpenChange,
  onComplete,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onComplete: () => void;
}) {
  const [state, setState] = useState<ModalState>("paste");
  const [text, setText] = useState("");
  const [runPipeline, setRunPipeline] = useState(true);
  const [score, setScore] = useState(false);
  const [importing, setImporting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [configs, setConfigs] = useState<AirportConfig[]>([]);
  const [failed, setFailed] = useState<string[]>([]);

  const parsedCodes = useMemo(() => {
    return text
      .split(/[\n,\s]+/)
      .map((s) => s.trim().toUpperCase())
      .filter((s) => /^[A-Z]{3}$/.test(s));
  }, [text]);

  const handleImport = async () => {
    if (parsedCodes.length === 0) return;
    setImporting(true);
    try {
      const password = useAuthStore.getState().password || "";
      const res = await adminBatchImport({
        data: {
          password,
          body: {
            iata_codes: parsedCodes,
            run_pipeline: runPipeline,
            score,
          },
        },
      });

      const batchResult = res as BatchResult;
      setFailed(batchResult.failed || []);
      setConfigs(
        (batchResult.resolved || []).map((a: ResolvedAirport) => ({
          iataCode: a.iataCode,
          name: a.name,
          countryCode: a.countryCode,
          skytraxReviewSlug: "",
          skytraxRatingSlug: "",
          googleMapsUrl: "",
        })),
      );
      setState("configure");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Import failed");
    } finally {
      setImporting(false);
    }
  };

  const updateConfig = (
    iata: string,
    field: keyof AirportConfig,
    value: string,
  ) => {
    setConfigs((prev) =>
      prev.map((c) => (c.iataCode === iata ? { ...c, [field]: value } : c)),
    );
  };

  const handleSave = async () => {
    setSaving(true);
    const password = useAuthStore.getState().password || "";
    try {
      for (const config of configs) {
        const body: Record<string, string | null> = {};
        if (config.skytraxReviewSlug)
          body.skytrax_review_slug = config.skytraxReviewSlug;
        if (config.skytraxRatingSlug)
          body.skytrax_rating_slug = config.skytraxRatingSlug;
        if (config.googleMapsUrl)
          body.google_maps_url = config.googleMapsUrl;

        if (Object.keys(body).length > 0) {
          await adminUpdateAirport({
            data: { password, iata: config.iataCode, body },
          });
        }
      }
      onComplete();
      handleClose();
    } catch (err) {
      console.error("Failed to save configs", err);
    } finally {
      setSaving(false);
    }
  };

  const handleClose = () => {
    setState("paste");
    setText("");
    setConfigs([]);
    setFailed([]);
    setError(null);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col">
        {state === "paste" ? (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <FileText className="size-4" />
                Batch Import Airports
              </DialogTitle>
              <DialogDescription>
                Paste IATA codes — one per line or comma-separated. Airports
                will be looked up from the global database.
              </DialogDescription>
            </DialogHeader>

            <div className="flex-1 min-h-0">
              <Textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder={"CDG\nORY\nLYS\nMRS"}
                className="h-48 font-mono text-sm resize-none"
                autoFocus
              />
            </div>

            <div className="flex items-center gap-4 py-2">
              <label className="flex items-center gap-2 text-sm">
                <Checkbox
                  checked={runPipeline}
                  onCheckedChange={(v) => setRunPipeline(v === true)}
                />
                Run pipeline after import
              </label>
              <label className="flex items-center gap-2 text-sm">
                <Checkbox
                  checked={score}
                  onCheckedChange={(v) => setScore(v === true)}
                />
                Score after pipeline
              </label>
            </div>

            <DialogFooter className="flex items-center justify-between">
              <span className={`text-xs ${error ? "text-destructive" : "text-muted-foreground"}`}>
                {error
                  ? error
                  : parsedCodes.length > 0
                    ? `${parsedCodes.length} airport${parsedCodes.length !== 1 ? "s" : ""} detected`
                    : "No valid codes"}
              </span>
              <div className="flex gap-2">
                <Button variant="outline" onClick={handleClose}>
                  Cancel
                </Button>
                <Button
                  onClick={handleImport}
                  disabled={parsedCodes.length === 0 || importing}
                  className="bg-green-600 hover:bg-green-700 text-white"
                >
                  {importing && <Loader2 className="size-3.5 animate-spin" />}
                  Import {parsedCodes.length} Airport
                  {parsedCodes.length !== 1 ? "s" : ""}
                </Button>
              </div>
            </DialogFooter>
          </>
        ) : (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                Configure Imported Airports
                <Badge
                  variant="secondary"
                  className="border-green-500/30 text-green-500 bg-green-500/10"
                >
                  {configs.length} resolved
                </Badge>
                {failed.length > 0 && (
                  <Badge variant="destructive">{failed.length} failed</Badge>
                )}
              </DialogTitle>
              <DialogDescription>
                Fill in scraper config for each airport. Leave blank to skip
                those sources.
              </DialogDescription>
            </DialogHeader>

            <div className="flex-1 overflow-y-auto space-y-3 min-h-0">
              {configs.map((config) => (
                <Card key={config.iataCode} className="p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div className="size-2 rounded-full bg-green-500" />
                      <span className="font-mono text-sm font-bold">
                        {config.iataCode}
                      </span>
                      <span className="text-sm text-muted-foreground">
                        {config.name}
                      </span>
                    </div>
                    <span className="text-xs text-muted-foreground">
                      {config.countryCode}
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div className="space-y-1">
                      <label className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
                        Skytrax Review Slug
                      </label>
                      <Input
                        value={config.skytraxReviewSlug}
                        onChange={(e) =>
                          updateConfig(
                            config.iataCode,
                            "skytraxReviewSlug",
                            e.target.value,
                          )
                        }
                        placeholder="e.g. paris-cdg-airport"
                        className="h-8 text-xs font-mono"
                      />
                    </div>
                    <div className="space-y-1">
                      <label className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
                        Skytrax Rating Slug
                      </label>
                      <Input
                        value={config.skytraxRatingSlug}
                        onChange={(e) =>
                          updateConfig(
                            config.iataCode,
                            "skytraxRatingSlug",
                            e.target.value,
                          )
                        }
                        placeholder="optional"
                        className="h-8 text-xs font-mono"
                      />
                    </div>
                  </div>
                  <div className="space-y-1">
                    <label className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
                      Google Maps URL
                    </label>
                    <Input
                      value={config.googleMapsUrl}
                      onChange={(e) =>
                        updateConfig(
                          config.iataCode,
                          "googleMapsUrl",
                          e.target.value,
                        )
                      }
                      placeholder="https://maps.app.goo.gl/..."
                      className="h-8 text-xs font-mono"
                    />
                  </div>
                </Card>
              ))}

              {failed.map((code) => (
                <Card
                  key={code}
                  className="p-3 flex items-center gap-2 border-destructive/30 bg-destructive/5"
                >
                  <AlertTriangle className="size-3.5 text-destructive" />
                  <span className="font-mono text-sm font-bold text-destructive">
                    {code}
                  </span>
                  <span className="text-xs text-destructive/70">
                    — Not found in the global airports database
                  </span>
                </Card>
              ))}
            </div>

            <DialogFooter className="flex items-center justify-between">
              <span className="text-xs text-muted-foreground">
                {configs.length} of {configs.length + failed.length} airports
                ready
                {failed.length > 0 ? ` · ${failed.length} not found` : ""}
              </span>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={handleClose}
                  disabled={saving}
                >
                  Skip Config
                </Button>
                <Button
                  onClick={handleSave}
                  disabled={saving}
                  className="bg-green-600 hover:bg-green-700 text-white"
                >
                  {saving && <Loader2 className="size-3.5 animate-spin" />}
                  Save Configuration
                </Button>
              </div>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
