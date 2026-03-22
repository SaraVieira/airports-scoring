import { n as TSS_SERVER_FUNCTION, t as createServerFn } from "../server.js";
import { config } from "dotenv";
import { drizzle } from "drizzle-orm/node-postgres";
import { Pool } from "pg";
import { bigint, boolean, char, date, doublePrecision, integer, jsonb, numeric, pgTable, serial, smallint, text, timestamp } from "drizzle-orm/pg-core";
import { eq, relations, sql } from "drizzle-orm";
//#region \0rolldown/runtime.js
var __defProp = Object.defineProperty;
var __exportAll = (all, no_symbols) => {
	let target = {};
	for (var name in all) __defProp(target, name, {
		get: all[name],
		enumerable: true
	});
	if (!no_symbols) __defProp(target, Symbol.toStringTag, { value: "Module" });
	return target;
};
//#endregion
//#region node_modules/.pnpm/@tanstack+start-server-core@1.167.1/node_modules/@tanstack/start-server-core/dist/esm/createServerRpc.js
var createServerRpc = (serverFnMeta, splitImportFn) => {
	const url = "/_serverFn/" + serverFnMeta.id;
	return Object.assign(splitImportFn, {
		url,
		serverFnMeta,
		[TSS_SERVER_FUNCTION]: true
	});
};
//#endregion
//#region app/db/schema.ts
var schema_exports = /* @__PURE__ */ __exportAll({
	airportScores: () => airportScores,
	airportSlugs: () => airportSlugs,
	airports: () => airports,
	allAirports: () => allAirports,
	countries: () => countries,
	frequencies: () => frequencies,
	metarDaily: () => metarDaily,
	navaids: () => navaids,
	operationalStats: () => operationalStats,
	operatorScores: () => operatorScores,
	organisations: () => organisations,
	paxYearly: () => paxYearly,
	pipelineRuns: () => pipelineRuns,
	regions: () => regions,
	reviewsRaw: () => reviewsRaw,
	routes: () => routes,
	runways: () => runways,
	sentimentSnapshots: () => sentimentSnapshots,
	wikipediaSnapshots: () => wikipediaSnapshots
});
var countries = pgTable("countries", {
	isoCode: char("iso_code", { length: 2 }).primaryKey(),
	name: text("name").notNull(),
	continent: char("continent", { length: 2 }).notNull()
});
var regions = pgTable("regions", {
	id: serial("id").primaryKey(),
	isoCode: text("iso_code").unique().notNull(),
	name: text("name").notNull(),
	countryCode: char("country_code", { length: 2 }).notNull().references(() => countries.isoCode)
});
var organisations = pgTable("organisations", {
	id: serial("id").primaryKey(),
	name: text("name").notNull(),
	shortName: text("short_name"),
	countryCode: char("country_code", { length: 2 }).references(() => countries.isoCode),
	orgType: text("org_type").notNull(),
	ownershipModel: text("ownership_model"),
	publicSharePct: numeric("public_share_pct", {
		precision: 5,
		scale: 2
	}),
	foundedYear: smallint("founded_year"),
	notes: text("notes"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var airports = pgTable("airports", {
	id: serial("id").primaryKey(),
	iataCode: char("iata_code", { length: 3 }).unique(),
	icaoCode: char("icao_code", { length: 4 }).unique(),
	ourairportsId: integer("ourairports_id").unique(),
	name: text("name").notNull(),
	shortName: text("short_name"),
	city: text("city").notNull(),
	countryCode: char("country_code", { length: 2 }).notNull().references(() => countries.isoCode),
	regionCode: text("region_code").references(() => regions.isoCode),
	elevationFt: integer("elevation_ft"),
	timezone: text("timezone"),
	airportType: text("airport_type").notNull(),
	scheduledService: boolean("scheduled_service").default(true),
	terminalCount: smallint("terminal_count"),
	totalGates: smallint("total_gates"),
	openedYear: smallint("opened_year"),
	lastMajorReno: smallint("last_major_reno"),
	operatorId: integer("operator_id").references(() => organisations.id),
	ownerId: integer("owner_id").references(() => organisations.id),
	ownershipNotes: text("ownership_notes"),
	annualCapacityM: numeric("annual_capacity_m", {
		precision: 6,
		scale: 2
	}),
	annualPax2019M: numeric("annual_pax_2019_m", {
		precision: 6,
		scale: 2
	}),
	annualPaxLatestM: numeric("annual_pax_latest_m", {
		precision: 6,
		scale: 2
	}),
	latestPaxYear: smallint("latest_pax_year"),
	wikipediaUrl: text("wikipedia_url"),
	websiteUrl: text("website_url"),
	skytraxUrl: text("skytrax_url"),
	inSeedSet: boolean("in_seed_set").default(false),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
	updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow()
});
var runways = pgTable("runways", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	ident: text("ident"),
	leIdent: text("le_ident"),
	heIdent: text("he_ident"),
	lengthFt: integer("length_ft"),
	widthFt: integer("width_ft"),
	surface: text("surface"),
	lighted: boolean("lighted"),
	closed: boolean("closed").default(false),
	leLatitudeDeg: doublePrecision("le_latitude_deg"),
	leLongitudeDeg: doublePrecision("le_longitude_deg"),
	leElevationFt: integer("le_elevation_ft"),
	leHeadingDegT: numeric("le_heading_degT", {
		precision: 6,
		scale: 2
	}),
	leDisplacedThresholdFt: integer("le_displaced_threshold_ft"),
	heLatitudeDeg: doublePrecision("he_latitude_deg"),
	heLongitudeDeg: doublePrecision("he_longitude_deg"),
	heElevationFt: integer("he_elevation_ft"),
	heHeadingDegT: numeric("he_heading_degT", {
		precision: 6,
		scale: 2
	}),
	heDisplacedThresholdFt: integer("he_displaced_threshold_ft"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var frequencies = pgTable("frequencies", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	freqType: text("freq_type"),
	description: text("description"),
	frequencyMhz: numeric("frequency_mhz", {
		precision: 7,
		scale: 3
	}).notNull(),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var paxYearly = pgTable("pax_yearly", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	year: smallint("year").notNull(),
	totalPax: bigint("total_pax", { mode: "number" }),
	domesticPax: bigint("domestic_pax", { mode: "number" }),
	internationalPax: bigint("international_pax", { mode: "number" }),
	aircraftMovements: integer("aircraft_movements"),
	cargoTonnes: numeric("cargo_tonnes", {
		precision: 10,
		scale: 2
	}),
	source: text("source"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var operationalStats = pgTable("operational_stats", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	periodYear: smallint("period_year").notNull(),
	periodMonth: smallint("period_month"),
	periodType: text("period_type").notNull(),
	totalFlights: integer("total_flights"),
	delayedFlights: integer("delayed_flights"),
	delayPct: numeric("delay_pct", {
		precision: 5,
		scale: 2
	}),
	avgDelayMinutes: numeric("avg_delay_minutes", {
		precision: 10,
		scale: 2
	}),
	cancelledFlights: integer("cancelled_flights"),
	cancellationPct: numeric("cancellation_pct", {
		precision: 5,
		scale: 2
	}),
	delayWeatherPct: numeric("delay_weather_pct", {
		precision: 5,
		scale: 2
	}),
	delayCarrierPct: numeric("delay_carrier_pct", {
		precision: 5,
		scale: 2
	}),
	delayAtcPct: numeric("delay_atc_pct", {
		precision: 5,
		scale: 2
	}),
	delaySecurityPct: numeric("delay_security_pct", {
		precision: 5,
		scale: 2
	}),
	delayAirportPct: numeric("delay_airport_pct", {
		precision: 5,
		scale: 2
	}),
	mishandledBagsPer1k: numeric("mishandled_bags_per_1k", {
		precision: 6,
		scale: 3
	}),
	source: text("source"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var sentimentSnapshots = pgTable("sentiment_snapshots", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	source: text("source").notNull(),
	snapshotYear: smallint("snapshot_year").notNull(),
	snapshotQuarter: smallint("snapshot_quarter"),
	avgRating: numeric("avg_rating", {
		precision: 3,
		scale: 2
	}),
	reviewCount: integer("review_count"),
	positivePct: numeric("positive_pct", {
		precision: 5,
		scale: 2
	}),
	negativePct: numeric("negative_pct", {
		precision: 5,
		scale: 2
	}),
	neutralPct: numeric("neutral_pct", {
		precision: 5,
		scale: 2
	}),
	scoreQueuing: numeric("score_queuing", {
		precision: 3,
		scale: 2
	}),
	scoreCleanliness: numeric("score_cleanliness", {
		precision: 3,
		scale: 2
	}),
	scoreStaff: numeric("score_staff", {
		precision: 3,
		scale: 2
	}),
	scoreFoodBev: numeric("score_food_bev", {
		precision: 3,
		scale: 2
	}),
	scoreShopping: numeric("score_shopping", {
		precision: 3,
		scale: 2
	}),
	scoreWifi: numeric("score_wifi", {
		precision: 3,
		scale: 2
	}),
	scoreWayfinding: numeric("score_wayfinding", {
		precision: 3,
		scale: 2
	}),
	scoreTransport: numeric("score_transport", {
		precision: 3,
		scale: 2
	}),
	skytraxStars: smallint("skytrax_stars"),
	notes: text("notes"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var airportScores = pgTable("airport_scores", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	scoreVersion: text("score_version").notNull().default("v1"),
	scoredAt: timestamp("scored_at", { withTimezone: true }).notNull().defaultNow(),
	referenceYear: smallint("reference_year").notNull(),
	scoreInfrastructure: numeric("score_infrastructure", {
		precision: 5,
		scale: 2
	}),
	scoreOperational: numeric("score_operational", {
		precision: 5,
		scale: 2
	}),
	scoreSentiment: numeric("score_sentiment", {
		precision: 5,
		scale: 2
	}),
	scoreSentimentVelocity: numeric("score_sentiment_velocity", {
		precision: 5,
		scale: 2
	}),
	scoreConnectivity: numeric("score_connectivity", {
		precision: 5,
		scale: 2
	}),
	scoreOperator: numeric("score_operator", {
		precision: 5,
		scale: 2
	}),
	scoreTotal: numeric("score_total", {
		precision: 5,
		scale: 2
	}),
	weightInfrastructure: numeric("weight_infrastructure", {
		precision: 3,
		scale: 2
	}),
	weightOperational: numeric("weight_operational", {
		precision: 3,
		scale: 2
	}),
	weightSentiment: numeric("weight_sentiment", {
		precision: 3,
		scale: 2
	}),
	weightSentimentVelocity: numeric("weight_sentiment_velocity", {
		precision: 3,
		scale: 2
	}),
	weightConnectivity: numeric("weight_connectivity", {
		precision: 3,
		scale: 2
	}),
	weightOperator: numeric("weight_operator", {
		precision: 3,
		scale: 2
	}),
	commentary: text("commentary"),
	isLatest: boolean("is_latest").default(true),
	notes: text("notes"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var allAirports = pgTable("all_airports", {
	icao: text("icao").primaryKey(),
	iata: text("iata"),
	name: text("name").notNull(),
	city: text("city").notNull(),
	country: char("country", { length: 2 }).notNull(),
	elevation: integer("elevation"),
	lat: doublePrecision("lat"),
	lon: doublePrecision("lon"),
	tz: text("tz")
});
var routes = pgTable("routes", {
	id: serial("id").primaryKey(),
	originId: integer("origin_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	destinationId: integer("destination_id").references(() => airports.id, { onDelete: "set null" }),
	destinationIcao: text("destination_icao"),
	destinationIata: char("destination_iata", { length: 3 }),
	airlineIcao: text("airline_icao"),
	airlineIata: text("airline_iata"),
	airlineName: text("airline_name"),
	flightsPerMonth: integer("flights_per_month"),
	firstObserved: date("first_observed"),
	lastObserved: date("last_observed"),
	dataSource: text("data_source").notNull(),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
	updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow()
});
var metarDaily = pgTable("metar_daily", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	observationDate: date("observation_date").notNull(),
	avgTempC: numeric("avg_temp_c", {
		precision: 5,
		scale: 2
	}),
	minTempC: numeric("min_temp_c", {
		precision: 5,
		scale: 2
	}),
	maxTempC: numeric("max_temp_c", {
		precision: 5,
		scale: 2
	}),
	avgVisibilityM: numeric("avg_visibility_m", {
		precision: 8,
		scale: 2
	}),
	minVisibilityM: numeric("min_visibility_m", {
		precision: 8,
		scale: 2
	}),
	avgWindSpeedKt: numeric("avg_wind_speed_kt", {
		precision: 5,
		scale: 2
	}),
	maxWindSpeedKt: numeric("max_wind_speed_kt", {
		precision: 5,
		scale: 2
	}),
	maxWindGustKt: numeric("max_wind_gust_kt", {
		precision: 5,
		scale: 2
	}),
	precipitationFlag: boolean("precipitation_flag").default(false),
	thunderstormFlag: boolean("thunderstorm_flag").default(false),
	fogFlag: boolean("fog_flag").default(false),
	lowCeilingFlag: boolean("low_ceiling_flag").default(false),
	metarCount: integer("metar_count"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var reviewsRaw = pgTable("reviews_raw", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	source: text("source").notNull().default("skytrax"),
	reviewDate: date("review_date"),
	author: text("author"),
	authorCountry: text("author_country"),
	overallRating: smallint("overall_rating"),
	scoreQueuing: smallint("score_queuing"),
	scoreCleanliness: smallint("score_cleanliness"),
	scoreStaff: smallint("score_staff"),
	scoreFoodBev: smallint("score_food_bev"),
	scoreWifi: smallint("score_wifi"),
	scoreWayfinding: smallint("score_wayfinding"),
	scoreTransport: smallint("score_transport"),
	recommended: boolean("recommended"),
	verified: boolean("verified").default(false),
	tripType: text("trip_type"),
	reviewTitle: text("review_title"),
	reviewText: text("review_text"),
	sourceUrl: text("source_url").unique(),
	processed: boolean("processed").default(false),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var wikipediaSnapshots = pgTable("wikipedia_snapshots", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	fetchedAt: timestamp("fetched_at", { withTimezone: true }).notNull().defaultNow(),
	openedYear: smallint("opened_year"),
	operatorRaw: text("operator_raw"),
	ownerRaw: text("owner_raw"),
	terminalCount: smallint("terminal_count"),
	terminalNames: text("terminal_names").array(),
	renovationNotes: text("renovation_notes"),
	ownershipNotes: text("ownership_notes"),
	milestoneNotes: text("milestone_notes"),
	skytraxHistory: jsonb("skytrax_history").$type(),
	aciAwards: jsonb("aci_awards").$type(),
	wikipediaUrl: text("wikipedia_url"),
	articleRevisionId: bigint("article_revision_id", { mode: "number" }),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var navaids = pgTable("navaids", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").references(() => airports.id, { onDelete: "cascade" }),
	ident: text("ident"),
	name: text("name"),
	navaidType: text("navaid_type"),
	frequencyKhz: integer("frequency_khz"),
	latitudeDeg: doublePrecision("latitude_deg"),
	longitudeDeg: doublePrecision("longitude_deg"),
	elevationFt: integer("elevation_ft"),
	associatedAirportIcao: text("associated_airport_icao"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var airportSlugs = pgTable("airport_slugs", {
	airportId: integer("airport_id").notNull().references(() => airports.id, { onDelete: "cascade" }),
	source: text("source").notNull(),
	slug: text("slug").notNull()
});
var pipelineRuns = pgTable("pipeline_runs", {
	id: serial("id").primaryKey(),
	airportId: integer("airport_id").references(() => airports.id, { onDelete: "cascade" }),
	source: text("source").notNull(),
	startedAt: timestamp("started_at", { withTimezone: true }).notNull().defaultNow(),
	completedAt: timestamp("completed_at", { withTimezone: true }),
	status: text("status").notNull().default("running"),
	recordsProcessed: integer("records_processed").default(0),
	lastRecordDate: date("last_record_date"),
	errorMessage: text("error_message"),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
var operatorScores = pgTable("operator_scores", {
	id: serial("id").primaryKey(),
	organisationId: integer("organisation_id").notNull().references(() => organisations.id),
	scoreVersion: text("score_version").notNull().default("v1"),
	scoredAt: timestamp("scored_at", { withTimezone: true }).notNull().defaultNow(),
	referenceYear: smallint("reference_year").notNull(),
	airportCount: smallint("airport_count"),
	avgScoreTotal: numeric("avg_score_total", {
		precision: 5,
		scale: 2
	}),
	avgScoreSentiment: numeric("avg_score_sentiment", {
		precision: 5,
		scale: 2
	}),
	avgScoreOperational: numeric("avg_score_operational", {
		precision: 5,
		scale: 2
	}),
	isLatest: boolean("is_latest").default(true),
	createdAt: timestamp("created_at", { withTimezone: true }).defaultNow()
});
//#endregion
//#region app/db/relations.ts
var relations_exports = /* @__PURE__ */ __exportAll({
	airportScoresRelations: () => airportScoresRelations,
	airportSlugsRelations: () => airportSlugsRelations,
	airportsRelations: () => airportsRelations,
	allAirportsRelations: () => allAirportsRelations,
	frequenciesRelations: () => frequenciesRelations,
	metarDailyRelations: () => metarDailyRelations,
	navaidsRelations: () => navaidsRelations,
	operationalStatsRelations: () => operationalStatsRelations,
	organisationsRelations: () => organisationsRelations,
	paxYearlyRelations: () => paxYearlyRelations,
	reviewsRawRelations: () => reviewsRawRelations,
	routesRelations: () => routesRelations,
	runwaysRelations: () => runwaysRelations,
	sentimentSnapshotsRelations: () => sentimentSnapshotsRelations,
	wikipediaSnapshotsRelations: () => wikipediaSnapshotsRelations
});
var airportsRelations = relations(airports, ({ one, many }) => ({
	country: one(countries, {
		fields: [airports.countryCode],
		references: [countries.isoCode]
	}),
	region: one(regions, {
		fields: [airports.regionCode],
		references: [regions.isoCode]
	}),
	operator: one(organisations, {
		fields: [airports.operatorId],
		references: [organisations.id],
		relationName: "operator"
	}),
	owner: one(organisations, {
		fields: [airports.ownerId],
		references: [organisations.id],
		relationName: "owner"
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
	slugs: many(airportSlugs)
}));
var organisationsRelations = relations(organisations, ({ one, many }) => ({
	country: one(countries, {
		fields: [organisations.countryCode],
		references: [countries.isoCode]
	}),
	operatedAirports: many(airports, { relationName: "operator" }),
	ownedAirports: many(airports, { relationName: "owner" })
}));
var runwaysRelations = relations(runways, ({ one }) => ({ airport: one(airports, {
	fields: [runways.airportId],
	references: [airports.id]
}) }));
var frequenciesRelations = relations(frequencies, ({ one }) => ({ airport: one(airports, {
	fields: [frequencies.airportId],
	references: [airports.id]
}) }));
var paxYearlyRelations = relations(paxYearly, ({ one }) => ({ airport: one(airports, {
	fields: [paxYearly.airportId],
	references: [airports.id]
}) }));
var operationalStatsRelations = relations(operationalStats, ({ one }) => ({ airport: one(airports, {
	fields: [operationalStats.airportId],
	references: [airports.id]
}) }));
var sentimentSnapshotsRelations = relations(sentimentSnapshots, ({ one }) => ({ airport: one(airports, {
	fields: [sentimentSnapshots.airportId],
	references: [airports.id]
}) }));
var airportScoresRelations = relations(airportScores, ({ one }) => ({ airport: one(airports, {
	fields: [airportScores.airportId],
	references: [airports.id]
}) }));
var routesRelations = relations(routes, ({ one }) => ({
	origin: one(airports, {
		fields: [routes.originId],
		references: [airports.id],
		relationName: "origin"
	}),
	destination: one(airports, {
		fields: [routes.destinationId],
		references: [airports.id],
		relationName: "destination"
	}),
	destinationAirport: one(allAirports, {
		fields: [routes.destinationIcao],
		references: [allAirports.icao]
	})
}));
var allAirportsRelations = relations(allAirports, ({ many }) => ({ routesTo: many(routes) }));
var metarDailyRelations = relations(metarDaily, ({ one }) => ({ airport: one(airports, {
	fields: [metarDaily.airportId],
	references: [airports.id]
}) }));
var reviewsRawRelations = relations(reviewsRaw, ({ one }) => ({ airport: one(airports, {
	fields: [reviewsRaw.airportId],
	references: [airports.id]
}) }));
var wikipediaSnapshotsRelations = relations(wikipediaSnapshots, ({ one }) => ({ airport: one(airports, {
	fields: [wikipediaSnapshots.airportId],
	references: [airports.id]
}) }));
var navaidsRelations = relations(navaids, ({ one }) => ({ airport: one(airports, {
	fields: [navaids.airportId],
	references: [airports.id]
}) }));
var airportSlugsRelations = relations(airportSlugs, ({ one }) => ({ airport: one(airports, {
	fields: [airportSlugs.airportId],
	references: [airports.id]
}) }));
//#endregion
//#region app/db/index.ts
config({ path: "../.env" });
var instance = null;
var db = new Proxy({}, { get(_target, prop) {
	if (!instance) instance = drizzle(new Pool({ connectionString: process.env.DATABASE_URL }), { schema: {
		...schema_exports,
		...relations_exports
	} });
	return instance[prop];
} });
//#endregion
//#region app/routes/airport.$iata.tsx?tss-serverfn-split
var getAirport_createServerFn_handler = createServerRpc({
	id: "a7899a09f8826539179c9ce57da0b073e046f507d82c452334188e7f2194b86f",
	name: "getAirport",
	filename: "app/routes/airport.$iata.tsx"
}, (opts) => getAirport.__executeServer(opts));
var getAirport = createServerFn({ method: "GET" }).inputValidator((iata) => iata.toUpperCase()).handler(getAirport_createServerFn_handler, async ({ data: iata }) => {
	const airport = await db.query.airports.findFirst({
		where: eq(airports.iataCode, iata),
		with: {
			operator: true,
			owner: true,
			country: true,
			runways: true,
			paxYearly: { orderBy: (p, { desc }) => [desc(p.year)] },
			operationalStats: { orderBy: (o, { desc }) => [desc(o.periodYear), desc(o.periodMonth)] },
			sentimentSnapshots: { orderBy: (s, { desc }) => [desc(s.snapshotYear), desc(s.snapshotQuarter)] },
			scores: {
				where: eq(airportScores.isLatest, true),
				limit: 1
			},
			routesOut: {
				with: {
					destination: true,
					destinationAirport: true
				},
				orderBy: () => [sql`flights_per_month DESC NULLS LAST`]
			},
			wikipediaSnapshots: {
				orderBy: (w, { desc }) => [desc(w.fetchedAt)],
				limit: 1
			},
			slugs: true
		}
	});
	if (!airport) throw new Error(`Airport ${iata} not found`);
	return airport;
});
//#endregion
export { getAirport_createServerFn_handler };
