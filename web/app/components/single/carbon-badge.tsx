import type { getAirport } from "~/server/get-airport";

type Airport = Awaited<ReturnType<typeof getAirport>>;

const LEVEL_DESCRIPTIONS: Record<number, string> = {
  1: "Measured their carbon footprint",
  2: "Set emission reduction targets",
  3: "Engaging stakeholders on Scope 3 emissions",
  4: "Absolute reductions aligned with Paris Agreement",
  5: "Offsetting residual Scope 1 & 2 emissions",
  6: "Paris-aligned reductions + offsetting residuals",
  7: "90%+ absolute CO2 reductions achieved",
};

const LEVEL_COLORS: Record<number, string> = {
  1: "bg-zinc-500/10 text-zinc-400 ring-zinc-500/20",
  2: "bg-blue-500/10 text-blue-400 ring-blue-500/20",
  3: "bg-emerald-500/10 text-emerald-400 ring-emerald-500/20",
  4: "bg-teal-500/10 text-teal-400 ring-teal-500/20",
  5: "bg-green-500/10 text-green-400 ring-green-500/20",
  6: "bg-lime-500/10 text-lime-400 ring-lime-500/20",
  7: "bg-yellow-500/10 text-yellow-400 ring-yellow-500/20",
};

export function CarbonBadge({ airport }: { airport: Airport }) {
  const accreditation = airport.carbonAccreditation?.[0];
  if (!accreditation) return null;

  const colorClass =
    LEVEL_COLORS[accreditation.level] ??
    "bg-zinc-500/10 text-zinc-400 ring-zinc-500/20";

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-xs font-medium uppercase tracking-widest text-zinc-500">
        Carbon Accreditation
      </h3>
      <div className="flex items-center gap-3">
        <div
          className={`flex items-center gap-2 rounded-md px-3 py-1.5 ring-1 ${colorClass}`}
        >
          <span className="font-mono text-xs font-bold">
            Level {accreditation.level}/7
          </span>
          <span className="text-sm font-medium">{accreditation.levelName}</span>
        </div>
        <span className="font-mono text-[11px] text-zinc-600">
          {accreditation.reportYear}
        </span>
      </div>
      <p className="text-xs text-zinc-500">
        {LEVEL_DESCRIPTIONS[accreditation.level]}
      </p>
    </div>
  );
}
