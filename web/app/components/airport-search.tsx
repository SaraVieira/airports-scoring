import { useState, useRef } from "react";
import { useRouter } from "@tanstack/react-router";
import { Command } from "cmdk";
import { searchAirports } from "../server/search";
import { scoreColor } from "~/utils/scoring";

export function AirportSearch({ compact = false }: { compact?: boolean }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<
    {
      iataCode: string | null;
      name: string;
      city: string;
      countryCode: string;
      scoreTotal: string | null;
    }[]
  >([]);
  const [open, setOpen] = useState(false);
  const router = useRouter();
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const handleSearch = async (value: string) => {
    setQuery(value);
    clearTimeout(debounceRef.current);

    if (value.length >= 1) {
      debounceRef.current = setTimeout(async () => {
        const res = await searchAirports({ data: value });
        setResults(res);
        setOpen(res.length > 0);
      }, 150);
    } else {
      setResults([]);
      setOpen(false);
    }
  };

  const handleSelect = (iata: string) => {
    setOpen(false);
    setQuery("");
    router.navigate({ to: `/airport/${iata}` });
  };

  return (
    <div className={`relative ${compact ? "w-72" : "w-[560px]"}`}>
      <Command className="w-full" shouldFilter={false} loop>
        <div
          className={`flex items-center gap-3 px-4 bg-[#1a1a1c] border border-white/[0.08] transition-colors focus-within:border-yellow-400/40 ${compact ? "h-9" : "h-14"}`}
        >
          <svg
            width={compact ? 14 : 16}
            height={compact ? 14 : 16}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-zinc-500 shrink-0"
          >
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          <Command.Input
            value={query}
            onValueChange={handleSearch}
            placeholder={
              compact
                ? "Search airports..."
                : "Search airports by name, IATA, or city..."
            }
            className={`flex-1 bg-transparent font-mono text-zinc-300 placeholder:text-zinc-600 outline-none ${compact ? "text-xs" : "text-[13px]"}`}
            onFocus={() => {
              if (results.length > 0) setOpen(true);
            }}
            onBlur={() => {
              setTimeout(() => setOpen(false), 200);
            }}
          />
        </div>

        {open && results.length > 0 && (
          <Command.List className="absolute top-full left-0 right-0 z-50 mt-1 bg-[#111113] border border-white/[0.08] py-1 max-h-[320px] overflow-y-auto">
            {results.map((airport) => {
              const score = airport.scoreTotal
                ? parseFloat(airport.scoreTotal)
                : null;
              return (
                <Command.Item
                  key={airport.iataCode}
                  value={airport.iataCode ?? ""}
                  onSelect={() => airport.iataCode && handleSelect(airport.iataCode)}
                  className="flex items-center gap-3 px-4 py-2.5 cursor-pointer hover:bg-white/[0.04] data-[selected=true]:bg-white/[0.04]"
                >
                  <span className="font-grotesk text-[13px] font-bold text-[#f5f5f0] tracking-wider w-10 shrink-0">
                    {airport.iataCode}
                  </span>
                  <span className="font-mono text-xs text-zinc-500 flex-1 truncate">
                    {airport.name}
                  </span>
                  <span
                    className={`font-grotesk text-sm font-bold w-8 shrink-0 text-right tabular-nums ${scoreColor(score)}`}
                  >
                    {score != null ? Math.round(score) : "—"}
                  </span>
                  <span className="font-mono text-[10px] text-zinc-700 w-24 shrink-0 text-right">
                    {airport.city}, {airport.countryCode}
                  </span>
                </Command.Item>
              );
            })}
          </Command.List>
        )}
      </Command>
    </div>
  );
}
