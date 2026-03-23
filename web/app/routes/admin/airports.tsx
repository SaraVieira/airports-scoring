import { useState, useEffect, useCallback } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { api } from "~/api/client";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];
type SourceStatus = components["schemas"]["SourceStatusResponse"];

export const Route = createFileRoute("/admin/airports")({
  component: AdminAirports,
});

function SourceIndicator({ source }: { source: SourceStatus }) {
  const now = Date.now();
  const fetched = source.lastFetchedAt
    ? new Date(source.lastFetchedAt).getTime()
    : 0;
  const daysSince = fetched ? (now - fetched) / (1000 * 60 * 60 * 24) : Infinity;

  let color = "bg-red-400"; // never fetched
  let title = `${source.source}: never fetched`;
  if (fetched && source.lastStatus === "ok") {
    if (daysSince < 30) {
      color = "bg-green-400";
      title = `${source.source}: ${Math.floor(daysSince)}d ago`;
    } else {
      color = "bg-yellow-400";
      title = `${source.source}: ${Math.floor(daysSince)}d ago (stale)`;
    }
  } else if (fetched && source.lastStatus === "failed") {
    color = "bg-red-400";
    title = `${source.source}: failed ${source.lastError ? `- ${source.lastError}` : ""}`;
  }

  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${color} mr-1`}
      title={title}
    />
  );
}

function AddAirportForm({ onCreated }: { onCreated: () => void }) {
  const [open, setOpen] = useState(false);
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
      await api.admin.createSupportedAirport({
        iata_code: form.iata_code.toUpperCase(),
        name: form.name,
        country_code: form.country_code.toUpperCase(),
        skytrax_review_slug: form.skytrax_review_slug || null,
        skytrax_rating_slug: form.skytrax_rating_slug || null,
        google_maps_url: form.google_maps_url || null,
      });
      setForm({
        iata_code: "",
        name: "",
        country_code: "",
        skytrax_review_slug: "",
        skytrax_rating_slug: "",
        google_maps_url: "",
      });
      setOpen(false);
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create");
    } finally {
      setLoading(false);
    }
  };

  if (!open) {
    return (
      <button
        onClick={() => setOpen(true)}
        className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-1.5 mb-6"
      >
        + Add Airport
      </button>
    );
  }

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-zinc-900 border border-zinc-800 p-4 mb-6"
    >
      <h3 className="font-grotesk text-sm font-bold text-zinc-300 mb-3">
        Add Supported Airport
      </h3>
      <div className="grid grid-cols-3 gap-3 mb-3">
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            IATA Code *
          </label>
          <input
            type="text"
            value={form.iata_code}
            onChange={(e) =>
              setForm({ ...form, iata_code: e.target.value })
            }
            maxLength={3}
            required
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 uppercase focus:outline-none focus:border-zinc-500"
          />
        </div>
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            Name *
          </label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            required
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 focus:outline-none focus:border-zinc-500"
          />
        </div>
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            Country Code *
          </label>
          <input
            type="text"
            value={form.country_code}
            onChange={(e) =>
              setForm({ ...form, country_code: e.target.value })
            }
            maxLength={2}
            required
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 uppercase focus:outline-none focus:border-zinc-500"
          />
        </div>
      </div>
      <div className="grid grid-cols-3 gap-3 mb-3">
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            Skytrax Review Slug
          </label>
          <input
            type="text"
            value={form.skytrax_review_slug}
            onChange={(e) =>
              setForm({ ...form, skytrax_review_slug: e.target.value })
            }
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 focus:outline-none focus:border-zinc-500"
          />
        </div>
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            Skytrax Rating Slug
          </label>
          <input
            type="text"
            value={form.skytrax_rating_slug}
            onChange={(e) =>
              setForm({ ...form, skytrax_rating_slug: e.target.value })
            }
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 focus:outline-none focus:border-zinc-500"
          />
        </div>
        <div>
          <label className="font-mono text-xs text-zinc-500 block mb-1">
            Google Maps URL
          </label>
          <input
            type="text"
            value={form.google_maps_url}
            onChange={(e) =>
              setForm({ ...form, google_maps_url: e.target.value })
            }
            className="w-full bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-2 py-1 focus:outline-none focus:border-zinc-500"
          />
        </div>
      </div>
      {error && (
        <p className="font-mono text-xs text-red-400 mb-3">{error}</p>
      )}
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={loading}
          className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-mono text-xs px-3 py-1.5 disabled:opacity-50"
        >
          {loading ? "Creating..." : "Create"}
        </button>
        <button
          type="button"
          onClick={() => setOpen(false)}
          className="text-zinc-500 hover:text-zinc-300 font-mono text-xs px-3 py-1.5"
        >
          Cancel
        </button>
      </div>
    </form>
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
      await api.admin.updateSupportedAirport(airport.iataCode, {
        enabled: form.enabled,
        name: form.name,
        skytrax_review_slug: form.skytrax_review_slug || null,
        skytrax_rating_slug: form.skytrax_rating_slug || null,
        google_maps_url: form.google_maps_url || null,
      });
      onSaved();
    } catch (err) {
      console.error("Failed to update", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <tr className="border-b border-zinc-800/50 bg-zinc-900/50">
      <td className="font-mono text-xs text-zinc-300 py-2">
        {airport.iataCode}
      </td>
      <td className="py-2">
        <input
          type="text"
          value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })}
          className="bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-1 py-0.5 w-full focus:outline-none focus:border-zinc-500"
        />
      </td>
      <td className="font-mono text-xs text-zinc-400 py-2">
        {airport.countryCode}
      </td>
      <td className="py-2">
        <label className="cursor-pointer">
          <input
            type="checkbox"
            checked={form.enabled}
            onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            className="accent-yellow-400"
          />
        </label>
      </td>
      <td className="py-2">
        {airport.sources.map((s) => (
          <SourceIndicator key={s.source} source={s} />
        ))}
      </td>
      <td className="py-2">
        <div className="flex gap-1">
          <button
            onClick={handleSave}
            disabled={loading}
            className="font-mono text-xs text-green-400 hover:text-green-300 disabled:opacity-50"
          >
            Save
          </button>
          <button
            onClick={onCancel}
            className="font-mono text-xs text-zinc-500 hover:text-zinc-300"
          >
            Cancel
          </button>
        </div>
      </td>
    </tr>
  );
}

function AdminAirports() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  const [airports, setAirports] = useState<SupportedAirport[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingIata, setEditingIata] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);
  const [fetchingIata, setFetchingIata] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  const fetchData = useCallback(async () => {
    try {
      const a = await api.admin.listSupportedAirports();
      setAirports(a);
    } catch {
      localStorage.removeItem("admin_password");
      setAuthenticated(false);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const password = localStorage.getItem("admin_password");
    if (!password) {
      setAuthenticated(false);
      setLoading(false);
      return;
    }
    api.admin
      .listSupportedAirports()
      .then(() => {
        setAuthenticated(true);
        fetchData();
      })
      .catch(() => {
        localStorage.removeItem("admin_password");
        setAuthenticated(false);
        setLoading(false);
      });
  }, [fetchData]);

  const handleDelete = async (iata: string) => {
    try {
      await api.admin.deleteSupportedAirport(iata);
      setDeleteConfirm(null);
      await fetchData();
    } catch (err) {
      console.error("Failed to delete", err);
    }
  };

  const handleFetchNow = async (iata: string) => {
    setFetchingIata(iata);
    try {
      await api.admin.startJob({ airports: [iata] });
    } catch (err) {
      console.error("Failed to start fetch", err);
    } finally {
      setFetchingIata(null);
    }
  };

  const filtered = airports.filter(
    (a) =>
      !filter ||
      a.iataCode.toLowerCase().includes(filter.toLowerCase()) ||
      a.name.toLowerCase().includes(filter.toLowerCase()) ||
      a.countryCode.toLowerCase().includes(filter.toLowerCase()),
  );

  if (authenticated === false) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
        <div className="text-center">
          <p className="font-mono text-sm text-zinc-500 mb-4">
            Not authenticated
          </p>
          <Link
            to="/admin"
            className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
          >
            Go to login
          </Link>
        </div>
      </div>
    );
  }

  if (loading || authenticated === null) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] flex items-center justify-center">
        <p className="font-mono text-sm text-zinc-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <h1 className="font-grotesk text-xl font-bold">
            Supported Airports
          </h1>
          <div className="flex items-center gap-4">
            <Link
              to="/admin"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Dashboard
            </Link>
            <Link
              to="/admin/jobs"
              className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
            >
              Jobs
            </Link>
          </div>
        </div>

        <AddAirportForm onCreated={fetchData} />

        {/* Filter */}
        <div className="mb-4">
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by IATA, name, or country..."
            className="bg-zinc-800 border border-zinc-700 text-zinc-100 font-mono text-xs px-3 py-1.5 w-64 focus:outline-none focus:border-zinc-500"
          />
          <span className="font-mono text-xs text-zinc-500 ml-3">
            {filtered.length} airports
          </span>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-4 mb-3">
          <span className="font-mono text-xs text-zinc-500">Sources:</span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-full bg-green-400" />
            <span className="font-mono text-xs text-zinc-500">recent</span>
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-full bg-yellow-400" />
            <span className="font-mono text-xs text-zinc-500">stale</span>
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-full bg-red-400" />
            <span className="font-mono text-xs text-zinc-500">
              never/failed
            </span>
          </span>
        </div>

        {/* Table */}
        <table className="w-full">
          <thead>
            <tr className="border-b border-zinc-800">
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                IATA
              </th>
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                Name
              </th>
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                Country
              </th>
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                Enabled
              </th>
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                Sources
              </th>
              <th className="font-mono text-xs text-zinc-500 text-left py-2">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
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
                <tr
                  key={airport.iataCode}
                  className="border-b border-zinc-800/50"
                >
                  <td className="font-mono text-xs text-zinc-300 py-2">
                    {airport.iataCode}
                  </td>
                  <td className="font-mono text-xs text-zinc-400 py-2">
                    {airport.name}
                  </td>
                  <td className="font-mono text-xs text-zinc-400 py-2">
                    {airport.countryCode}
                  </td>
                  <td className="py-2">
                    <span
                      className={`font-mono text-xs ${airport.enabled ? "text-green-400" : "text-zinc-500"}`}
                    >
                      {airport.enabled ? "yes" : "no"}
                    </span>
                  </td>
                  <td className="py-2">
                    {airport.sources.map((s) => (
                      <SourceIndicator key={s.source} source={s} />
                    ))}
                  </td>
                  <td className="py-2">
                    <div className="flex gap-2">
                      <button
                        onClick={() => setEditingIata(airport.iataCode)}
                        className="font-mono text-xs text-zinc-400 hover:text-zinc-100"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => handleFetchNow(airport.iataCode)}
                        disabled={fetchingIata === airport.iataCode}
                        className="font-mono text-xs text-yellow-400 hover:text-yellow-300 disabled:opacity-50"
                      >
                        {fetchingIata === airport.iataCode
                          ? "..."
                          : "Fetch"}
                      </button>
                      {deleteConfirm === airport.iataCode ? (
                        <span className="flex gap-1">
                          <button
                            onClick={() => handleDelete(airport.iataCode)}
                            className="font-mono text-xs text-red-400 hover:text-red-300"
                          >
                            Confirm
                          </button>
                          <button
                            onClick={() => setDeleteConfirm(null)}
                            className="font-mono text-xs text-zinc-500 hover:text-zinc-300"
                          >
                            No
                          </button>
                        </span>
                      ) : (
                        <button
                          onClick={() => setDeleteConfirm(airport.iataCode)}
                          className="font-mono text-xs text-red-400/50 hover:text-red-400"
                        >
                          Delete
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ),
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
