import { useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { eq, desc } from "drizzle-orm";
import { AirportSearch } from "../components/airport-search";

// ── Server Functions ─────────────────────────────────────

const getHomepageData = createServerFn({ method: "GET" }).handler(async () => {
  const ranked = await db
    .select({
      iataCode: airports.iataCode,
      name: airports.name,
      city: airports.city,
      countryCode: airports.countryCode,
      scoreTotal: airportScores.scoreTotal,
      scoreSentimentVelocity: airportScores.scoreSentimentVelocity,
    })
    .from(airportScores)
    .innerJoin(airports, eq(airportScores.airportId, airports.id))
    .where(eq(airportScores.isLatest, true))
    .orderBy(desc(airportScores.scoreTotal));

  return ranked;
});

// ── Route ────────────────────────────────────────────────

export const Route = createFileRoute("/")({
  loader: () => getHomepageData(),
  component: Home,
});

// ── Helpers ──────────────────────────────────────────────

function scoreColor(score: number | null | undefined): string {
  if (score == null) return "text-zinc-600";
  if (score >= 70) return "text-green-500";
  if (score >= 40) return "text-yellow-500";
  return "text-red-500";
}

function verdict(score: number | null | undefined): string {
  if (score == null) return "Unscored";
  if (score >= 81) return "Fine. We'll allow it.";
  if (score >= 61) return "Surprisingly not awful";
  if (score >= 41) return "Could be worse";
  if (score >= 21) return "A masterclass in mediocrity";
  return "Impressively terrible";
}

// Deduplicate city names like "Luton, Luton" → "Luton"
function cleanCity(city: string): string {
  const parts = city.split(", ");
  if (parts.length === 2 && parts[0] === parts[1]) return parts[0];
  return city;
}

// ── Main Component ───────────────────────────────────────

function Home() {
  const ranked = Route.useLoaderData();

  const mostImproved = useMemo(
    () =>
      [...ranked]
        .filter((a) => a.scoreSentimentVelocity != null)
        .sort(
          (a, b) =>
            parseFloat(b.scoreSentimentVelocity!) -
            parseFloat(a.scoreSentimentVelocity!)
        )
        .slice(0, 3),
    [ranked]
  );

  const wallOfShame = useMemo(
    () =>
      [...ranked]
        .filter((a) => a.scoreSentimentVelocity != null)
        .sort(
          (a, b) =>
            parseFloat(a.scoreSentimentVelocity!) -
            parseFloat(b.scoreSentimentVelocity!)
        )
        .slice(0, 3),
    [ranked]
  );

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 flex flex-col">
        {/* ── Hero ──────────────────────────────── */}
        {/* P4: generous hero padding */}
        <section className="flex flex-col items-center gap-5 pt-32 pb-20">
          <h1 className="font-grotesk text-[48px] font-bold text-[#f5f5f0] tracking-[2px]">
            Airport Intelligence
          </h1>
          <p className="font-mono text-sm text-zinc-600 italic">
            Scoring Europe's airports so you don't have to.
          </p>
          {/* P2: search is the primary action */}
          <div className="mt-2">
            <AirportSearch />
          </div>
          <p className="font-mono text-[11px] text-zinc-500 tracking-wide">
            {ranked.length} airports scored · 20+ years of history
          </p>
        </section>

        <div className="w-full h-px bg-zinc-800" />

        {/* ── Rankings ──────────────────────────── */}
        {/* P4: tight/dense rankings section */}
        <section className="flex flex-col gap-4 py-12">
          <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
            The Rankings
          </h2>
          <p className="font-mono text-xs text-zinc-600 italic">
            All-time composite scores. Higher is better. We think.
          </p>

          <div className="flex flex-col mt-2">
            {/* Header */}
            <div className="flex items-center gap-2 py-2">
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-6 shrink-0">
                #
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-12 shrink-0">
                IATA
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider flex-1">
                AIRPORT
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-20 shrink-0 text-right pr-1">
                SCORE
              </span>
            </div>
            <div className="h-px bg-zinc-800" />

            {/* Rows — P3: visual tiers */}
            {ranked.map((airport, i) => {
              const score = airport.scoreTotal
                ? parseFloat(airport.scoreTotal)
                : null;
              const isTop3 = i < 3;
              const isBottom3 = i >= ranked.length - 3;
              const barWidth = score != null ? Math.min(score, 100) : 0;

              return (
                <a
                  key={airport.iataCode}
                  href={`/airport/${airport.iataCode}`}
                  className={`group flex items-center gap-2 border-b transition-colors ${
                    isTop3
                      ? "py-3.5 border-white/[0.06] hover:bg-white/[0.04]"
                      : isBottom3
                        ? "py-2 border-white/[0.03] hover:bg-white/[0.03] opacity-70 hover:opacity-100"
                        : "py-2.5 border-white/[0.03] hover:bg-white/[0.03]"
                  }`}
                >
                  {/* P3: top 3 accent border */}
                  {isTop3 && (
                    <span className={`w-0.5 self-stretch shrink-0 ${i === 0 ? "bg-yellow-400" : "bg-yellow-400/40"}`} />
                  )}
                  <span className={`font-mono font-bold tabular-nums w-6 shrink-0 ${
                    isTop3 ? "text-sm text-zinc-400" : "text-xs text-zinc-600"
                  }`}>
                    {i + 1}
                  </span>
                  <span className={`font-grotesk font-bold tracking-wider w-12 shrink-0 ${
                    isTop3 ? "text-[15px] text-[#f5f5f0]" : "text-[13px] text-zinc-300"
                  }`}>
                    {airport.iataCode}
                  </span>
                  {/* P5: airport name neutral, let score carry color */}
                  <span className="font-mono text-xs text-zinc-600 flex-1 truncate group-hover:text-zinc-400 transition-colors">
                    {airport.name}
                  </span>
                  {/* P3: inline mini score bar */}
                  <span className="w-20 shrink-0 flex items-center gap-2 justify-end">
                    <span className="w-12 h-1 bg-zinc-900 relative hidden sm:block">
                      <span
                        className={`h-1 absolute left-0 top-0 ${
                          score != null && score >= 70 ? "bg-green-500" :
                          score != null && score >= 40 ? "bg-yellow-500/70" :
                          "bg-red-500/70"
                        }`}
                        style={{ width: `${barWidth}%` }}
                      />
                    </span>
                    <span
                      className={`font-grotesk font-bold tabular-nums text-right ${
                        isTop3 ? "text-lg" : "text-sm"
                      } ${scoreColor(score)}`}
                    >
                      {score != null ? Math.round(score) : "—"}
                    </span>
                  </span>
                </a>
              );
            })}
          </div>
        </section>

        <div className="w-full h-px bg-zinc-800" />

        {/* ── Walls of Fame / Shame ────────────── */}
        {/* P4: generous walls section */}
        <section className="flex gap-16 py-16">
          <WallColumn
            title="MOST IMPROVED"
            subtitle="Sentiment velocity — who's actually getting better"
            color="green"
            airports={mostImproved}
            field="scoreSentimentVelocity"
          />
          <WallColumn
            title="WALL OF SHAME"
            subtitle="Getting worse — and passengers know it"
            color="red"
            airports={wallOfShame}
            field="scoreSentimentVelocity"
          />
        </section>
      </div>
    </div>
  );
}

