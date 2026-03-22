import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { eq, sql } from "drizzle-orm";

const getAirport = createServerFn({ method: "GET" })
  .inputValidator((iata: string) => iata.toUpperCase())
  .handler(async ({ data: iata }) => {
    const airport = await db.query.airports.findFirst({
      where: eq(airports.iataCode, iata),
      with: {
        operator: true,
        owner: true,
        country: true,
        runways: true,
        paxYearly: { orderBy: (p, { desc }) => [desc(p.year)] },
        operationalStats: {
          orderBy: (o, { desc }) => [desc(o.periodYear), desc(o.periodMonth)],
        },
        sentimentSnapshots: {
          orderBy: (s, { desc }) => [
            desc(s.snapshotYear),
            desc(s.snapshotQuarter),
          ],
        },
        scores: {
          where: eq(airportScores.isLatest, true),
          limit: 1,
        },
        routesOut: {
          with: { destination: true, destinationAirport: true },
          orderBy: () => [sql`flights_per_month DESC NULLS LAST`],
        },
        wikipediaSnapshots: {
          orderBy: (w, { desc }) => [desc(w.fetchedAt)],
          limit: 1,
        },
        slugs: true,
      },
    });

    if (!airport) {
      throw new Error(`Airport ${iata} not found`);
    }

    return airport;
  });

export const Route = createFileRoute("/airport/$iata")({
  loader: ({ params }) => getAirport({ data: params.iata! }),
  component: AirportDetail,
});

// ── Helpers ──────────────────────────────────────────────

function fmt(n: number | string | null | undefined): string {
  if (n == null) return "—";
  const num = typeof n === "string" ? parseFloat(n) : n;
  if (isNaN(num)) return "—";
  return num.toLocaleString("en-US");
}

function fmtM(n: number | string | null | undefined): string {
  if (n == null) return "—";
  const num = typeof n === "string" ? parseFloat(n) : n;
  if (isNaN(num)) return "—";
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`;
  if (num >= 1_000) return `${(num / 1_000).toFixed(0)}K`;
  return fmt(num);
}

function scoreColor(score: number | null | undefined): string {
  if (score == null) return "text-zinc-600";
  if (score >= 70) return "text-green-500";
  if (score >= 40) return "text-yellow-500";
  return "text-red-500";
}

function scoreBg(score: number | null | undefined): string {
  if (score == null) return "bg-zinc-600";
  if (score >= 70) return "bg-green-500";
  if (score >= 40) return "bg-yellow-500";
  return "bg-red-500";
}

function scoreVerdict(score: number | null | undefined): string {
  if (score == null) return "No data";
  if (score >= 90) return "Suspiciously good";
  if (score >= 70) return "Actually decent";
  if (score >= 50) return "Passable";
  if (score >= 30) return "Painful";
  return "Dire";
}

function totalVerdict(score: number | null | undefined): string {
  if (score == null) return "Unscored";
  if (score >= 81) return "Fine. We'll allow it.";
  if (score >= 61) return "Surprisingly not awful";
  if (score >= 41) return "Could be worse (but not by much)";
  if (score >= 21) return "A masterclass in mediocrity";
  return "Impressively terrible";
}

function totalCommentary(
  score: {
    scoreInfrastructure?: string | null;
    scoreOperational?: string | null;
    scoreSentiment?: string | null;
    scoreConnectivity?: string | null;
    scoreSentimentVelocity?: string | null;
    commentary?: string | null;
  } | undefined
): string {
  if (!score) return "";
  if (score.commentary) return score.commentary;

  const infra = parseFloat(score.scoreInfrastructure ?? "0");
  const ops = parseFloat(score.scoreOperational ?? "0");
  const sent = parseFloat(score.scoreSentiment ?? "0");
  const conn = parseFloat(score.scoreConnectivity ?? "0");

  const parts: string[] = [];
  if (conn >= 70 && ops < 50)
    parts.push(
      "Strong connectivity can't save poor operations."
    );
  if (infra < 40) parts.push("Infrastructure is the weak link.");
  if (sent < 40) parts.push("Passengers have noticed — and they're not happy about it.");

  const vel = parseFloat(score.scoreSentimentVelocity ?? "50");
  if (vel > 60) parts.push("At least the trend is improving.");
  else if (vel < 40) parts.push("And it's getting worse.");
  else parts.push("The trajectory is flat — no improvement in sight.");

  return parts.join(" ") || "The data speaks for itself.";
}

function paxSnark(
  latest: number | null,
  capacity: number | null
): string {
  if (!latest || !capacity) return "";
  const pct = Math.round((latest / capacity) * 100);
  if (pct > 100)
    return `Running at ${pct}% capacity. The airport is literally bursting.`;
  if (pct > 85)
    return `Running at ${pct}% capacity. Efficiently full without feeling cramped. Show-offs.`;
  if (pct > 60)
    return `Running at ${pct}% capacity. The remaining ${100 - pct}% is probably the baggage claim area everyone avoids.`;
  return `Running at ${pct}% capacity. Plenty of room — and plenty of reasons people aren't coming.`;
}

