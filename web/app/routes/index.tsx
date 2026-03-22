import { useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { eq, desc } from "drizzle-orm";
import { Hero } from "~/components/home/hero";
import { Rankings } from "~/components/home/rankings";
import { WallColumn } from "~/components/home/wall-column";

const getHomepageData = createServerFn({ method: "GET" }).handler(async () => {
  const ranked = await db
    .select({
      iataCode: airports.iataCode,
      name: airports.name,
      city: airports.city,
      countryCode: airports.countryCode,
      scoreTotal: airportScores.scoreTotal,
      scoreSentimentVelocity: airportScores.scoreSentimentVelocity,
    })
    .from(airportScores)
    .innerJoin(airports, eq(airportScores.airportId, airports.id))
    .where(eq(airportScores.isLatest, true))
    .orderBy(desc(airportScores.scoreTotal));

  return ranked;
});

export const Route = createFileRoute("/")({
  loader: () => getHomepageData(),
  component: Home,
});

function Home() {
  const ranked = Route.useLoaderData();

  const { mostImproved, wallOfShame } = useMemo(() => {
    const filtered = ranked.filter((a) => a.scoreSentimentVelocity != null);
    filtered.sort(
      (a, b) =>
        parseFloat(b.scoreSentimentVelocity!) -
        parseFloat(a.scoreSentimentVelocity!),
    );
    return {
      mostImproved: filtered.slice(0, 3),
      wallOfShame: filtered.slice(-3).reverse(),
    };
  }, [ranked]);

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 flex flex-col">
        <Hero scored={ranked.length} />
        <div className="w-full h-px bg-zinc-800" />
        <Rankings ranked={ranked} />
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
      </div>
    </div>
  );
}
