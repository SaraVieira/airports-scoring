import type { getAirport } from "~/server/get-airport";

type Airport = Awaited<ReturnType<typeof getAirport>>;

const modes = [
  { key: "hasMetro" as const, label: "Metro", icon: "M" },
  { key: "hasRail" as const, label: "Rail", icon: "R" },
  { key: "hasTram" as const, label: "Tram", icon: "T" },
  { key: "hasBus" as const, label: "Bus", icon: "B" },
];

export function GroundTransport({ airport }: { airport: Airport }) {
  const transport = airport.groundTransport?.[0];
  if (!transport) return null;

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-xs font-medium uppercase tracking-widest text-zinc-500">
        Ground Transport
      </h3>
      <div className="flex gap-2">
        {modes.map((mode) => {
          const active = transport[mode.key];
          return (
            <div
              key={mode.key}
              className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium ${
                active
                  ? "bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20"
                  : "bg-zinc-800/50 text-zinc-600"
              }`}
            >
              <span className="text-xs font-bold">{mode.icon}</span>
              {mode.label}
            </div>
          );
        })}
      </div>
      {transport.hasDirectRail && (
        <p className="text-xs text-zinc-500">Direct rail connection to terminal</p>
      )}
    </div>
  );
}