type OpsRow = typeof import("../db/schema")["operationalStats"]["$inferSelect"];

function aggregateOps(rows: OpsRow[]) {
  let totalFlights = 0;
  let delayedFlights = 0;
  let totalDelayMin = 0;
  let delayMinCount = 0;
  let weatherMin = 0;
  let carrierMin = 0;
  let atcMin = 0;
  let airportMin = 0;
  let totalAtfmMin = 0;
  let cancelledFlights = 0;
  let mishandledSum = 0;
  let mishandledCount = 0;

  for (const r of rows) {
    const flights = r.totalFlights ?? 0;
    totalFlights += flights;

    if (r.delayPct != null && flights > 0) {
      delayedFlights += Math.round((parseFloat(r.delayPct) / 100) * flights);
    }

    if (r.avgDelayMinutes != null) {
      totalDelayMin += parseFloat(r.avgDelayMinutes) * flights;
      delayMinCount += flights;
    }

    if (r.cancellationPct != null && flights > 0) {
      cancelledFlights += Math.round((parseFloat(r.cancellationPct) / 100) * flights);
    }

    // Sum ATFM cause minutes (pct * total_delay for that month)
    const monthAtfm = r.delayPct != null ? (parseFloat(r.delayPct) / 100) * flights : 0;
    if (r.delayWeatherPct != null) weatherMin += (parseFloat(r.delayWeatherPct) / 100) * monthAtfm;
    if (r.delayCarrierPct != null) carrierMin += (parseFloat(r.delayCarrierPct) / 100) * monthAtfm;
    if (r.delayAtcPct != null) atcMin += (parseFloat(r.delayAtcPct) / 100) * monthAtfm;
    if (r.delayAirportPct != null) airportMin += (parseFloat(r.delayAirportPct) / 100) * monthAtfm;
    if (monthAtfm > 0) totalAtfmMin += monthAtfm;

    if (r.mishandledBagsPer1k != null) {
      mishandledSum += parseFloat(r.mishandledBagsPer1k);
      mishandledCount++;
    }
  }

  const delayPct = totalFlights > 0 ? (delayedFlights / totalFlights) * 100 : null;
  const avgDelay = delayMinCount > 0 ? totalDelayMin / delayMinCount : null;
  const cancellationPct = totalFlights > 0 ? (cancelledFlights / totalFlights) * 100 : null;

  return {
    totalFlights,
    delayPct,
    avgDelayMinutes: avgDelay,
    cancellationPct: cancellationPct && cancellationPct > 0 ? cancellationPct : null,
    delayWeatherPct: totalAtfmMin > 0 ? (weatherMin / totalAtfmMin) * 100 : null,
    delayCarrierPct: totalAtfmMin > 0 ? (carrierMin / totalAtfmMin) * 100 : null,
    delayAtcPct: totalAtfmMin > 0 ? (atcMin / totalAtfmMin) * 100 : null,
    delayAirportPct: totalAtfmMin > 0 ? (airportMin / totalAtfmMin) * 100 : null,
    mishandledBagsPer1k: mishandledCount > 0 ? mishandledSum / mishandledCount : null,
    periodLabel: rows.length > 1
      ? `${rows[rows.length - 1].periodYear}/${String(rows[rows.length - 1].periodMonth).padStart(2, "0")}–${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}`
      : rows[0]?.periodYear ? `${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}` : "",
  };
}

