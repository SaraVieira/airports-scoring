import { createFileRoute, Link } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { scoreColor } from "~/utils/scoring";
import { fmtM } from "~/utils/format";
import { OwnershipBadge } from "~/components/ownership-badge";

const getOperator = createServerFn({ method: "GET" })
  .inputValidator((id: string) => id)
  .handler(async ({ data: id }) => {
    return api.getOperator(Number(id));
  });

export const Route = createFileRoute("/operators/$id")({
  loader: ({ params }) => getOperator({ data: params.id }),
  component: OperatorDetailPage,
});

function OperatorDetailPage() {
  const op = Route.useLoaderData();

  const totalPax = op.airports.reduce(
    (sum, a) => sum + (a.latestPax ?? 0),
    0,
  );
  const scoredAirports = op.airports.filter((a) => a.scoreTotal != null);
  const avgScore =
    scoredAirports.length > 0
      ? scoredAirports.reduce((sum, a) => sum + (a.scoreTotal ?? 0), 0) /
        scoredAirports.length
      : null;

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-16 flex flex-col">
        {/* Breadcrumb */}
        <div className="flex items-center gap-2 mb-6">
          <Link
            to="/operators"
            className="font-mono text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Operators
          </Link>
          <span className="font-mono text-xs text-zinc-700">/</span>
          <span className="font-mono text-xs text-zinc-400">
            {op.shortName || op.name}
          </span>
        </div>

        {/* Header */}
        <div className="flex items-start justify-between mb-8">
          <div>
            <h1 className="font-grotesk text-3xl font-bold text-[#f5f5f0]">
              {op.name}
            </h1>
            <div className="flex items-center gap-3 mt-2">
              {op.countryCode && (
                <span className="font-mono text-sm text-zinc-500">
                  {op.countryCode}
                </span>
              )}
              <OwnershipBadge model={op.ownershipModel} />
              {op.publicSharePct != null && (
                <span className="font-mono text-sm text-zinc-500">
                  {op.publicSharePct.toFixed(0)}% public
                </span>
              )}
            </div>
          </div>
          {avgScore != null && (
            <div className="text-right">
              <span
                className={`font-grotesk text-4xl font-bold ${scoreColor(avgScore)}`}
              >
                {Math.round(avgScore)}
              </span>
              <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-1">
                Avg Score
              </p>
            </div>
          )}
        </div>

        {/* Notes */}
        {op.notes && (
          <p className="font-mono text-xs text-zinc-500 leading-relaxed mb-8 max-w-2xl">
            {op.notes}
          </p>
        )}

        {/* Stats row */}
        <div className="flex gap-8 mb-10">
          <div>
            <span className="font-grotesk text-2xl font-bold text-zinc-100">
              {op.airports.length}
            </span>
            <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-0.5">
              Airports
            </p>
          </div>
          {totalPax > 0 && (
            <div>
              <span className="font-grotesk text-2xl font-bold text-zinc-100">
                {fmtM(totalPax)}
              </span>
              <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-0.5">
                Total Passengers
              </p>
            </div>
          )}
        </div>

        {/* Airport table */}
        <div className="flex flex-col">
          <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase mb-4">
            Airports
          </h2>

          <div className="flex items-center gap-2 py-2">
            <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-12 shrink-0">
              IATA
            </span>
            <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider flex-1">
              AIRPORT
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

          {op.airports.map((airport) => (
            <Link
              key={airport.iataCode}
              to="/airport/$iata"
              params={{ iata: airport.iataCode }}
              className="group flex items-center gap-2 py-3 border-b border-white/3 hover:bg-white/3 transition-colors"
            >
              <span className="font-grotesk text-sm font-bold text-[#f5f5f0] tracking-wider w-12 shrink-0">
                {airport.iataCode}
              </span>
              <div className="flex-1 min-w-0">
                <span className="font-mono text-xs text-zinc-400 group-hover:text-zinc-300 transition-colors truncate">
                  {airport.name}
                </span>
                <span className="font-mono text-xs text-zinc-600 ml-2">
                  {airport.city}, {airport.countryCode}
                </span>
              </div>
              <span className="font-mono text-xs text-zinc-400 w-16 text-right">
                {fmtM(airport.latestPax)}
              </span>
              <span className="font-mono text-xs text-zinc-400 w-14 text-right">
                {airport.avgDelayPct != null
                  ? `${airport.avgDelayPct.toFixed(1)}%`
                  : "—"}
              </span>
              <span
                className={`font-grotesk text-sm font-bold tabular-nums w-14 text-right ${scoreColor(airport.scoreTotal)}`}
              >
                {airport.scoreTotal != null
                  ? Math.round(airport.scoreTotal)
                  : "—"}
              </span>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
