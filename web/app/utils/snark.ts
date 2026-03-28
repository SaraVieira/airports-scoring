// Simple hash to pick a deterministic-but-varied template per airport/value
function pick(templates: string[], seed: number): string {
  return templates[Math.abs(seed) % templates.length];
}

function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return h;
}

export function delaySnark(
  delayPct: number | null,
  iata?: string,
): string {
  if (delayPct == null) return "";
  const pct = parseFloat(String(delayPct));
  const seed = hashStr(iata ?? String(pct));

  if (pct > 40)
    return pick(
      [
        "Nearly half of flights delayed. At this point, 'on time' is the exception.",
        "More flights are delayed than not. That's not a statistic, it's a lifestyle.",
        "At this rate, arriving on time should come with a loyalty bonus.",
        "The delay rate here makes rush hour traffic look efficient.",
        "If delays were a sport, this airport would be in the Olympics.",
      ],
      seed,
    );
  if (pct > 25)
    return pick(
      [
        "Nearly a third of flights delayed. Pack a book. Maybe two.",
        "One in three flights delayed. The odds aren't in your favour.",
        "A quarter of flights don't leave on time. That's not a glitch, it's a feature.",
        "With these delay rates, 'departure time' is more of a suggestion.",
        "The delay percentage here would make a Swiss train conductor weep.",
      ],
      seed,
    );
  if (pct > 15)
    return pick(
      [
        "One in five flights delayed. Not great, not apocalyptic.",
        "Delays hovering around the 'noticeable but not catastrophic' range.",
        "Enough delays to be annoying, not enough to make the news.",
        "The kind of delay rate that makes you arrive at the airport 'just in case.'",
        "Not the worst, but you'd still want to keep an eye on your gate screen.",
      ],
      seed,
    );
  if (pct > 8)
    return pick(
      [
        `${pct.toFixed(0)}% of flights delayed. Under ten percent. We checked twice.`,
        "Single-digit delays. Someone's doing their job properly.",
        "Delays are low enough that complaining feels petty. Almost.",
        "A respectable delay rate. The kind that doesn't ruin your day.",
        "Under ten percent. That's genuinely decent by European standards.",
      ],
      seed,
    );
  return pick(
    [
      "Delays are genuinely rare here. We're suspicious.",
      "Almost nothing is delayed. Either this airport is excellent or the data is lying.",
      "On-time performance so good it feels like a statistical error.",
      "If punctuality were a religion, this airport would be the cathedral.",
      "Delays? What delays? This place runs like clockwork.",
    ],
    seed,
  );
}

export function paxSnark(
  latest: number | null,
  capacity: number | null,
): string {
  if (!latest || !capacity) return "";
  const pct = Math.round((latest / capacity) * 100);
  const seed = hashStr(String(pct));

  if (pct > 100)
    return pick(
      [
        `Running at ${pct}% capacity. The airport is literally bursting.`,
        `${pct}% capacity. More people than the building was designed for. Bold strategy.`,
        `Over capacity at ${pct}%. The architects are nervous.`,
        `${pct}% utilisation. Somewhere, a fire marshal is having palpitations.`,
        `Running beyond capacity. Expansion plans are presumably stuck in planning permission.`,
      ],
      seed,
    );
  if (pct > 85)
    return pick(
      [
        `Running at ${pct}% capacity. Efficiently full without feeling cramped. Show-offs.`,
        `${pct}% capacity. Busy but functional. The sweet spot, apparently.`,
        `Nearly full at ${pct}%. The airport equivalent of a well-packed suitcase.`,
        `${pct}% utilisation. Just enough room to breathe, barely.`,
        `Running hot at ${pct}%. Peak efficiency or peak discomfort — depends who you ask.`,
      ],
      seed,
    );
  if (pct > 60)
    return pick(
      [
        `Running at ${pct}% capacity. The remaining ${100 - pct}% is probably the baggage claim area everyone avoids.`,
        `${pct}% capacity. Comfortably busy. The Goldilocks zone.`,
        `${pct}% full. Enough passengers to keep the shops open, not enough to cause a stampede.`,
        `Running at a sensible ${pct}%. Room to grow, or room to breathe — perspective matters.`,
        `${pct}% capacity. Neither empty nor overwhelming. Pleasantly mediocre.`,
      ],
      seed,
    );
  return pick(
    [
      `Running at ${pct}% capacity. Plenty of room — and plenty of reasons people aren't coming.`,
      `${pct}% capacity. The terminal echoes. That's not a feature.`,
      `Only ${pct}% full. Either very spacious or very unpopular.`,
      `${pct}% utilisation. The good news: no queues. The bad news: no passengers.`,
      `Running at ${pct}%. So much empty space you could park a second airport in here.`,
    ],
    seed,
  );
}

