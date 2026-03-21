import {
  pgTable,
  serial,
  text,
  char,
  integer,
  smallint,
  numeric,
  boolean,
  date,
  bigint,
  doublePrecision,
  uniqueIndex,
  index,
  timestamp,
  jsonb,
} from "drizzle-orm/pg-core";

// PostGIS geography column (location) is NOT in the Drizzle schema.
// It's created by the seed script and managed by the Rust CLI.
// When running `drizzle-kit push`, select "No, abort" if it tries
// to drop the location column, or use --force to skip the prompt.

// ============================================================
// REFERENCE / LOOKUP TABLES
// ============================================================

export const countries = pgTable("countries", {
  isoCode: char("iso_code", { length: 2 }).primaryKey(),
  name: text("name").notNull(),
  continent: char("continent", { length: 2 }).notNull(),
});

export const regions = pgTable("regions", {
  id: serial("id").primaryKey(),
  isoCode: text("iso_code").unique().notNull(),
  name: text("name").notNull(),
  countryCode: char("country_code", { length: 2 })
    .notNull()
    .references(() => countries.isoCode),
});

// ============================================================
// ORGANISATIONS
// ============================================================

export const organisations = pgTable("organisations", {
  id: serial("id").primaryKey(),
  name: text("name").notNull(),
  shortName: text("short_name"),
  countryCode: char("country_code", { length: 2 }).references(
    () => countries.isoCode
  ),
  orgType: text("org_type").notNull(),
  ownershipModel: text("ownership_model"),
  publicSharePct: numeric("public_share_pct", { precision: 5, scale: 2 }),
  foundedYear: smallint("founded_year"),
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// AIRPORTS
// ============================================================

export const airports = pgTable("airports", {
  id: serial("id").primaryKey(),
  iataCode: char("iata_code", { length: 3 }).unique(),
  icaoCode: char("icao_code", { length: 4 }).unique(),
  ourairportsId: integer("ourairports_id").unique(),
  name: text("name").notNull(),
  shortName: text("short_name"),
  city: text("city").notNull(),
  countryCode: char("country_code", { length: 2 })
    .notNull()
    .references(() => countries.isoCode),
  regionCode: text("region_code").references(() => regions.isoCode),
  // location: geography(POINT, 4326) — managed by seed.ts and Rust CLI, not Drizzle
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
  annualCapacityM: numeric("annual_capacity_m", { precision: 6, scale: 2 }),
  annualPax2019M: numeric("annual_pax_2019_m", { precision: 6, scale: 2 }),
  annualPaxLatestM: numeric("annual_pax_latest_m", { precision: 6, scale: 2 }),
  latestPaxYear: smallint("latest_pax_year"),
  wikipediaUrl: text("wikipedia_url"),
  websiteUrl: text("website_url"),
  skytraxUrl: text("skytrax_url"),
  inSeedSet: boolean("in_seed_set").default(false),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// RUNWAYS
// ============================================================

export const runways = pgTable("runways", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
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
  leHeadingDegT: numeric("le_heading_degT", { precision: 6, scale: 2 }),
  leDisplacedThresholdFt: integer("le_displaced_threshold_ft"),
  heLatitudeDeg: doublePrecision("he_latitude_deg"),
  heLongitudeDeg: doublePrecision("he_longitude_deg"),
  heElevationFt: integer("he_elevation_ft"),
  heHeadingDegT: numeric("he_heading_degT", { precision: 6, scale: 2 }),
  heDisplacedThresholdFt: integer("he_displaced_threshold_ft"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// FREQUENCIES
// ============================================================

export const frequencies = pgTable("frequencies", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  freqType: text("freq_type"),
  description: text("description"),
  frequencyMhz: numeric("frequency_mhz", { precision: 7, scale: 3 }).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// PAX YEARLY
// ============================================================

export const paxYearly = pgTable("pax_yearly", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  year: smallint("year").notNull(),
  totalPax: bigint("total_pax", { mode: "number" }),
  domesticPax: bigint("domestic_pax", { mode: "number" }),
  internationalPax: bigint("international_pax", { mode: "number" }),
  aircraftMovements: integer("aircraft_movements"),
  cargoTonnes: numeric("cargo_tonnes", { precision: 10, scale: 2 }),
  source: text("source"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// OPERATIONAL STATS
// ============================================================

export const operationalStats = pgTable("operational_stats", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  periodYear: smallint("period_year").notNull(),
  periodMonth: smallint("period_month"),
  periodType: text("period_type").notNull(),
  totalFlights: integer("total_flights"),
  delayedFlights: integer("delayed_flights"),
  delayPct: numeric("delay_pct", { precision: 5, scale: 2 }),
  avgDelayMinutes: numeric("avg_delay_minutes", { precision: 10, scale: 2 }),
  cancelledFlights: integer("cancelled_flights"),
  cancellationPct: numeric("cancellation_pct", { precision: 5, scale: 2 }),
  delayWeatherPct: numeric("delay_weather_pct", { precision: 5, scale: 2 }),
  delayCarrierPct: numeric("delay_carrier_pct", { precision: 5, scale: 2 }),
  delayAtcPct: numeric("delay_atc_pct", { precision: 5, scale: 2 }),
  delaySecurityPct: numeric("delay_security_pct", { precision: 5, scale: 2 }),
  delayAirportPct: numeric("delay_airport_pct", { precision: 5, scale: 2 }),
  mishandledBagsPer1k: numeric("mishandled_bags_per_1k", {
    precision: 6,
    scale: 3,
  }),
  source: text("source"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// SENTIMENT SNAPSHOTS
// ============================================================

export const sentimentSnapshots = pgTable("sentiment_snapshots", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  source: text("source").notNull(),
  snapshotYear: smallint("snapshot_year").notNull(),
  snapshotQuarter: smallint("snapshot_quarter"),
  avgRating: numeric("avg_rating", { precision: 3, scale: 2 }),
  reviewCount: integer("review_count"),
  positivePct: numeric("positive_pct", { precision: 5, scale: 2 }),
  negativePct: numeric("negative_pct", { precision: 5, scale: 2 }),
  neutralPct: numeric("neutral_pct", { precision: 5, scale: 2 }),
  scoreQueuing: numeric("score_queuing", { precision: 3, scale: 2 }),
  scoreCleanliness: numeric("score_cleanliness", { precision: 3, scale: 2 }),
  scoreStaff: numeric("score_staff", { precision: 3, scale: 2 }),
  scoreFoodBev: numeric("score_food_bev", { precision: 3, scale: 2 }),
  scoreShopping: numeric("score_shopping", { precision: 3, scale: 2 }),
  scoreWifi: numeric("score_wifi", { precision: 3, scale: 2 }),
  scoreWayfinding: numeric("score_wayfinding", { precision: 3, scale: 2 }),
  scoreTransport: numeric("score_transport", { precision: 3, scale: 2 }),
  skytraxStars: smallint("skytrax_stars"),
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// AIRPORT SCORES
// ============================================================

export const airportScores = pgTable("airport_scores", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  scoreVersion: text("score_version").notNull().default("v1"),
  scoredAt: timestamp("scored_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  referenceYear: smallint("reference_year").notNull(),
  scoreInfrastructure: numeric("score_infrastructure", {
    precision: 5,
    scale: 2,
  }),
  scoreOperational: numeric("score_operational", { precision: 5, scale: 2 }),
  scoreSentiment: numeric("score_sentiment", { precision: 5, scale: 2 }),
  scoreSentimentVelocity: numeric("score_sentiment_velocity", {
    precision: 5,
    scale: 2,
  }),
  scoreConnectivity: numeric("score_connectivity", { precision: 5, scale: 2 }),
  scoreOperator: numeric("score_operator", { precision: 5, scale: 2 }),
  scoreTotal: numeric("score_total", { precision: 5, scale: 2 }),
  weightInfrastructure: numeric("weight_infrastructure", {
    precision: 3,
    scale: 2,
  }),
  weightOperational: numeric("weight_operational", { precision: 3, scale: 2 }),
  weightSentiment: numeric("weight_sentiment", { precision: 3, scale: 2 }),
  weightSentimentVelocity: numeric("weight_sentiment_velocity", {
    precision: 3,
    scale: 2,
  }),
  weightConnectivity: numeric("weight_connectivity", {
    precision: 3,
    scale: 2,
  }),
  weightOperator: numeric("weight_operator", { precision: 3, scale: 2 }),
  commentary: text("commentary"),
  isLatest: boolean("is_latest").default(true),
  notes: text("notes"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// ROUTES
// ============================================================

export const routes = pgTable("routes", {
  id: serial("id").primaryKey(),
  originId: integer("origin_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  destinationId: integer("destination_id").references(() => airports.id, {
    onDelete: "set null",
  }),
  destinationIcao: char("destination_icao", { length: 4 }),
  destinationIata: char("destination_iata", { length: 3 }),
  airlineIcao: text("airline_icao"),
  airlineIata: text("airline_iata"),
  airlineName: text("airline_name"),
  flightsPerMonth: integer("flights_per_month"),
  firstObserved: date("first_observed"),
  lastObserved: date("last_observed"),
  dataSource: text("data_source").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// METAR DAILY
// ============================================================

export const metarDaily = pgTable("metar_daily", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  observationDate: date("observation_date").notNull(),
  avgTempC: numeric("avg_temp_c", { precision: 5, scale: 2 }),
  minTempC: numeric("min_temp_c", { precision: 5, scale: 2 }),
  maxTempC: numeric("max_temp_c", { precision: 5, scale: 2 }),
  avgVisibilityM: numeric("avg_visibility_m", { precision: 8, scale: 2 }),
  minVisibilityM: numeric("min_visibility_m", { precision: 8, scale: 2 }),
  avgWindSpeedKt: numeric("avg_wind_speed_kt", { precision: 5, scale: 2 }),
  maxWindSpeedKt: numeric("max_wind_speed_kt", { precision: 5, scale: 2 }),
  maxWindGustKt: numeric("max_wind_gust_kt", { precision: 5, scale: 2 }),
  precipitationFlag: boolean("precipitation_flag").default(false),
  thunderstormFlag: boolean("thunderstorm_flag").default(false),
  fogFlag: boolean("fog_flag").default(false),
  lowCeilingFlag: boolean("low_ceiling_flag").default(false),
  metarCount: integer("metar_count"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// REVIEWS RAW
// ============================================================

export const reviewsRaw = pgTable("reviews_raw", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
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
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// WIKIPEDIA SNAPSHOTS
// ============================================================

export const wikipediaSnapshots = pgTable("wikipedia_snapshots", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  fetchedAt: timestamp("fetched_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  openedYear: smallint("opened_year"),
  operatorRaw: text("operator_raw"),
  ownerRaw: text("owner_raw"),
  terminalCount: smallint("terminal_count"),
  terminalNames: text("terminal_names").array(),
  renovationNotes: text("renovation_notes"),
  ownershipNotes: text("ownership_notes"),
  milestoneNotes: text("milestone_notes"),
  skytraxHistory: jsonb("skytrax_history").$type<Record<string, {}> | null>(),
  aciAwards: jsonb("aci_awards").$type<Record<string, {}> | null>(),
  wikipediaUrl: text("wikipedia_url"),
  articleRevisionId: bigint("article_revision_id", { mode: "number" }),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// NAVAIDS
// ============================================================

export const navaids = pgTable("navaids", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id").references(() => airports.id, {
    onDelete: "cascade",
  }),
  ident: text("ident"),
  name: text("name"),
  navaidType: text("navaid_type"),
  frequencyKhz: integer("frequency_khz"),
  latitudeDeg: doublePrecision("latitude_deg"),
  longitudeDeg: doublePrecision("longitude_deg"),
  elevationFt: integer("elevation_ft"),
  associatedAirportIcao: text("associated_airport_icao"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// AIRPORT SLUGS
// ============================================================

export const airportSlugs = pgTable("airport_slugs", {
  airportId: integer("airport_id")
    .notNull()
    .references(() => airports.id, { onDelete: "cascade" }),
  source: text("source").notNull(),
  slug: text("slug").notNull(),
});

// ============================================================
// PIPELINE RUNS
// ============================================================

export const pipelineRuns = pgTable("pipeline_runs", {
  id: serial("id").primaryKey(),
  airportId: integer("airport_id").references(() => airports.id, {
    onDelete: "cascade",
  }),
  source: text("source").notNull(),
  startedAt: timestamp("started_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  completedAt: timestamp("completed_at", { withTimezone: true }),
  status: text("status").notNull().default("running"),
  recordsProcessed: integer("records_processed").default(0),
  lastRecordDate: date("last_record_date"),
  errorMessage: text("error_message"),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});

// ============================================================
// OPERATOR SCORES
// ============================================================

export const operatorScores = pgTable("operator_scores", {
  id: serial("id").primaryKey(),
  organisationId: integer("organisation_id")
    .notNull()
    .references(() => organisations.id),
  scoreVersion: text("score_version").notNull().default("v1"),
  scoredAt: timestamp("scored_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  referenceYear: smallint("reference_year").notNull(),
  airportCount: smallint("airport_count"),
  avgScoreTotal: numeric("avg_score_total", { precision: 5, scale: 2 }),
  avgScoreSentiment: numeric("avg_score_sentiment", { precision: 5, scale: 2 }),
  avgScoreOperational: numeric("avg_score_operational", {
    precision: 5,
    scale: 2,
  }),
  isLatest: boolean("is_latest").default(true),
  createdAt: timestamp("created_at", { withTimezone: true }).defaultNow(),
});
