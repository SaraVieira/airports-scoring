import { scoreColor } from "~/utils/scoring";
import { Link } from "@tanstack/react-router";

export const Rankings = ({
  ranked,
}: {
  ranked: {
    iataCode: string | null;
    name: string;
    city: string;
    countryCode: string;
    scoreTotal: string | null;
    scoreSentimentVelocity: string | null;
  }[];
}) => {
  return (
    <section className="flex flex-col gap-4 py-12">
      <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
        The Rankings
      </h2>
      <p className="font-mono text-xs text-zinc-600 italic">
        All-time composite scores. Higher is better. We think.
      </p>

      <div className="flex flex-col mt-2">
        <div className="flex items-center gap-2 py-2">
          <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-6 shrink-0">
            #
          </span>
          <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-12 shrink-0">
            IATA
          </span>
          <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider flex-1">
            AIRPORT
          </span>
          <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-20 shrink-0 text-right pr-1">
            SCORE
          </span>
        </div>
        <div className="h-px bg-zinc-800" />

        {ranked.map((airport, i) => {
          const score = airport.scoreTotal
            ? parseFloat(airport.scoreTotal)
            : null;
          const isTop3 = i < 3;
          const isBottom3 = i >= ranked.length - 3;
          const barWidth = score != null ? Math.min(score, 100) : 0;

          return (
            <Link
              key={airport.iataCode}
              to="/airport/$iata"
              params={{ iata: airport.iataCode! }}
              className={`group flex items-center gap-2 border-b transition-colors ${
                isTop3
                  ? "py-3.5 border-white/6 hover:bg-white/4"
                  : isBottom3
                    ? "py-2 border-white/3 hover:bg-white/3 opacity-70 hover:opacity-100"
                    : "py-2.5 border-white/3 hover:bg-white/3"
              }`}
            >
              {isTop3 && (
                <span
                  className={`w-0.5 self-stretch shrink-0 ${i === 0 ? "bg-yellow-400" : "bg-yellow-400/40"}`}
                />
              )}
              <span
                className={`font-mono font-bold tabular-nums w-6 shrink-0 ${
                  isTop3 ? "text-sm text-zinc-400" : "text-xs text-zinc-600"
                }`}
              >
                {i + 1}
              </span>
              <span
                className={`font-grotesk font-bold tracking-wider w-12 shrink-0 ${
                  isTop3
                    ? "text-[15px] text-[#f5f5f0]"
                    : "text-[13px] text-zinc-300"
                }`}
              >
                {airport.iataCode}
              </span>

              <span className="font-mono text-xs text-zinc-600 flex-1 truncate group-hover:text-zinc-400 transition-colors">
                {airport.name}
              </span>

              <span className="w-20 shrink-0 flex items-center gap-2 justify-end">
                <span className="w-12 h-1 bg-zinc-900 relative hidden sm:block">
                  <span
                    className={`h-1 absolute left-0 top-0 ${
                      score != null && score >= 70
                        ? "bg-green-500"
                        : score != null && score >= 40
                          ? "bg-yellow-500/70"
                          : "bg-red-500/70"
                    }`}
                    style={{ width: `${barWidth}%` }}
                  />
                </span>
                <span
                  className={`font-grotesk font-bold tabular-nums text-right ${
                    isTop3 ? "text-lg" : "text-sm"
                  } ${scoreColor(score)}`}
                >
                  {score != null ? Math.round(score) : "—"}
                </span>
              </span>
            </Link>
          );
        })}
      </div>
    </section>
  );
};
