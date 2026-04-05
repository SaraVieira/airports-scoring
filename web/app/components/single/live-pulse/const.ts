export const RADAR_SIZE = 280;
export const RADAR_RADIUS = RADAR_SIZE / 2 - 16;
export const CENTER = RADAR_SIZE / 2;
export const RADIUS_KM = 50;

// Range ring distances in km
export const RINGS = [15, 30, 50];
export const POLL_INTERVAL = 30_000;
export const COLOR_LABELS = [
  { color: "bg-green-400", label: "arriving" },
  { color: "bg-yellow-400", label: "departing" },
  { color: "bg-zinc-500", label: "cruising" },
];
