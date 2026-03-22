import { useState, useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { eq, sql } from "drizzle-orm";
import { aggregateOps, computeOpsTrend } from "~/utils/agregator";
import {
  delaySnark,
  paxSnark,
  totalCommentary,
  totalVerdict,
} from "~/utils/snark";
import { Header } from "~/components/single/header";
import { ScoreBar } from "~/components/single/score-bar";
import { Stat } from "~/components/single/stat";
import { PaxSparkline } from "~/components/single/pax-bar";
import { fmt, fmtM } from "~/utils/format";
import { SentimentBar } from "~/components/single/sentiment/bar";
import { SentimentTimeline } from "~/components/single/sentiment/timeline";
import { RouteSection } from "~/components/single/routes";
import { scoreColor } from "~/components/single/score-bar";

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

function Divider() {
  return <div className="w-full h-px bg-zinc-800" />;
}

// ── Exhibit Header ───────────────────────────────────────

export function ExhibitHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
      {children}
    </h3>
  );
}

// ── Trend Indicator ──────────────────────────────────────

function TrendIndicator({
  value,
  suffix = "",
  invert = false,
}: {
  value: number | null;
  suffix?: string;
  invert?: boolean;
}) {
  if (value == null) return null;
  const improved = invert ? value < 0 : value > 0;
  const color = improved ? "text-green-500" : "text-red-500";
  const arrow = value > 0 ? "+" : "";
  return (
    <span className={`font-mono text-[11px] font-bold ${color}`}>
      {arrow}
      {value.toFixed(1)}
      {suffix} vs prior year
    </span>
  );
}

