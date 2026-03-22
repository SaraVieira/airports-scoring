import { useMemo } from "react";
import { aggregateOps, computeOpsTrend } from "~/utils/agregator";
import { Airport } from "~/utils/types";
import { useLatestSentiment } from "./use-sentiment";

export const useSingleAirport = ({ airport }: { airport: Airport }) => {
  const latestSentiment = useLatestSentiment({ airport });
  const score = airport.scores[0];
  const totalNum = score?.scoreTotal ? parseFloat(score.scoreTotal) : null;

  const recentOps = airport.operationalStats.slice(0, 12);
  const opsAgg = recentOps.length > 0 ? aggregateOps(recentOps) : null;
  const opsTrend = computeOpsTrend(airport.operationalStats);

  const wiki = airport.wikipediaSnapshots[0];
  // Deduplicate routes by destination
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

  // Source breakdown for sentiment
  const googleCount =
    airport.sourceBreakdown.find((s) => s.source === "google")?.count ?? 0;
  const skytraxCount =
    airport.sourceBreakdown.find((s) => s.source === "skytrax")?.count ?? 0;

  return {
    totalNum,
    latestSentiment,
    opsAgg,
    opsTrend,
    wiki,
    routesWithFlights,
    dataRange,
    googleCount,
    skytraxCount,
    score,
  };
};
