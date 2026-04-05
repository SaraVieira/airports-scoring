import { LivePulseAircraft } from "~/api/client";
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

export function AircraftDot({
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
