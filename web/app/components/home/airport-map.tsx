import { Link } from "@tanstack/react-router";
import { useState, memo } from "react";
import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
} from "react-simple-maps";
import { scoreColor } from "~/utils/scoring";

interface MapAirport {
  iataCode: string;
  name: string;
  city: string;
  scoreTotal?: number | null;
  lat?: number | null;
  lng?: number | null;
}

const GEO_URL = "https://cdn.jsdelivr.net/npm/world-atlas@2/countries-50m.json";

function dotColor(score: number | null | undefined): string {
  if (score == null) return "#52525b";
  if (score >= 70) return "#22c55e";
  if (score >= 50) return "#eab308";
  if (score >= 30) return "#f97316";
  return "#ef4444";
}

const EuropeMap = memo(function EuropeMap({
  airports,
  hovered,
  setHovered,
}: {
  airports: MapAirport[];
  hovered: MapAirport | null;
  setHovered: (a: MapAirport | null) => void;
}) {
  return (
    <ComposableMap
      projection="geoAzimuthalEqualArea"
      projectionConfig={{
        rotate: [-15, -52, 0],
        scale: 750,
        center: [0, 0],
      }}
      width={900}
      height={500}
      style={{ width: "100%", height: "auto" }}
    >
      <Geographies geography={GEO_URL}>
        {({ geographies }) =>
          geographies.map((geo) => (
            <Geography
              key={geo.rsmKey}
              geography={geo}
              fill="#18181b"
              stroke="#27272a"
              strokeWidth={0.5}
              style={{
                default: { outline: "none" },
                hover: { outline: "none" },
                pressed: { outline: "none" },
              }}
            />
          ))
        }
      </Geographies>
      {airports
        .filter((a) => a.lat != null && a.lng != null)
        .map((a) => (
          <Marker key={a.iataCode} coordinates={[a.lng!, a.lat!]}>
            <Link to="/airport/$iata" params={{ iata: a.iataCode }}>
              <g
                onMouseEnter={() => setHovered(a)}
                onMouseLeave={() => setHovered(null)}
                className="cursor-pointer"
              >
                {/* Glow */}
                <circle
                  r={hovered?.iataCode === a.iataCode ? 10 : 6}
                  fill={dotColor(a.scoreTotal)}
                  opacity={0.2}
                />
                {/* Dot */}
                <circle
                  r={hovered?.iataCode === a.iataCode ? 4.5 : 3}
                  fill={dotColor(a.scoreTotal)}
                  className="transition-all duration-150"
                />
                {/* Label on hover */}
                {hovered?.iataCode === a.iataCode && (
                  <text
                    y={-10}
                    textAnchor="middle"
                    className="text-[10px] font-bold fill-zinc-200"
                    style={{ fontFamily: "Space Grotesk, sans-serif" }}
                  >
                    {a.iataCode}
                  </text>
                )}
              </g>
            </Link>
          </Marker>
        ))}
    </ComposableMap>
  );
});

export function AirportMap({ airports }: { airports: MapAirport[] }) {
  const [hovered, setHovered] = useState<MapAirport | null>(null);

  return (
    <section className="py-12 flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
            The Map
          </h2>
          <p className="font-mono text-xs text-zinc-600 italic mt-1">
            {airports.length} airports tracked. Colored by overall score.
          </p>
        </div>
        <Link
          to="/rankings"
          className="font-mono text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          View full rankings →
        </Link>
      </div>

      <div className="relative bg-zinc-900/30 border border-zinc-800/50 rounded-lg overflow-hidden">
        <EuropeMap
          airports={airports}
          hovered={hovered}
          setHovered={setHovered}
        />

        {/* Tooltip */}
        {hovered && (
          <div className="absolute bottom-4 left-4 bg-zinc-900/90 border border-zinc-700 rounded-md px-3 py-2 pointer-events-none">
            <div className="flex items-center gap-2">
              <span className="font-grotesk text-sm font-bold text-zinc-100">
                {hovered.iataCode}
              </span>
              <span className="font-mono text-xs text-zinc-500">
                {hovered.city}
              </span>
            </div>
            <div className="flex items-center gap-2 mt-0.5">
              <span className="font-mono text-xs text-zinc-500">Score:</span>
              <span
                className={`font-grotesk text-sm font-bold ${scoreColor(hovered.scoreTotal)}`}
              >
                {hovered.scoreTotal != null
                  ? Math.round(hovered.scoreTotal)
                  : "—"}
              </span>
            </div>
          </div>
        )}

        {/* Legend */}
        <div className="absolute top-4 right-4 flex items-center gap-3 bg-zinc-900/80 rounded px-3 py-1.5">
          {[
            { color: "#22c55e", label: "70+" },
            { color: "#eab308", label: "50-70" },
            { color: "#f97316", label: "30-50" },
            { color: "#ef4444", label: "<30" },
          ].map((l) => (
            <div key={l.label} className="flex items-center gap-1">
              <div
                className="w-2 h-2 rounded-full"
                style={{ backgroundColor: l.color }}
              />
              <span className="font-mono text-[10px] text-zinc-500">
                {l.label}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
