export function PaxSparkline({
  data,
}: {
  data: { year: number; pax: number | null }[];
}) {
  if (data.length === 0) return null;
  const maxPax = Math.max(...data.map((d) => d.pax ?? 0));
  if (maxPax === 0) return null;

  // Find covid dip (lowest non-zero year between 2019-2022)
  const covidYear = data.find(
    (d) =>
      d.year >= 2020 && d.year <= 2021 && d.pax != null && d.pax < maxPax * 0.5,
  );

  return (
    <div className="flex items-end gap-1 h-16">
      {data.map((d) => {
        const h = d.pax ? Math.max((d.pax / maxPax) * 100, 3) : 3;
        const isCovid = d.year === covidYear?.year;
        const isLatest = d === data[0];
        const bg = isCovid
          ? "bg-red-500/70"
          : isLatest
            ? "bg-yellow-400"
            : "bg-zinc-600";
        return (
          <div key={d.year} className="flex flex-col items-center gap-1 flex-1">
            <div className="w-full flex flex-col items-center justify-end h-12">
              <div
                className={`w-full max-w-6 ${bg} transition-all`}
                style={{ height: `${h}%` }}
              />
            </div>
            <span
              className={`font-mono text-[9px] tabular-nums ${isLatest ? "text-zinc-300" : isCovid ? "text-red-500" : "text-zinc-600"}`}
            >
              {String(d.year).slice(2)}
            </span>
          </div>
        );
      })}
    </div>
  );
}
