import { useState } from "react";
import { SkytraxHistory, TimelineEvent } from "~/utils/types";

function TruncatedText({
  text,
  maxLength = 200,
}: {
  text: string;
  maxLength?: number;
}) {
  const [expanded, setExpanded] = useState(false);
  if (text.length <= maxLength) {
    return (
      <span className="font-mono text-[10px] text-zinc-600 leading-relaxed">
        {text}
      </span>
    );
  }
  return (
    <span className="font-mono text-[10px] text-zinc-600 leading-relaxed">
      {expanded ? text : `${text.slice(0, maxLength).trim()}...`}
      <button
        onClick={() => setExpanded(!expanded)}
        className="font-grotesk text-[10px] font-bold text-yellow-400/70 hover:text-yellow-400 hover:underline tracking-wider ml-2 transition-colors"
      >
        {expanded ? "LESS" : "MORE"}
      </button>
    </span>
  );
}

export function BackstoryTimeline({
  airport,
  wiki,
}: {
  airport: { openedYear?: number | null; lastMajorReno?: number | null };
  wiki: {
    terminalNames?: string[] | null;
    renovationNotes?: string | null;
    skytraxHistory?: unknown;
  };
}) {
  const events: TimelineEvent[] = [];

  if (airport.openedYear) {
    events.push({
      year: airport.openedYear,
      label: "Opened",
      color: "text-green-500",
    });
  }

  if (airport.lastMajorReno) {
    events.push({
      year: airport.lastMajorReno,
      label: "Major Renovation",
      detail: wiki.renovationNotes ?? undefined,
      color: "text-yellow-400",
    });
  }

  if (wiki.skytraxHistory && typeof wiki.skytraxHistory === "object") {
    for (const [year, stars] of Object.entries(
      wiki.skytraxHistory as SkytraxHistory,
    )) {
      events.push({
        year: parseInt(year),
        label: `${stars}-Star Skytrax Rating`,
        color: "text-yellow-400",
      });
    }
  }

  events.sort((a, b) => a.year - b.year);

  if (
    events.length === 0 &&
    !wiki.terminalNames?.length &&
    !wiki.renovationNotes
  ) {
    return null;
  }

  return (
    <div className="flex flex-col gap-0">
      {wiki.terminalNames && wiki.terminalNames.length > 0 && (
        <div className="flex gap-2 items-center mb-4">
          <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
            TERMINALS:
          </span>
          <span className="font-mono text-xs text-zinc-400">
            {wiki.terminalNames.join(" · ")}
          </span>
        </div>
      )}

      {events.length > 0 && (
        <div className="flex flex-col">
          {events.map((ev, i) => (
            <div key={`${ev.year}-${i}`} className="flex gap-4 items-start">
              <div className="flex flex-col items-center">
                <span className="font-mono text-xs font-bold text-zinc-400 tabular-nums w-12 shrink-0">
                  {ev.year}
                </span>
                {i < events.length - 1 && (
                  <div className="w-px h-6 bg-zinc-800 mt-1" />
                )}
              </div>
              <div className="flex flex-col gap-0.5 pb-4">
                <span
                  className={`font-grotesk text-[11px] font-bold ${ev.color} tracking-wider uppercase`}
                >
                  {ev.label}
                </span>
                {ev.detail && <TruncatedText text={ev.detail} />}
              </div>
            </div>
          ))}
        </div>
      )}

      {wiki.renovationNotes && !airport.lastMajorReno && (
        <div className="flex flex-col gap-2 mt-2">
          <span className="font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase">
            History
          </span>
          <TruncatedText text={wiki.renovationNotes} maxLength={250} />
        </div>
      )}
    </div>
  );
}
