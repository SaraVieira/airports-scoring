import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";

export const getAirport = createServerFn({ method: "GET" })
  .inputValidator((iata: string) => iata.toUpperCase())
  .handler(async ({ data: iata }) => {
    const result = await api.getAirport(iata);
    return JSON.parse(JSON.stringify(result));
  });
