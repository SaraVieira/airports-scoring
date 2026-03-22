import {
  BarChart,
  Bar,
  Cell,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

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
    .filter((y) => y.reviews >= 5)
    .sort((a, b) => a.year - b.year);

  if (years.length < 2) return null;

  const meaningful = years.filter((y) => y.reviews >= 10);
  const first = meaningful[0] ?? years[0];
  const last = meaningful[meaningful.length - 1] ?? years[years.length - 1];
  const delta = last.avg - first.avg;

  const chartData = years.map((y) => ({
    name: String(y.year),
    value: y.avg,
    reviews: y.reviews,
    fill:
      y.avg >= 3.5
        ? "#22c55e"
        : y.avg >= 2.5
          ? "#eab308"
          : "#ef4444",
  }));

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
      <div style={{ width: "100%", height: 80 }}>
        <ResponsiveContainer>
          <BarChart data={chartData} margin={{ top: 5, right: 0, bottom: 0, left: 0 }}>
            <Tooltip
              cursor={false}
              content={({ active, payload }) => {
                if (!active || !payload?.[0]) return null;
                const d = payload[0].payload;
                return (
                  <div style={{
                    backgroundColor: "#18181b",
                    border: "1px solid #27272a",
                    borderRadius: 4,
                    padding: "6px 10px",
                    fontSize: 11,
                    fontFamily: "monospace",
                    color: "#d4d4d8",
                  }}>
                    <div style={{ color: "#a1a1aa", marginBottom: 2 }}>{d.name}</div>
                    <div>{Number(d.value).toFixed(1)}/5 ({Number(d.reviews).toLocaleString()} reviews)</div>
                  </div>
                );
              }}
            />
            <Bar dataKey="value" radius={[2, 2, 0, 0]} maxBarSize={28}>
              {chartData.map((entry, index) => (
                <Cell key={index} fill={entry.fill} fillOpacity={0.7} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
      <div className="flex justify-between">
        <span className="font-mono text-[10px] text-zinc-600">
          Then: {first.avg.toFixed(1)}/5 ({first.reviews} reviews)
        </span>
        <span
          className={`font-mono text-[10px] font-bold ${last.avg > first.avg ? "text-green-500" : "text-red-500"}`}
        >
          <span className={last.avg > first.avg ? "text-green-500 font-bold" : "text-red-500 font-bold"}>Now</span>: {last.avg.toFixed(1)}/5 ({last.reviews} reviews)
        </span>
      </div>
    </div>
  );
}
