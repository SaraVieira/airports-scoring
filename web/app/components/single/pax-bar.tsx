import {
  BarChart,
  Bar,
  Cell,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

export function PaxSparkline({
  data,
}: {
  data: { year: number; pax: number | null }[];
}) {
  if (data.length === 0) return null;
  const maxPax = Math.max(...data.map((d) => d.pax ?? 0));
  if (maxPax === 0) return null;

  const chartData = data.map((d) => ({
    name: String(d.year),
    value: d.pax ?? 0,
    fill:
      d.year >= 2020 && d.year <= 2021
        ? "#ef4444" // red for covid years
        : d.year > 2021
          ? "#22c55e" // green for recovery
          : "#71717a", // grey for normal years
  }));

  return (
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
                  <div>{Number(d.value) >= 1_000_000 ? `${(Number(d.value) / 1_000_000).toFixed(1)}M` : Number(d.value).toLocaleString()} passengers</div>
                </div>
              );
            }}
          />
          <Bar dataKey="value" radius={[2, 2, 0, 0]} maxBarSize={24}>
            {chartData.map((entry, index) => (
              <Cell key={index} fill={entry.fill} fillOpacity={0.8} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