// ── Wall Column ──────────────────────────────────────────

function WallColumn({
  title,
  subtitle,
  color,
  airports: airportList,
  field,
}: {
  title: string;
  subtitle: string;
  color: "green" | "red";
  airports: {
    iataCode: string | null;
    name: string;
    city: string;
    [key: string]: unknown;
  }[];
  field: string;
}) {
  const textColor = color === "green" ? "text-green-500" : "text-red-500";
  const bgTint = color === "green" ? "bg-green-500/[0.04]" : "bg-red-500/[0.04]";
  const borderTint = color === "green" ? "border-green-500/10" : "border-red-500/10";

  return (
    <div className={`flex-1 flex flex-col gap-4 p-6 ${bgTint} border ${borderTint}`}>
      <h3
        className={`font-grotesk text-[13px] font-bold ${textColor} tracking-[2px] uppercase`}
      >
        {title}
      </h3>
      <p className="font-mono text-[11px] text-zinc-600 italic">{subtitle}</p>
      <div className="flex flex-col">
        {airportList.map((airport) => {
          const val = airport[field];
          const num = val != null ? parseFloat(String(val)) : null;
          return (
            <a
              key={airport.iataCode}
              href={`/airport/${airport.iataCode}`}
              className="flex items-center justify-between py-3 border-b border-white/[0.05] last:border-0 hover:bg-white/[0.03] transition-colors -mx-2 px-2"
            >
              <div className="flex items-center gap-3">
                <span className="font-grotesk text-sm font-bold text-[#f5f5f0] tracking-wider">
                  {airport.iataCode}
                </span>
                <span className="font-mono text-xs text-zinc-600">
                  {cleanCity(airport.city)}
                </span>
              </div>
              <div className="flex items-center gap-1.5">
                <span className={`font-mono text-sm ${textColor}`}>
                  {num != null
                    ? color === "red"
                      ? num < 15 ? "↓↓" : "↓"
                      : num > 85 ? "↑↑" : "↑"
                    : ""}
                </span>
                <span className={`font-grotesk text-xl font-bold tabular-nums ${textColor}`}>
                  {num != null ? Math.round(num) : "—"}
                </span>
              </div>
            </a>
          );
        })}
      </div>
    </div>
  );
}
