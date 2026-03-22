import { Stat } from "../stat";
import { fmt, fmtM } from "~/utils/format";
import { PaxSparkline } from "../pax-bar";
import {
  getYoyGrowthColor,
  getCapacityUtilizationColor,
  calculatePaxPercentage,
  type PaxYearly,
} from "./helpers";
import { useGrowthNarrative } from "./hooks";

// ============================================================================
// TopStats Component
// ============================================================================

interface TopStatsProps {
  latestPax: PaxYearly | null;
  yoyGrowth: number | null;
  capacityNum: number | null;
}

export const TopStats = ({
  latestPax,
  yoyGrowth,
  capacityNum,
}: TopStatsProps) => (
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
      color={getYoyGrowthColor(yoyGrowth)}
    />
    {capacityNum && (
      <Stat
        value={fmtM(capacityNum)}
        label="Annual Capacity"
        color="text-zinc-600"
      />
    )}
  </div>
);

// ============================================================================
// GrowthNarrative Component
// ============================================================================

type GrowthNarrativeData = ReturnType<typeof useGrowthNarrative>;

interface GrowthNarrativeProps {
  narrative: GrowthNarrativeData;
}

export const GrowthNarrative = ({ narrative }: GrowthNarrativeProps) => {
  if (!narrative) return null;

  const { isRecord, latestPaxVal, latestYear, vsPre, recoveryPct, covidLow } =
    narrative;

  return (
    <div className="border-l-2 border-green-500/50 bg-[#0d1a0d] px-4 py-3">
      <p className="font-mono text-xs text-zinc-300 leading-relaxed">
        {isRecord ? (
          <RecordYearNarrative
            latestPaxVal={latestPaxVal}
            latestYear={latestYear}
            vsPre={vsPre}
            recoveryPct={recoveryPct}
            covidYear={covidLow.year}
          />
        ) : (
          <RecoveryNarrative
            latestPaxVal={latestPaxVal}
            latestYear={latestYear}
            vsPre={vsPre}
            recoveryPct={recoveryPct}
            covidYear={covidLow.year}
          />
        )}
      </p>
    </div>
  );
};

// ============================================================================
// RecordYearNarrative Sub-Component
// ============================================================================

interface RecordYearNarrativeProps {
  latestPaxVal: number;
  latestYear: number | null | undefined;
  vsPre: number | null;
  recoveryPct: number | null;
  covidYear: number;
}

const RecordYearNarrative = ({
  latestPaxVal,
  latestYear,
  vsPre,
  recoveryPct,
  covidYear,
}: RecordYearNarrativeProps) => (
  <>
    <span className="text-green-400 font-bold">Record year!</span>{" "}
    {fmtM(latestPaxVal)} passengers in {latestYear}
    {vsPre != null && (
      <>
        , up{" "}
        <span className="text-green-400 font-bold">{vsPre.toFixed(0)}%</span>{" "}
        from pre-pandemic levels
      </>
    )}
    {recoveryPct != null && (
      <>
        {" "}
        and a staggering{" "}
        <span className="text-green-400 font-bold">
          {recoveryPct.toFixed(0)}%
        </span>{" "}
        rebound from the {covidYear} COVID low
      </>
    )}
    .
  </>
);

// ============================================================================
// RecoveryNarrative Sub-Component
// ============================================================================

interface RecoveryNarrativeProps {
  latestPaxVal: number;
  latestYear: number | null | undefined;
  vsPre: number | null;
  recoveryPct: number | null;
  covidYear: number;
}

const RecoveryNarrative = ({
  latestPaxVal,
  latestYear,
  vsPre,
  recoveryPct,
  covidYear,
}: RecoveryNarrativeProps) => (
  <>
    {fmtM(latestPaxVal)} passengers in {latestYear}
    {vsPre != null && (
      <>
        {" "}
        — still{" "}
        <span className="text-yellow-400 font-bold">
          {Math.abs(vsPre).toFixed(0)}% below
        </span>{" "}
        the 2019 peak
      </>
    )}
    {recoveryPct != null && (
      <>
        , though up {recoveryPct.toFixed(0)}% from the {covidYear} COVID crater
      </>
    )}
    .
  </>
);

// ============================================================================
// PaxBreakdown Component
// ============================================================================

interface PaxBreakdownProps {
  latestPax: PaxYearly | null;
}

export const PaxBreakdown = ({ latestPax }: PaxBreakdownProps) => {
  if (
    !latestPax ||
    (!latestPax.internationalPax &&
      !latestPax.domesticPax &&
      !latestPax.aircraftMovements)
  ) {
    return null;
  }

  return (
    <div className="flex gap-8">
      {latestPax.internationalPax && (
        <Stat
          value={fmtM(latestPax.internationalPax)}
          label={`International${
            latestPax.totalPax
              ? ` (${calculatePaxPercentage(latestPax.internationalPax, latestPax.totalPax)}%)`
              : ""
          }`}
          size="text-[28px]"
        />
      )}
      {latestPax.domesticPax && (
        <Stat
          value={fmtM(latestPax.domesticPax)}
          label={`Domestic${
            latestPax.totalPax
              ? ` (${calculatePaxPercentage(latestPax.domesticPax, latestPax.totalPax)}%)`
              : ""
          }`}
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
  );
};

// ============================================================================
// PassengerHistorySection Component
// ============================================================================

interface PassengerHistorySectionProps {
  paxSparkData: Array<{ year: number; pax: number | null }>;
}

export const PassengerHistorySection = ({
  paxSparkData,
}: PassengerHistorySectionProps) => {
  if (paxSparkData.length <= 2) return null;

  return (
    <>
      <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
        Passenger History
      </span>
      <PaxSparkline data={paxSparkData} />
    </>
  );
};

// ============================================================================
// CapacityUtilizationBar Component
// ============================================================================

interface CapacityUtilizationBarProps {
  latestPaxNum: number | null;
  capacityNum: number | null;
}

export const CapacityUtilizationBar = ({
  latestPaxNum,
  capacityNum,
}: CapacityUtilizationBarProps) => {
  if (!latestPaxNum || !capacityNum) return null;

  const utilization = latestPaxNum / capacityNum;
  const percentage = Math.round(utilization * 100);

  return (
    <div className="flex flex-col gap-1">
      <div className="flex justify-between">
        <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
          Capacity Utilization
        </span>
        <span className="font-mono text-[11px] font-bold text-zinc-400 tabular-nums">
          {percentage}%
        </span>
      </div>
      <div className="h-1.5 bg-zinc-900 relative">
        <div
          className={`h-1.5 absolute left-0 top-0 ${getCapacityUtilizationColor(utilization)}`}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />
      </div>
    </div>
  );
};
