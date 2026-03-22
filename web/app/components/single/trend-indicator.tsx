export function TrendIndicator({
  value,
  suffix = "",
  invert = false,
}: {
  value: number | null;
  suffix?: string;
  invert?: boolean;
}) {
  if (value == null) return null;
  const improved = invert ? value < 0 : value > 0;
  const color = improved ? "text-green-500" : "text-red-500";
  const arrow = value > 0 ? "+" : "";
  return (
    <span className={`font-mono text-[11px] font-bold ${color}`}>
      {arrow}
      {value.toFixed(1)}
      {suffix} vs prior year
    </span>
  );
}
