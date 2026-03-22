import { AciAwards, Airport } from "~/utils/types";
import { HeaderText } from "../header-text";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { BackstoryTimeline } from "./timeline";

export const Backstory = ({ airport }: { airport: Airport }) => {
  const { wiki } = useSingleAirport({ airport });
  if (!wiki) return null;

  return (
    <section className="flex flex-col gap-4">
      <HeaderText>The Backstory</HeaderText>

      <BackstoryTimeline airport={airport} wiki={wiki} />

      <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4">
        ACI Service Quality Awards
      </span>
      {wiki.aciAwards &&
      typeof wiki.aciAwards === "object" &&
      Object.keys(wiki.aciAwards).length > 0 ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1">
          {Object.entries(wiki.aciAwards as AciAwards)
            .sort(([a], [b]) => b.localeCompare(a))
            .map(([year, placements]) => {
              const entries = Object.entries(placements);
              const place = entries[0]?.[0] ?? "";
              const category = entries[0]?.[1] ?? "";
              const medal =
                place === "1st"
                  ? "🥇"
                  : place === "2nd"
                    ? "🥈"
                    : place === "3rd"
                      ? "🥉"
                      : "🏆";
              return (
                <div
                  key={year}
                  className="border border-zinc-800 rounded px-3 py-2 flex flex-col gap-0.5"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-xs font-bold text-zinc-300">
                      {year}
                    </span>
                    <span className="text-sm">{medal}</span>
                  </div>
                  <span className="font-mono text-[10px] text-zinc-500 uppercase tracking-wide">
                    {place}
                  </span>
                  <span className="font-mono text-[10px] text-zinc-600 leading-tight">
                    {category}
                  </span>
                </div>
              );
            })}
        </div>
      ) : (
        <p className="font-mono text-xs text-zinc-500 italic mt-1">
          None recorded. A clean record — in the worst sense.
        </p>
      )}
    </section>
  );
};
