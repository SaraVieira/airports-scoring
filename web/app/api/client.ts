import { useAuthStore } from "~/stores/admin";
import type { components } from "./types";

export type CountrySummary = components["schemas"]["CountrySummary"];

export interface LivePulseAircraft {
  icao24: string;
  callsign: string | null;
  lat: number;
  lon: number;
  altitude: number | null;
  velocity: number | null;
  heading: number | null;
  verticalRate: number | null;
  onGround: boolean;
  status: "arriving" | "departing" | "cruising" | "ground";
}

export interface LivePulseResponse {
  iata: string;
  airportLat: number;
  airportLon: number;
  timestamp: number;
  aircraft: LivePulseAircraft[];
  counts: {
    total: number;
    inAir: number;
    onGround: number;
    arriving: number;
    departing: number;
    cruising: number;
  };
  cached: boolean;
}

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3001";
const API_KEY = import.meta.env.VITE_API_KEY || "";

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    ...options,
    headers: {
      ...(API_KEY ? { "X-API-Key": API_KEY } : {}),
      ...options?.headers,
    },
  });
  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`);
  }
  if (res.status === 204 || res.headers.get("content-length") === "0") {
    return undefined as T;
  }
  return res.json();
}

async function adminFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const password =
    typeof window !== "undefined"
      ? useAuthStore.getState().password || ""
      : "";
  return apiFetch<T>(path, {
    ...options,
    headers: {
      "X-Admin-Password": password,
      ...options?.headers,
    },
  });
}

export const api = {
  getAirport: (iata: string) =>
    apiFetch<import("./types").components["schemas"]["AirportDetailResponse"]>(
      `/api/airports/${iata}`,
    ),
  searchAirports: (q: string) =>
    apiFetch<import("./types").components["schemas"]["SearchResult"][]>(
      `/api/airports/search?q=${encodeURIComponent(q)}`,
    ),
  listAirports: () =>
    apiFetch<import("./types").components["schemas"]["AirportListItem"][]>(
      `/api/airports`,
    ),
  getRankings: () =>
    apiFetch<import("./types").components["schemas"]["AirportListItem"][]>(
      `/api/airports/rankings`,
    ),
  getDelayRankings: () =>
    apiFetch<import("./types").components["schemas"]["DelayRankingItem"][]>(
      `/api/airports/delays`,
    ),
  getBusiest: () =>
    apiFetch<import("./types").components["schemas"]["BusiestAirportItem"][]>(
      `/api/airports/busiest`,
    ),
  getBestReviewed: () =>
    apiFetch<import("./types").components["schemas"]["BestReviewedItem"][]>(
      `/api/airports/best-reviewed`,
    ),
  getMostConnected: () =>
    apiFetch<import("./types").components["schemas"]["ConnectivityItem"][]>(
      `/api/airports/most-connected`,
    ),
  getMapAirports: () =>
    apiFetch<import("./types").components["schemas"]["MapAirportItem"][]>(
      `/api/airports/map`,
    ),
  listOperators: () =>
    apiFetch<import("./types").components["schemas"]["OperatorListItem"][]>(
      `/api/operators`,
    ),
  getOperator: (slug: string) =>
    apiFetch<import("./types").components["schemas"]["OperatorDetail"]>(
      `/api/operators/${slug}`,
    ),
  getCountryAirports: (code: string) =>
    apiFetch<import("./types").components["schemas"]["AirportListItem"][]>(
      `/api/countries/${code}/airports`,
    ),
  listCountries: () =>
    apiFetch<import("./types").components["schemas"]["CountrySummary"][]>(
      `/api/countries`,
    ),
  getLivePulse: (iata: string) =>
    apiFetch<LivePulseResponse>(`/api/airports/${iata}/live`),
  admin: {
    listSupportedAirports: () =>
      adminFetch<
        import("./types").components["schemas"]["SupportedAirportWithStatus"][]
      >(`/api/admin/supported-airports`),
    createSupportedAirport: (
      body: import("./types").components["schemas"]["CreateSupportedAirport"],
    ) =>
      adminFetch<
        import("./types").components["schemas"]["SupportedAirportWithStatus"]
      >(`/api/admin/supported-airports`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    updateSupportedAirport: (
      iata: string,
      body: import("./types").components["schemas"]["UpdateSupportedAirport"],
    ) =>
      adminFetch<
        import("./types").components["schemas"]["SupportedAirportWithStatus"]
      >(`/api/admin/supported-airports/${iata}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    deleteSupportedAirport: (iata: string) =>
      adminFetch<void>(`/api/admin/supported-airports/${iata}`, {
        method: "DELETE",
      }),
    refresh: () =>
      adminFetch<import("./types").components["schemas"]["JobInfo"]>(
        `/api/admin/refresh`,
        { method: "POST" },
      ),
    startJob: (
      body: import("./types").components["schemas"]["StartJobRequest"],
    ) =>
      adminFetch<import("./types").components["schemas"]["JobInfo"]>(
        `/api/admin/jobs`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        },
      ),
    listJobs: () =>
      adminFetch<import("./types").components["schemas"]["JobInfo"][]>(
        `/api/admin/jobs`,
      ),
    getJob: (id: string) =>
      adminFetch<import("./types").components["schemas"]["JobInfo"]>(
        `/api/admin/jobs/${id}`,
      ),
    cancelJob: (id: string) =>
      adminFetch<void>(`/api/admin/jobs/${id}/cancel`, { method: "POST" }),
    dataGaps: () =>
      adminFetch<import("./types").components["schemas"]["DataGapResponse"][]>(
        `/api/admin/data-gaps`,
      ),
    triggerScoring: () =>
      adminFetch<void>(`/api/admin/score`, { method: "POST" }),
  },
};
