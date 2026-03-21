import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/start";
import { db } from "../db";
import { airports, airportScores } from "../db/schema";
import { eq } from "drizzle-orm";

const getAirport = createServerFn({ method: "GET" })
  .validator((iata: string) => iata.toUpperCase())
  .handler(async ({ input: iata }) => {
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
          with: { destination: true },
          orderBy: (r, { desc }) => [desc(r.flightsPerMonth)],
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

    return airport;
  });

export const Route = createFileRoute("/airport/$iata")({
  loader: ({ params }) => getAirport({ data: params.iata }),
  component: AirportDetail,
});

function AirportDetail() {
  const airport = Route.useLoaderData();
  const score = airport.scores[0];

  return (
    <div>
      <h1>
        {airport.name} ({airport.iataCode})
      </h1>
      <p>
        {airport.city}, {airport.country?.name}
      </p>

      {airport.operator && <p>Operator: {airport.operator.shortName}</p>}

      {score && (
        <section>
          <h2>Score: {score.scoreTotal}/100</h2>
        </section>
      )}

      <section>
        <h2>Runways ({airport.runways.length})</h2>
      </section>

      <section>
        <h2>Passenger Traffic ({airport.paxYearly.length} years)</h2>
      </section>

      <section>
        <h2>Routes ({airport.routesOut.length})</h2>
      </section>

      <section>
        <h2>
          Sentiment ({airport.sentimentSnapshots.length} snapshots)
        </h2>
      </section>
    </div>
  );
}
