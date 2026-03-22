import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { sql, ilike, or } from "drizzle-orm";

export const searchAirports = createServerFn({ method: "GET" })
  .inputValidator((query: string) => query)
  .handler(async ({ data: query }) => {
    if (!query || query.length < 1) return [];

    const pattern = `%${query}%`;
    const results = await db
      .select({
        iataCode: airports.iataCode,
        name: airports.name,
        city: airports.city,
        countryCode: airports.countryCode,
        scoreTotal: airportScores.scoreTotal,
      })
      .from(airports)
      .leftJoin(
        airportScores,
        sql`${airportScores.airportId} = ${airports.id} AND ${airportScores.isLatest} = true`
      )
      .where(
        or(
          ilike(airports.name, pattern),
          ilike(airports.iataCode, pattern),
          ilike(airports.city, pattern),
          ilike(airports.icaoCode, pattern)
        )
      )
      .orderBy(sql`${airportScores.scoreTotal} DESC NULLS LAST`)
      .limit(8);

    return results;
  });
