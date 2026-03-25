import { createServerFn } from "@tanstack/react-start";

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3001";
const API_KEY = import.meta.env.VITE_API_KEY || "";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function adminFetch(
  path: string,
  password: string,
  options?: RequestInit,
): Promise<any> {
  const res = await fetch(`${API_URL}${path}`, {
    ...options,
    headers: {
      ...(API_KEY ? { "X-API-Key": API_KEY } : {}),
      "X-Admin-Password": password,
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });
  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`);
  }
  if (res.status === 204 || res.headers.get("content-length") === "0") {
    return null;
  }
  return res.json();
}

export const adminListAirports = createServerFn({ method: "GET" })
  .inputValidator((password: string) => password)
  .handler(async ({ data: password }) => {
    return adminFetch("/api/admin/supported-airports", password);
  });

export const adminDataGaps = createServerFn({ method: "GET" })
  .inputValidator((password: string) => password)
  .handler(async ({ data: password }) => {
    return adminFetch("/api/admin/data-gaps", password);
  });

export const adminListJobs = createServerFn({ method: "GET" })
  .inputValidator((password: string) => password)
  .handler(async ({ data: password }) => {
    return adminFetch("/api/admin/jobs", password);
  });

export const adminRefresh = createServerFn({ method: "POST" })
  .inputValidator((password: string) => password)
  .handler(async ({ data: password }) => {
    return adminFetch("/api/admin/refresh", password, { method: "POST" });
  });

export const adminTriggerScoring = createServerFn({ method: "POST" })
  .inputValidator((password: string) => password)
  .handler(async ({ data: password }) => {
    return adminFetch("/api/admin/score", password, { method: "POST" });
  });

export const adminStartJob = createServerFn({ method: "POST" })
  .inputValidator((d: { password: string; body: any }) => d)
  .handler(async ({ data: { password, body } }) => {
    return adminFetch("/api/admin/jobs", password, {
      method: "POST",
      body: JSON.stringify(body),
    });
  });

export const adminGetJob = createServerFn({ method: "GET" })
  .inputValidator((d: { password: string; id: string }) => d)
  .handler(async ({ data: { password, id } }) => {
    return adminFetch(`/api/admin/jobs/${id}`, password);
  });

export const adminCancelJob = createServerFn({ method: "POST" })
  .inputValidator((d: { password: string; id: string }) => d)
  .handler(async ({ data: { password, id } }) => {
    return adminFetch(`/api/admin/jobs/${id}/cancel`, password, {
      method: "POST",
    });
  });

export const adminCreateAirport = createServerFn({ method: "POST" })
  .inputValidator((d: { password: string; body: any }) => d)
  .handler(async ({ data: { password, body } }) => {
    return adminFetch("/api/admin/supported-airports", password, {
      method: "POST",
      body: JSON.stringify(body),
    });
  });

export const adminUpdateAirport = createServerFn({ method: "POST" })
  .inputValidator((d: { password: string; iata: string; body: any }) => d)
  .handler(async ({ data: { password, iata, body } }) => {
    return adminFetch(`/api/admin/supported-airports/${iata}`, password, {
      method: "PATCH",
      body: JSON.stringify(body),
    });
  });

export const adminDeleteAirport = createServerFn({ method: "POST" })
  .inputValidator((d: { password: string; iata: string }) => d)
  .handler(async ({ data: { password, iata } }) => {
    return adminFetch(`/api/admin/supported-airports/${iata}`, password, {
      method: "DELETE",
    });
  });

export const adminBatchImport = createServerFn({ method: "POST" })
  .inputValidator(
    (d: {
      password: string;
      body: { iata_codes: string[]; run_pipeline?: boolean; score?: boolean };
    }) => d,
  )
  .handler(async ({ data: { password, body } }) => {
    return adminFetch("/api/admin/batch-import", password, {
      method: "POST",
      body: JSON.stringify(body),
    });
  });
