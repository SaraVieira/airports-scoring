import { useMemo } from "react";
import { Airport } from "~/utils/types";

const calculateAverage = (sum: number, count: number): string | null =>
  count > 0 ? String((sum / count).toFixed(2)) : null;

const parseScore = (value: unknown): number =>
  value ? parseFloat(String(value)) : 0;

const aggregateSnapshots = (snaps: Airport["sentimentSnapshots"]) => {
  if (snaps.length === 0) return null;

  const result = snaps.reduce(
    (
      acc,
      {
        avgRating,
        reviewCount,
        positivePct,
        negativePct,
        neutralPct,
        scoreQueuing,
        scoreCleanliness,
        scoreStaff,
        scoreFoodBev,
        scoreWifi,
        scoreWayfinding,
        scoreTransport,
        scoreShopping,
        skytraxStars,
        notes,
      },
    ) => ({
      totalRating: acc.totalRating + parseScore(avgRating),
      ratingCount: acc.ratingCount + (avgRating != null ? 1 : 0),
      totalReviews: acc.totalReviews + (reviewCount ?? 0),
      totalPositive: acc.totalPositive + parseScore(positivePct),
      totalNegative: acc.totalNegative + parseScore(negativePct),
      totalNeutral: acc.totalNeutral + parseScore(neutralPct),
      pctCount: acc.pctCount + (positivePct != null ? 1 : 0),
      queueSum: acc.queueSum + parseScore(scoreQueuing),
      queueN: acc.queueN + (scoreQueuing != null ? 1 : 0),
      cleanSum: acc.cleanSum + parseScore(scoreCleanliness),
      cleanN: acc.cleanN + (scoreCleanliness != null ? 1 : 0),
      staffSum: acc.staffSum + parseScore(scoreStaff),
      staffN: acc.staffN + (scoreStaff != null ? 1 : 0),
      foodSum: acc.foodSum + parseScore(scoreFoodBev),
      foodN: acc.foodN + (scoreFoodBev != null ? 1 : 0),
      wifiSum: acc.wifiSum + parseScore(scoreWifi),
      wifiN: acc.wifiN + (scoreWifi != null ? 1 : 0),
      waySum: acc.waySum + parseScore(scoreWayfinding),
      wayN: acc.wayN + (scoreWayfinding != null ? 1 : 0),
      transSum: acc.transSum + parseScore(scoreTransport),
      transN: acc.transN + (scoreTransport != null ? 1 : 0),
      shopSum: acc.shopSum + parseScore(scoreShopping),
      shopN: acc.shopN + (scoreShopping != null ? 1 : 0),
      latestSkytrax: skytraxStars ?? acc.latestSkytrax,
      latestNotes: acc.latestNotes ?? notes ?? null,
    }),
    {
      totalRating: 0,
      ratingCount: 0,
      totalReviews: 0,
      totalPositive: 0,
      totalNegative: 0,
      totalNeutral: 0,
      pctCount: 0,
      queueSum: 0,
      queueN: 0,
      cleanSum: 0,
      cleanN: 0,
      staffSum: 0,
      staffN: 0,
      foodSum: 0,
      foodN: 0,
      wifiSum: 0,
      wifiN: 0,
      waySum: 0,
      wayN: 0,
      transSum: 0,
      transN: 0,
      shopSum: 0,
      shopN: 0,
      latestSkytrax: snaps[0]?.skytraxStars ?? null,
      latestNotes: null as string | null,
    },
  );

  return {
    avgRating: calculateAverage(result.totalRating, result.ratingCount),
    reviewCount: result.totalReviews,
    positivePct: calculateAverage(result.totalPositive, result.pctCount),
    negativePct: calculateAverage(result.totalNegative, result.pctCount),
    neutralPct: calculateAverage(result.totalNeutral, result.pctCount),
    scoreQueuing: calculateAverage(result.queueSum, result.queueN),
    scoreCleanliness: calculateAverage(result.cleanSum, result.cleanN),
    scoreStaff: calculateAverage(result.staffSum, result.staffN),
    scoreFoodBev: calculateAverage(result.foodSum, result.foodN),
    scoreWifi: calculateAverage(result.wifiSum, result.wifiN),
    scoreWayfinding: calculateAverage(result.waySum, result.wayN),
    scoreTransport: calculateAverage(result.transSum, result.transN),
    scoreShopping: calculateAverage(result.shopSum, result.shopN),
    skytraxStars: result.latestSkytrax,
    snapshotCount: snaps.length,
    notes: result.latestNotes,
  };
};

export const useLatestSentiment = ({ airport }: { airport: Airport }) => {
  const latestSentiment = useMemo(
    () => aggregateSnapshots(airport.sentimentSnapshots),
    [airport.sentimentSnapshots],
  );

  return latestSentiment;
};
