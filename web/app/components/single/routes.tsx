import { useMemo, useState } from "react";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { routeDisplayName, routeIata, routeRegion } from "~/utils/routes";
import { Airport, RouteRow } from "~/utils/types";

const HUB_STATUS_STYLES: Record<string, { color: string; label: string }> = {
  hub: { color: "bg-yellow-500/10 text-yellow-400 ring-yellow-500/20", label: "Hub" },
  focus_city: { color: "bg-blue-500/10 text-blue-400 ring-blue-500/20", label: "Focus City" },
  operating_base: { color: "bg-zinc-500/10 text-zinc-400 ring-zinc-500/20", label: "Base" },
};

export function RouteSection({ airport }: { airport: Airport }) {
  const { routesWithFlights } = useSingleAirport({ airport });
  const [showAll, setShowAll] = useState(false);
  const [search, setSearch] = useState("");
  const query = search.toLowerCase().trim();

  const topRoutes = routesWithFlights.slice(0, 10);
  const displayRoutes = showAll ? routesWithFlights : topRoutes;

  const filtered = query
    ? displayRoutes.filter((r) => {
        const name = routeDisplayName(r).toLowerCase();
        const iata = (routeIata(r) ?? "").toLowerCase();
        const icao = (r.destinationIcao ?? "").toLowerCase();
        return (
          name.includes(query) || iata.includes(query) || icao.includes(query)
        );
      })
    : displayRoutes;

  // Group by region
  const grouped = useMemo(() => {
    const map = new Map<string, RouteRow[]>();
    for (const r of filtered) {
      const region = routeRegion(r);
      const list = map.get(region) ?? [];
      list.push(r);
      map.set(region, list);
    }
    // Sort regions: Europe first, then by count
    return Array.from(map.entries()).sort((a, b) => {
      if (a[0] === "Europe") return -1;
      if (b[0] === "Europe") return 1;
      return b[1].length - a[1].length;
    });
  }, [filtered]);

  // Compute summary stats
  const summary = useMemo(() => {
    const regionSet = new Set<string>();
    for (const r of routesWithFlights) {
      regionSet.add(routeRegion(r));
    }
    const top = routesWithFlights[0];
    const topName = top ? routeDisplayName(top) : null;
    const topFlights = top?.flightsPerMonth ?? null;
    return {
      total: routesWithFlights.length,
      regions: regionSet.size,
      topName,
      topFlights,
    };
  }, [routesWithFlights]);

  return (
    <section className="flex flex-col gap-4">
      <h3 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
        Where You Can Escape To
      </h3>

      {/* Hub status pills */}
      {airport.hubStatus && airport.hubStatus.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {airport.hubStatus.map((hub, idx) => {
            const style = HUB_STATUS_STYLES[hub.statusType ?? ""] ?? {
              color: "bg-zinc-500/10 text-zinc-400 ring-zinc-500/20",
              label: (hub.statusType ?? "").replace(/_/g, " "),
            };
            return (
              <div
                key={idx}
                className={`flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] ring-1 ${style.color}`}
              >
                <span className="font-medium">{hub.airlineName}</span>
                <span className="opacity-40">·</span>
                <span className="opacity-70">{style.label}</span>
              </div>
            );
          })}
        </div>
      )}

      {/* Route summary stats */}
      <span className="font-mono text-[11px] text-zinc-400">
        {summary.total} routes · {summary.regions} region
        {summary.regions !== 1 ? "s" : ""}
        {summary.topName && summary.topFlights
          ? ` · Top: ${summary.topName} (${summary.topFlights}/mo)`
          : ""}
      </span>

      <input
        type="text"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Search destinations..."
        className="w-full bg-zinc-900/50 border border-white/5 px-3 py-2 font-mono text-xs text-zinc-300 placeholder:text-zinc-600 focus:outline-none focus:border-yellow-400/30 transition-colors"
      />

      <div className="max-h-125 overflow-y-auto scrollbar-thin">
        {grouped.map(([region, routes]) => (
          <div key={region} className="mb-4">
            <div className="flex items-center gap-2 mb-2 sticky top-0 bg-[#0a0a0b] py-1 z-10">
              <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-[1.5px] uppercase">
                {region}
              </span>
              <span className="font-mono text-[10px] text-zinc-700">
                {routes.length}
              </span>
            </div>
            {routes.map((r, idx) => {
              const isTop = idx === 0;
              return (
                <div
                  key={r.destinationIata ?? idx}
                  className={`flex justify-between items-center border-b border-white/5 last:border-0 ${
                    isTop ? "py-3 px-4 bg-[#111113] -mx-4" : "py-1.5"
                  }`}
                >
                  <span
                    className={`font-mono truncate ${
                      isTop
                        ? "text-[13px] font-bold text-zinc-200"
                        : "text-[11px] text-zinc-500"
                    }`}
                  >
                    {routeDisplayName(r)}
                    {routeIata(r) ? ` (${routeIata(r)})` : ""}
                  </span>
                  <span
                    className={`font-mono font-bold tabular-nums shrink-0 ${
                      isTop
                        ? "text-base text-green-500"
                        : "text-xs text-zinc-400"
                    }`}
                  >
                    {r.flightsPerMonth ?? "—"}
                  </span>
                </div>
              );
            })}
          </div>
        ))}
        {filtered.length === 0 && (
          <p className="font-mono text-xs text-zinc-600 italic py-4">
            No routes matching "{search}". Trapped.
          </p>
        )}
      </div>

      {routesWithFlights.length > 10 && !query && (
        <button
          onClick={() => setShowAll(!showAll)}
          className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors uppercase self-start"
        >
          {showAll
            ? `Show Top 10`
            : `Show All ${routesWithFlights.length} Routes`}
        </button>
      )}
    </section>
  );
}
