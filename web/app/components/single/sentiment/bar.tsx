export function SentimentBar({
  label,
  score,
}: {
  label: string;
  score: string | number | null | undefined;
}) {
  const num = score != null ? parseFloat(String(score)) : null;
  const width = num != null ? `${(num / 10) * 100}%` : "0%";
  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider w-24 shrink-0 uppercase">
        {label}
      </span>
      <div className="flex-1 h-1.5 bg-zinc-900 relative">
        <div
          className={`h-1.5 absolute left-0 top-0 ${num != null && num >= 6 ? "bg-green-500" : num != null && num >= 4 ? "bg-yellow-500" : "bg-red-500"}`}
          style={{ width }}
        />
      </div>
      <span
        className={`font-mono text-[11px] font-bold w-7 shrink-0 tabular-nums ${num != null && num >= 6 ? "text-green-500" : num != null && num >= 4 ? "text-yellow-500" : "text-red-500"}`}
      >
        {num != null ? num.toFixed(1) : "—"}
      </span>
    </div>
  );
}
