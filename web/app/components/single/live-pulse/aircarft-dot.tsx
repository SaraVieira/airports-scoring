import type { LivePulseAircraft } from "~/api/client";
import { CENTER, RADAR_RADIUS, RADIUS_KM } from "./const";

function toSvg(
  aircraft: LivePulseAircraft,
  airportLat: number,
  airportLon: number,
): { x: number; y: number } | null {
  const dx =
    (aircraft.lon - airportLon) * Math.cos((airportLat * Math.PI) / 180) * 111;
  const dy = (aircraft.lat - airportLat) * 111;

  const svgX = CENTER + (dx / RADIUS_KM) * RADAR_RADIUS;
  const svgY = CENTER - (dy / RADIUS_KM) * RADAR_RADIUS;

  const dist = Math.sqrt((svgX - CENTER) ** 2 + (svgY - CENTER) ** 2);
  if (dist > RADAR_RADIUS + 4) return null;

  return { x: svgX, y: svgY };
}

function statusColor(status: string): string {
  switch (status) {
    case "arriving":
      return "#4ade80";
    case "departing":
      return "#facc15";
    case "ground":
      return "#52525b";
    default:
      return "#71717a";
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case "arriving":
      return "↓ Arriving";
    case "departing":
      return "↑ Departing";
    case "ground":
      return "◼ On ground";
    default:
      return "— Cruising";
  }
}

function formatAlt(meters: number | null): string {
  if (meters == null) return "—";
  const ft = Math.round(meters * 3.28084);
  return `${ft.toLocaleString()} ft`;
}

function formatSpeed(ms: number | null): string {
  if (ms == null) return "—";
  const kts = Math.round(ms * 1.94384);
  return `${kts} kts`;
}

function formatVRate(ms: number | null): string {
  if (ms == null) return "";
  const fpm = Math.round(ms * 196.85);
  if (fpm > 0) return `+${fpm.toLocaleString()} ft/min`;
  return `${fpm.toLocaleString()} ft/min`;
}

export function AircraftDot({
  aircraft,
  airportLat,
  airportLon,
  onHover,
  onLeave,
}: {
  aircraft: LivePulseAircraft;
  airportLat: number;
  airportLon: number;
  onHover: (aircraft: LivePulseAircraft, x: number, y: number) => void;
  onLeave: () => void;
}) {
  const pos = toSvg(aircraft, airportLat, airportLon);
  if (!pos) return null;

  const color = statusColor(aircraft.status);
  const heading = aircraft.heading ?? 0;

  const handleMouseEnter = () => onHover(aircraft, pos.x, pos.y);

  if (aircraft.onGround) {
    return (
      <circle
        cx={pos.x}
        cy={pos.y}
        r={2}
        fill={color}
        opacity={0.5}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={onLeave}
        className="cursor-pointer"
      />
    );
  }

  return (
    <g
      transform={`translate(${pos.x}, ${pos.y}) rotate(${heading})`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={onLeave}
      className="cursor-pointer"
    >
      <polygon points="0,-4 2.5,3 -2.5,3" fill={color} opacity={0.85} />
      {/* Larger invisible hit area */}
      <circle r={8} fill="transparent" />
    </g>
  );
}

export function AircraftTooltip({
  aircraft,
  x,
  y,
}: {
  aircraft: LivePulseAircraft;
  x: number;
  y: number;
}) {
  const color = statusColor(aircraft.status);
  const vRate = formatVRate(aircraft.verticalRate);

  return (
    <div
      className="absolute pointer-events-none z-50"
      style={{
        left: x,
        top: y - 8,
        transform: "translate(-50%, -100%)",
      }}
    >
      <div className="bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1.5 shadow-lg whitespace-nowrap">
        <div className="flex items-center gap-2 mb-0.5">
          <span className="font-mono text-[11px] font-bold text-zinc-100">
            {aircraft.callsign || aircraft.icao24}
          </span>
          <span className="text-[10px]" style={{ color }}>
            {statusLabel(aircraft.status)}
          </span>
        </div>
        <div className="flex items-center gap-2 text-[10px] text-zinc-400 font-mono">
          <span>{formatAlt(aircraft.altitude)}</span>
          <span className="text-zinc-600">·</span>
          <span>{formatSpeed(aircraft.velocity)}</span>
          {vRate && (
            <>
              <span className="text-zinc-600">·</span>
              <span>{vRate}</span>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
