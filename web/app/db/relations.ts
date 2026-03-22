import { relations } from "drizzle-orm";
import {
  airports,
  organisations,
  countries,
  regions,
  runways,
  frequencies,
  paxYearly,
  operationalStats,
  sentimentSnapshots,
  airportScores,
  routes,
  allAirports,
  metarDaily,
  reviewsRaw,
  wikipediaSnapshots,
  navaids,
  airportSlugs,
} from "./schema";

// ============================================================
// AIRPORT RELATIONS
// ============================================================

export const airportsRelations = relations(airports, ({ one, many }) => ({
  country: one(countries, {
    fields: [airports.countryCode],
    references: [countries.isoCode],
  }),
  region: one(regions, {
    fields: [airports.regionCode],
    references: [regions.isoCode],
  }),
  operator: one(organisations, {
    fields: [airports.operatorId],
    references: [organisations.id],
    relationName: "operator",
  }),
  owner: one(organisations, {
    fields: [airports.ownerId],
    references: [organisations.id],
    relationName: "owner",
  }),
  runways: many(runways),
  frequencies: many(frequencies),
  paxYearly: many(paxYearly),
  operationalStats: many(operationalStats),
  sentimentSnapshots: many(sentimentSnapshots),
  scores: many(airportScores),
  routesOut: many(routes, { relationName: "origin" }),
  routesIn: many(routes, { relationName: "destination" }),
  metarDaily: many(metarDaily),
  reviews: many(reviewsRaw),
  wikipediaSnapshots: many(wikipediaSnapshots),
  navaids: many(navaids),
  slugs: many(airportSlugs),
}));

// ============================================================
// ORGANISATION RELATIONS
// ============================================================

export const organisationsRelations = relations(
  organisations,
  ({ one, many }) => ({
    country: one(countries, {
      fields: [organisations.countryCode],
      references: [countries.isoCode],
    }),
    operatedAirports: many(airports, { relationName: "operator" }),
    ownedAirports: many(airports, { relationName: "owner" }),
  })
);

// ============================================================
// CHILD TABLE RELATIONS
// ============================================================

export const runwaysRelations = relations(runways, ({ one }) => ({
  airport: one(airports, {
    fields: [runways.airportId],
    references: [airports.id],
  }),
}));

export const frequenciesRelations = relations(frequencies, ({ one }) => ({
  airport: one(airports, {
    fields: [frequencies.airportId],
    references: [airports.id],
  }),
}));

export const paxYearlyRelations = relations(paxYearly, ({ one }) => ({
  airport: one(airports, {
    fields: [paxYearly.airportId],
    references: [airports.id],
  }),
}));

export const operationalStatsRelations = relations(
  operationalStats,
  ({ one }) => ({
    airport: one(airports, {
      fields: [operationalStats.airportId],
      references: [airports.id],
    }),
  })
);

export const sentimentSnapshotsRelations = relations(
  sentimentSnapshots,
  ({ one }) => ({
    airport: one(airports, {
      fields: [sentimentSnapshots.airportId],
      references: [airports.id],
    }),
  })
);

export const airportScoresRelations = relations(airportScores, ({ one }) => ({
  airport: one(airports, {
    fields: [airportScores.airportId],
    references: [airports.id],
  }),
}));

export const routesRelations = relations(routes, ({ one }) => ({
  origin: one(airports, {
    fields: [routes.originId],
    references: [airports.id],
    relationName: "origin",
  }),
  destination: one(airports, {
    fields: [routes.destinationId],
    references: [airports.id],
    relationName: "destination",
  }),
  destinationAirport: one(allAirports, {
    fields: [routes.destinationIcao],
    references: [allAirports.icao],
  }),
}));

export const allAirportsRelations = relations(allAirports, ({ many }) => ({
  routesTo: many(routes),
}));

export const metarDailyRelations = relations(metarDaily, ({ one }) => ({
  airport: one(airports, {
    fields: [metarDaily.airportId],
    references: [airports.id],
  }),
}));

export const reviewsRawRelations = relations(reviewsRaw, ({ one }) => ({
  airport: one(airports, {
    fields: [reviewsRaw.airportId],
    references: [airports.id],
  }),
}));

export const wikipediaSnapshotsRelations = relations(
  wikipediaSnapshots,
  ({ one }) => ({
    airport: one(airports, {
      fields: [wikipediaSnapshots.airportId],
      references: [airports.id],
    }),
  })
);

export const navaidsRelations = relations(navaids, ({ one }) => ({
  airport: one(airports, {
    fields: [navaids.airportId],
    references: [airports.id],
  }),
}));

export const airportSlugsRelations = relations(airportSlugs, ({ one }) => ({
  airport: one(airports, {
    fields: [airportSlugs.airportId],
    references: [airports.id],
  }),
}));
