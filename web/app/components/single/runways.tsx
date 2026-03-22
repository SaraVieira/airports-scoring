import { Airport } from "~/utils/types";
import { HeaderText } from "./header-text";
import { fmt } from "~/utils/format";

export const Runways = ({ airport }: { airport: Airport }) => {
  return (
    <section className="flex flex-col gap-4">
      <HeaderText>The Runway Report</HeaderText>
      <span className="font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase">
        {airport.runways.length} Runway
        {airport.runways.length !== 1 ? "s" : ""}
      </span>
      <div className="flex gap-6">
        {airport.runways.map((rw) => (
          <div
            key={rw.id}
            className="flex-1 flex flex-col gap-2 p-5 bg-[#111113] border border-zinc-800"
          >
            <span className="font-grotesk text-lg font-bold text-zinc-100 tracking-wider">
              {rw.leIdent && rw.heIdent
                ? `${rw.leIdent}/${rw.heIdent}`
                : (rw.ident ?? `Runway ${rw.id}`)}
            </span>
            <span className="font-mono text-[11px] text-zinc-500">
              {[
                rw.lengthFt ? `${fmt(rw.lengthFt)}ft` : null,
                rw.widthFt ? `${rw.widthFt}ft` : null,
              ]
                .filter(Boolean)
                .join(" × ")}
              {rw.surface ? ` · ${rw.surface}` : ""}
              {rw.lighted ? " · Lighted" : ""}
              {rw.closed ? " · CLOSED" : ""}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
};
