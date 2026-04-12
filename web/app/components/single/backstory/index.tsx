import { Airport } from "~/utils/types";
import { HeaderText } from "../header-text";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { BackstoryTimeline } from "./timeline";
import { Trophy } from "lucide-react";

function humanizeCategory(category: string): string {
  return category
    .replace(/_/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .replace("Worlds ", "World's ")
    .replace("Best Airport In ", "Best in ")
    .replace("Best Airport Staff In ", "Best Staff in ")
    .replace("Best Airport ", "Best ")
    .replace("Aci Asq", "ACI ASQ");
}

type Award = {
  source: string;
  year: number;
  category: string;
  region?: string | null;
  sizeBucket?: string | null;
  rank?: number | null;
};

function AwardCard({ award }: { award: Award }) {
  const medal =
    award.source === "aci_asq"
      ? award.rank === 1
        ? "🥇"
        : award.rank === 2
          ? "🥈"
          : award.rank === 3
            ? "🥉"
            : "🏆"
      : "🏆";

  const label =
    award.source === "aci_asq"
      ? `ACI ASQ${award.sizeBucket ? ` · ${award.sizeBucket.replace(/_/g, " ")}` : ""}${award.region ? ` · ${award.region.replace(/_/g, " ")}` : ""}`
      : humanizeCategory(award.category);

  return (
    <div className="border border-zinc-800 rounded px-3 py-2 flex flex-col gap-0.5">
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs font-bold text-zinc-300">
          {award.year}
        </span>
        <span className="text-sm">{medal}</span>
      </div>
      <span className="font-mono text-[10px] text-zinc-500 uppercase tracking-wide">
        {award.source === "aci_asq" ? "ACI ASQ" : "Skytrax"}
      </span>
      <span className="font-mono text-[10px] text-zinc-600 leading-tight">
        {label}
      </span>
    </div>
  );
}

export const Backstory = ({ airport }: { airport: Airport }) => {
  const { wiki } = useSingleAirport({ airport });
  if (!wiki) return null;

  const awards = airport.awards ?? [];
  const skytraxAwards = awards.filter((a) => a.source === "skytrax");
  const aciAwards = awards.filter((a) => a.source === "aci_asq");

  return (
    <section className="flex flex-col gap-4">
      <HeaderText>The Backstory</HeaderText>

      <BackstoryTimeline airport={airport} wiki={wiki} />

      {awards.length > 0 ? (
        <>
          {skytraxAwards.length > 0 && (
            <>
              <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4 flex items-center gap-1.5">
                <Trophy className="size-3" />
                Skytrax World Airport Awards
              </span>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1">
                {skytraxAwards.map((award) => (
                  <AwardCard
                    key={`${award.source}-${award.year}-${award.category}`}
                    award={award}
                  />
                ))}
              </div>
            </>
          )}

          {aciAwards.length > 0 && (
            <>
              <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4 flex items-center gap-1.5">
                <Trophy className="size-3" />
                ACI Service Quality Awards
              </span>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1">
                {aciAwards.map((award) => (
                  <AwardCard
                    key={`${award.source}-${award.year}-${award.category}-${award.region}-${award.sizeBucket}`}
                    award={award}
                  />
                ))}
              </div>
            </>
          )}
        </>
      ) : (
        <>
          <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4">
            Awards
          </span>
          <p className="font-mono text-xs text-zinc-500 italic mt-1">
            None recorded. A clean record — in the worst sense.
          </p>
        </>
      )}
    </section>
  );
};
