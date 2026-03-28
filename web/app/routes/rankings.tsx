import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";
import { Rankings } from "~/components/home/rankings";

const getRankingsData = createServerFn({ method: "GET" }).handler(async () => {
  try {
    return await api.getRankings();
  } catch (error) {
    console.error("Error fetching rankings:", error);
    return api.listAirports();
  }
});

export const Route = createFileRoute("/rankings")({
  loader: () => getRankingsData(),
  head: () => ({
    meta: [
      { title: "Airport Rankings — airports.report" },
      { name: "description", content: "All European airports ranked by composite score. Infrastructure, delays, passenger sentiment, connectivity, and more." },
      { property: "og:title", content: "Airport Rankings — airports.report" },
      { property: "og:description", content: "All European airports ranked by composite score." },
      { property: "og:url", content: "https://airports.report/rankings" },
    ],
    links: [{ rel: "canonical", href: "https://airports.report/rankings" }],
  }),
  component: RankingsPage,
});

function RankingsPage() {
  const ranked = Route.useLoaderData();

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-16 flex flex-col">
        <Rankings ranked={ranked} />
      </div>
    </div>
  );
}
