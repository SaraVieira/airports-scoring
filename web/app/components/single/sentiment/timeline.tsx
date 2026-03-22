type SentimentSnap =
  (typeof import("../../../db/schema"))["sentimentSnapshots"]["$inferSelect"];

export function SentimentTimeline({
  snapshots,
}: {
  snapshots: SentimentSnap[];
}) {
  const byYear = new Map<number, { ratings: number[]; reviews: number }>();
  for (const s of snapshots) {
    if (s.avgRating == null) continue;
    const year = s.snapshotYear;
    const entry = byYear.get(year) ?? { ratings: [], reviews: 0 };
    entry.ratings.push(parseFloat(String(s.avgRating)));
    entry.reviews += s.reviewCount ?? 0;
    byYear.set(year, entry);
  }

  const years = Array.from(byYear.entries())
    .map(([year, data]) => ({
      year,
      avg: data.ratings.reduce((a, b) => a + b, 0) / data.ratings.length,
      reviews: data.reviews,
    }))
    .filter((y) => y.reviews >= 5) // Filter out noise from low-count years
    .sort((a, b) => a.year - b.year);

  if (years.length < 2) return null;

  const maxRating = 5;
  // Use first/last year with meaningful review count (>10) for "Then vs Now"
  const meaningful = years.filter((y) => y.reviews >= 10);
  const first = meaningful[0] ?? years[0];
  const last = meaningful[meaningful.length - 1] ?? years[years.length - 1];
  const delta = last.avg - first.avg;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
          Sentiment Trajectory ({first.year}–{last.year})
        </span>
        <span
          className={`font-mono text-[11px] font-bold ${delta > 0.2 ? "text-green-500" : delta < -0.2 ? "text-red-500" : "text-zinc-500"}`}
        >
          {delta > 0 ? "+" : ""}
          {delta.toFixed(2)} over {years.length} years
        </span>
      </div>
      <div className="flex items-end gap-0.5 h-20">
        {years.map((y) => {
          const h = Math.max((y.avg / maxRating) * 100, 8);
          const color =
            y.avg >= 3.5
              ? "bg-green-500/70"
              : y.avg >= 2.5
                ? "bg-yellow-500/70"
                : "bg-red-500/70";
          return (
            <div
              key={y.year}
              className="flex flex-col items-center gap-1 flex-1"
              title={`${y.year}: ${y.avg.toFixed(2)} (${y.reviews} reviews)`}
            >
              <span className="font-mono text-[9px] text-zinc-600 tabular-nums">
                {y.avg.toFixed(1)}
              </span>
              <div className="w-full flex justify-center h-14">
                <div
                  className={`w-full max-w-7 ${color}`}
                  style={{ height: `${h}%`, alignSelf: "flex-end" }}
                />
              </div>
              <span className="font-mono text-[9px] text-zinc-600 tabular-nums">
                {String(y.year).slice(2)}
              </span>
            </div>
          );
        })}
      </div>
      <div className="flex justify-between">
        <span className="font-mono text-[10px] text-zinc-600">
          Then: {first.avg.toFixed(1)}/5 ({first.reviews} reviews)
        </span>
        <span
          className={`font-mono text-[10px] font-bold ${last.avg > first.avg ? "text-green-500" : "text-red-500"}`}
        >
          Now: {last.avg.toFixed(1)}/5 ({last.reviews} reviews)
        </span>
      </div>
    </div>
  );
}
