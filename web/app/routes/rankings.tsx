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
