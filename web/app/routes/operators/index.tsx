import { createFileRoute, Link } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { scoreColor } from "~/utils/scoring";
import { fmtM } from "~/utils/format";
import { OwnershipBadge } from "~/components/ownership-badge";

const getOperators = createServerFn({ method: "GET" }).handler(async () => {
  return api.listOperators();
});

export const Route = createFileRoute("/operators/")({
  loader: () => getOperators(),
  component: OperatorsPage,
});

function OperatorsPage() {
  const operators = Route.useLoaderData();

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-16 flex flex-col">
        <section className="flex flex-col gap-4 py-12">
          <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
            Airport Operators
          </h2>
          <p className="font-mono text-xs text-zinc-600 italic">
            Who runs Europe's airports — and how well they score.
          </p>

          <div className="flex flex-col mt-4">
            {/* Header */}
            <div className="flex items-center gap-2 py-2">
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider flex-1">
                OPERATOR
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-16 text-center">
                MODEL
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-14 text-right">
                PUBLIC
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-12 text-right">
                APTS
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-16 text-right">
                PAX
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-14 text-right">
                DELAY
              </span>
              <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-14 text-right">
                SCORE
              </span>
            </div>
            <div className="h-px bg-zinc-800" />

            {operators.length === 0 && (
              <p className="font-mono text-xs text-zinc-600 py-8 text-center">
                No operators found. Run the operator seed script first.
              </p>
            )}
            {operators.map((op) => (
              <Link
                key={op.id}
                to="/operators/$id"
                params={{ id: String(op.id) }}
                className="group flex items-center gap-2 py-3 border-b border-white/3 hover:bg-white/3 transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <span className="font-grotesk text-sm font-bold text-[#f5f5f0] tracking-wider">
                    {op.shortName || op.name}
                  </span>
                  <span className="font-mono text-xs text-zinc-600 ml-2">
                    {op.countryCode}
                  </span>
                </div>
                <div className="w-16 flex justify-center">
                  <OwnershipBadge model={op.ownershipModel} />
                </div>
                <span className="font-mono text-xs text-zinc-500 w-14 text-right">
                  {op.publicSharePct != null
                    ? `${op.publicSharePct.toFixed(0)}%`
                    : "—"}
                </span>
                <span className="font-mono text-xs text-zinc-400 w-12 text-right">
                  {op.airportCount}
                </span>
                <span className="font-mono text-xs text-zinc-400 w-16 text-right">
                  {fmtM(op.totalPax)}
                </span>
                <span className="font-mono text-xs text-zinc-400 w-14 text-right">
                  {op.avgDelayPct != null
                    ? `${op.avgDelayPct.toFixed(1)}%`
                    : "—"}
                </span>
                <span
                  className={`font-grotesk text-sm font-bold tabular-nums w-14 text-right ${scoreColor(op.avgScore)}`}
                >
                  {op.avgScore != null ? Math.round(op.avgScore) : "—"}
                </span>
              </Link>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