export function scoreVerdict(score: number | null | undefined): string {
  if (score == null) return "No data";
  const seed = hashStr(String(Math.round(score)));

  if (score >= 90)
    return pick(
      [
        "Suspiciously good",
        "Almost too good to be true",
        "Exceptional",
        "The gold standard",
        "Top tier",
      ],
      seed,
    );
  if (score >= 70)
    return pick(
      [
        "Actually decent",
        "Genuinely solid",
        "Above average",
        "Reliably good",
        "Pleasantly competent",
      ],
      seed,
    );
  if (score >= 50)
    return pick(
      [
        "Passable",
        "Middling",
        "Unremarkable",
        "It'll do",
        "Solidly average",
      ],
      seed,
    );
  if (score >= 30)
    return pick(
      [
        "Painful",
        "Below par",
        "Struggling",
        "Room for improvement",
        "Not great",
      ],
      seed,
    );
  return pick(
    [
      "Dire",
      "Rock bottom",
      "Impressively bad",
      "A cautionary tale",
      "Beyond help",
    ],
    seed,
  );
}

export function totalVerdict(score: number | null | undefined): string {
  if (score == null) return "Unscored";
  const seed = hashStr(String(Math.round(score)));

  if (score >= 81)
    return pick(
      [
        "Fine. We'll allow it.",
        "Actually good. We're as surprised as you.",
        "Impressive. Don't let it go to their heads.",
        "Top marks. Someone's trying.",
        "Genuinely excellent. No sarcasm needed.",
      ],
      seed,
    );
  if (score >= 61)
    return pick(
      [
        "Surprisingly not awful",
        "Better than expected, honestly",
        "Decent enough to recommend without guilt",
        "Above the median. Celebrate accordingly.",
        "Solid. Not spectacular, but solid.",
      ],
      seed,
    );
  if (score >= 41)
    return pick(
      [
        "Could be worse (but not by much)",
        "The definition of 'meh'",
        "Aggressively average",
        "Not broken, just… tired",
        "Functional in the way a paperweight is functional",
      ],
      seed,
    );
  if (score >= 21)
    return pick(
      [
        "A masterclass in mediocrity",
        "Below average with commitment",
        "Needs work. Lots of work.",
        "The bar was low and they still tripped",
        "Disappointing, but consistently so",
      ],
      seed,
    );
  return pick(
    [
      "Impressively terrible",
      "A case study in what not to do",
      "Beyond redemption",
      "The airport equivalent of a one-star review",
      "Someone should be held accountable",
    ],
    seed,
  );
}

export function totalCommentary(
  score:
    | {
        scoreInfrastructure?: number | string | null;
        scoreOperational?: number | string | null;
        scoreSentiment?: number | string | null;
        scoreConnectivity?: number | string | null;
        scoreSentimentVelocity?: number | string | null;
        commentary?: string | null;
      }
    | undefined,
): string {
  if (!score) return "";
  if (score.commentary) return score.commentary;

  const infra = Number(score.scoreInfrastructure ?? 0);
  const ops = Number(score.scoreOperational ?? 0);
  const sent = Number(score.scoreSentiment ?? 0);
  const conn = Number(score.scoreConnectivity ?? 0);

  const parts: string[] = [];
  if (conn >= 70 && ops < 50)
    parts.push("Strong connectivity can't save poor operations.");
  if (infra < 40) parts.push("Infrastructure is the weak link.");
  if (sent < 40)
    parts.push("Passengers have noticed — and they're not happy about it.");

  const vel = Number(score.scoreSentimentVelocity ?? 50);
  if (vel > 60) parts.push("At least the trend is improving.");
  else if (vel < 40) parts.push("And it's getting worse.");
  else parts.push("The trajectory is flat — no improvement in sight.");

  return parts.join(" ") || "The data speaks for itself.";
}
