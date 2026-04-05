import { useEffect, useState, useCallback } from "react";
import { getLivePulse } from "~/server/get-live-pulse";
import type { LivePulseResponse, LivePulseAircraft } from "~/api/client";

const POLL_INTERVAL = 30_000;
const RADAR_SIZE = 280;
const RADAR_RADIUS = RADAR_SIZE / 2 - 16;
const CENTER = RADAR_SIZE / 2;
const RADIUS_KM = 50;

// Range ring distances in km
const RINGS = [15, 30, 50];

function toSvg(
  aircraft: LivePulseAircraft,
  airportLat: number,
  airportLon: number,
): { x: number; y: number } | null {
  const dx =
    (aircraft.lon - airportLon) *
    Math.cos((airportLat * Math.PI) / 180) *
    111;
  const dy = (aircraft.lat - airportLat) * 111;

  const svgX = CENTER + (dx / RADIUS_KM) * RADAR_RADIUS;
  const svgY = CENTER - (dy / RADIUS_KM) * RADAR_RADIUS;

  // Skip if outside the radar circle
  const dist = Math.sqrt((svgX - CENTER) ** 2 + (svgY - CENTER) ** 2);
  if (dist > RADAR_RADIUS + 4) return null;

  return { x: svgX, y: svgY };
}

function statusColor(status: string): string {
  switch (status) {
    case "arriving":
      return "#4ade80"; // green-400
    case "departing":
      return "#facc15"; // yellow-400
    case "ground":
      return "#52525b"; // zinc-600
    default:
      return "#71717a"; // zinc-500
  }
}

function AircraftDot({
  aircraft,
  airportLat,
  airportLon,
}: {
  aircraft: LivePulseAircraft;
  airportLat: number;
  airportLon: number;
}) {
  const pos = toSvg(aircraft, airportLat, airportLon);
  if (!pos) return null;

  const color = statusColor(aircraft.status);
  const heading = aircraft.heading ?? 0;

  // Small triangle pointing in heading direction
  if (aircraft.onGround) {
    return <circle cx={pos.x} cy={pos.y} r={2} fill={color} opacity={0.5} />;
  }

  return (
    <g transform={`translate(${pos.x}, ${pos.y}) rotate(${heading})`}>
      <polygon points="0,-4 2.5,3 -2.5,3" fill={color} opacity={0.85} />
    </g>
  );
}

function RadarScope({
  data,
}: {
  data: LivePulseResponse;
}) {
  return (
    <svg
      width={RADAR_SIZE}
      height={RADAR_SIZE}
      viewBox={`0 0 ${RADAR_SIZE} ${RADAR_SIZE}`}
      className="shrink-0"
    >
      {/* Background */}
      <circle
        cx={CENTER}
        cy={CENTER}
        r={RADAR_RADIUS}
        fill="oklch(0.13 0.005 285)"
        stroke="oklch(1 0 0 / 0.08)"
        strokeWidth={1}
      />

      {/* Range rings */}
      {RINGS.map((km) => (
        <circle
          key={km}
          cx={CENTER}
          cy={CENTER}
          r={(km / RADIUS_KM) * RADAR_RADIUS}
          fill="none"
          stroke="oklch(1 0 0 / 0.06)"
          strokeWidth={0.5}
          strokeDasharray="3 3"
        />
      ))}

      {/* Crosshairs */}
      <line
        x1={CENTER}
        y1={CENTER - RADAR_RADIUS}
        x2={CENTER}
        y2={CENTER + RADAR_RADIUS}
        stroke="oklch(1 0 0 / 0.04)"
        strokeWidth={0.5}
      />
      <line
        x1={CENTER - RADAR_RADIUS}
        y1={CENTER}
        x2={CENTER + RADAR_RADIUS}
        y2={CENTER}
        stroke="oklch(1 0 0 / 0.04)"
        strokeWidth={0.5}
      />

      {/* Airport center */}
      <circle cx={CENTER} cy={CENTER} r={3} fill="#fff" opacity={0.9} />
      <circle cx={CENTER} cy={CENTER} r={6} fill="none" stroke="#fff" strokeWidth={0.5} opacity={0.3} />

      {/* Aircraft */}
      {data.aircraft.map((ac) => (
        <AircraftDot
          key={ac.icao24}
          aircraft={ac}
          airportLat={data.airportLat}
          airportLon={data.airportLon}
        />
      ))}

      {/* Range labels */}
      {RINGS.map((km) => (
        <text
          key={`label-${km}`}
          x={CENTER + (km / RADIUS_KM) * RADAR_RADIUS + 2}
          y={CENTER - 2}
          fill="oklch(1 0 0 / 0.2)"
          fontSize={7}
          fontFamily="monospace"
        >
          {km}km
        </text>
      ))}
    </svg>
  );
}

function TimeSince({ timestamp }: { timestamp: number }) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

  const secs = Math.max(0, Math.floor((now - timestamp * 1000) / 1000));
  const label = secs < 60 ? `${secs}s ago` : `${Math.floor(secs / 60)}m ago`;

  return (
    <span className="text-[10px] text-muted-foreground font-mono">{label}</span>
  );
}

export function LivePulse({ iata }: { iata: string }) {
  const [data, setData] = useState<LivePulseResponse | null>(null);

  const fetchPulse = useCallback(async () => {
    const result = await getLivePulse({ data: iata });
    setData(result as LivePulseResponse | null);
  }, [iata]);

  useEffect(() => {
    fetchPulse();
    const interval = setInterval(fetchPulse, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [fetchPulse]);

  if (!data || data.counts.total === 0) return null;

  return (
    <div className="flex flex-col sm:flex-row items-center gap-6 py-6">
      <RadarScope data={data} />

      <div className="flex flex-col gap-3">
        {/* LIVE badge */}
        <div className="flex items-center gap-2">
          <span className="relative flex h-2.5 w-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
            <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-green-500" />
          </span>
          <span className="font-grotesk text-xs font-bold uppercase tracking-widest text-green-400">
            Live
          </span>
          <TimeSince timestamp={data.timestamp} />
        </div>

        {/* Main counts */}
        <div className="flex items-baseline gap-4">
          {data.counts.arriving > 0 && (
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-2xl font-bold text-green-400">
                {data.counts.arriving}
              </span>
              <span className="text-xs text-muted-foreground">arriving</span>
            </div>
          )}
          {data.counts.departing > 0 && (
            <div className="flex items-baseline gap-1.5">
              <span className="font-mono text-2xl font-bold text-yellow-400">
                {data.counts.departing}
              </span>
              <span className="text-xs text-muted-foreground">departing</span>
            </div>
          )}
        </div>

        {/* Secondary counts */}
        <div className="flex items-center gap-3 text-xs text-muted-foreground font-mono">
          <span>{data.counts.total} aircraft nearby</span>
          <span className="text-border">|</span>
          <span>{data.counts.inAir} in air</span>
          <span className="text-border">|</span>
          <span>{data.counts.onGround} on ground</span>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-3 mt-1">
          {[
            { color: "bg-green-400", label: "arriving" },
            { color: "bg-yellow-400", label: "departing" },
            { color: "bg-zinc-500", label: "cruising" },
          ].map((l) => (
            <span key={l.label} className="flex items-center gap-1">
              <span className={`inline-block w-1.5 h-1.5 rounded-full ${l.color}`} />
              <span className="text-[10px] text-muted-foreground">{l.label}</span>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
