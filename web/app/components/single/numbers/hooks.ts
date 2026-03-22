import { useMemo } from "react";
import {
  findLatestPaxYear,
  findPreviousPaxYear,
  calculateYoyGrowth,
  findCovidLow,
  calculateRecoveryMetrics,
  type PaxYearly,
} from "./helpers";

export const useLatestPaxData = (paxData: PaxYearly[]) => {
  return useMemo(() => {
    const latestPax = findLatestPaxYear(paxData);
    const prevPax = findPreviousPaxYear(paxData, latestPax);
    const yoyGrowth = calculateYoyGrowth(latestPax, prevPax);

    return { latestPax, prevPax, yoyGrowth };
  }, [paxData]);
};

export const useGrowthNarrative = (
  paxData: PaxYearly[],
  latestPax: PaxYearly | undefined
) => {
  return useMemo(() => {
    if (paxData.length < 3 || !latestPax?.totalPax) return null;

    const paxByYear = new Map(paxData.map((p) => [p.year, p.totalPax]));
    const covidLow = findCovidLow(paxByYear);

    if (!covidLow) return null;

    const prePandemic = paxByYear.get(2019);
    const { recoveryPct, vsPre, isRecord } = calculateRecoveryMetrics(
      latestPax.totalPax,
      covidLow,
      prePandemic ?? null
    );

    return {
      covidLow,
      prePandemic,
      recoveryPct,
      vsPre,
      isRecord,
      latestYear: latestPax.year,
      latestPaxVal: latestPax.totalPax,
    };
  }, [paxData, latestPax]);
};
