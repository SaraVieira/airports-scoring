import { Link } from "@tanstack/react-router";
import { cleanCity } from "~/utils/format";

export function DelayColumn({
  title,
  subtitle,
  color,
  airports,
}: {
  title: string;
  subtitle: string;
  color: "green" | "red" | "yellow";
  airports: {
    iataCode: string;
    name: string;
    city: string;
    countryCode: string;
    avgDelayPct: number;
  }[];
}) {
  const textColor =
    color === "green"
      ? "text-green-500"
      : color === "yellow"
        ? "text-yellow-500"
        : "text-red-500";
  const bgTint =
    color === "green"
      ? "bg-green-500/[0.04]"
      : color === "yellow"
        ? "bg-yellow-500/[0.04]"
        : "bg-red-500/[0.04]";
  const borderTint =
    color === "green"
      ? "border-green-500/10"
      : color === "yellow"
        ? "border-yellow-500/10"
        : "border-red-500/10";

  return (
    <div
      className={`flex-1 flex flex-col gap-4 p-6 ${bgTint} border ${borderTint}`}
    >
      <h3
        className={`font-grotesk text-[13px] font-bold ${textColor} tracking-[2px] uppercase`}
      >
        {title}
      </h3>
      <p className="font-mono text-[11px] text-zinc-600 italic">{subtitle}</p>
      <div className="flex flex-col">
        {airports.map((airport) => (
          <Link
            key={airport.iataCode}
            to="/airport/$iata"
            params={{ iata: airport.iataCode }}
            className="flex items-center justify-between py-3 border-b border-white/5 last:border-0 hover:bg-white/3 transition-colors -mx-2 px-2"
          >
            <div className="flex items-center gap-3">
              <span className="font-grotesk text-sm font-bold text-[#f5f5f0] tracking-wider">
                {airport.iataCode}
              </span>
              <span className="font-mono text-xs text-zinc-600">
                {cleanCity(airport.city)}
              </span>
            </div>
            <span
              className={`font-grotesk text-xl font-bold tabular-nums ${textColor}`}
            >
              {airport.avgDelayPct.toFixed(1)}%
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}
