import { Link } from "@tanstack/react-router";
import { useMemo } from "react";
import { CountrySummary } from "~/api/client";
import { getFlag } from "~/routes/countries";
import { fmtM } from "~/utils/format";
import { scoreHex } from "~/utils/scoring";

export function CountryDetailPanel({
  country,
  airports,
  onCompare,
}: {
  country: CountrySummary;
  airports: {
    iataCode: string;
    name: string;
    city: string;
    scoreTotal?: number | null;
  }[];
  onCompare: () => void;
}) {
  const sorted = useMemo(
    () =>
      [...airports].sort((a, b) => (b.scoreTotal ?? 0) - (a.scoreTotal ?? 0)),
    [airports],
  );

  return (
    <div className="flex flex-col gap-5 h-full overflow-y-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <img
            src={getFlag(country.code)}
            alt={`${country.name} flag`}
            className="w-5 h-4 object-cover rounded-sm"
          />

          <div>
            <h2 className="text-xl font-semibold text-zinc-100">
              {country.name}
            </h2>
            <p className="text-xs text-zinc-500">
              {country.airportCount} airport
              {country.airportCount !== 1 ? "s" : ""} tracked
            </p>
          </div>
        </div>
        <button
          onClick={onCompare}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-800 text-zinc-400 text-xs font-medium hover:bg-zinc-700 hover:text-zinc-200 transition-colors"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="18" cy="18" r="3" />
            <circle cx="6" cy="6" r="3" />
            <path d="M13 6h3a2 2 0 0 1 2 2v7" />
            <path d="M11 18H8a2 2 0 0 1-2-2V9" />
          </svg>
          Compare
        </button>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-3 gap-3">
        <StatCard
          label="AVG SCORE"
          value={
            country.avgScore != null
              ? Math.round(country.avgScore).toString()
              : "—"
          }
          color={scoreHex(country.avgScore)}
          sub={`${country.airportCount} airport${country.airportCount !== 1 ? "s" : ""}`}
        />
        <StatCard
          label="TOTAL PAX"
          value={fmtM(country.totalPax)}
          color="#fafafa"
        />
        <StatCard
          label="SENTIMENT"
          value={
            country.avgSentimentPositive != null
              ? `${Math.round(country.avgSentimentPositive)}%`
              : "—"
          }
          color={scoreHex(country.avgSentimentPositive)}
          sub="positive reviews"
        />
      </div>

      {/* Airport list */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <span className="text-[10px] font-semibold tracking-wider text-zinc-500">
            AIRPORTS
          </span>
          <span className="text-[10px] font-medium text-zinc-600">Score</span>
        </div>
        <div className="h-px bg-zinc-800/50" />
        <div className="flex flex-col">
          {sorted.map((airport, i) => (
            <Link
              key={airport.iataCode}
              to="/airport/$iata"
              params={{ iata: airport.iataCode }}
              className="flex items-center justify-between py-2.5 border-b border-zinc-800/50 hover:bg-zinc-800/20 transition-colors -mx-1 px-1 rounded"
            >
              <div className="flex items-center gap-2.5">
                <span className="text-[11px] font-medium text-zinc-600 w-4 text-right">
                  {i + 1}
                </span>
                <span className="text-[13px] font-semibold text-zinc-100 tracking-wide">
                  {airport.iataCode}
                </span>
                <span className="text-xs text-zinc-500">{airport.name}</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-24 h-1.5 rounded-full bg-zinc-800 overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all"
                    style={{
                      width: `${airport.scoreTotal ?? 0}%`,
                      backgroundColor: scoreHex(airport.scoreTotal),
                    }}
                  />
                </div>
                <span
                  className="text-[13px] font-semibold w-7 text-right"
                  style={{ color: scoreHex(airport.scoreTotal) }}
                >
                  {airport.scoreTotal != null
                    ? Math.round(airport.scoreTotal)
                    : "—"}
                </span>
              </div>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  color,
  sub,
}: {
  label: string;
  value: string;
  color: string;
  sub?: string;
}) {
  return (
    <div className="bg-zinc-900 rounded-lg p-4 flex flex-col gap-1">
      <span className="text-[9px] font-semibold tracking-wider text-zinc-500">
        {label}
      </span>
      <span className="text-3xl font-bold" style={{ color }}>
        {value}
      </span>
      {sub && <span className="text-[11px] text-zinc-600">{sub}</span>}
    </div>
  );
}
