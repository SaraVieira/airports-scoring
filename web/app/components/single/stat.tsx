export function Stat({
  value,
  label,
  color = "text-zinc-100",
  size = "text-[42px]",
}: {
  value: string;
  label: string;
  color?: string;
  size?: string;
}) {
  return (
    <div className="flex-1 flex flex-col gap-1">
      <span className={`font-grotesk ${size} font-bold ${color} tabular-nums`}>
        {value}
      </span>
      <span className="font-mono text-[11px] text-zinc-500 tracking-wider uppercase">
        {label}
      </span>
    </div>
  );
}
