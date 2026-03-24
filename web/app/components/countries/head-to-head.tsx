import { useMemo } from "react";
import { CountrySummary } from "~/api/client";
import { getFlag } from "~/routes/countries";
import { fmtM } from "~/utils/format";
import { scoreHex } from "~/utils/scoring";

export function HeadToHead({
  left,
  right,
  onBack,
}: {
  left: CountrySummary;
  right: CountrySummary;
  onBack: () => void;
}) {
  const metrics = useMemo(() => {
    const m: {
      label: string;
      leftVal: number;
      rightVal: number;
      leftDisplay: string;
      rightDisplay: string;
      higher: "left" | "right" | "tie";
    }[] = [];

    const add = (
      label: string,
      l: number | null | undefined,
      r: number | null | undefined,
      fmt?: (n: number) => string,
    ) => {
      const lv = l ?? 0;
      const rv = r ?? 0;
      const format = fmt || ((n: number) => Math.round(n).toString());
      m.push({
        label,
        leftVal: lv,
        rightVal: rv,
        leftDisplay: l != null ? format(lv) : "—",
        rightDisplay: r != null ? format(rv) : "—",
        higher: lv > rv ? "left" : rv > lv ? "right" : "tie",
      });
    };

    add("AVG SCORE", left.avgScore, right.avgScore);
    add(
      "POSITIVE SENTIMENT",
      left.avgSentimentPositive,
      right.avgSentimentPositive,
      (n) => `${Math.round(n)}%`,
    );
    add(
      "ON-TIME PERFORMANCE",
      left.avgOnTime,
      right.avgOnTime,
      (n) => `${Math.round(n)}%`,
    );
    add("TOTAL ROUTES", left.totalRoutes, right.totalRoutes, (n) =>
      n.toLocaleString(),
    );
    add("TOTAL PASSENGERS", left.totalPax, right.totalPax, fmtM);

    return m;
  }, [left, right]);

  const leftWins = metrics.filter((m) => m.higher === "left").length;
  const rightWins = metrics.filter((m) => m.higher === "right").length;
  const winner =
    leftWins > rightWins ? left : rightWins > leftWins ? right : null;

  return (
    <div className="flex flex-col gap-8 max-w-4xl mx-auto w-full">
      <div className="flex items-center justify-between">
        <div>
          <span className="text-[10px] font-semibold tracking-wider text-zinc-500">
            HEAD TO HEAD
          </span>
          <h1 className="text-2xl font-semibold text-zinc-100">
            Country Comparison
          </h1>
        </div>
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-800 text-zinc-400 text-xs font-medium hover:bg-zinc-700 hover:text-zinc-200 transition-colors"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m12 19-7-7 7-7" />
            <path d="M19 12H5" />
          </svg>
          Back to Globe
        </button>
      </div>

      <div className="flex items-center justify-center gap-10">
        <div className="flex items-center gap-3 flex-1 justify-end">
          <div className="text-right">
            <h2 className="text-3xl font-bold text-zinc-100">{left.name}</h2>
            <p className="text-xs text-zinc-500">
              {left.airportCount} airports · {fmtM(left.totalPax)} pax
            </p>
          </div>
          <img
            src={getFlag(left.code)}
            alt={`${left.name} flag`}
            className="w-5 h-4 object-cover rounded-sm"
          />
        </div>
        <div className="px-4 py-2 rounded-lg bg-zinc-800">
          <span className="text-sm font-bold tracking-widest text-zinc-600">
            VS
          </span>
        </div>
        <div className="flex items-center gap-3 flex-1">
          <img
            src={getFlag(right.code)}
            alt={`${right.name} flag`}
            className="w-5 h-4 object-cover rounded-sm"
          />
          <div>
            <h2 className="text-3xl font-bold text-zinc-100">{right.name}</h2>
            <p className="text-xs text-zinc-500">
              {right.airportCount} airports · {fmtM(right.totalPax)} pax
            </p>
          </div>
        </div>
      </div>

      <div className="h-px bg-zinc-800" />

      {/* Comparison bars */}
      <div className="flex flex-col gap-6">
        {metrics.map((m) => {
          const total = m.leftVal + m.rightVal;
          const leftPct = total > 0 ? (m.leftVal / total) * 100 : 50;
          const rightPct = total > 0 ? (m.rightVal / total) * 100 : 50;
          const leftColor =
            m.higher === "left"
              ? "#22c55e"
              : m.higher === "tie"
                ? "#eab308"
                : "#eab308";
          const rightColor =
            m.higher === "right"
              ? "#22c55e"
              : m.higher === "tie"
                ? "#eab308"
                : "#eab308";

          return (
            <div key={m.label} className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span
                  className="text-lg font-bold"
                  style={{
                    color:
                      m.higher === "left"
                        ? "#22c55e"
                        : m.higher === "tie"
                          ? "#eab308"
                          : scoreHex(m.leftVal),
                  }}
                >
                  {m.leftDisplay}
                </span>
                <span className="text-[10px] font-semibold tracking-wider text-zinc-500">
                  {m.label}
                </span>
                <span
                  className="text-lg font-bold"
                  style={{
                    color:
                      m.higher === "right"
                        ? "#22c55e"
                        : m.higher === "tie"
                          ? "#eab308"
                          : scoreHex(m.rightVal),
                  }}
                >
                  {m.rightDisplay}
                </span>
              </div>
              <div className="flex gap-1 h-2 rounded-full overflow-hidden bg-zinc-900">
                <div
                  className="h-full rounded-full transition-all duration-500"
                  style={{ width: `${leftPct}%`, backgroundColor: leftColor }}
                />
                <div
                  className="h-full rounded-full transition-all duration-500"
                  style={{ width: `${rightPct}%`, backgroundColor: rightColor }}
                />
              </div>
            </div>
          );
        })}
      </div>

      <div className="h-px bg-zinc-800" />

      <div className="flex items-center justify-center gap-3 py-2">
        {winner ? (
          <>
            <img
              src={getFlag(winner.code)}
              alt={`${winner.name} flag`}
              className="w-5 h-4 object-cover rounded-sm"
            />

            <span className="text-base font-semibold text-zinc-100">
              {winner.name} wins {Math.max(leftWins, rightWins)} of{" "}
              {metrics.length} categories
            </span>
            <span className="text-[10px] font-bold tracking-wider text-green-500 bg-green-500/10 px-2.5 py-1 rounded">
              {Math.max(leftWins, rightWins) >= 4 ? "DOMINANT" : "WINNER"}
            </span>
          </>
        ) : (
          <span className="text-base font-semibold text-yellow-500">
            It's a tie!
          </span>
        )}
      </div>
    </div>
  );
}
