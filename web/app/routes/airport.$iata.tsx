import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { airports, airportScores, reviewsRaw } from "../db/schema";
import { eq, sql, and, ne, isNotNull, gt } from "drizzle-orm";
import { Header } from "~/components/single/header";
import { ScoreBar } from "~/components/single/score-bar";
import { RouteSection } from "~/components/single/routes";
import { SCORE_EXPLANATIONS } from "~/utils/constants";
import { Numbers } from "~/components/single/numbers";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { Divider } from "~/components/divider";
import { Verdict } from "~/components/single/verdict";
import { Sentiment } from "~/components/single/sentiment";
import { Tardiness } from "~/components/single/tardiness";
import { Runways } from "~/components/single/runways";
import { Backstory } from "~/components/single/backtsory";

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

export const Route = createFileRoute("/airport/$iata")({
  loader: ({ params }) => getAirport({ data: params.iata! }),
  component: AirportDetail,
});

function AirportDetail() {
  const airport = Route.useLoaderData();
  const { score } = useSingleAirport({ airport });

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-12 flex flex-col gap-9">
        <Divider />
        <Header airport={airport} />

        <Verdict airport={airport} />

        <div className="flex flex-col gap-3 pb-6">
          <ScoreBar
            label="Operational"
            score={score?.scoreOperational}
            weight="25%"
            explanation={SCORE_EXPLANATIONS["Operational"]}
          />
          <ScoreBar
            label="Sentiment"
            score={score?.scoreSentiment}
            weight="25%"
            explanation={SCORE_EXPLANATIONS["Sentiment"]}
          />
          <ScoreBar
            label="Infrastructure"
            score={score?.scoreInfrastructure}
            weight="15%"
            explanation={SCORE_EXPLANATIONS["Infrastructure"]}
          />
          <ScoreBar
            label="Sent. Velocity"
            score={score?.scoreSentimentVelocity}
            weight="15%"
            explanation={SCORE_EXPLANATIONS["Sent. Velocity"]}
          />
          <ScoreBar
            label="Connectivity"
            score={score?.scoreConnectivity}
            weight="10%"
            explanation={SCORE_EXPLANATIONS["Connectivity"]}
          />
          <ScoreBar
            label="Operator"
            score={score?.scoreOperator}
            weight="10%"
            explanation={SCORE_EXPLANATIONS["Operator"]}
          />
        </div>

        <Divider />
        <Sentiment airport={airport} />

        <Divider />

        <Numbers airport={airport} />
        <Divider />

        <Tardiness airport={airport} />
        <Divider />
        <RouteSection airport={airport} />
        <Divider />
        <Runways airport={airport} />
        <Divider />
        <Backstory airport={airport} />
        <Divider />
        <footer className="flex gap-6">
          {airport.wikipediaUrl && (
            <a
              href={airport.wikipediaUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              WIKIPEDIA ↗
            </a>
          )}
          {airport.websiteUrl && (
            <a
              href={airport.websiteUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              WEBSITE ↗
            </a>
          )}
          {airport.skytraxUrl && (
            <a
              href={airport.skytraxUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors"
            >
              SKYTRAX ↗
            </a>
          )}
        </footer>
      </div>
    </div>
  );
}
