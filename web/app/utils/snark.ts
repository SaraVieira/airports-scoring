export function delaySnark(delayPct: number | null): string {
  if (delayPct == null) return "";
  const pct = parseFloat(String(delayPct));
  if (pct > 40)
    return "Nearly half of flights delayed. At this point, 'on time' is the exception.";
  if (pct > 25)
    return "Nearly a third of flights delayed. Pack a book. Maybe two.";
  if (pct > 15)
    return "One in five flights delayed. Not great, not apocalyptic.";
  if (pct > 8)
    return `${pct.toFixed(0)}% of flights delayed. Under ten percent. We checked twice.`;
  return "Delays are genuinely rare here. We're suspicious.";
}

export function paxSnark(
  latest: number | null,
  capacity: number | null,
): string {
  if (!latest || !capacity) return "";
  const pct = Math.round((latest / capacity) * 100);
  if (pct > 100)
    return `Running at ${pct}% capacity. The airport is literally bursting.`;
  if (pct > 85)
    return `Running at ${pct}% capacity. Efficiently full without feeling cramped. Show-offs.`;
  if (pct > 60)
    return `Running at ${pct}% capacity. The remaining ${100 - pct}% is probably the baggage claim area everyone avoids.`;
  return `Running at ${pct}% capacity. Plenty of room — and plenty of reasons people aren't coming.`;
}

export function scoreVerdict(score: number | null | undefined): string {
  if (score == null) return "No data";
  if (score >= 90) return "Suspiciously good";
  if (score >= 70) return "Actually decent";
  if (score >= 50) return "Passable";
  if (score >= 30) return "Painful";
  return "Dire";
}

export function totalVerdict(score: number | null | undefined): string {
  if (score == null) return "Unscored";
  if (score >= 81) return "Fine. We'll allow it.";
  if (score >= 61) return "Surprisingly not awful";
  if (score >= 41) return "Could be worse (but not by much)";
  if (score >= 21) return "A masterclass in mediocrity";
  return "Impressively terrible";
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
