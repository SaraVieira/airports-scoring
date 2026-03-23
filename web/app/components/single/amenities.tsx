import type { getAirport } from "~/server/get-airport";

type Airport = Awaited<ReturnType<typeof getAirport>>;

export function Amenities({ airport }: { airport: Airport }) {
  const lounges = airport.lounges ?? [];
  if (lounges.length === 0) return null;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-baseline gap-3">
        <h3 className="text-xs font-medium uppercase tracking-widest text-zinc-500">
          Lounges
        </h3>
        <span className="font-mono text-[11px] text-zinc-600">
          {lounges.length} available
        </span>
      </div>
      <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        {lounges.map((lounge) => (
          <div
            key={lounge.id}
            className="flex flex-col gap-1 rounded-md bg-zinc-900/50 px-4 py-3 ring-1 ring-white/5"
          >
            <span className="text-sm font-medium text-zinc-200">
              {lounge.loungeName}
            </span>
            <div className="flex items-center gap-2">
              {lounge.terminal && (
                <span className="font-mono text-[10px] text-zinc-500">
                  Terminal {lounge.terminal}
                </span>
              )}
              {lounge.terminal && lounge.openingHours && (
                <span className="text-zinc-700">·</span>
              )}
              {lounge.openingHours && (
                <span className="font-mono text-[10px] text-zinc-500">
                  {lounge.openingHours}
                </span>
              )}
            </div>
            <span className="font-mono text-[10px] uppercase tracking-wider text-zinc-700">
              {lounge.source.replace(/_/g, " ")}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
