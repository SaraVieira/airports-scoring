import type { components } from "~/api/types";
import { PIPELINE_SOURCES } from "~/utils/constants";

type SourceStatus = components["schemas"]["SourceStatusResponse"];

function SourceDot({
  name,
  source,
}: {
  name: string;
  source?: SourceStatus;
}) {
  const now = Date.now();
  const fetched = source?.lastFetchedAt
    ? new Date(source.lastFetchedAt).getTime()
    : 0;
  const daysSince = fetched
    ? (now - fetched) / (1000 * 60 * 60 * 24)
    : Infinity;

  let color: string;
  let status: string;
  let statusColor: string;

  if (!source || !fetched) {
    color = "bg-zinc-600";
    status = "never ran";
    statusColor = "text-zinc-500";
  } else if (source.lastStatus === "success") {
    if (daysSince < 30) {
      color = "bg-green-400";
      status = `${Math.floor(daysSince)}d ago`;
      statusColor = "text-green-400";
    } else {
      color = "bg-yellow-400";
      status = `${Math.floor(daysSince)}d ago (stale)`;
      statusColor = "text-yellow-400";
    }
  } else if (source.lastStatus === "failed") {
    color = "bg-red-400";
    status = `failed${source.lastError ? `: ${source.lastError}` : ""}`;
    statusColor = "text-red-400";
  } else {
    color = "bg-zinc-600";
    status = source.lastStatus;
    statusColor = "text-zinc-500";
  }

  return (
    <span className="relative group">
      <span
        className={`inline-block w-2 h-2 rounded-full ${color} mr-1 cursor-help`}
      />
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:flex flex-col items-start bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1.5 whitespace-nowrap z-50 shadow-lg pointer-events-none">
        <span className="font-mono text-[10px] font-bold text-zinc-300">
          {name}
        </span>
        <span className={`font-mono text-[10px] ${statusColor}`}>
          {status}
        </span>
      </span>
    </span>
  );
}

function ScoreDot({ hasScore }: { hasScore: boolean }) {
  const color = hasScore ? "bg-blue-400" : "bg-zinc-600";
  const status = hasScore ? "scored" : "not scored";
  const statusColor = hasScore ? "text-blue-400" : "text-zinc-500";

  return (
    <span className="relative group ml-1">
      <span
        className={`inline-block w-2 h-2 rounded-sm ${color} cursor-help`}
      />
      <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:flex flex-col items-start bg-zinc-900 border border-zinc-700 rounded px-2.5 py-1.5 whitespace-nowrap z-50 shadow-lg pointer-events-none">
        <span className="font-mono text-[10px] font-bold text-zinc-300">
          score
        </span>
        <span className={`font-mono text-[10px] ${statusColor}`}>
          {status}
        </span>
      </span>
    </span>
  );
}

export function SourceIndicators({
  sources,
  hasScore,
}: {
  sources: SourceStatus[];
  hasScore: boolean;
}) {
  const byName = new Map(sources.map((s) => [s.source, s]));
  return (
    <span className="flex flex-wrap items-center gap-0">
      {PIPELINE_SOURCES.map((name) => (
        <SourceDot key={name} name={name} source={byName.get(name)} />
      ))}
      <ScoreDot hasScore={hasScore} />
    </span>
  );
}
