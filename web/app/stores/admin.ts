import { create } from "zustand";
import { persist } from "zustand/middleware";
import {
  adminListAirports,
  adminListJobs,
  adminDataGaps,
  adminListOperators,
} from "~/server/admin";
import type { components } from "~/api/types";

type SupportedAirport = components["schemas"]["SupportedAirportWithStatus"];
type JobInfo = components["schemas"]["JobInfo"];

interface DataGap {
  iataCode: string;
  name: string;
  source: string;
  lastFetchedAt: string | null;
  lastStatus: string;
}

interface Operator {
  id: number;
  name: string;
  shortName: string | null;
  countryCode: string | null;
  orgType: string;
  ownershipModel: string | null;
  publicSharePct: number | null;
  notes: string | null;
  airportCount: number;
}

// ── Auth Store ──────────────────────────────────────────

interface AuthState {
  password: string | null;
  authenticated: boolean | null;
  setPassword: (password: string) => void;
  logout: () => void;
  verify: () => Promise<boolean>;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      password: null,
      authenticated: null,

      setPassword: (password: string) => {
        set({ password, authenticated: true });
      },

      logout: () => {
        set({ password: null, authenticated: false });
        // Clear other stores
        useAdminStore.getState().reset();
      },

      verify: async () => {
        const password = get().password;
        if (!password) {
          set({ authenticated: false });
          return false;
        }
        try {
          await adminListAirports({ data: password });
          set({ password, authenticated: true });
          return true;
        } catch {
          set({ password: null, authenticated: false });
          return false;
        }
      },
    }),
    {
      name: "admin-auth",
      partialize: (state) => ({ password: state.password }),
    },
  ),
);

// ── Admin Data Store ────────────────────────────────────

interface AdminState {
  airports: SupportedAirport[];
  jobs: JobInfo[];
  dataGaps: DataGap[];
  operators: Operator[];
  loading: boolean;
  lastFetched: number | null;

  fetchAll: () => Promise<void>;
  fetchAirports: () => Promise<void>;
  fetchJobs: () => Promise<void>;
  fetchDataGaps: () => Promise<void>;
  fetchOperators: () => Promise<void>;
  reset: () => void;
}

function getPassword(): string {
  return useAuthStore.getState().password || "";
}

export const useAdminStore = create<AdminState>()((set) => ({
  airports: [],
  jobs: [],
  dataGaps: [],
  operators: [],
  loading: false,
  lastFetched: null,

  fetchAll: async () => {
    const password = getPassword();
    if (!password) return;
    set({ loading: true });
    try {
      const [airports, jobs, dataGaps, operators] = await Promise.all([
        adminListAirports({ data: password }),
        adminListJobs({ data: password }),
        adminDataGaps({ data: password }),
        adminListOperators({ data: password }),
      ]);
      set({
        airports,
        jobs,
        dataGaps,
        operators,
        loading: false,
        lastFetched: Date.now(),
      });
    } catch (err) {
      console.error("Failed to fetch admin data", err);
      set({ loading: false });
    }
  },

  fetchAirports: async () => {
    const password = getPassword();
    if (!password) return;
    try {
      const airports = await adminListAirports({ data: password });
      set({ airports });
    } catch (err) {
      console.error("Failed to fetch airports", err);
    }
  },

  fetchJobs: async () => {
    const password = getPassword();
    if (!password) return;
    try {
      const jobs = await adminListJobs({ data: password });
      set({ jobs });
    } catch (err) {
      console.error("Failed to fetch jobs", err);
    }
  },

  fetchDataGaps: async () => {
    const password = getPassword();
    if (!password) return;
    try {
      const dataGaps = await adminDataGaps({ data: password });
      set({ dataGaps });
    } catch (err) {
      console.error("Failed to fetch data gaps", err);
    }
  },

  fetchOperators: async () => {
    const password = getPassword();
    if (!password) return;
    try {
      const operators = await adminListOperators({ data: password });
      set({ operators });
    } catch (err) {
      console.error("Failed to fetch operators", err);
    }
  },

  reset: () =>
    set({
      airports: [],
      jobs: [],
      dataGaps: [],
      operators: [],
      loading: false,
      lastFetched: null,
    }),
}));
