import { Link } from "@tanstack/react-router";
import { fmt } from "~/utils/format";
import { Airport } from "~/utils/types";
import { OwnershipBadge } from "~/components/ownership-badge";

export const Header = ({
  airport,
}: {
  airport: Airport;
}) => {
  return (
    <header className="flex flex-col gap-1">
      <span className="font-grotesk text-[100px] font-bold text-white/10 leading-none tracking-[8px]">
        {airport.iataCode}
      </span>
      <h1 className="font-grotesk text-[32px] font-bold text-zinc-100 tracking-wide">
        {airport.name}
      </h1>
      <p className="font-mono text-[13px] text-zinc-500 tracking-[1.5px] uppercase">
        {airport.city}, {airport.country?.name}
      </p>
      {airport.operator && (
        <div className="flex items-center gap-2 mt-0.5">
          <span className="font-mono text-[11px] text-zinc-600 tracking-wider uppercase">
            Operated by{" "}
            <Link
              to="/operators/$id"
              params={{ id: String(airport.operator.id) }}
              className="text-zinc-400 hover:text-zinc-200 transition-colors underline underline-offset-2 decoration-zinc-700"
            >
              {airport.operator.name}
            </Link>
          </span>
          {airport.operator.ownershipModel && (
            <OwnershipBadge model={airport.operator.ownershipModel} />
          )}
        </div>
      )}
      <div className="flex gap-3 mt-3 flex-wrap">
        {airport.openedYear && (
          <Badge label="Opened" value={String(airport.openedYear)} bright />
        )}
        {airport.icaoCode && <Badge label="ICAO" value={airport.icaoCode} />}
        {airport.elevationFt && (
          <Badge label="Elev" value={`${fmt(airport.elevationFt)} ft`} />
        )}
      </div>
    </header>
  );
};

function Badge({
  label,
  value,
  bright = false,
}: {
  label: string;
  value: string;
  bright?: boolean;
}) {
  return (
    <span className="inline-flex items-center gap-1.5 bg-white/3 px-2.5 py-1">
      <span className="font-grotesk text-[9px] font-bold text-zinc-600 tracking-wider uppercase">
        {label}
      </span>
      <span
        className={`font-mono text-xs font-bold ${bright ? "text-zinc-100" : "text-zinc-400"}`}
      >
        {value}
      </span>
    </span>
  );
}

