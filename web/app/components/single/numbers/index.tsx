import { paxSnark } from "~/utils/snark";
import { HeaderText } from "../header-text";
import { createPaxSparkData, calculateCapacityNum } from "./helpers";
import { useLatestPaxData, useGrowthNarrative } from "./hooks";
import {
  TopStats,
  GrowthNarrative,
  PaxBreakdown,
  PassengerHistorySection,
  CapacityUtilizationBar,
} from "./sub-components";
import { Airport } from "~/utils/types";

export const Numbers = ({ airport }: { airport: Airport }) => {
  const { latestPax, yoyGrowth } = useLatestPaxData(airport.paxYearly);

  const growthNarrative = useGrowthNarrative(
    airport.paxYearly,
    latestPax ?? null,
  );

  const capacityNum = calculateCapacityNum(airport.annualCapacityM);
  const latestPaxNum = latestPax?.totalPax ?? null;
  const paxSparkData = createPaxSparkData(airport.paxYearly);

  return (
    <section className="flex flex-col gap-5 bg-[#0a0d0a] -mx-16 px-16 py-8">
      <HeaderText>The Numbers</HeaderText>

      <TopStats
        latestPax={latestPax ?? null}
        yoyGrowth={yoyGrowth}
        capacityNum={capacityNum}
      />

      <GrowthNarrative narrative={growthNarrative} />

      {capacityNum != null && capacityNum > 0 && latestPaxNum != null && latestPaxNum > 0 && (
        <p className="font-mono text-xs text-zinc-600 italic leading-relaxed">
          {paxSnark(latestPaxNum, capacityNum)}
        </p>
      )}

      <PaxBreakdown latestPax={latestPax ?? null} />

      <PassengerHistorySection paxSparkData={paxSparkData} />

      <CapacityUtilizationBar
        latestPaxNum={latestPaxNum}
        capacityNum={capacityNum}
      />
    </section>
  );
};
