import { scoreVerdict } from "~/utils/snark";

export function scoreColor(score: number | null | undefined): string {
  if (score == null) return "text-zinc-600";
  if (score >= 70) return "text-green-500";
  if (score >= 40) return "text-yellow-500";
  return "text-red-500";
}

function scoreBg(score: number | null | undefined): string {
  if (score == null) return "bg-zinc-600";
  if (score >= 70) return "bg-green-500";
  if (score >= 40) return "bg-yellow-500";
  return "bg-red-500";
}

export function ScoreBar({
  label,
  score,
  weight,
}: {
  label: string;
  score: string | null | undefined;
  weight: string;
}) {
  const num = score ? parseFloat(score) : null;
  const width = num != null ? `${Math.min(num, 100)}%` : "0%";
  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider w-36 shrink-0 uppercase">
        {label}
      </span>
      <span className="font-mono text-[10px] text-zinc-600 w-8 shrink-0 tabular-nums">
        {weight}
      </span>
      <div className="flex-1 h-2 bg-zinc-900 relative">
        <div
          className={`h-2 ${scoreBg(num)} absolute left-0 top-0 transition-all duration-500`}
          style={{ width }}
        />
      </div>
      <span
        className={`font-mono text-xs font-bold w-7 shrink-0 tabular-nums ${scoreColor(num)}`}
      >
        {num != null ? Math.round(num) : "—"}
      </span>
      <span
        className={`font-mono text-[11px] italic w-30 shrink-0 ${scoreColor(num)}`}
      >
        {scoreVerdict(num)}
      </span>
    </div>
  );
}
