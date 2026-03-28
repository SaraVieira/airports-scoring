import { Link } from "@tanstack/react-router";

function cleanCity(city: string): string {
  const parts = city.split(", ");
  if (parts.length === 2 && parts[0] === parts[1]) return parts[0];
  return city;
}

function formatPax(pax: number): string {
  if (pax >= 1_000_000) return `${(pax / 1_000_000).toFixed(1)}M`;
  if (pax >= 1_000) return `${(pax / 1_000).toFixed(0)}K`;
  return String(pax);
}

export function BusiestColumn({
  airports,
}: {
  airports: {
    iataCode: string;
    name: string;
    city: string;
    year: number;
    totalPax: number;
  }[];
}) {
  const maxPax = airports[0]?.totalPax ?? 1;

  return (
    <div className="flex-1 flex flex-col gap-4 p-6 bg-blue-500/[0.04] border border-blue-500/10">
      <h3 className="font-grotesk text-[13px] font-bold text-blue-400 tracking-[2px] uppercase">
        Busiest Airports
      </h3>
      <p className="font-mono text-[11px] text-zinc-600 italic">
        By annual passenger count ({airports[0]?.year ?? ""})
      </p>
      <div className="flex flex-col">
        {airports.slice(0, 5).map((airport) => (
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
            <div className="flex items-center gap-2">
              <div className="w-16 h-1.5 bg-zinc-900 rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500/60 rounded-full"
                  style={{
                    width: `${(airport.totalPax / maxPax) * 100}%`,
                  }}
                />
              </div>
              <span className="font-grotesk text-sm font-bold tabular-nums text-blue-400 w-12 text-right">
                {formatPax(airport.totalPax)}
              </span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

export function BestReviewedColumn({
  airports,
}: {
  airports: {
    iataCode: string;
    name: string;
    city: string;
    avgRating: number;
    reviewCount: number;
  }[];
}) {
  return (
    <div className="flex-1 flex flex-col gap-4 p-6 bg-purple-500/[0.04] border border-purple-500/10">
      <h3 className="font-grotesk text-[13px] font-bold text-purple-400 tracking-[2px] uppercase">
        Best Reviewed
      </h3>
      <p className="font-mono text-[11px] text-zinc-600 italic">
        Highest average sentiment rating
      </p>
      <div className="flex flex-col">
        {airports.slice(0, 5).map((airport) => (
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
            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-zinc-600">
                {airport.reviewCount} reviews
              </span>
              <span className="font-grotesk text-xl font-bold tabular-nums text-purple-400">
                {airport.avgRating.toFixed(1)}
              </span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

export function ConnectivityColumns({
  airports,
}: {
  airports: {
    iataCode: string;
    name: string;
    city: string;
    routeCount: number;
  }[];
}) {
  const mostConnected = airports.slice(0, 5);
  const leastConnected = [...airports].reverse().slice(0, 5);

  return (
    <>
      <div className="flex-1 flex flex-col gap-4 p-6 bg-cyan-500/[0.04] border border-cyan-500/10">
        <h3 className="font-grotesk text-[13px] font-bold text-cyan-400 tracking-[2px] uppercase">
          Most Connected
        </h3>
        <p className="font-mono text-[11px] text-zinc-600 italic">
          Airports with the most destinations
        </p>
        <div className="flex flex-col">
          {mostConnected.map((airport) => (
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
              <span className="font-grotesk text-xl font-bold tabular-nums text-cyan-400">
                {airport.routeCount}
              </span>
            </Link>
          ))}
        </div>
      </div>
      <div className="flex-1 flex flex-col gap-4 p-6 bg-orange-500/[0.04] border border-orange-500/10">
        <h3 className="font-grotesk text-[13px] font-bold text-orange-400 tracking-[2px] uppercase">
          Island Airports
        </h3>
        <p className="font-mono text-[11px] text-zinc-600 italic">
          Fewest destinations — limited connectivity
        </p>
        <div className="flex flex-col">
          {leastConnected.map((airport) => (
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
              <span className="font-grotesk text-xl font-bold tabular-nums text-orange-400">
                {airport.routeCount}
              </span>
            </Link>
          ))}
        </div>
      </div>
    </>
  );
}
