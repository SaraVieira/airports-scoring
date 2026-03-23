import { createFileRoute } from "@tanstack/react-router";

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
import { Backstory } from "~/components/single/backstory";
import { getAirport } from "~/server/get-airport";
import { CarbonBadge } from "~/components/single/carbon-badge";
import { GroundTransport } from "~/components/single/ground-transport";
import { Amenities } from "~/components/single/amenities";
import type { Airport } from "~/utils/types";

export const Route = createFileRoute("/airport/$iata")({
  loader: ({ params }) => getAirport({ data: params.iata! }),
  component: AirportDetail,
});

function AirportDetail() {
  const airport = Route.useLoaderData() as Airport;
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

        <CarbonBadge airport={airport} />

        <Divider />
        <Sentiment airport={airport} />

        <Divider />

        <Numbers airport={airport} />
        <Amenities airport={airport} />
        <Divider />

        <Tardiness airport={airport} />
        <Divider />
        <RouteSection airport={airport} />
        <GroundTransport airport={airport} />
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
