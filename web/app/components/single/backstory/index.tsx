import { Airport } from "~/utils/types";
import { HeaderText } from "../header-text";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { BackstoryTimeline } from "./timeline";

import { Awards } from "./awards";

export const Backstory = ({ airport }: { airport: Airport }) => {
  const { wiki } = useSingleAirport({ airport });
  if (!wiki) return null;

  return (
    <section className="flex flex-col gap-4">
      <HeaderText>The Backstory</HeaderText>

      <BackstoryTimeline airport={airport} wiki={wiki} />

      <Awards airport={airport} />
    </section>
  );
};
