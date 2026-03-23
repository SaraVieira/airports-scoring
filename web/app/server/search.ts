import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";

export const searchAirports = createServerFn({ method: "GET" })
  .inputValidator((query: string) => query)
  .handler(async ({ data: query }) => {
    if (query.length < 1) return [];
    return api.searchAirports(query);
  });
