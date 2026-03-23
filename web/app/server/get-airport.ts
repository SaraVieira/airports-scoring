import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores, reviewsRaw } from "../db/schema";
import { eq, sql, and, ne, isNotNull, gt } from "drizzle-orm";

export const getAirport = createServerFn({ method: "GET" })
  .inputValidator((iata: string) => iata.toUpperCase())
  .handler(async ({ data: iata }) => {
    const airport = await db.query.airports.findFirst({
      where: eq(airports.iataCode, iata),
      with: {
        operator: true,
        owner: true,
        country: true,
        runways: true,
        paxYearly: { orderBy: (p, { desc }) => [desc(p.year)] },
        operationalStats: {
          orderBy: (o, { desc }) => [desc(o.periodYear), desc(o.periodMonth)],
        },
        sentimentSnapshots: {
          orderBy: (s, { desc }) => [
            desc(s.snapshotYear),
            desc(s.snapshotQuarter),
          ],
        },
        scores: {
          where: eq(airportScores.isLatest, true),
          limit: 1,
        },
        routesOut: {
          with: { destination: true, destinationAirport: true },
          orderBy: () => [sql`flights_per_month DESC NULLS LAST`],
        },
        wikipediaSnapshots: {
          orderBy: (w, { desc }) => [desc(w.fetchedAt)],
          limit: 1,
        },
        carbonAccreditation: {
          orderBy: (c, { desc }) => [desc(c.reportYear)],
          limit: 1,
        },
        groundTransport: {
          orderBy: (g, { desc }) => [desc(g.fetchedAt)],
          limit: 1,
        },
        lounges: true,
        hubStatus: true,
        slugs: true,
      },
    });

    if (!airport) {
      throw new Error(`Airport ${iata} not found`);
    }

    // Query recent reviews (anonymous - no author)
    const recentReviews = await db
      .select({
        reviewDate: reviewsRaw.reviewDate,
        overallRating: reviewsRaw.overallRating,
        reviewText: reviewsRaw.reviewText,
        source: reviewsRaw.source,
      })
      .from(reviewsRaw)
      .where(
        and(
          eq(reviewsRaw.airportId, airport.id),
          isNotNull(reviewsRaw.reviewText),
          ne(reviewsRaw.reviewText, ""),
        ),
      )
      .orderBy(sql`review_date DESC`)
      .limit(5);

    // Query ranking
    const thisScore = airport.scores?.[0]?.scoreTotal;
    let ranking = { position: 0, total: 0 };
    if (thisScore) {
      const [rankResult] = await db
        .select({
          position: sql<number>`COUNT(*) + 1`.as("position"),
        })
        .from(airportScores)
        .where(
          and(
            eq(airportScores.isLatest, true),
            gt(airportScores.scoreTotal, thisScore),
          ),
        );
      const [totalResult] = await db
        .select({
          total: sql<number>`COUNT(*)`.as("total"),
        })
        .from(airportScores)
        .where(eq(airportScores.isLatest, true));
      ranking = {
        position: Number(rankResult?.position ?? 0),
        total: Number(totalResult?.total ?? 0),
      };
    }

    // Query Google aggregate rating (overall_rating is 1-10 for Skytrax, 1-5 for Google)
    const [googleAgg] = await db
      .select({
        googleRating: sql<number>`AVG(overall_rating::numeric)`.as(
          "google_rating",
        ),
        googleCount: sql<number>`COUNT(*)`.as("google_count"),
      })
      .from(reviewsRaw)
      .where(
        and(
          eq(reviewsRaw.airportId, airport.id),
          eq(reviewsRaw.source, "google"),
        ),
      );

    // Source breakdown from reviews_raw
    const sourceBreakdown = await db
      .select({
        source: reviewsRaw.source,
        count: sql<number>`COUNT(*)`.as("count"),
      })
      .from(reviewsRaw)
      .where(eq(reviewsRaw.airportId, airport.id))
      .groupBy(reviewsRaw.source);

    return {
      ...airport,
      recentReviews,
      ranking,
      googleAgg: {
        rating: googleAgg?.googleRating ? Number(googleAgg.googleRating) : null,
        count: googleAgg?.googleCount ? Number(googleAgg.googleCount) : 0,
      },
      sourceBreakdown: sourceBreakdown.map((s) => ({
        source: s.source,
        count: Number(s.count),
      })),
    };
  });
