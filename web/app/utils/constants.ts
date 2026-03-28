/** All data sources that run per-airport in the pipeline. */
export const PIPELINE_SOURCES = [
  "eurocontrol",
  "metar",
  "opensky",
  "routes",
  "eurostat",
  "caa",
  "aena",
  "wikipedia",
  "reviews",
  "sentiment",
  "carbon_accreditation",
  "priority_pass",
] as const;

/** Sources available in the job form (includes aliases). */
export const ALL_SOURCES = [
  "ourairports",
  "wikipedia",
  "eurocontrol",
  "eurostat",
  "routes",
  "metar",
  "reviews",
  "skytrax",
  "google_reviews",
  "sentiment",
  "opensky",
  "caa",
  "aena",
  "carbon_accreditation",
  "priority_pass",
] as const;

export const SCORE_EXPLANATIONS: Record<
  string,
  { plain: string; technical: string }
> = {
  Operational: {
    plain: "Delays, cancellations, on-time performance",
    technical: "Eurocontrol ATFM delay data, monthly aggregation",
  },
  Sentiment: {
    plain: "What passengers actually think",
    technical: "RoBERTa + NLI sentiment analysis on Skytrax & Google reviews",
  },
  Infrastructure: {
    plain: "Runways, age, facilities",
    technical: "OurAirports data, Wikipedia infrastructure info",
  },
  "Sent. Velocity": {
    plain: "Is sentiment getting better or worse?",
    technical: "8-quarter rolling comparison of sentiment scores",
  },
  Connectivity: {
    plain: "Route network breadth",
    technical: "OPDI + FlightRadar24 route data, destination count",
  },
};

export const SENTIMENT_EXPLANATIONS: Record<
  string,
  { plain: string; technical: string }
> = {
  Queuing: {
    plain: "How long passengers wait at security, check-in, and boarding gates",
    technical: "NLI zero-shot classification on review text for 'queuing & security' topic",
  },
  Cleanliness: {
    plain: "How clean the terminal, bathrooms, and gates are",
    technical: "NLI classification for 'cleanliness' topic across all review sources",
  },
  Staff: {
    plain: "How helpful, friendly, and professional airport staff are",
    technical: "NLI classification for 'staff & service' topic",
  },
  "Food & Bev": {
    plain: "Quality and variety of restaurants, cafes, and bars",
    technical: "NLI classification for 'food & beverage' topic",
  },
  Wifi: {
    plain: "Quality and availability of airport WiFi",
    technical: "NLI classification for 'wifi & connectivity' topic",
  },
  Wayfinding: {
    plain: "How easy it is to navigate the airport with signs and directions",
    technical: "NLI classification for 'wayfinding & signage' topic",
  },
  Transport: {
    plain: "Quality of connections to the city — trains, buses, taxis",
    technical: "NLI classification for 'transport links' topic",
  },
  Shopping: {
    plain: "Variety and quality of shops in the terminal",
    technical: "Explicit sub-scores from Skytrax reviews (when available)",
  },
};
