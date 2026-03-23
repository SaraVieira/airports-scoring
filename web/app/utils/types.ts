import type { components } from "~/api/types";

export type Airport = components["schemas"]["AirportDetailResponse"];
export type RouteRow = components["schemas"]["RouteResponse"];
export type SentimentSnapshot = components["schemas"]["SentimentSnapshotResponse"];
export type OpsRow = components["schemas"]["OperationalStatResponse"];
export type PaxData = { year: number; pax: number | null };
export type Review = components["schemas"]["RecentReviewResponse"];
export type TimelineEvent = {
  year: number;
  label: string;
  detail?: string;
  color: string;
};
export type AciAwards = Record<string, Record<string, string>>;
export type SkytraxHistory = Record<string, number>;
