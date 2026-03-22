import { config } from "dotenv";
config({ path: "../.env" });
import { db } from "./app/db";
import { countries, organisations, airports, airportSlugs } from "./app/db/schema";
import { eq, sql } from "drizzle-orm";

async function seed() {
  console.log("Creating extensions...");
  await db.execute(sql`CREATE EXTENSION IF NOT EXISTS postgis`);
  await db.execute(sql`CREATE EXTENSION IF NOT EXISTS pg_trgm`);

  // ── Countries ──────────────────────────────────────────────
  console.log("Seeding countries...");
  const countryData = [
    // Western Europe
    { isoCode: "GB", name: "United Kingdom", continent: "EU" },
    { isoCode: "IE", name: "Ireland", continent: "EU" },
    { isoCode: "FR", name: "France", continent: "EU" },
    { isoCode: "DE", name: "Germany", continent: "EU" },
    { isoCode: "NL", name: "Netherlands", continent: "EU" },
    { isoCode: "BE", name: "Belgium", continent: "EU" },
    { isoCode: "LU", name: "Luxembourg", continent: "EU" },
    { isoCode: "AT", name: "Austria", continent: "EU" },
    { isoCode: "CH", name: "Switzerland", continent: "EU" },
    // Southern Europe
    { isoCode: "ES", name: "Spain", continent: "EU" },
    { isoCode: "PT", name: "Portugal", continent: "EU" },
    { isoCode: "IT", name: "Italy", continent: "EU" },
    { isoCode: "GR", name: "Greece", continent: "EU" },
    { isoCode: "MT", name: "Malta", continent: "EU" },
    { isoCode: "CY", name: "Cyprus", continent: "EU" },
    { isoCode: "HR", name: "Croatia", continent: "EU" },
    { isoCode: "SI", name: "Slovenia", continent: "EU" },
    { isoCode: "ME", name: "Montenegro", continent: "EU" },
    { isoCode: "AL", name: "Albania", continent: "EU" },
    { isoCode: "MK", name: "North Macedonia", continent: "EU" },
    { isoCode: "RS", name: "Serbia", continent: "EU" },
    { isoCode: "BA", name: "Bosnia and Herzegovina", continent: "EU" },
    { isoCode: "XK", name: "Kosovo", continent: "EU" },
    // Northern Europe
    { isoCode: "DK", name: "Denmark", continent: "EU" },
    { isoCode: "SE", name: "Sweden", continent: "EU" },
    { isoCode: "NO", name: "Norway", continent: "EU" },
    { isoCode: "FI", name: "Finland", continent: "EU" },
    { isoCode: "IS", name: "Iceland", continent: "EU" },
    // Eastern Europe
    { isoCode: "PL", name: "Poland", continent: "EU" },
    { isoCode: "CZ", name: "Czech Republic", continent: "EU" },
    { isoCode: "SK", name: "Slovakia", continent: "EU" },
    { isoCode: "HU", name: "Hungary", continent: "EU" },
    { isoCode: "RO", name: "Romania", continent: "EU" },
    { isoCode: "BG", name: "Bulgaria", continent: "EU" },
    { isoCode: "LT", name: "Lithuania", continent: "EU" },
    { isoCode: "LV", name: "Latvia", continent: "EU" },
    { isoCode: "EE", name: "Estonia", continent: "EU" },
    { isoCode: "UA", name: "Ukraine", continent: "EU" },
    { isoCode: "MD", name: "Moldova", continent: "EU" },
    { isoCode: "BY", name: "Belarus", continent: "EU" },
    // Türkiye + Caucasus (Eurocontrol area)
    { isoCode: "TR", name: "Türkiye", continent: "AS" },
    { isoCode: "GE", name: "Georgia", continent: "AS" },
    { isoCode: "AM", name: "Armenia", continent: "AS" },
    { isoCode: "AZ", name: "Azerbaijan", continent: "AS" },
    // North Africa (common destination from EU)
    { isoCode: "MA", name: "Morocco", continent: "AF" },
    { isoCode: "TN", name: "Tunisia", continent: "AF" },
    { isoCode: "EG", name: "Egypt", continent: "AF" },
    // Middle East (common hubs)
    { isoCode: "AE", name: "United Arab Emirates", continent: "AS" },
    { isoCode: "QA", name: "Qatar", continent: "AS" },
    { isoCode: "IL", name: "Israel", continent: "AS" },
    // Russia
    { isoCode: "RU", name: "Russia", continent: "EU" },
  ] as const;

  for (const c of countryData) {
    await db
      .insert(countries)
      .values(c)
      .onConflictDoNothing({ target: countries.isoCode });
  }
  console.log(`  ${countryData.length} countries seeded`);

  // ── Organisations ──────────────────────────────────────────
  console.log("Seeding organisations...");
  const orgData = [
    // Spain
    { name: "AENA", shortName: "AENA", countryCode: "ES", orgType: "both", ownershipModel: "mixed", publicSharePct: "51.00", notes: "Spanish state holds 51%, remainder free float on Madrid stock exchange. Operates 46 Spanish airports." },
    // France
    { name: "Groupe ADP", shortName: "ADP", countryCode: "FR", orgType: "both", ownershipModel: "mixed", publicSharePct: "50.60", notes: "French state holds majority. Operates CDG, Orly, and international concessions." },
    { name: "VINCI Airports", shortName: "VINCI", countryCode: "FR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "Subsidiary of VINCI Group. Operates 70+ airports globally including Porto, Lyon, Belgrade, Gatwick." },
    { name: "Edeis", shortName: "Edeis", countryCode: "FR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "French airport operator managing 20+ regional airports." },
    // UK
    { name: "Heathrow Airport Holdings", shortName: "HAH", countryCode: "GB", orgType: "owner", ownershipModel: "private", publicSharePct: "0.00", notes: "Ferrovial 25%, Qatar Investment Authority 20%, others. Owns LHR." },
    { name: "Gatwick Airport Ltd", shortName: "Gatwick", countryCode: "GB", orgType: "both", ownershipModel: "private", publicSharePct: "0.00", notes: "VINCI Airports 50.01% since 2019. Global Infrastructure Partners minority." },
    { name: "Luton Rising", shortName: "Luton Rising", countryCode: "GB", orgType: "owner", ownershipModel: "public", publicSharePct: "100.00", notes: "Wholly owned by Luton Borough Council. Contracts operations to private partners." },
    { name: "MAG (Manchester Airports Group)", shortName: "MAG", countryCode: "GB", orgType: "both", ownershipModel: "mixed", publicSharePct: "64.00", notes: "Owns Manchester, Stansted, East Midlands. Manchester City Council 64%, IFM Investors 36%." },
    { name: "AGS Airports", shortName: "AGS", countryCode: "GB", orgType: "both", ownershipModel: "private", publicSharePct: "0.00", notes: "Ferrovial/Macquarie JV. Owns Glasgow, Aberdeen, Southampton." },
    // Spain (cont.)
    { name: "Ferrovial Airports", shortName: "Ferrovial", countryCode: "ES", orgType: "both", ownershipModel: "private", publicSharePct: "0.00", notes: "Spanish infrastructure group. Major shareholder in Heathrow, AGS Airports." },
    // Germany
    { name: "Flughafen München GmbH", shortName: "MUC GmbH", countryCode: "DE", orgType: "both", ownershipModel: "mixed", publicSharePct: "100.00", notes: "Free State of Bavaria 51%, Federal Republic of Germany 26%, City of Munich 23%." },
    { name: "Flughafen Berlin Brandenburg GmbH", shortName: "FBB", countryCode: "DE", orgType: "both", ownershipModel: "mixed", publicSharePct: "100.00", notes: "State of Brandenburg 37%, State of Berlin 37%, Federal Republic 26%." },
    { name: "Fraport AG", shortName: "Fraport", countryCode: "DE", orgType: "both", ownershipModel: "mixed", publicSharePct: "51.40", notes: "Operates Frankfurt (FRA), also Lima, Antalya, Greek regionals. State of Hesse 31.3%, City of Frankfurt 20.1%." },
    { name: "Flughafen Düsseldorf GmbH", shortName: "DUS GmbH", countryCode: "DE", orgType: "both", ownershipModel: "mixed", publicSharePct: "50.00", notes: "City of Düsseldorf 50%, Airport Partners GmbH 50%." },
    { name: "Flughafen Hamburg GmbH", shortName: "HAM GmbH", countryCode: "DE", orgType: "both", ownershipModel: "mixed", publicSharePct: "100.00", notes: "City of Hamburg 51%, AviAlliance (PSP Investments) 49%." },
    // Netherlands
    { name: "Schiphol Group", shortName: "Schiphol", countryCode: "NL", orgType: "both", ownershipModel: "mixed", publicSharePct: "69.80", notes: "Dutch state 69.8%, City of Amsterdam 20.03%, City of Rotterdam 2.87%. Operates AMS, also owns Brisbane Airport stake." },
    // Belgium
    { name: "Brussels Airport Company", shortName: "BAC", countryCode: "BE", orgType: "both", ownershipModel: "private", publicSharePct: "25.00", notes: "Ontario Teachers 39%, Macquarie 36%, Belgian state 25%. Operates BRU." },
    // Italy
    { name: "Aeroporti di Roma", shortName: "ADR", countryCode: "IT", orgType: "both", ownershipModel: "mixed", publicSharePct: "0.00", notes: "Mundys (Benetton family) majority. Operates FCO and CIA." },
    { name: "SEA Aeroporti di Milano", shortName: "SEA Milano", countryCode: "IT", orgType: "both", ownershipModel: "mixed", publicSharePct: "54.81", notes: "City of Milan 54.81%, 2i Aeroporti (Ardian/Benetton) 36.38%. Operates MXP and LIN." },
    { name: "SAVE S.p.A.", shortName: "SAVE", countryCode: "IT", orgType: "both", ownershipModel: "mixed", publicSharePct: "0.00", notes: "Operates Venice (VCE), Treviso, Verona. Finint and Infravia major shareholders." },
    // Denmark
    { name: "Copenhagen Airports A/S", shortName: "CPH Airports", countryCode: "DK", orgType: "both", ownershipModel: "mixed", publicSharePct: "39.20", notes: "Danish state 39.2% via Steen & Strøm, Macquarie 29.4%, Ontario Teachers 27.2%." },
    // Sweden
    { name: "Swedavia AB", shortName: "Swedavia", countryCode: "SE", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Swedish state-owned. Operates 10 airports including ARN, GOT, MMX." },
    // Norway
    { name: "Avinor AS", shortName: "Avinor", countryCode: "NO", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Norwegian state-owned. Operates 43 airports including OSL, BGO, TRD." },
    // Finland
    { name: "Finavia Oyj", shortName: "Finavia", countryCode: "FI", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Finnish state-owned. Operates 21 airports including HEL." },
    // Ireland
    { name: "daa plc", shortName: "daa", countryCode: "IE", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Irish state-owned. Operates Dublin (DUB) and Cork (ORK)." },
    // Poland
    { name: "Polska Agencja Żeglugi Powietrznej", shortName: "PPL", countryCode: "PL", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Polish state enterprise. Operates Warsaw Chopin and other Polish airports." },
    // Hungary
    { name: "Budapest Airport Zrt", shortName: "BUD Airport", countryCode: "HU", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Renationalised by Hungarian state in 2023 for €3.1bn after years of private ownership under AviAlliance." },
    // Czech Republic
    { name: "Letiště Praha a.s.", shortName: "Prague Airport", countryCode: "CZ", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Czech state-owned via Ministry of Finance. Operates PRG." },
    // Austria
    { name: "Flughafen Wien AG", shortName: "VIE AG", countryCode: "AT", orgType: "both", ownershipModel: "mixed", publicSharePct: "40.00", notes: "Airports Group Europe 39.8% (IFM/Flughafen Zürich), Province of Lower Austria 20%, City of Vienna 20%." },
    // Switzerland
    { name: "Flughafen Zürich AG", shortName: "ZRH AG", countryCode: "CH", orgType: "both", ownershipModel: "mixed", publicSharePct: "33.33", notes: "Canton of Zürich 33.33%, City of Zürich 5%. Publicly traded. Also invests in international airports." },
    // Portugal
    { name: "ANA Aeroportos de Portugal", shortName: "ANA", countryCode: "PT", orgType: "both", ownershipModel: "private", publicSharePct: "0.00", notes: "VINCI Airports subsidiary since 2013. Operates Lisbon, Porto, Faro, Azores, Madeira." },
    // Greece
    { name: "Athens International Airport SA", shortName: "AIA", countryCode: "GR", orgType: "both", ownershipModel: "mixed", publicSharePct: "30.00", notes: "Greek state 30%, AviAlliance/PSP 40%, rest traded on ATHEX. Operates ATH." },
    { name: "Fraport Greece", shortName: "Fraport GR", countryCode: "GR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "Fraport subsidiary operating 14 Greek regional airports under 40-year concession." },
    // Romania
    { name: "Bucharest Airports National Company", shortName: "CNAB", countryCode: "RO", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Romanian state-owned. Operates OTP and BBU." },
    // Türkiye
    { name: "İGA Havalimanı İşletmesi", shortName: "İGA", countryCode: "TR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "Cengiz-Kolin-Limak-Mapa-Kalyon consortium. Operates Istanbul Airport (IST) under 25-year concession." },
    { name: "TAV Airports", shortName: "TAV", countryCode: "TR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "Groupe ADP subsidiary. Operates Ankara, Izmir, Tbilisi, Medina, and others." },
    // Iceland
    { name: "Isavia ohf.", shortName: "Isavia", countryCode: "IS", orgType: "both", ownershipModel: "public", publicSharePct: "100.00", notes: "Icelandic state-owned. Operates Keflavík (KEF) and domestic airports." },
    // Croatia
    { name: "MZLZ (Zagreb Airport)", shortName: "MZLZ", countryCode: "HR", orgType: "operator", ownershipModel: "private", publicSharePct: "0.00", notes: "Groupe ADP/TAV/ZAIC consortium. Operates Zagreb Airport (ZAG) under 30-year concession." },
  ] as const;

  for (const org of orgData) {
    const existing = await db
      .select()
      .from(organisations)
      .where(eq(organisations.name, org.name))
      .limit(1);

    if (existing.length === 0) {
      await db.insert(organisations).values(org);
    }
  }
  console.log(`  ${orgData.length} organisations seeded`);

  // ── Airport Slugs ──────────────────────────────────────────
  // These require airports to exist first (from OurAirports fetcher).
  console.log("Seeding airport slugs...");

  const slugData = [
    { iata: "LHR", source: "skytrax", slug: "london-heathrow-airport" },
    { iata: "LHR", source: "skytrax_ratings", slug: "london-heathrow-airport" },
    { iata: "LGW", source: "skytrax", slug: "london-gatwick-airport" },
    { iata: "LGW", source: "skytrax_ratings", slug: "london-gatwick-airport" },
    { iata: "LTN", source: "skytrax", slug: "london-luton-airport" },
    { iata: "LTN", source: "skytrax_ratings", slug: "london-luton-airport" },
    { iata: "OPO", source: "skytrax", slug: "porto-airport" },
    { iata: "OPO", source: "skytrax_ratings", slug: "porto-airport" },
    { iata: "MAD", source: "skytrax", slug: "madrid-barajas-airport" },
    { iata: "MAD", source: "skytrax_ratings", slug: "madrid-barajas-airport" },
    { iata: "BCN", source: "skytrax", slug: "barcelona-el-prat-airport" },
    { iata: "BCN", source: "skytrax_ratings", slug: "barcelona-el-prat-airport" },
    { iata: "BER", source: "skytrax", slug: "berlin-brandenburg-airport" },
    { iata: "BER", source: "skytrax_ratings", slug: "berlin-brandenburg-airport" },
    { iata: "MUC", source: "skytrax", slug: "munich-airport" },
    { iata: "MUC", source: "skytrax_ratings", slug: "munich-airport" },
    { iata: "CDG", source: "skytrax", slug: "paris-charles-de-gaulle-airport" },
    { iata: "CDG", source: "skytrax_ratings", slug: "paris-charles-de-gaulle-airport" },
    { iata: "NCE", source: "skytrax", slug: "nice-cote-dazur-airport" },
    { iata: "NCE", source: "skytrax_ratings", slug: "nice-cote-dazur-airport" },
    { iata: "AMS", source: "skytrax", slug: "amsterdam-schiphol-airport" },
    { iata: "AMS", source: "skytrax_ratings", slug: "amsterdam-schiphol-airport" },
    { iata: "CPH", source: "skytrax", slug: "copenhagen-airport" },
    { iata: "CPH", source: "skytrax_ratings", slug: "copenhagen-airport" },
    { iata: "FCO", source: "skytrax", slug: "rome-fiumicino-airport" },
    { iata: "FCO", source: "skytrax_ratings", slug: "rome-fiumicino-airport" },
    { iata: "WAW", source: "skytrax", slug: "warsaw-chopin-airport" },
    { iata: "WAW", source: "skytrax_ratings", slug: "warsaw-chopin-airport" },
    { iata: "BUD", source: "skytrax", slug: "budapest-airport" },
    { iata: "BUD", source: "skytrax_ratings", slug: "budapest-airport" },
  ];

  let slugCount = 0;
  let slugSkipped = 0;
  for (const s of slugData) {
    const airport = await db
      .select({ id: airports.id })
      .from(airports)
      .where(eq(airports.iataCode, s.iata))
      .limit(1);

    if (airport.length === 0) {
      slugSkipped++;
      continue;
    }

    await db
      .insert(airportSlugs)
      .values({
        airportId: airport[0].id,
        source: s.source,
        slug: s.slug,
      })
      .onConflictDoUpdate({
        target: [airportSlugs.airportId, airportSlugs.source],
        set: { slug: s.slug },
      });
    slugCount++;
  }
  if (slugSkipped > 0) {
    console.log(`  ${slugCount} airport slugs seeded (${slugSkipped} skipped — airports not yet in DB, re-run after OurAirports fetch)`);
  } else {
    console.log(`  ${slugCount} airport slugs seeded`);
  }

  // ── Views ──────────────────────────────────────────────────
  console.log("Creating views...");

  await db.execute(sql`
    CREATE OR REPLACE VIEW v_airport_scores_latest AS
    SELECT
      a.iata_code,
      a.name,
      a.city,
      a.country_code,
      op.short_name AS operator,
      op.ownership_model,
      op.public_share_pct,
      s.score_total,
      s.score_infrastructure,
      s.score_operational,
      s.score_sentiment,
      s.score_sentiment_velocity,
      s.score_connectivity,
      s.score_operator,
      s.reference_year,
      s.scored_at
    FROM airports a
    LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
    LEFT JOIN organisations op ON op.id = a.operator_id
    ORDER BY s.score_total DESC NULLS LAST
  `);

  await db.execute(sql`
    CREATE OR REPLACE VIEW v_sentiment_trajectory AS
    SELECT
      a.iata_code,
      a.name,
      ss.source,
      ss.snapshot_year,
      ss.snapshot_quarter,
      ss.avg_rating,
      ss.review_count,
      ss.positive_pct,
      ss.skytrax_stars,
      ss.avg_rating - LAG(ss.avg_rating) OVER (
        PARTITION BY a.id, ss.source
        ORDER BY ss.snapshot_year, ss.snapshot_quarter
      ) AS rating_delta
    FROM airports a
    JOIN sentiment_snapshots ss ON ss.airport_id = a.id
    ORDER BY a.iata_code, ss.source, ss.snapshot_year, ss.snapshot_quarter
  `);

  await db.execute(sql`
    CREATE OR REPLACE VIEW v_operator_comparison AS
    SELECT
      o.short_name AS operator,
      o.ownership_model,
      o.public_share_pct,
      COUNT(a.id) AS airports_in_dataset,
      ROUND(AVG(s.score_total), 1) AS avg_score,
      ROUND(AVG(s.score_sentiment), 1) AS avg_sentiment,
      ROUND(AVG(s.score_operational), 1) AS avg_operational,
      ROUND(AVG(s.score_sentiment_velocity), 1) AS avg_velocity
    FROM organisations o
    JOIN airports a ON a.operator_id = o.id
    LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
    GROUP BY o.id, o.short_name, o.ownership_model, o.public_share_pct
    ORDER BY avg_score DESC NULLS LAST
  `);

  console.log("  3 views created");

  // ── PostGIS column + indexes that Drizzle doesn't handle ───
  console.log("Creating PostGIS location column and indexes...");

  // Add the PostGIS geography column if it doesn't exist.
  // Drizzle can't manage this type, so the Rust CLI populates it via ST_MakePoint().
  await db.execute(sql`
    DO $$
    BEGIN
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'airports' AND column_name = 'location'
      ) THEN
        ALTER TABLE airports ADD COLUMN location geography(POINT, 4326);
      END IF;
    END $$
  `);

  await db.execute(sql`
    CREATE INDEX IF NOT EXISTS airports_location_gix ON airports USING GIST (location)
  `);
  await db.execute(sql`
    CREATE INDEX IF NOT EXISTS airports_name_trgm ON airports USING GIN (name gin_trgm_ops)
  `);
  await db.execute(sql`
    CREATE INDEX IF NOT EXISTS airports_city_trgm ON airports USING GIN (city gin_trgm_ops)
  `);

  console.log("  spatial + trigram indexes created");

  // ── Unique constraints Drizzle doesn't create ──────────────
  console.log("Creating unique constraints for ON CONFLICT upserts...");

  await db.execute(sql`
    DO $$ BEGIN
      ALTER TABLE pax_yearly ADD CONSTRAINT pax_yearly_airport_year_unique UNIQUE (airport_id, year);
    EXCEPTION WHEN duplicate_object THEN NULL; END $$
  `);
  await db.execute(sql`
    DO $$ BEGIN
      ALTER TABLE metar_daily ADD CONSTRAINT metar_daily_airport_date_unique UNIQUE (airport_id, observation_date);
    EXCEPTION WHEN duplicate_object THEN NULL; END $$
  `);
  await db.execute(sql`
    DO $$ BEGIN
      ALTER TABLE operational_stats ADD CONSTRAINT ops_stats_unique UNIQUE (airport_id, period_year, period_month, source);
    EXCEPTION WHEN duplicate_object THEN NULL; END $$
  `);
  await db.execute(sql`
    DO $$ BEGIN
      ALTER TABLE sentiment_snapshots ADD CONSTRAINT sentiment_unique UNIQUE (airport_id, source, snapshot_year, snapshot_quarter);
    EXCEPTION WHEN duplicate_object THEN NULL; END $$
  `);
  await db.execute(sql`
    CREATE UNIQUE INDEX IF NOT EXISTS routes_icao_unique_idx ON routes (origin_id, destination_icao, airline_icao, data_source)
      WHERE data_source IN ('opdi', 'opensky')
  `);
  await db.execute(sql`
    CREATE UNIQUE INDEX IF NOT EXISTS routes_iata_unique_idx ON routes (origin_id, destination_iata, airline_iata, data_source)
      WHERE data_source = 'openflights'
  `);

  console.log("  unique constraints created");

  console.log("\nSeed complete!");
}

seed()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error("Seed failed:", err);
    process.exit(1);
  });
