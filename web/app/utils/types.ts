export type Airport = Awaited<
  ReturnType<(typeof import("../routes/airport.$iata"))["getAirport"]>
>;

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

export type SentimentSnapshot =
  (typeof import("../db/schema"))["sentimentSnapshots"]["$inferSelect"];

export type OpsRow =
  (typeof import("../db/schema"))["operationalStats"]["$inferSelect"];

export type PaxData = { year: number; pax: number | null };

export type Review = {
  reviewDate: string | null;
  overallRating: number | null;
  reviewText: string | null;
  source: string;
};

export type TimelineEvent = {
  year: number;
  label: string;
  detail?: string;
  color: string;
};

export type AciAwards = Record<string, Record<string, string>>;
export type SkytraxHistory = Record<string, number>;
