import * as Popover from "@radix-ui/react-popover";
import { SENTIMENT_EXPLANATIONS } from "~/utils/constants";

export function SentimentBar({
  label,
  score,
}: {
  label: string;
  score: string | number | null | undefined;
}) {
  const num = score != null ? parseFloat(String(score)) : null;
  const width = num != null ? `${(num / 5) * 100}%` : "0%";

  // Color by value on 1-5 scale: >= 3.5 green, 2.5-3.5 yellow, < 2.5 red
  const barColor =
    num != null && num >= 3.5
      ? "bg-green-500"
      : num != null && num >= 2.5
        ? "bg-yellow-500"
        : "bg-red-500";
  const textColor =
    num != null && num >= 3.5
      ? "text-green-500"
      : num != null && num >= 2.5
        ? "text-yellow-500"
        : "text-red-500";

  const explanation = SENTIMENT_EXPLANATIONS[label];

  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider w-24 shrink-0 uppercase flex items-center gap-1">
        {label}
        {explanation && (
          <Popover.Root>
            <Popover.Trigger asChild>
              <button className="text-zinc-600 hover:text-zinc-400 transition-colors cursor-pointer text-[9px]">
                ⓘ
              </button>
            </Popover.Trigger>
            <Popover.Portal>
              <Popover.Content
                side="top"
                sideOffset={5}
                className="bg-[#18181b] border border-zinc-700 p-3 max-w-[260px] z-50"
              >
                <p className="font-mono text-[11px] text-zinc-300 leading-relaxed">
                  {explanation.plain}
                </p>
                <p className="font-mono text-[9px] text-zinc-600 mt-2 leading-relaxed">
                  {explanation.technical}
                </p>
                <Popover.Arrow className="fill-zinc-700" />
              </Popover.Content>
            </Popover.Portal>
          </Popover.Root>
        )}
      </span>
      <div className="flex-1 h-1.5 bg-zinc-900 relative">
        <div
          className={`h-1.5 absolute left-0 top-0 ${barColor}`}
          style={{ width }}
        />
      </div>
      <span
        className={`font-mono text-[11px] font-bold w-7 shrink-0 tabular-nums ${textColor}`}
      >
        {num != null ? num.toFixed(1) : "—"}
      </span>
    </div>
  );
}
