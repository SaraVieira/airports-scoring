import { AirportSearch } from "../airport-search";

export const Hero = ({ scored }: { scored: number }) => {
  return (
    <section className="flex flex-col items-center gap-5 pt-32 pb-20">
      <h1 className="font-grotesk text-[48px] font-bold text-[#f5f5f0] tracking-[2px]">
        Airport Intelligence
      </h1>
      <p className="font-mono text-sm text-zinc-600 italic">
        Scoring Europe's airports so you don't have to.
      </p>
      <div className="mt-2">
        <AirportSearch />
      </div>
      <p className="font-mono text-[11px] text-zinc-500 tracking-wide">
        {scored} airports scored · 20+ years of history
      </p>
    </section>
  );
};
