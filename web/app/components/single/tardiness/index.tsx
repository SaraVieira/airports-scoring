import { useSingleAirport } from "~/hooks/use-single-airport";
import { Airport } from "~/utils/types";
import { HeaderText } from "../header-text";
import { fmt } from "~/utils/format";
import { scoreColor } from "~/utils/scoring";
import { Stat } from "../stat";
import { TrendIndicator } from "../trend-indicator";
import { delaySnark } from "~/utils/snark";

export const Tardiness = ({ airport }: { airport: Airport }) => {
  const { opsAgg, opsTrend } = useSingleAirport({ airport });
  if (!opsAgg) return null;

  const reasons = [
    { label: "Weather", val: opsAgg.delayWeatherPct },
    { label: "Carrier", val: opsAgg.delayCarrierPct },
    { label: "ATC", val: opsAgg.delayAtcPct },
    { label: "Airport", val: opsAgg.delayAirportPct },
  ];

  const hasDelayReason =
    opsAgg.delayWeatherPct != null ||
    opsAgg.delayAtcPct != null ||
    opsAgg.delayAirportPct != null;
  return (
    <section className="flex flex-col gap-5 bg-[#0f0a0a] -mx-16 px-16 py-8">
      <HeaderText>Tardiness Report</HeaderText>
      {opsAgg.periodLabel && (
        <span className="font-mono text-[10px] text-zinc-600 tracking-wider uppercase">
          {opsAgg.periodLabel} · {fmt(opsAgg.totalFlights)} flights
        </span>
      )}
      <div className="flex gap-8">
        <div className="flex-1 flex flex-col gap-1">
          <span
            className={`font-grotesk text-[42px] font-bold tabular-nums ${scoreColor(
              opsAgg.delayPct != null ? 100 - opsAgg.delayPct * 2.5 : null,
            )}`}
          >
            {opsAgg.delayPct != null ? `${opsAgg.delayPct.toFixed(1)}%` : "—"}
          </span>
          <span className="font-mono text-[11px] text-zinc-500 tracking-wider uppercase">
            Flights Delayed
          </span>
          {opsAgg.delayPct != null && opsAgg.totalFlights > 0 && (
            <span className="font-mono text-[10px] text-zinc-500">
              {fmt(Math.round((opsAgg.delayPct / 100) * opsAgg.totalFlights))}{" "}
              of {fmt(opsAgg.totalFlights)} flights
            </span>
          )}
        </div>
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

      {opsTrend && (
        <div className="flex gap-6">
          <TrendIndicator value={opsTrend.delayChange} suffix="pp" invert />
          {opsTrend.avgDelayChange != null && (
            <TrendIndicator
              value={opsTrend.avgDelayChange}
              suffix="min"
              invert
            />
          )}
        </div>
      )}

      {hasDelayReason && (
        <>
          <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
            Delay Causes (ATFM)
          </span>

          {opsAgg.delayAirportPct != null && opsAgg.delayAirportPct > 50 && (
            <div className="flex items-center gap-3 py-2 px-3 bg-red-500/8 border border-red-500/20">
              <span className="font-grotesk text-[28px] font-bold text-red-500 tabular-nums">
                {opsAgg.delayAirportPct.toFixed(0)}%
              </span>
              <div className="flex flex-col gap-0.5">
                <span className="font-grotesk text-[11px] font-bold text-red-500 tracking-wider uppercase">
                  Airport-Caused
                </span>
                <span className="font-mono text-[10px] text-red-400/70 italic">
                  The airport itself is the primary reason for delays. Not
                  weather. Not ATC. Them.
                </span>
              </div>
            </div>
          )}

          <div className="flex gap-4">
            {reasons.map((c) => (
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
  );
};
