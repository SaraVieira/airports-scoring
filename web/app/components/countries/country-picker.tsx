import { useState } from "react";
import { CountrySummary } from "~/api/client";
import { getFlag } from "~/routes/countries";
import { scoreHex } from "~/utils/scoring";

export function CountryPicker({
  countries,
  exclude,
  onPick,
  onCancel,
}: {
  countries: CountrySummary[];
  exclude: string;
  onPick: (code: string) => void;
  onCancel: () => void;
}) {
  const [search, setSearch] = useState("");
  const filtered = countries.filter(
    (c) =>
      c.code !== exclude &&
      (c.name.toLowerCase().includes(search.toLowerCase()) ||
        c.code.toLowerCase().includes(search.toLowerCase())),
  );

  return (
    <div className="absolute inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center">
      <div className="bg-zinc-900 border border-zinc-700 rounded-xl p-6 w-96 max-h-[500px] flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-zinc-100">
            Pick a country to compare
          </h3>
          <button
            onClick={onCancel}
            className="text-zinc-500 hover:text-zinc-300"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
        <input
          type="text"
          placeholder="Search countries..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="bg-zinc-800 border border-zinc-700 rounded-md px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 outline-none focus:border-zinc-500"
          autoFocus
        />
        <div className="flex flex-col overflow-y-auto gap-0.5">
          {filtered.map((c) => (
            <button
              key={c.code}
              onClick={() => onPick(c.code)}
              className="flex items-center justify-between px-3 py-2.5 rounded-md hover:bg-zinc-800 transition-colors text-left"
            >
              <div className="flex items-center gap-2.5">
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
          {filtered.length === 0 && (
            <p className="text-sm text-zinc-600 text-center py-4">
              No countries found
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