function delaySnark(delayPct: number | null): string {
  if (delayPct == null) return "";
  const pct = parseFloat(String(delayPct));
  if (pct > 40) return "Nearly half of flights delayed. At this point, 'on time' is the exception.";
  if (pct > 25) return "Nearly a third of flights delayed. Pack a book. Maybe two.";
  if (pct > 15) return "One in five flights delayed. Not great, not apocalyptic.";
  if (pct > 8) return `${pct.toFixed(0)}% of flights delayed. Under ten percent. We checked twice.`;
  return "Delays are genuinely rare here. We're suspicious.";
}

// ── Score Bar ────────────────────────────────────────────

function ScoreBar({
  label,
  score,
}: {
  label: string;
  score: string | null | undefined;
}) {
  const num = score ? parseFloat(score) : null;
  const width = num != null ? `${Math.min(num, 100)}%` : "0%";
  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider w-40 shrink-0 uppercase">
        {label}
      </span>
      <div className="flex-1 h-2 bg-zinc-900 relative">
        <div
          className={`h-2 ${scoreBg(num)} absolute left-0 top-0`}
          style={{ width }}
        />
      </div>
      <span
        className={`font-mono text-xs font-bold w-7 shrink-0 tabular-nums ${scoreColor(num)}`}
      >
        {num != null ? Math.round(num) : "—"}
      </span>
      <span
        className={`font-mono text-[11px] italic w-[120px] shrink-0 ${scoreColor(num)}`}
      >
        {scoreVerdict(num)}
      </span>
    </div>
  );
}

// ── Sentiment Bar ────────────────────────────────────────

function SentimentBar({
  label,
  score,
}: {
  label: string;
  score: string | number | null | undefined;
}) {
  const num = score != null ? parseFloat(String(score)) : null;
  const width = num != null ? `${(num / 10) * 100}%` : "0%";
  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider w-24 shrink-0 uppercase">
        {label}
      </span>
      <div className="flex-1 h-1.5 bg-zinc-900 relative">
        <div
          className={`h-1.5 absolute left-0 top-0 ${num != null && num >= 6 ? "bg-green-500" : num != null && num >= 4 ? "bg-yellow-500" : "bg-red-500"}`}
          style={{ width }}
        />
      </div>
      <span
        className={`font-mono text-[11px] font-bold w-7 shrink-0 tabular-nums ${num != null && num >= 6 ? "text-green-500" : num != null && num >= 4 ? "text-yellow-500" : "text-red-500"}`}
      >
        {num != null ? num.toFixed(1) : "—"}
      </span>
    </div>
  );
}

// ── Divider ──────────────────────────────────────────────

function Divider() {
  return <div className="w-full h-px bg-zinc-800" />;
}

// ── Exhibit Header ───────────────────────────────────────

function ExhibitHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
      {children}
    </h3>
  );
}

// ── Stat ─────────────────────────────────────────────────

function Stat({
  value,
  label,
  color = "text-zinc-100",
  size = "text-[42px]",
}: {
  value: string;
  label: string;
  color?: string;
  size?: string;
}) {
  return (
    <div className="flex-1 flex flex-col gap-1">
      <span className={`font-grotesk ${size} font-bold ${color} tabular-nums`}>
        {value}
      </span>
      <span className="font-mono text-[11px] text-zinc-500 tracking-wider uppercase">
        {label}
      </span>
    </div>
  );
}

// ── Main Component ───────────────────────────────────────

