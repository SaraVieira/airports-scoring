import * as Popover from "@radix-ui/react-popover";
import { scoreVerdict } from "~/utils/snark";
import { scoreColor, scoreBg } from "~/utils/scoring";

export function ScoreBar({
  label,
  score,
  weight,
  explanation,
}: {
  label: string;
  score: string | null | undefined;
  weight: string;
  explanation?: { plain: string; technical: string };
}) {
  const num = score ? parseFloat(score) : null;
  const width = num != null ? `${Math.min(num, 100)}%` : "0%";
  return (
    <div className="flex items-center gap-2 w-full">
      <span className="font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider w-36 shrink-0 uppercase flex items-center gap-1.5">
        {label}
        {explanation && (
          <Popover.Root>
            <Popover.Trigger asChild>
              <button
                className="text-zinc-600 hover:text-zinc-400 transition-colors cursor-pointer text-[11px] leading-none"
                aria-label={`Info about ${label}`}
              >
                ⓘ
              </button>
            </Popover.Trigger>
            <Popover.Portal>
              <Popover.Content
                className="bg-[#18181b] border border-zinc-700 rounded px-4 py-3 max-w-xs z-50 shadow-lg"
                sideOffset={5}
                align="start"
              >
                <div className="flex flex-col gap-2">
                  <p className="font-mono text-[11px] text-zinc-300 leading-relaxed">
                    <span className="font-bold text-zinc-100">What it measures: </span>
                    {explanation.plain}
                  </p>
                  <p className="font-mono text-[10px] text-zinc-500 leading-relaxed">
                    <span className="font-bold text-zinc-400">Data: </span>
                    {explanation.technical}
                  </p>
                </div>
                <Popover.Arrow className="fill-zinc-700" />
              </Popover.Content>
            </Popover.Portal>
          </Popover.Root>
        )}
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
