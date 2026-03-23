import { useMemo } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { Hero } from "~/components/home/hero";
import { Rankings } from "~/components/home/rankings";
import { WallColumn } from "~/components/home/wall-column";

const getHomepageData = createServerFn({ method: "GET" }).handler(async () => {
  return api.listAirports();
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
      (a, b) => (b.scoreSentimentVelocity ?? 0) - (a.scoreSentimentVelocity ?? 0),
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
