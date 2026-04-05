import { useEffect, useState } from "react";

export function TimeSince({ timestamp }: { timestamp: number }) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

  const secs = Math.max(0, Math.floor((now - timestamp * 1000) / 1000));
  const label = secs < 60 ? `${secs}s ago` : `${Math.floor(secs / 60)}m ago`;

  return (
    <span className="text-[10px] text-muted-foreground font-mono">{label}</span>
  );
}
