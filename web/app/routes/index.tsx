import { useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { Hero } from "~/components/home/hero";
import { AirportMap } from "~/components/home/airport-map";
import { WallColumn } from "~/components/home/wall-column";
import { DelayColumn } from "~/components/home/delay-column";
import {
  BusiestColumn,
  BestReviewedColumn,
  ConnectivityColumns,
} from "~/components/home/stat-column";

const getHomepageData = createServerFn({ method: "GET" }).handler(async () => {
  const [rankings, delays, busiest, bestReviewed, connected, mapData] =
    await Promise.all([
      api.getRankings().catch(() => []),
      api.getDelayRankings().catch(() => []),
      api.getBusiest().catch(() => []),
      api.getBestReviewed().catch(() => []),
      api.getMostConnected().catch(() => []),
      api.getMapAirports().catch(() => []),
    ]);
  return { rankings, delays, busiest, bestReviewed, connected, mapData };
});

export const Route = createFileRoute("/")({
  loader: () => getHomepageData(),
  component: Home,
});

function Home() {
  const { rankings: ranked, delays, busiest, bestReviewed, connected, mapData } =
    Route.useLoaderData();

  const { mostImproved, wallOfShame } = useMemo(() => {
    const filtered = ranked.filter((a) => a.scoreSentimentVelocity != null);
    filtered.sort(
      (a, b) =>
        (b.scoreSentimentVelocity ?? 0) - (a.scoreSentimentVelocity ?? 0),
    );
    return {
      mostImproved: filtered.slice(0, 3),
      wallOfShame: filtered.slice(-3).reverse(),
    };
  }, [ranked]);

  const { mostDelayed, leastDelayed } = useMemo(() => {
    if (delays.length === 0) return { mostDelayed: [], leastDelayed: [] };
    return {
      mostDelayed: delays.slice(0, 5),
      leastDelayed: [...delays].reverse().slice(0, 5),
    };
  }, [delays]);

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 flex flex-col">
        <Hero scored={ranked.length} />

        {/* Map */}
        {mapData.length > 0 && (
          <>
            <div className="w-full h-px bg-zinc-800" />
            <AirportMap airports={mapData} />
          </>
        )}

        {/* Sentiment: Most Improved / Wall of Shame */}
        {mostImproved.length > 0 && (
          <>
            <div className="w-full h-px bg-zinc-800" />
            <section className="flex gap-16 py-16">
              <WallColumn
                title="MOST IMPROVED"
                subtitle="Sentiment velocity — who's actually getting better"
                color="green"
                airports={mostImproved}
                field="scoreSentimentVelocity"
              />
              <WallColumn
                title="WALL OF SHAME"
                subtitle="Getting worse — and passengers know it"
                color="red"
                airports={wallOfShame}
                field="scoreSentimentVelocity"
              />
            </section>
          </>
        )}

        {/* Delays: Tardiest / Punctuality Kings */}
        {mostDelayed.length > 0 && (
          <>
            <div className="w-full h-px bg-zinc-800" />
            <section className="flex gap-16 py-16">
              <DelayColumn
                title="TARDIEST AIRPORTS"
                subtitle="Highest average flight delay percentage (last 12 months)"
                color="red"
                airports={mostDelayed}
              />
              <DelayColumn
                title="PUNCTUALITY KINGS"
                subtitle="Lowest average flight delay percentage (last 12 months)"
                color="green"
                airports={leastDelayed}
              />
            </section>
          </>
        )}

        {/* Busiest / Best Reviewed */}
        {(busiest.length > 0 || bestReviewed.length > 0) && (
          <>
            <div className="w-full h-px bg-zinc-800" />
            <section className="flex gap-16 py-16">
              {busiest.length > 0 && <BusiestColumn airports={busiest} />}
              {bestReviewed.length > 0 && (
                <BestReviewedColumn airports={bestReviewed} />
              )}
            </section>
          </>
        )}

        {/* Connectivity: Most Connected / Island Airports */}
        {connected.length > 0 && (
          <>
            <div className="w-full h-px bg-zinc-800" />
            <section className="flex gap-16 py-16">
              <ConnectivityColumns airports={connected} />
            </section>
          </>
        )}
      </div>
    </div>
  );
}
