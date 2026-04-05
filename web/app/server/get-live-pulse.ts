import { createServerFn } from "@tanstack/react-start";
import { api } from "~/api/client";

export const getLivePulse = createServerFn({ method: "GET" })
  .inputValidator((iata: string) => iata.toUpperCase())
  .handler(async ({ data: iata }) => {
    try {
      const result = await api.getLivePulse(iata);
      return JSON.parse(JSON.stringify(result));
    } catch {
      return null;
    }
  });