function AirportDetail() {
  const airport = Route.useLoaderData();
  const score = airport.scores[0];
  const totalNum = score?.scoreTotal ? parseFloat(score.scoreTotal) : null;
  const latestPax = airport.paxYearly[0];
  const prevPax = airport.paxYearly[1];
  // Aggregate last 12 months of operational stats for meaningful averages
  const recentOps = airport.operationalStats.slice(0, 12);
  const opsAgg = recentOps.length > 0 ? aggregateOps(recentOps) : null;
  const latestSentiment = airport.sentimentSnapshots[0];
  const wiki = airport.wikipediaSnapshots[0];
  const routesWithFlights = airport.routesOut.filter((r) => r.flightsPerMonth != null && r.flightsPerMonth > 0);
  const topRoute = routesWithFlights[0];
  const restRoutes = routesWithFlights.slice(1);

  const yoyGrowth =
    latestPax?.totalPax && prevPax?.totalPax
      ? ((latestPax.totalPax - prevPax.totalPax) / prevPax.totalPax) * 100
      : null;

  const capacityNum = airport.annualCapacityM
    ? parseFloat(airport.annualCapacityM) * 1_000_000
    : null;
  const latestPaxNum = latestPax?.totalPax ?? null;

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 py-12 flex flex-col gap-12">
        {/* ── Header ──────────────────────────────── */}
        <header className="flex flex-col gap-0">
          <span className="font-grotesk text-[120px] font-bold text-white/7 leading-none tracking-[8px]">
            {airport.iataCode}
          </span>
          <h1 className="font-grotesk text-[28px] font-bold text-zinc-100 tracking-wide">
            {airport.name}
          </h1>
          <p className="font-mono text-[13px] text-zinc-500 tracking-[1.5px] uppercase">
            {airport.city}, {airport.country?.name}
          </p>
          {airport.operator && (
            <p className="font-mono text-[11px] text-zinc-600 tracking-wider uppercase">
              Operated by {airport.operator.name}
            </p>
          )}
          <div className="flex gap-3 mt-3 flex-wrap">
            {airport.openedYear && (
              <Badge label="Opened" value={String(airport.openedYear)} bright />
            )}
            {airport.icaoCode && (
              <Badge label="ICAO" value={airport.icaoCode} />
            )}
            {airport.terminalCount && (
              <Badge label="Terminals" value={String(airport.terminalCount)} />
            )}
            {airport.totalGates && (
              <Badge label="Gates" value={String(airport.totalGates)} />
            )}
            {airport.elevationFt && (
              <Badge label="Elev" value={`${fmt(airport.elevationFt)} ft`} />
            )}
          </div>
          {airport.owner && (
            <p className="font-mono text-[11px] text-zinc-600 mt-2 uppercase">
              <span className="font-grotesk text-[9px] font-bold text-zinc-600 tracking-wider">
                Owner:
              </span>{" "}
              {airport.owner.name}
              {airport.ownershipNotes ? ` — ${airport.ownershipNotes}` : ""}
            </p>
          )}
        </header>

        <Divider />

        {/* ── The Verdict ─────────────────────────── */}
        <section className="flex flex-col items-center gap-2 py-8">
          <span className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-[2px] uppercase">
            The Verdict
          </span>
          <span
            className={`font-grotesk text-[96px] font-bold leading-none tabular-nums ${scoreColor(totalNum)}`}
          >
            {totalNum != null ? Math.round(totalNum) : "?"}
          </span>
          <span className="font-mono text-lg text-zinc-600">/100</span>
          <span
            className={`font-mono text-sm italic ${scoreColor(totalNum)}`}
          >
            {totalVerdict(totalNum)}
          </span>
          <p className="font-mono text-xs text-zinc-600 italic text-center max-w-2xl mt-2 leading-relaxed">
            {totalCommentary(score)}
          </p>
        </section>

        {/* ── Score Bars ──────────────────────────── */}
        <div className="flex flex-col gap-3 pb-6">
          <ScoreBar label="Infrastructure" score={score?.scoreInfrastructure} />
          <ScoreBar label="Operational" score={score?.scoreOperational} />
          <ScoreBar label="Sentiment" score={score?.scoreSentiment} />
          <ScoreBar label="Connectivity" score={score?.scoreConnectivity} />
          <ScoreBar label="Sent. Velocity" score={score?.scoreSentimentVelocity} />
          <ScoreBar label="Operator" score={score?.scoreOperator} />
        </div>

        <Divider />

        {/* ── Exhibit A: The Numbers ──────────────── */}
        <section className="flex flex-col gap-5">
          <ExhibitHeader>Exhibit A — The Numbers</ExhibitHeader>
          <div className="flex gap-8">
            <Stat
              value={latestPax ? fmtM(latestPax.totalPax) : "—"}
              label={`Passengers${latestPax ? ` (${latestPax.year})` : ""}`}
            />
            <Stat
              value={yoyGrowth != null ? `${yoyGrowth > 0 ? "+" : ""}${yoyGrowth.toFixed(1)}%` : "—"}
              label="YoY Growth"
              color={yoyGrowth != null ? (yoyGrowth > 0 ? "text-green-500" : "text-red-500") : "text-zinc-600"}
            />
            <Stat
              value={capacityNum ? fmtM(capacityNum) : "—"}
              label="Annual Capacity"
              color="text-zinc-600"
            />
          </div>
          <p className="font-mono text-xs text-zinc-600 italic leading-relaxed">
            {paxSnark(latestPaxNum, capacityNum)}
          </p>

          {latestPax && (
            <div className="flex gap-8">
              <Stat
                value={latestPax.internationalPax ? fmtM(latestPax.internationalPax) : "—"}
                label={`International${latestPax.totalPax && latestPax.internationalPax ? ` (${Math.round((latestPax.internationalPax / latestPax.totalPax) * 100)}%)` : ""}`}
                size="text-[28px]"
              />
              <Stat
                value={latestPax.domesticPax ? fmtM(latestPax.domesticPax) : "—"}
                label={`Domestic${latestPax.totalPax && latestPax.domesticPax ? ` (${Math.round((latestPax.domesticPax / latestPax.totalPax) * 100)}%)` : ""}`}
                size="text-[28px]"
                color="text-zinc-600"
              />
              <Stat
                value={latestPax.aircraftMovements ? fmt(latestPax.aircraftMovements) : "—"}
                label="Aircraft Movements"
                size="text-[28px]"
                color="text-zinc-600"
              />
            </div>
          )}

          {airport.paxYearly.length > 1 && (
            <>
              <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                Passenger History
              </span>
              <div className="flex gap-4">
                {airport.paxYearly.slice(0, 8).map((p) => (
                  <div
                    key={p.year}
                    className="flex-1 flex flex-col items-center gap-0.5"
                  >
                    <span
                      className={`font-mono text-xs font-bold tabular-nums ${
                        p === latestPax
                          ? "text-zinc-100"
                          : p.totalPax && latestPax?.totalPax && p.totalPax < latestPax.totalPax * 0.3
                            ? "text-red-500"
                            : "text-zinc-400"
                      }`}
                    >
                      {fmtM(p.totalPax)}
                    </span>
                    <span className="font-mono text-[10px] text-zinc-600">
                      {p.year}
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}
        </section>

        <Divider />

        {/* ── Exhibit B: Tardiness Report ─────────── */}
        {opsAgg && (
          <section className="flex flex-col gap-5">
            <ExhibitHeader>Exhibit B — Tardiness Report</ExhibitHeader>
            {opsAgg.periodLabel && (
              <span className="font-mono text-[10px] text-zinc-600 tracking-wider uppercase">
                {opsAgg.periodLabel} · {fmt(opsAgg.totalFlights)} flights
              </span>
            )}
            <div className="flex gap-8">
              <Stat
                value={opsAgg.delayPct != null ? `${opsAgg.delayPct.toFixed(1)}%` : "—"}
                label="Flights Delayed"
                color={scoreColor(opsAgg.delayPct != null ? 100 - opsAgg.delayPct * 2.5 : null)}
              />
              <Stat
                value={opsAgg.avgDelayMinutes != null ? `${opsAgg.avgDelayMinutes.toFixed(1)}min` : "—"}
                label="Avg Delay"
                color={scoreColor(opsAgg.avgDelayMinutes != null ? 100 - opsAgg.avgDelayMinutes * 3 : null)}
              />
              <Stat
                value={opsAgg.cancellationPct != null ? `${opsAgg.cancellationPct.toFixed(1)}%` : "—"}
                label="Cancelled"
                color={scoreColor(opsAgg.cancellationPct != null ? 100 - opsAgg.cancellationPct * 10 : null)}
              />
            </div>
            <p className="font-mono text-xs text-zinc-600 italic leading-relaxed">
              {delaySnark(opsAgg.delayPct)}
            </p>

            {(opsAgg.delayWeatherPct != null || opsAgg.delayAtcPct != null || opsAgg.delayAirportPct != null) && (
              <>
                <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                  Delay Causes (ATFM)
                </span>
                <div className="flex gap-4">
                  {[
                    { label: "Weather", val: opsAgg.delayWeatherPct },
                    { label: "Carrier", val: opsAgg.delayCarrierPct },
                    { label: "ATC", val: opsAgg.delayAtcPct },
                    { label: "Airport", val: opsAgg.delayAirportPct },
                  ].map((c) => (
                    <div key={c.label} className="flex-1 flex justify-between">
                      <span className="font-mono text-[11px] text-zinc-500">
                        {c.label}
                      </span>
                      <span
                        className={`font-mono text-[11px] font-bold ${
                          c.val != null && c.val > 25
                            ? "text-red-500"
                            : c.val != null && c.val > 15
                              ? "text-orange-500"
                              : "text-zinc-400"
                        }`}
                      >
                        {c.val != null ? `${c.val.toFixed(0)}%` : "—"}
                      </span>
                    </div>
                  ))}
                </div>
              </>
            )}

            {opsAgg.mishandledBagsPer1k != null && (
              <div className="flex gap-2 items-center">
                <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
                  MISHANDLED BAGS:
                </span>
                <span className="font-mono text-[11px] font-bold text-orange-500">
                  {opsAgg.mishandledBagsPer1k.toFixed(1)} per 1,000 passengers
                </span>
              </div>
            )}
          </section>
        )}

        <Divider />

        {/* ── Exhibit C: What People Think ─────────── */}
        <section className="flex flex-col gap-5">
          <ExhibitHeader>Exhibit C — What People Think</ExhibitHeader>
          {latestSentiment ? (
            <>
              <div className="flex gap-8">
                <Stat
                  value={latestSentiment.avgRating ? parseFloat(latestSentiment.avgRating).toFixed(1) : "—"}
                  label="Avg Rating / 10"
                  color={scoreColor(latestSentiment.avgRating ? parseFloat(latestSentiment.avgRating) * 10 : null)}
                />
                <Stat
                  value={latestSentiment.reviewCount ? fmt(latestSentiment.reviewCount) : "—"}
                  label="Reviews"
                />
                <Stat
                  value={latestSentiment.positivePct ? `${parseFloat(latestSentiment.positivePct).toFixed(0)}%` : "—"}
                  label="Positive"
                  color={latestSentiment.positivePct && parseFloat(latestSentiment.positivePct) < 30 ? "text-red-500" : "text-zinc-100"}
                />
              </div>

              {latestSentiment.skytraxStars && (
                <div className="flex gap-6 items-center">
                  <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
                    SKYTRAX STARS:
                  </span>
                  <span className="font-mono text-sm font-bold text-yellow-400">
                    {"★".repeat(latestSentiment.skytraxStars)}
                    {"☆".repeat(5 - latestSentiment.skytraxStars)}
                  </span>
                </div>
              )}

              <div className="flex gap-6">
                {[
                  { l: "Positive", v: latestSentiment.positivePct, c: "text-red-500" },
                  { l: "Neutral", v: latestSentiment.neutralPct, c: "text-zinc-400" },
                  { l: "Negative", v: latestSentiment.negativePct, c: "text-red-500" },
                ].map((s) => (
                  <div key={s.l} className="flex gap-2 items-center">
                    <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider uppercase">
                      {s.l}
                    </span>
                    <span className={`font-mono text-xs font-bold ${s.c}`}>
                      {s.v ? `${parseFloat(s.v).toFixed(0)}%` : "—"}
                    </span>
                  </div>
                ))}
              </div>

              <div className="flex gap-5">
                <div className="flex-1 flex flex-col gap-2">
                  <SentimentBar label="Queuing" score={latestSentiment.scoreQueuing} />
                  <SentimentBar label="Cleanliness" score={latestSentiment.scoreCleanliness} />
                  <SentimentBar label="Staff" score={latestSentiment.scoreStaff} />
                  <SentimentBar label="Food & Bev" score={latestSentiment.scoreFoodBev} />
                </div>
                <div className="flex-1 flex flex-col gap-2">
                  <SentimentBar label="Wifi" score={latestSentiment.scoreWifi} />
                  <SentimentBar label="Wayfinding" score={latestSentiment.scoreWayfinding} />
                  <SentimentBar label="Transport" score={latestSentiment.scoreTransport} />
                  <SentimentBar label="Shopping" score={latestSentiment.scoreShopping} />
                </div>
              </div>
            </>
          ) : (
            <p className="font-mono text-xs text-zinc-600 italic">
              No sentiment data yet. The silence is deafening.
            </p>
          )}
        </section>

        <Divider />

        {/* ── Exhibit D: Routes ───────────────────── */}
        <RouteSection
          routesWithFlights={routesWithFlights}
          topRoute={topRoute}
          restRoutes={restRoutes}
        />

        <Divider />

        {/* ── Exhibit E: Runways ──────────────────── */}
        <section className="flex flex-col gap-4">
          <ExhibitHeader>Exhibit E — The Runway Report</ExhibitHeader>
          <span className="font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase">
            {airport.runways.length} Runway{airport.runways.length !== 1 ? "s" : ""}
          </span>
          <div className="flex gap-6">
            {airport.runways.map((rw) => (
              <div
                key={rw.id}
                className="flex-1 flex flex-col gap-2 p-5 bg-[#111113] border border-zinc-800"
              >
                <span className="font-grotesk text-lg font-bold text-zinc-100 tracking-wider">
                  {rw.leIdent && rw.heIdent
                    ? `${rw.leIdent}/${rw.heIdent}`
                    : rw.ident ?? `Runway ${rw.id}`}
                </span>
                <span className="font-mono text-[11px] text-zinc-500">
                  {[
                    rw.lengthFt ? `${fmt(rw.lengthFt)}ft` : null,
                    rw.widthFt ? `${rw.widthFt}ft` : null,
                  ]
                    .filter(Boolean)
                    .join(" × ")}
                  {rw.surface ? ` · ${rw.surface}` : ""}
                  {rw.lighted ? " · Lighted" : ""}
                  {rw.closed ? " · CLOSED" : ""}
                </span>
              </div>
            ))}
          </div>
        </section>

        <Divider />

        {/* ── Exhibit F: The Backstory ────────────── */}
        {wiki && (
          <section className="flex flex-col gap-4">
            <ExhibitHeader>Exhibit F — The Backstory</ExhibitHeader>

            {wiki.terminalNames && wiki.terminalNames.length > 0 && (
              <div className="flex gap-2 items-center">
                <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
                  TERMINALS:
                </span>
                <span className="font-mono text-xs text-zinc-400">
                  {wiki.terminalNames.join(" · ")}
                </span>
              </div>
            )}

            {wiki.renovationNotes && (
              <>
                <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                  Renovation Notes
                </span>
                <p className="font-mono text-xs text-zinc-500 leading-relaxed">
                  {wiki.renovationNotes}
                </p>
              </>
            )}

            {wiki.skytraxHistory && typeof wiki.skytraxHistory === "object" && (
              <>
                <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                  Skytrax Star History
                </span>
                <div className="flex gap-4">
                  {Object.entries(wiki.skytraxHistory)
                    .sort(([a], [b]) => a.localeCompare(b))
                    .map(([year, stars]) => (
                      <div key={year} className="flex gap-1.5 items-center">
                        <span className="font-mono text-[11px] text-zinc-500">
                          {year}
                        </span>
                        <span className="font-mono text-[11px] font-bold text-yellow-400">
                          {"★".repeat(Number(stars))}
                        </span>
                      </div>
                    ))}
                </div>
              </>
            )}

            <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
              ACI Service Quality Awards
            </span>
            {wiki.aciAwards && typeof wiki.aciAwards === "object" && Object.keys(wiki.aciAwards).length > 0 ? (
              <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1">
                {Object.entries(wiki.aciAwards as Record<string, Record<string, string>>)
                  .sort(([a], [b]) => b.localeCompare(a))
                  .map(([year, placements]) => {
                    const entries = Object.entries(placements);
                    const place = entries[0]?.[0] ?? "";
                    const category = entries[0]?.[1] ?? "";
                    const medal = place === "1st" ? "🥇" : place === "2nd" ? "🥈" : place === "3rd" ? "🥉" : "🏆";
                    return (
                      <div
                        key={year}
                        className="border border-zinc-800 rounded px-3 py-2 flex flex-col gap-0.5"
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-mono text-xs font-bold text-zinc-300">{year}</span>
                          <span className="text-sm">{medal}</span>
                        </div>
                        <span className="font-mono text-[10px] text-zinc-500 uppercase tracking-wide">{place}</span>
                        <span className="font-mono text-[10px] text-zinc-600 leading-tight">{category}</span>
                      </div>
                    );
                  })}
              </div>
            ) : (
              <p className="font-mono text-xs text-zinc-500 italic mt-1">
                None recorded. A clean record — in the worst sense.
              </p>
            )}
          </section>
        )}

        <Divider />

        {/* ── Footer Links ────────────────────────── */}
        <footer className="flex gap-6">
          {airport.wikipediaUrl && (
            <a
              href={airport.wikipediaUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              WIKIPEDIA ↗
            </a>
          )}
          {airport.websiteUrl && (
            <a
              href={airport.websiteUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              WEBSITE ↗
            </a>
          )}
          {airport.skytraxUrl && (
            <a
              href={airport.skytraxUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              SKYTRAX ↗
            </a>
          )}
        </footer>
      </div>
    </div>
  );
}

// ── Route Section (with search) ──────────────────────────

type RouteRow = (typeof import("../db/schema"))["routes"]["$inferSelect"] & {
  destination?: Record<string, unknown> | null;
  destinationAirport?: { name: string; iata: string | null; icao: string; city: string; country: string } | null;
};

function routeDisplayName(r: RouteRow): string {
  return (
    r.destinationAirport?.name ??
    (r.destination as { name?: string } | null)?.name ??
    r.destinationIata ??
    r.destinationIcao ??
    "Unknown"
  );
}

function routeIata(r: RouteRow): string | null {
  return r.destinationAirport?.iata ?? r.destinationIata ?? null;
}

function RouteSection({
  routesWithFlights,
  topRoute,
  restRoutes,
}: {
  routesWithFlights: RouteRow[];
  topRoute: RouteRow | undefined;
  restRoutes: RouteRow[];
}) {
  const [search, setSearch] = useState("");
  const query = search.toLowerCase().trim();

  const filtered = query
    ? restRoutes.filter((r) => {
        const name = routeDisplayName(r).toLowerCase();
        const iata = (routeIata(r) ?? "").toLowerCase();
        const icao = (r.destinationIcao ?? "").toLowerCase();
        return name.includes(query) || iata.includes(query) || icao.includes(query);
      })
    : restRoutes;

  return (
    <section className="flex flex-col gap-4">
      <ExhibitHeader>Exhibit D — Where You Can Escape To</ExhibitHeader>
      <span className="font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase">
        {routesWithFlights.length} Routes Served
      </span>

      {topRoute && (
        <div className="flex justify-between items-center p-5 bg-[#111113] border border-zinc-800">
          <div className="flex flex-col gap-1">
            <span className="font-grotesk text-xl font-bold text-zinc-100 tracking-wider uppercase">
              {routeDisplayName(topRoute)}{" "}
              {routeIata(topRoute) ? `(${routeIata(topRoute)})` : ""}
            </span>
            {topRoute.airlineName && (
              <span className="font-mono text-[11px] text-zinc-500">
                {topRoute.airlineName}
              </span>
            )}
          </div>
          <div className="flex flex-col items-end gap-0.5">
            <span className="font-grotesk text-[32px] font-bold text-green-500 tabular-nums">
              {topRoute.flightsPerMonth ?? "—"}
            </span>
            <span className="font-mono text-[10px] text-zinc-500 tracking-wider">
              FLIGHTS/MO
            </span>
          </div>
        </div>
      )}

      <input
        type="text"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Search destinations..."
        className="w-full bg-zinc-900/50 border border-white/5 px-3 py-2 font-mono text-xs text-zinc-300 placeholder:text-zinc-600 focus:outline-none focus:border-yellow-400/30 transition-colors"
      />

      <div className="max-h-[400px] overflow-y-auto scrollbar-thin">
        <div className="flex flex-col">
          {filtered.length > 0 ? (
            filtered.map((r) => (
              <div
                key={r.id}
                className="flex justify-between items-center py-2.5 border-b border-white/5 last:border-0"
              >
                <span className="font-mono text-xs text-zinc-400">
                  {routeDisplayName(r)}
                  {routeIata(r) ? ` (${routeIata(r)})` : ""}
                </span>
                {r.airlineName && (
                  <span className="font-mono text-[10px] text-zinc-600">
                    {r.airlineName}
                  </span>
                )}
                <span className="font-mono text-xs font-bold text-zinc-100 tabular-nums shrink-0">
                  {r.flightsPerMonth ?? "—"}
                </span>
              </div>
            ))
          ) : (
            <p className="font-mono text-xs text-zinc-600 italic py-4">
              No routes matching "{search}". Trapped.
            </p>
          )}
        </div>
      </div>
    </section>
  );
}

// ── Badge ────────────────────────────────────────────────

function Badge({
  label,
  value,
  bright = false,
}: {
  label: string;
  value: string;
  bright?: boolean;
}) {
  return (
    <span className="inline-flex items-center gap-1.5 bg-white/[0.03] px-2.5 py-1">
      <span className="font-grotesk text-[9px] font-bold text-zinc-600 tracking-wider uppercase">
        {label}
      </span>
      <span
        className={`font-mono text-xs font-bold ${bright ? "text-zinc-100" : "text-zinc-400"}`}
      >
        {value}
      </span>
    </span>
  );
}
