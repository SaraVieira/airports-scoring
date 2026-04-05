import { useEffect, useState, useCallback } from "react";
import { getLivePulse } from "~/server/get-live-pulse";
import type { LivePulseResponse } from "~/api/client";
import { HeaderText } from "../header-text";
import { TimeSince } from "./time-since";
import { RadarScope } from "./radar-scope";
import { COLOR_LABELS, POLL_INTERVAL } from "./const";

export function LivePulse({ iata }: { iata: string }) {
  const [data, setData] = useState<LivePulseResponse | null>(null);

  const fetchPulse = useCallback(async () => {
    const result = await getLivePulse({ data: iata });
    setData(result as LivePulseResponse | null);
  }, [iata]);

  useEffect(() => {
    fetchPulse();
    const interval = setInterval(fetchPulse, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [fetchPulse]);

  if (!data || data.counts.total === 0) return null;

  return (
    <div className="flex flex-col">
      <HeaderText>The Skies above</HeaderText>

      <div className="flex flex-col sm:flex-row items-center gap-6 py-6">
        <RadarScope data={data} />

        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <span className="relative flex h-2.5 w-2.5">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
              <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-green-500" />
            </span>
            <span className="font-grotesk text-xs font-bold uppercase tracking-widest text-green-400">
              Live
            </span>
            <TimeSince timestamp={data.timestamp} />
          </div>

          <div className="flex items-baseline gap-4">
            {data.counts.arriving > 0 && (
              <div className="flex items-baseline gap-1.5">
                <span className="font-mono text-2xl font-bold text-green-400">
                  {data.counts.arriving}
                </span>
                <span className="text-xs text-muted-foreground">arriving</span>
              </div>
            )}
            {data.counts.departing > 0 && (
              <div className="flex items-baseline gap-1.5">
                <span className="font-mono text-2xl font-bold text-yellow-400">
                  {data.counts.departing}
                </span>
                <span className="text-xs text-muted-foreground">departing</span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-3 text-xs text-muted-foreground font-mono">
            <span>{data.counts.total} aircraft nearby</span>
            <span className="text-border">|</span>
            <span>{data.counts.inAir} in air</span>
            <span className="text-border">|</span>
            <span>{data.counts.onGround} on ground</span>
          </div>

          <div className="flex items-center gap-3 mt-1">
            {COLOR_LABELS.map((l) => (
              <span key={l.label} className="flex items-center gap-1">
                <span
                  className={`inline-block w-1.5 h-1.5 rounded-full ${l.color}`}
                />
                <span className="text-[10px] text-muted-foreground">
                  {l.label}
                </span>
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
