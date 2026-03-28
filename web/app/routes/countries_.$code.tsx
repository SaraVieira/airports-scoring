import { createFileRoute, Link } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { scoreColor } from "~/utils/scoring";

const getCountryData = createServerFn({ method: "GET" })
  .inputValidator((code: string) => code)
  .handler(async ({ data: code }) => {
    const [countries, airports] = await Promise.all([
      api.listCountries(),
      api.getCountryAirports(code.toUpperCase()),
    ]);
    const country = countries.find(
      (c) => c.code.toUpperCase() === code.toUpperCase(),
    );
    return { country: country ?? null, airports };
  });

export const Route = createFileRoute("/countries_/$code")({
  loader: ({ params }) => getCountryData({ data: params.code }),
  head: ({ loaderData }) => {
    const name = loaderData?.country?.name ?? loaderData?.airports?.[0]?.countryCode ?? "Country";
    const count = loaderData?.airports?.length ?? 0;
    const title = `${name} Airports — airports.report`;
    const description = `${count} airport${count !== 1 ? "s" : ""} tracked in ${name}. Scores, delays, passenger data, and sentiment rankings.`;
    return {
      meta: [
        { title },
        { name: "description", content: description },
        { property: "og:title", content: title },
        { property: "og:description", content: description },
        { property: "og:url", content: `https://airports.report/countries/${loaderData?.country?.code}` },
      ],
      links: [
        { rel: "canonical", href: `https://airports.report/countries/${loaderData?.country?.code}` },
      ],
    };
  },
  component: CountryDetailPage,
});

function CountryDetailPage() {
  const { country, airports } = Route.useLoaderData();

  const avgScore =
    airports.length > 0
      ? airports.reduce((sum, a) => sum + (a.scoreTotal ?? 0), 0) /
        airports.filter((a) => a.scoreTotal != null).length
      : null;

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-16 flex flex-col">
        {/* Breadcrumb */}
        <div className="flex items-center gap-2 mb-6">
          <Link
            to="/countries"
            className="font-mono text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Countries
          </Link>
          <span className="font-mono text-xs text-zinc-700">/</span>
          <span className="font-mono text-xs text-zinc-400">
            {country?.name ?? airports[0]?.countryCode}
          </span>
        </div>

        {/* Header */}
        <div className="flex items-start justify-between mb-8">
          <div>
            <h1 className="font-grotesk text-3xl font-bold text-[#f5f5f0]">
              {country?.name ?? "Country"}
            </h1>
            <p className="font-mono text-sm text-zinc-500 mt-1">
              {airports.length} airport{airports.length !== 1 ? "s" : ""}{" "}
              tracked
              {country?.totalPax != null && country.totalPax > 0 && (
                <> · {(country.totalPax / 1_000_000).toFixed(1)}M total passengers</>
              )}
            </p>
          </div>
          {avgScore != null && !isNaN(avgScore) && (
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

        {/* Stats */}
        {country && (
          <div className="flex gap-8 mb-10 flex-wrap">
            {country.avgSentimentPositive != null && (
              <div>
                <span className="font-grotesk text-2xl font-bold text-zinc-100">
                  {Math.round(country.avgSentimentPositive)}%
                </span>
                <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-0.5">
                  Positive Sentiment
                </p>
              </div>
            )}
            {country.avgOnTime != null && (
              <div>
                <span className="font-grotesk text-2xl font-bold text-zinc-100">
                  {country.avgOnTime.toFixed(1)}%
                </span>
                <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-0.5">
                  On-Time
                </p>
              </div>
            )}
            {country.totalRoutes != null && country.totalRoutes > 0 && (
              <div>
                <span className="font-grotesk text-2xl font-bold text-zinc-100">
                  {country.totalRoutes.toLocaleString()}
                </span>
                <p className="font-mono text-[10px] text-zinc-600 uppercase tracking-wider mt-0.5">
                  Routes
                </p>
              </div>
            )}
          </div>
        )}

        {/* Airport table */}
        <div className="flex flex-col">
          <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase mb-4">
            Airports
          </h2>

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
            <span className="font-grotesk text-[10px] font-bold text-zinc-700 tracking-wider w-14 text-right">
              SCORE
            </span>
          </div>
          <div className="h-px bg-zinc-800" />

          {airports.map((airport, i) => {
            const score = airport.scoreTotal ?? null;
            return (
              <Link
                key={airport.iataCode}
                to="/airport/$iata"
                params={{ iata: airport.iataCode }}
                className="group flex items-center gap-2 py-3 border-b border-white/3 hover:bg-white/3 transition-colors"
              >
                <span className="font-mono text-xs text-zinc-600 w-6 shrink-0">
                  {i + 1}
                </span>
                <span className="font-grotesk text-sm font-bold text-[#f5f5f0] tracking-wider w-12 shrink-0">
                  {airport.iataCode}
                </span>
                <span className="font-mono text-xs text-zinc-400 flex-1 truncate group-hover:text-zinc-300 transition-colors">
                  {airport.name}
                </span>
                <span
                  className={`font-grotesk text-sm font-bold tabular-nums w-14 text-right ${scoreColor(score)}`}
                >
                  {score != null ? Math.round(score) : "—"}
                </span>
              </Link>
            );
          })}

          {airports.length === 0 && (
            <p className="font-mono text-xs text-zinc-600 py-8 text-center">
              No airports tracked in this country yet.
            </p>
          )}
        </div>

        {/* Cross-links */}
        <div className="flex gap-6 mt-10 pt-6 border-t border-zinc-800">
          <Link
            to="/rankings"
            className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider hover:text-zinc-300 transition-colors"
          >
            FULL RANKINGS →
          </Link>
          <Link
            to="/operators"
            className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider hover:text-zinc-300 transition-colors"
          >
            ALL OPERATORS →
          </Link>
          <Link
            to="/countries"
            className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider hover:text-zinc-300 transition-colors"
          >
            ALL COUNTRIES →
          </Link>
        </div>
      </div>
    </div>
  );
}
