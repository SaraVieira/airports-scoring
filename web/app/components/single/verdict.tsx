import { useSingleAirport } from "~/hooks/use-single-airport";
import { scoreColor } from "~/utils/scoring";
import { totalCommentary, totalVerdict } from "~/utils/snark";
import { Airport } from "~/utils/types";

export const Verdict = ({ airport }: { airport: Airport }) => {
  const { totalNum, dataRange, score } = useSingleAirport({ airport });
  return (
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
      {airport.ranking.position > 0 && airport.ranking.total > 0 && (
        <span className="font-mono text-[11px] text-zinc-600 mt-1">
          Ranked #{airport.ranking.position} of {airport.ranking.total} airports
        </span>
      )}
      {dataRange && (
        <span className="font-mono text-[9px] text-zinc-700 mt-1">
          {dataRange}
        </span>
      )}
    </section>
  );
};
