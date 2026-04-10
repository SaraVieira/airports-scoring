import { useAuthStore } from "~/stores/admin";
import { useEffect, useRef, useState } from "react";
import { Card } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Pause, Play, Trash2 } from "lucide-react";

interface LogLine {
  timestamp: string;
  level: string;
  message: string;
  target?: string;
  fields?: Record<string, string | number | boolean>;
}

function levelColor(level: string): string {
  switch (level) {
    case "ERROR":
      return "text-destructive";
    case "WARN":
      return "text-yellow-500";
    case "INFO":
      return "text-muted-foreground";
    case "DEBUG":
      return "text-muted-foreground/50";
    default:
      return "text-muted-foreground";
  }
}

function messageColor(level: string): string {
  switch (level) {
    case "ERROR":
      return "text-destructive";
    case "WARN":
      return "text-yellow-500";
    default:
      return "text-foreground/70";
  }
}

export function LogTerminal() {
  const [lines, setLines] = useState<LogLine[]>([]);
  const [connected, setConnected] = useState(false);
  const [paused, setPaused] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    // SSE connects directly to the Rust API since EventSource can't go through
    // server functions. In production, VITE_PUBLIC_API_URL must be set to the
    // public API URL, or the API must be accessible from the browser.
    // Note: password in query param will appear in server logs.
    const apiUrl =
      import.meta.env.VITE_PUBLIC_API_URL ||
      import.meta.env.VITE_API_URL ||
      "http://localhost:3001";
    const password = useAuthStore.getState().password || "";
    const url = `${apiUrl}/api/admin/logs/stream?password=${encodeURIComponent(password)}`;

    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);

    es.onmessage = (event) => {
      try {
        const line: LogLine = JSON.parse(event.data);
        setLines((prev) => {
          const next = [...prev, line];
          return next.length > 500 ? next.slice(-500) : next;
        });
      } catch {
        // ignore malformed
      }
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, []);

  // Auto-scroll
  useEffect(() => {
    if (!paused && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines, paused]);

  const handleClear = () => setLines([]);

  return (
    <Card className="flex flex-col h-full overflow-hidden border-border">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border bg-card">
        <div className="flex items-center gap-2">
          <div
            className={`size-2 rounded-full ${connected ? "bg-green-500" : "bg-destructive"}`}
          />
          <span className="text-xs font-medium text-muted-foreground">
            Live Logs
          </span>
          {connected && (
            <Badge variant="secondary" className="text-[10px]">
              streaming
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="xs"
            onClick={() => setPaused(!paused)}
          >
            {paused ? <Play className="size-3" /> : <Pause className="size-3" />}
          </Button>
          <Button variant="ghost" size="xs" onClick={handleClear}>
            <Trash2 className="size-3" />
          </Button>
        </div>
      </div>
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto p-3 font-mono text-[11px] leading-5 scrollbar-thin"
      >
        {lines.length === 0 && (
          <span className="text-muted-foreground/50">
            Waiting for log output...
          </span>
        )}
        {lines.map((line, i) => (
          <div key={i} className="flex gap-2 hover:bg-muted/30 px-1 -mx-1 rounded">
            <span className="text-muted-foreground/50 shrink-0">
              {line.timestamp.includes("T")
                ? line.timestamp.split("T")[1]?.slice(0, 8)
                : line.timestamp}
            </span>
            <span className={`shrink-0 w-12 ${levelColor(line.level)}`}>
              {line.level.padEnd(5)}
            </span>
            <span className={messageColor(line.level)}>
              {line.message}
              {line.fields && Object.keys(line.fields).length > 0 && (
                <span className="text-muted-foreground/50">
                  {" "}
                  {Object.entries(line.fields)
                    .map(([k, v]) => `${k}=${v}`)
                    .join(" ")}
                </span>
              )}
            </span>
          </div>
        ))}
      </div>
    </Card>
  );
}
