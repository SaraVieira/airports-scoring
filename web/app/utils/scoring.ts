export function scoreColor(score: number | null | undefined): string {
  if (score == null) return "text-zinc-600";
  if (score >= 70) return "text-green-500";
  if (score >= 40) return "text-yellow-500";
  return "text-red-500";
}

export function scoreBg(score: number | null | undefined): string {
  if (score == null) return "bg-zinc-600";
  if (score >= 70) return "bg-green-500";
  if (score >= 40) return "bg-yellow-500";
  return "bg-red-500";
}
