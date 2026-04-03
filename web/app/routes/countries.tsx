import { useState, useMemo, useCallback, useEffect } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { api, type CountrySummary } from "~/api/client";
import { scoreHex } from "~/utils/scoring";
import { HeadToHead } from "~/components/countries/head-to-head";
import { CountryPicker } from "~/components/countries/country-picker";
import { CountryDetailPanel } from "~/components/countries/country-detail";
import { EmptyPanel } from "~/components/countries/empty";

let GlobeCanvas: React.ComponentType<any> | null = null;

function useGlobeCanvas() {
  const [Comp, setComp] = useState<React.ComponentType<any> | null>(
    GlobeCanvas,
  );
  useEffect(() => {
    if (!GlobeCanvas) {
      import("~/components/countries/globe-canvas").then((m) => {
        GlobeCanvas = m.default;
        setComp(() => m.default);
      });
    }
  }, []);
  return Comp;
}

// --- Server function ---

const getCountries = createServerFn({ method: "GET" }).handler(async () => {
  const [countries, airports] = await Promise.all([
    api.listCountries(),
    api.listAirports(),
  ]);
  return { countries, airports };
});

export const Route = createFileRoute("/countries")({
  loader: () => getCountries(),
  head: () => ({
    meta: [
      { title: "Countries — airports.report" },
      {
        name: "description",
        content:
          "Compare European countries by airport quality. Average scores, passenger volumes, sentiment, and on-time performance by country.",
      },
      { property: "og:title", content: "Countries — airports.report" },
      {
        property: "og:description",
        content: "Compare European countries by airport quality.",
      },
      { property: "og:url", content: "https://airports.report/countries" },
    ],
    links: [{ rel: "canonical", href: "https://airports.report/countries" }],
  }),
  component: CountriesPage,
});

export const getFlag = (code: string) =>
  `https://flagcdn.com/w80/${code.toLowerCase()}.png`;

function CountriesPage() {
  const { countries, airports } = Route.useLoaderData();
  const [selected, setSelected] = useState<string | null>(null);
  const [hovered, setHovered] = useState<CountrySummary | null>(null);
  const Globe = useGlobeCanvas();
  const [compareMode, setCompareMode] = useState(false);
  const [compareRight, setCompareRight] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);

  const selectedCountry = useMemo(
    () => countries.find((c) => c.code === selected) ?? null,
    [countries, selected],
  );

  const selectedAirports = useMemo(
    () => (selected ? airports.filter((a) => a.countryCode === selected) : []),
    [airports, selected],
  );

  const compareLeftCountry = selectedCountry;
  const compareRightCountry = useMemo(
    () => countries.find((c) => c.code === compareRight) ?? null,
    [countries, compareRight],
  );

  const handleCompare = useCallback(() => {
    setPicking(true);
  }, []);

  const handlePick = useCallback((code: string) => {
    setCompareRight(code);
    setCompareMode(true);
    setPicking(false);
  }, []);

  const handleBackFromCompare = useCallback(() => {
    setCompareMode(false);
    setCompareRight(null);
  }, []);

  // Head to head view
  if (compareMode && compareLeftCountry && compareRightCountry) {
    return (
      <div className="min-h-screen bg-[#0a0a0b] text-zinc-100 pt-16 px-16 pb-8">
        <HeadToHead
          left={compareLeftCountry}
          right={compareRightCountry}
          onBack={handleBackFromCompare}
        />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100 relative">
      {picking && selected && (
        <CountryPicker
          countries={countries}
          exclude={selected}
          onPick={handlePick}
          onCancel={() => setPicking(false)}
        />
      )}

      <div className="flex h-screen pt-12">
        <div className="flex-1 flex flex-col">
          <div className="px-10 pt-8 pb-4">
            <span className="text-[10px] font-semibold tracking-wider text-zinc-500">
              COUNTRY RANKINGS
            </span>
            <h1 className="text-2xl font-semibold text-zinc-100">
              The Global Picture
            </h1>
            <p className="text-xs text-zinc-600 mt-1">
              {countries.reduce((sum, c) => sum + c.airportCount, 0)} airports ·{" "}
              {countries.length} countries
            </p>
          </div>
          <div className="flex-1 relative">
            {Globe ? (
              <Globe
                countries={countries}
                selected={selected}
                onSelect={setSelected}
                onHover={setHovered}
              />
            ) : (
              <div className="flex-1 flex items-center justify-center text-zinc-600 text-sm">
                Loading globe...
              </div>
            )}
            {hovered && hovered.code !== selected && (
              <div className="absolute top-4 left-4 bg-zinc-900/90 border border-zinc-700 rounded-lg px-3 py-2 flex items-center gap-2 pointer-events-none backdrop-blur-sm">
                <img
                  src={getFlag(hovered.code)}
                  alt={`${hovered.name} flag`}
                  className="w-5 h-4 object-cover rounded-sm"
                />
                <span className="text-sm font-medium text-zinc-100">
                  {hovered.name}
                </span>
                <span
                  className="text-sm font-bold"
                  style={{ color: scoreHex(hovered.avgScore) }}
                >
                  {hovered.avgScore != null
                    ? Math.round(hovered.avgScore)
                    : "—"}
                </span>
                <span className="text-xs text-zinc-500">
                  {hovered.airportCount} airports
                </span>
              </div>
            )}
          </div>
          <p className="text-center text-[11px] text-zinc-700 pb-4">
            Click a country to explore · Drag to rotate
          </p>
        </div>

        <div className="w-120 border-l border-zinc-800 bg-[#111113] p-7 overflow-y-auto">
          {selectedCountry ? (
            <CountryDetailPanel
              country={selectedCountry}
              airports={selectedAirports}
              onCompare={handleCompare}
            />
          ) : (
            <EmptyPanel countries={countries} onSelect={setSelected} />
          )}
        </div>
      </div>
    </div>
  );
}
