const COLORS: Record<string, string> = {
  public: "text-green-400 bg-green-400/10 border-green-400/20",
  private: "text-red-400 bg-red-400/10 border-red-400/20",
  mixed: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
};

export function OwnershipBadge({ model }: { model?: string | null }) {
  if (!model) return null;
  return (
    <span
      className={`font-mono text-[10px] px-1.5 py-0.5 border rounded ${COLORS[model] || "text-zinc-500 bg-zinc-500/10 border-zinc-500/20"}`}
    >
      {model}
    </span>
  );
}
