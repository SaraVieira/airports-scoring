import { CountrySummary } from "~/api/client";
import { getFlag } from "~/routes/countries";
import { scoreHex } from "~/utils/scoring";

export function EmptyPanel({
  countries,
  onSelect,
}: {
  countries: CountrySummary[];
  onSelect: (code: string) => void;
}) {
  const sorted = [...countries].sort(
    (a, b) => (b.avgScore ?? 0) - (a.avgScore ?? 0),
  );

  return (
    <div className="flex flex-col gap-6 h-full">
      <div>
        <span className="text-[10px] font-semibold tracking-wider text-zinc-500">
          COUNTRY RANKINGS
        </span>
        <p className="text-sm text-zinc-500 mt-1">Click a country to explore</p>
      </div>
      <div className="flex flex-col">
        {sorted.map((c, i) => (
          <button
            key={c.code}
            onClick={() => onSelect(c.code)}
            className="flex items-center justify-between py-2 border-b border-zinc-800/50 hover:bg-zinc-800/20 transition-colors -mx-1 px-1 rounded text-left"
          >
            <div className="flex items-center gap-2.5">
              <span className="text-[11px] font-medium text-zinc-600 w-4 text-right">
                {i + 1}
              </span>
              <img
                src={getFlag(c.code)}
                alt={`${c.name} flag`}
                className="w-5 h-4 object-cover rounded-sm"
              />
              <span className="text-sm font-medium text-zinc-100">
                {c.name}
              </span>
              <span className="text-xs text-zinc-600">
                {c.airportCount} airports
              </span>
            </div>
            <span
              className="text-sm font-semibold"
              style={{ color: scoreHex(c.avgScore) }}
            >
              {c.avgScore != null ? Math.round(c.avgScore) : "—"}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
