import { Airport } from "~/utils/types";

export type PaxYearly = Airport["paxYearly"][0];

export const isCurrentYearPartial = (
  year: number | null,
  totalPax: number | null,
  nextYearPax: number | null,
): boolean => {
  if (!year || !totalPax || !nextYearPax) return false;
  const currentYear = new Date().getFullYear();
  return year === currentYear && totalPax < nextYearPax * 0.5;
};

export const findLatestPaxYear = (paxData: PaxYearly[]): PaxYearly => {
  if (paxData.length < 2) return paxData[0];

  const isPartial = isCurrentYearPartial(
    paxData[0]?.year ?? null,
    paxData[0]?.totalPax ?? null,
    paxData[1]?.totalPax ?? null,
  );

  return isPartial ? paxData[1] : paxData[0];
};

export const findPreviousPaxYear = (
  paxData: PaxYearly[],
  latestPax: PaxYearly,
): PaxYearly | null => {
  const latestPaxIdx = paxData.indexOf(latestPax);
  const prevCandidates = paxData.slice(latestPaxIdx + 1);

  return (
    prevCandidates.find((p) => p.year !== 2020 && p.year !== 2021) ??
    prevCandidates[0] ??
    null
  );
};

export const calculateYoyGrowth = (
  latestPax: PaxYearly | null,
  prevPax: PaxYearly | null,
): number | null => {
  if (!latestPax?.totalPax || !prevPax?.totalPax) return null;
  return ((latestPax.totalPax - prevPax.totalPax) / prevPax.totalPax) * 100;
};

export const calculateCapacityNum = (
  capacityM: string | null | undefined,
): number | null => {
  if (!capacityM) return null;
  return parseFloat(capacityM) * 1_000_000;
};

export const createPaxSparkData = (paxYearly: PaxYearly[]) => {
  return [...paxYearly]
    .reverse()
    .map((p) => ({ year: p.year!, pax: p.totalPax ?? null }))
    .slice(-15);
};

export const getYoyGrowthColor = (growth: number | null): string => {
  if (growth === null) return "text-zinc-600";
  return growth > 0 ? "text-green-500" : "text-red-500";
};

export const getCapacityUtilizationColor = (utilization: number): string => {
  if (utilization > 0.9) return "bg-red-500";
  if (utilization > 0.7) return "bg-yellow-500";
  return "bg-green-500";
};

export const calculatePaxPercentage = (
  pax: number | null,
  total: number | null,
): number | null => {
  if (!pax || !total) return null;
  return Math.round((pax / total) * 100);
};

export const findCovidLow = (
  paxByYear: Map<number | null, number | null>,
): { year: number; pax: number } | null => {
  const covid2020 = paxByYear.get(2020);
  const covid2021 = paxByYear.get(2021);

  if (covid2020 != null && covid2021 != null) {
    return covid2020 < covid2021
      ? { year: 2020, pax: covid2020 }
      : { year: 2021, pax: covid2021 };
  }

  if (covid2020 != null) return { year: 2020, pax: covid2020 };
  if (covid2021 != null) return { year: 2021, pax: covid2021 };

  return null;
};

export const calculateRecoveryMetrics = (
  latestPax: number,
  covidLow: { year: number; pax: number },
  prePandemic: number | null,
) => {
  const recoveryPct =
    covidLow.pax > 0 ? ((latestPax - covidLow.pax) / covidLow.pax) * 100 : null;

  const vsPre =
    prePandemic && prePandemic > 0
      ? ((latestPax - prePandemic) / prePandemic) * 100
      : null;

  return { recoveryPct, vsPre, isRecord: vsPre != null && vsPre > 0 };
};
