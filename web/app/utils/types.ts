export type RouteRow =
  (typeof import("../db/schema"))["routes"]["$inferSelect"] & {
    destination?: Record<string, unknown> | null;
    destinationAirport?: {
      name: string;
      iata: string | null;
      icao: string;
      city: string;
      country: string;
    } | null;
  };
