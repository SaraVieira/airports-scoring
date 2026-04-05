import { useState } from "react";
import type { LivePulseResponse, LivePulseAircraft } from "~/api/client";
import { AircraftDot, AircraftTooltip } from "./aircarft-dot";
import { CENTER, RADAR_RADIUS, RADAR_SIZE, RADIUS_KM, RINGS } from "./const";

interface HoverState {
  aircraft: LivePulseAircraft;
  x: number;
  y: number;
}

export function RadarScope({ data }: { data: LivePulseResponse }) {
  const [hovered, setHovered] = useState<HoverState | null>(null);

  return (
    <div className="relative shrink-0" style={{ width: RADAR_SIZE, height: RADAR_SIZE }}>
      <svg
        width={RADAR_SIZE}
        height={RADAR_SIZE}
        viewBox={`0 0 ${RADAR_SIZE} ${RADAR_SIZE}`}
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

        <circle cx={CENTER} cy={CENTER} r={3} fill="#fff" opacity={0.9} />
        <circle
          cx={CENTER}
          cy={CENTER}
          r={6}
          fill="none"
          stroke="#fff"
          strokeWidth={0.5}
          opacity={0.3}
        />

        {data.aircraft.map((ac) => (
          <AircraftDot
            key={ac.icao24}
            aircraft={ac}
            airportLat={data.airportLat}
            airportLon={data.airportLon}
            onHover={(aircraft, x, y) => setHovered({ aircraft, x, y })}
            onLeave={() => setHovered(null)}
          />
        ))}

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

      {hovered && (
        <AircraftTooltip
          aircraft={hovered.aircraft}
          x={hovered.x}
          y={hovered.y}
        />
      )}
    </div>
  );
}
