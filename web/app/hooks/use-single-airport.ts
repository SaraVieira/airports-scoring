import { useMemo } from "react";
import { aggregateOps, computeOpsTrend } from "~/utils/agregator";
import { Airport } from "~/utils/types";
import { useLatestSentiment } from "./use-sentiment";

export const useSingleAirport = ({ airport }: { airport: Airport }) => {
  const latestSentiment = useLatestSentiment({ airport });
  const score = airport.scores[0];
  const totalNum = score?.scoreTotal ? parseFloat(score.scoreTotal) : null;

  const { recentOps, opsAgg, opsTrend } = useMemo(() => {
    const recent = airport.operationalStats.slice(0, 12);
    return {
      recentOps: recent,
      opsAgg: recent.length > 0 ? aggregateOps(recent) : null,
      opsTrend: computeOpsTrend(airport.operationalStats),
    };
  }, [airport.operationalStats]);

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

  // Data range for subtitle — single pass tracking min/max
  const dataRange = useMemo(() => {
    let minYear = Infinity,
      maxYear = -Infinity,
      hasYear = false;
    for (const p of airport.paxYearly) {
      if (p.year) {
        minYear = Math.min(minYear, p.year);
        maxYear = Math.max(maxYear, p.year);
        hasYear = true;
      }
    }
    for (const o of airport.operationalStats) {
      if (o.periodYear) {
        minYear = Math.min(minYear, o.periodYear);
        maxYear = Math.max(maxYear, o.periodYear);
        hasYear = true;
      }
    }
    for (const s of airport.sentimentSnapshots) {
      if (s.snapshotYear) {
        minYear = Math.min(minYear, s.snapshotYear);
        maxYear = Math.max(maxYear, s.snapshotYear);
        hasYear = true;
      }
    }
    return hasYear
      ? `Based on data from ${minYear}\u2013${maxYear}`
      : null;
  }, [airport.paxYearly, airport.operationalStats, airport.sentimentSnapshots]);

  // Source breakdown for sentiment
  const { googleCount, skytraxCount } = useMemo(() => {
    return {
      googleCount:
        airport.sourceBreakdown.find((s) => s.source === "google")?.count ?? 0,
      skytraxCount:
        airport.sourceBreakdown.find((s) => s.source === "skytrax")?.count ?? 0,
    };
  }, [airport.sourceBreakdown]);

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