function AirportDetail() {
  const airport = Route.useLoaderData();
  const score = airport.scores[0];
  const totalNum = score?.scoreTotal ? parseFloat(score.scoreTotal) : null;
  // Find latest full year (skip current partial year — if latest year's pax is <50% of previous, it's likely partial)
  const currentYear = new Date().getFullYear();
  const paxData = airport.paxYearly;
  const latestPax =
    paxData[0]?.year === currentYear &&
    paxData.length > 1 &&
    paxData[1]?.totalPax &&
    paxData[0].totalPax &&
    paxData[0].totalPax < paxData[1].totalPax * 0.5
      ? paxData[1] // skip partial current year
      : paxData[0];
  // For YoY, compare against the year before latestPax, skipping 2020 (covid anomaly)
  const latestPaxIdx = paxData.indexOf(latestPax);
  const prevCandidates = paxData.slice(latestPaxIdx + 1);
  const prevPax =
    prevCandidates.find((p) => p.year !== 2020 && p.year !== 2021) ??
    prevCandidates[0];
  const recentOps = airport.operationalStats.slice(0, 12);
  const opsAgg = recentOps.length > 0 ? aggregateOps(recentOps) : null;
  const opsTrend = computeOpsTrend(airport.operationalStats);

  // Aggregate ALL sentiment snapshots
  const latestSentiment = useMemo(() => {
    const snaps = airport.sentimentSnapshots;
    if (snaps.length === 0) return null;

    let totalRating = 0,
      ratingCount = 0;
    let totalReviews = 0;
    let totalPositive = 0,
      totalNegative = 0,
      totalNeutral = 0,
      pctCount = 0;
    let queueSum = 0,
      queueN = 0;
    let cleanSum = 0,
      cleanN = 0;
    let staffSum = 0,
      staffN = 0;
    let foodSum = 0,
      foodN = 0;
    let wifiSum = 0,
      wifiN = 0;
    let waySum = 0,
      wayN = 0;
    let transSum = 0,
      transN = 0;
    let shopSum = 0,
      shopN = 0;
    let skytraxStars = snaps[0].skytraxStars;

    for (const s of snaps) {
      if (s.avgRating != null) {
        totalRating += parseFloat(String(s.avgRating));
        ratingCount++;
      }
      if (s.reviewCount != null) totalReviews += s.reviewCount;
      if (s.positivePct != null) {
        totalPositive += parseFloat(String(s.positivePct));
        pctCount++;
      }
      if (s.negativePct != null)
        totalNegative += parseFloat(String(s.negativePct));
      if (s.neutralPct != null)
        totalNeutral += parseFloat(String(s.neutralPct));
      if (s.scoreQueuing != null) {
        queueSum += parseFloat(String(s.scoreQueuing));
        queueN++;
      }
      if (s.scoreCleanliness != null) {
        cleanSum += parseFloat(String(s.scoreCleanliness));
        cleanN++;
      }
      if (s.scoreStaff != null) {
        staffSum += parseFloat(String(s.scoreStaff));
        staffN++;
      }
      if (s.scoreFoodBev != null) {
        foodSum += parseFloat(String(s.scoreFoodBev));
        foodN++;
      }
      if (s.scoreWifi != null) {
        wifiSum += parseFloat(String(s.scoreWifi));
        wifiN++;
      }
      if (s.scoreWayfinding != null) {
        waySum += parseFloat(String(s.scoreWayfinding));
        wayN++;
      }
      if (s.scoreTransport != null) {
        transSum += parseFloat(String(s.scoreTransport));
        transN++;
      }
      if (s.scoreShopping != null) {
        shopSum += parseFloat(String(s.scoreShopping));
        shopN++;
      }
      if (s.skytraxStars != null) skytraxStars = s.skytraxStars;
    }

    return {
      avgRating:
        ratingCount > 0 ? String((totalRating / ratingCount).toFixed(2)) : null,
      reviewCount: totalReviews,
      positivePct:
        pctCount > 0 ? String((totalPositive / pctCount).toFixed(2)) : null,
      negativePct:
        pctCount > 0 ? String((totalNegative / pctCount).toFixed(2)) : null,
      neutralPct:
        pctCount > 0 ? String((totalNeutral / pctCount).toFixed(2)) : null,
      scoreQueuing: queueN > 0 ? String((queueSum / queueN).toFixed(2)) : null,
      scoreCleanliness:
        cleanN > 0 ? String((cleanSum / cleanN).toFixed(2)) : null,
      scoreStaff: staffN > 0 ? String((staffSum / staffN).toFixed(2)) : null,
      scoreFoodBev: foodN > 0 ? String((foodSum / foodN).toFixed(2)) : null,
      scoreWifi: wifiN > 0 ? String((wifiSum / wifiN).toFixed(2)) : null,
      scoreWayfinding: wayN > 0 ? String((waySum / wayN).toFixed(2)) : null,
      scoreTransport:
        transN > 0 ? String((transSum / transN).toFixed(2)) : null,
      scoreShopping: shopN > 0 ? String((shopSum / shopN).toFixed(2)) : null,
      skytraxStars,
      snapshotCount: snaps.length,
    };
  }, [airport.sentimentSnapshots]);

  const wiki = airport.wikipediaSnapshots[0];
  // Deduplicate routes by destination — keep the one with highest flights/mo
  const routesWithFlights = useMemo(() => {
    const all = airport.routesOut.filter(
      (r) =>
        (r.flightsPerMonth != null && r.flightsPerMonth > 0) || r.airlineName,
    );
    const byDest = new Map<string, (typeof all)[number]>();
    for (const r of all) {
      const key = r.destinationIata ?? r.destinationIcao ?? `${r.id}`;
      const existing = byDest.get(key);
      if (
        !existing ||
        (r.flightsPerMonth ?? 0) > (existing.flightsPerMonth ?? 0)
      ) {
        byDest.set(key, r);
      }
    }
    return Array.from(byDest.values()).sort(
      (a, b) => (b.flightsPerMonth ?? 0) - (a.flightsPerMonth ?? 0),
    );
  }, [airport.routesOut]);

  const yoyGrowth =
    latestPax?.totalPax && prevPax?.totalPax
      ? ((latestPax.totalPax - prevPax.totalPax) / prevPax.totalPax) * 100
      : null;

  const capacityNum = airport.annualCapacityM
    ? parseFloat(airport.annualCapacityM) * 1_000_000
    : null;
  const latestPaxNum = latestPax?.totalPax ?? null;

  // Data range for subtitle
  const paxYears = airport.paxYearly
    .map((p) => p.year)
    .filter(Boolean) as number[];
  const opsYears = airport.operationalStats
    .map((o) => o.periodYear)
    .filter(Boolean) as number[];
  const sentYears = airport.sentimentSnapshots
    .map((s) => s.snapshotYear)
    .filter(Boolean) as number[];
  const allYears = [...paxYears, ...opsYears, ...sentYears];
  const dataRange =
    allYears.length > 0
      ? `Based on data from ${Math.min(...allYears)}–${Math.max(...allYears)}`
      : null;

  // Pax sparkline data (reversed to show oldest first)
  const paxSparkData = [...airport.paxYearly]
    .reverse()
    .map((p) => ({ year: p.year!, pax: p.totalPax }))
    .slice(-15); // Cap at last 15 years for readability

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-12 flex flex-col gap-9">
        <Divider />
        <Header airport={airport} />
        {/* ── The Verdict ─────────────────────────── */}
        <section className="flex flex-col gap-1 py-6">
          <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[2px] uppercase">
            The Verdict
          </span>
          <div className="flex items-end gap-3">
            <span
              className={`font-grotesk text-[72px] font-bold leading-none tabular-nums ${scoreColor(totalNum)}`}
            >
              {totalNum != null ? Math.round(totalNum) : "?"}
            </span>
            <span className="font-mono text-sm text-zinc-600 pb-2">/100</span>
            <span
              className={`font-mono text-sm italic pb-2 ${scoreColor(totalNum)}`}
            >
              {totalVerdict(totalNum)}
            </span>
          </div>
          <p className="font-mono text-[11px] text-zinc-600 italic max-w-2xl mt-1 leading-relaxed">
            {totalCommentary(score)}
          </p>
          {dataRange && (
            <span className="font-mono text-[9px] text-zinc-700 mt-1">
              {dataRange}
            </span>
          )}
        </section>

        {/* ── Score Bars ──────────────────────────── */}
        <div className="flex flex-col gap-3 pb-6">
          <ScoreBar
            label="Operational"
            score={score?.scoreOperational}
            weight="25%"
          />
          <ScoreBar
            label="Sentiment"
            score={score?.scoreSentiment}
            weight="25%"
          />
          <ScoreBar
            label="Infrastructure"
            score={score?.scoreInfrastructure}
            weight="15%"
          />
          <ScoreBar
            label="Sent. Velocity"
            score={score?.scoreSentimentVelocity}
            weight="15%"
          />
          <ScoreBar
            label="Connectivity"
            score={score?.scoreConnectivity}
            weight="10%"
          />
          <ScoreBar
            label="Operator"
            score={score?.scoreOperator}
            weight="10%"
          />
        </div>

        <Divider />

        {/* ── Exhibit A: The Numbers ──────────────── */}
        <section className="flex flex-col gap-5">
          <ExhibitHeader>The Numbers</ExhibitHeader>
          <div className="flex gap-8">
            <Stat
              value={latestPax ? fmtM(latestPax.totalPax) : "—"}
              label={`Passengers${latestPax ? ` (${latestPax.year})` : ""}`}
            />
            <Stat
              value={
                yoyGrowth != null
                  ? `${yoyGrowth > 0 ? "+" : ""}${yoyGrowth.toFixed(1)}%`
                  : "—"
              }
              label="YoY Growth"
              color={
                yoyGrowth != null
                  ? yoyGrowth > 0
                    ? "text-green-500"
                    : "text-red-500"
                  : "text-zinc-600"
              }
            />
            {capacityNum && (
              <Stat
                value={fmtM(capacityNum)}
                label="Annual Capacity"
                color="text-zinc-600"
              />
            )}
          </div>
          {capacityNum && latestPaxNum && (
            <p className="font-mono text-xs text-zinc-600 italic leading-relaxed">
              {paxSnark(latestPaxNum, capacityNum)}
            </p>
          )}

          {latestPax &&
            (latestPax.internationalPax ||
              latestPax.domesticPax ||
              latestPax.aircraftMovements) && (
              <div className="flex gap-8">
                {latestPax.internationalPax && (
                  <Stat
                    value={fmtM(latestPax.internationalPax)}
                    label={`International${latestPax.totalPax ? ` (${Math.round((latestPax.internationalPax / latestPax.totalPax) * 100)}%)` : ""}`}
                    size="text-[28px]"
                  />
                )}
                {latestPax.domesticPax && (
                  <Stat
                    value={fmtM(latestPax.domesticPax)}
                    label={`Domestic${latestPax.totalPax ? ` (${Math.round((latestPax.domesticPax / latestPax.totalPax) * 100)}%)` : ""}`}
                    size="text-[28px]"
                    color="text-zinc-600"
                  />
                )}
                {latestPax.aircraftMovements && (
                  <Stat
                    value={fmt(latestPax.aircraftMovements)}
                    label="Aircraft Movements"
                    size="text-[28px]"
                    color="text-zinc-600"
                  />
                )}
              </div>
            )}

          {/* Passenger History Sparkline */}
          {paxSparkData.length > 2 && (
            <>
              <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                Passenger History
              </span>
              <PaxSparkline data={paxSparkData} />
            </>
          )}

          {/* Capacity utilization bar */}
          {latestPaxNum && capacityNum && (
            <div className="flex flex-col gap-1">
              <div className="flex justify-between">
                <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                  Capacity Utilization
                </span>
                <span className="font-mono text-[11px] font-bold text-zinc-400 tabular-nums">
                  {Math.round((latestPaxNum / capacityNum) * 100)}%
                </span>
              </div>
              <div className="h-1.5 bg-zinc-900 relative">
                <div
                  className={`h-1.5 absolute left-0 top-0 ${
                    latestPaxNum / capacityNum > 0.9
                      ? "bg-red-500"
                      : latestPaxNum / capacityNum > 0.7
                        ? "bg-yellow-500"
                        : "bg-green-500"
                  }`}
                  style={{
                    width: `${Math.min((latestPaxNum / capacityNum) * 100, 100)}%`,
                  }}
                />
              </div>
            </div>
          )}
        </section>

        <Divider />

        {/* ── Exhibit B: Tardiness Report ─────────── */}
        {opsAgg && (
          <section className="flex flex-col gap-5">
            <ExhibitHeader>Tardiness Report</ExhibitHeader>
            {opsAgg.periodLabel && (
              <span className="font-mono text-[10px] text-zinc-600 tracking-wider uppercase">
                {opsAgg.periodLabel} · {fmt(opsAgg.totalFlights)} flights
              </span>
            )}
            <div className="flex gap-8">
              <Stat
                value={
                  opsAgg.delayPct != null
                    ? `${opsAgg.delayPct.toFixed(1)}%`
                    : "—"
                }
                label="Flights Delayed"
                color={scoreColor(
                  opsAgg.delayPct != null ? 100 - opsAgg.delayPct * 2.5 : null,
                )}
              />
              <Stat
                value={
                  opsAgg.avgDelayMinutes != null
                    ? `${opsAgg.avgDelayMinutes.toFixed(1)}min`
                    : "—"
                }
                label="Avg Delay"
                color={scoreColor(
                  opsAgg.avgDelayMinutes != null
                    ? 100 - opsAgg.avgDelayMinutes * 3
                    : null,
                )}
              />
              {opsAgg.cancellationPct != null && (
                <Stat
                  value={`${opsAgg.cancellationPct.toFixed(1)}%`}
                  label="Cancelled"
                  color={scoreColor(100 - opsAgg.cancellationPct * 10)}
                />
              )}
            </div>
            <p className="font-mono text-xs text-zinc-600 italic leading-relaxed">
              {delaySnark(opsAgg.delayPct)}
            </p>

            {/* Year-over-year trend */}
            {opsTrend && (
              <div className="flex gap-6">
                <TrendIndicator
                  value={opsTrend.delayChange}
                  suffix="pp"
                  invert
                />
                {opsTrend.avgDelayChange != null && (
                  <TrendIndicator
                    value={opsTrend.avgDelayChange}
                    suffix="min"
                    invert
                  />
                )}
              </div>
            )}

            {(opsAgg.delayWeatherPct != null ||
              opsAgg.delayAtcPct != null ||
              opsAgg.delayAirportPct != null) && (
              <>
                <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
                  Delay Causes (ATFM)
                </span>

                {/* Call out airport-caused delays when dominant */}
                {opsAgg.delayAirportPct != null &&
                  opsAgg.delayAirportPct > 50 && (
                    <div className="flex items-center gap-3 py-2 px-3 bg-red-500/8 border border-red-500/20">
                      <span className="font-grotesk text-[28px] font-bold text-red-500 tabular-nums">
                        {opsAgg.delayAirportPct.toFixed(0)}%
                      </span>
                      <div className="flex flex-col gap-0.5">
                        <span className="font-grotesk text-[11px] font-bold text-red-500 tracking-wider uppercase">
                          Airport-Caused
                        </span>
                        <span className="font-mono text-[10px] text-red-400/70 italic">
                          The airport itself is the primary reason for delays.
                          Not weather. Not ATC. Them.
                        </span>
                      </div>
                    </div>
                  )}

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
                          c.val != null && c.val > 50
                            ? "text-red-500"
                            : c.val != null && c.val > 25
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
          <ExhibitHeader>What People Think</ExhibitHeader>
          {latestSentiment ? (
            <>
              <div className="flex gap-8">
                <Stat
                  value={
                    latestSentiment.avgRating
                      ? parseFloat(latestSentiment.avgRating).toFixed(1)
                      : "—"
                  }
                  label="Avg Rating / 10"
                  color={scoreColor(
                    latestSentiment.avgRating
                      ? parseFloat(latestSentiment.avgRating) * 10
                      : null,
                  )}
                />
                <Stat
                  value={
                    latestSentiment.reviewCount
                      ? fmt(latestSentiment.reviewCount)
                      : "—"
                  }
                  label="Reviews"
                />
                <Stat
                  value={
                    latestSentiment.positivePct
                      ? `${parseFloat(latestSentiment.positivePct).toFixed(0)}%`
                      : "—"
                  }
                  label="Positive"
                  color={
                    latestSentiment.positivePct &&
                    parseFloat(latestSentiment.positivePct) < 30
                      ? "text-red-500"
                      : "text-zinc-100"
                  }
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

              {/* Sentiment trajectory chart */}
              <SentimentTimeline snapshots={airport.sentimentSnapshots} />

              <div className="flex gap-6">
                {[
                  {
                    l: "Positive",
                    v: latestSentiment.positivePct,
                    c:
                      latestSentiment.positivePct &&
                      parseFloat(latestSentiment.positivePct) >= 50
                        ? "text-green-500"
                        : latestSentiment.positivePct &&
                            parseFloat(latestSentiment.positivePct) >= 30
                          ? "text-yellow-500"
                          : "text-red-500",
                  },
                  {
                    l: "Neutral",
                    v: latestSentiment.neutralPct,
                    c: "text-zinc-400",
                  },
                  {
                    l: "Negative",
                    v: latestSentiment.negativePct,
                    c: "text-red-500",
                  },
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
                  <SentimentBar
                    label="Queuing"
                    score={latestSentiment.scoreQueuing}
                  />
                  <SentimentBar
                    label="Cleanliness"
                    score={latestSentiment.scoreCleanliness}
                  />
                  <SentimentBar
                    label="Staff"
                    score={latestSentiment.scoreStaff}
                  />
                  <SentimentBar
                    label="Food & Bev"
                    score={latestSentiment.scoreFoodBev}
                  />
                </div>
                <div className="flex-1 flex flex-col gap-2">
                  <SentimentBar
                    label="Wifi"
                    score={latestSentiment.scoreWifi}
                  />
                  <SentimentBar
                    label="Wayfinding"
                    score={latestSentiment.scoreWayfinding}
                  />
                  <SentimentBar
                    label="Transport"
                    score={latestSentiment.scoreTransport}
                  />
                  <SentimentBar
                    label="Shopping"
                    score={latestSentiment.scoreShopping}
                  />
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
        <RouteSection routesWithFlights={routesWithFlights} />

        <Divider />

        {/* ── Exhibit E: Runways ──────────────────── */}
        <section className="flex flex-col gap-4">
          <ExhibitHeader>The Runway Report</ExhibitHeader>
          <span className="font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase">
            {airport.runways.length} Runway
            {airport.runways.length !== 1 ? "s" : ""}
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
                    : (rw.ident ?? `Runway ${rw.id}`)}
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
            <ExhibitHeader>The Backstory</ExhibitHeader>

            {/* Timeline of key events */}
            <BackstoryTimeline airport={airport} wiki={wiki} />

            {/* ACI Awards */}
            <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4">
              ACI Service Quality Awards
            </span>
            {wiki.aciAwards &&
            typeof wiki.aciAwards === "object" &&
            Object.keys(wiki.aciAwards).length > 0 ? (
              <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1">
                {Object.entries(
                  wiki.aciAwards as Record<string, Record<string, string>>,
                )
                  .sort(([a], [b]) => b.localeCompare(a))
                  .map(([year, placements]) => {
                    const entries = Object.entries(placements);
                    const place = entries[0]?.[0] ?? "";
                    const category = entries[0]?.[1] ?? "";
                    const medal =
                      place === "1st"
                        ? "🥇"
                        : place === "2nd"
                          ? "🥈"
                          : place === "3rd"
                            ? "🥉"
                            : "🏆";
                    return (
                      <div
                        key={year}
                        className="border border-zinc-800 rounded px-3 py-2 flex flex-col gap-0.5"
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-mono text-xs font-bold text-zinc-300">
                            {year}
                          </span>
                          <span className="text-sm">{medal}</span>
                        </div>
                        <span className="font-mono text-[10px] text-zinc-500 uppercase tracking-wide">
                          {place}
                        </span>
                        <span className="font-mono text-[10px] text-zinc-600 leading-tight">
                          {category}
                        </span>
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

// ── Backstory Timeline ──────────────────────────────────

function TruncatedText({
  text,
  maxLength = 200,
}: {
  text: string;
  maxLength?: number;
}) {
  const [expanded, setExpanded] = useState(false);
  if (text.length <= maxLength) {
    return (
      <span className="font-mono text-[10px] text-zinc-600 leading-relaxed">
        {text}
      </span>
    );
  }
  return (
    <span className="font-mono text-[10px] text-zinc-600 leading-relaxed">
      {expanded ? text : `${text.slice(0, maxLength).trim()}...`}
      <button
        onClick={() => setExpanded(!expanded)}
        className="font-grotesk text-[10px] font-bold text-yellow-400/70 hover:text-yellow-400 tracking-wider ml-2 transition-colors"
      >
        {expanded ? "LESS" : "MORE"}
      </button>
    </span>
  );
}

function BackstoryTimeline({
  airport,
  wiki,
}: {
  airport: { openedYear?: number | null; lastMajorReno?: number | null };
  wiki: {
    terminalNames?: string[] | null;
    renovationNotes?: string | null;
    skytraxHistory?: unknown;
  };
}) {
  type TimelineEvent = {
    year: number;
    label: string;
    detail?: string;
    color: string;
  };
  const events: TimelineEvent[] = [];

  if (airport.openedYear) {
    events.push({
      year: airport.openedYear,
      label: "Opened",
      color: "text-green-500",
    });
  }

  if (airport.lastMajorReno) {
    events.push({
      year: airport.lastMajorReno,
      label: "Major Renovation",
      detail: wiki.renovationNotes ?? undefined,
      color: "text-yellow-400",
    });
  }

  if (wiki.skytraxHistory && typeof wiki.skytraxHistory === "object") {
    for (const [year, stars] of Object.entries(
      wiki.skytraxHistory as Record<string, number>,
    )) {
      events.push({
        year: parseInt(year),
        label: `${stars}-Star Skytrax Rating`,
        color: "text-yellow-400",
      });
    }
  }

  events.sort((a, b) => a.year - b.year);

  if (
    events.length === 0 &&
    !wiki.terminalNames?.length &&
    !wiki.renovationNotes
  ) {
    return null;
  }

  return (
    <div className="flex flex-col gap-0">
      {wiki.terminalNames && wiki.terminalNames.length > 0 && (
        <div className="flex gap-2 items-center mb-4">
          <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
            TERMINALS:
          </span>
          <span className="font-mono text-xs text-zinc-400">
            {wiki.terminalNames.join(" · ")}
          </span>
        </div>
      )}

      {events.length > 0 && (
        <div className="flex flex-col">
          {events.map((ev, i) => (
            <div key={`${ev.year}-${i}`} className="flex gap-4 items-start">
              <div className="flex flex-col items-center">
                <span className="font-mono text-xs font-bold text-zinc-400 tabular-nums w-12 shrink-0">
                  {ev.year}
                </span>
                {i < events.length - 1 && (
                  <div className="w-px h-6 bg-zinc-800 mt-1" />
                )}
              </div>
              <div className="flex flex-col gap-0.5 pb-4">
                <span
                  className={`font-grotesk text-[11px] font-bold ${ev.color} tracking-wider uppercase`}
                >
                  {ev.label}
                </span>
                {ev.detail && <TruncatedText text={ev.detail} />}
              </div>
            </div>
          ))}
        </div>
      )}

      {wiki.renovationNotes && !airport.lastMajorReno && (
        <div className="flex flex-col gap-2 mt-2">
          <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
            History
          </span>
          <TruncatedText text={wiki.renovationNotes} maxLength={250} />
        </div>
      )}
    </div>
  );
}
