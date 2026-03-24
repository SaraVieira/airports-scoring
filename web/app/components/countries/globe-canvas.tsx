import { useState, useEffect } from "react";
import type { CountrySummary } from "~/api/client";
import { scoreHex } from "~/utils/scoring";

// All Three.js/R3F imports are deferred to client-side only.
// This prevents SSR from touching `window`.

let loaded = false;
let modules: {
  Canvas: any;
  OrbitControls: any;
  R3fGlobe: any;
} | null = null;

async function loadModules() {
  if (modules) return modules;
  const [fiber, drei, globe] = await Promise.all([
    import("@react-three/fiber"),
    import("@react-three/drei"),
    import("r3f-globe"),
  ]);
  modules = {
    Canvas: fiber.Canvas,
    OrbitControls: drei.OrbitControls,
    R3fGlobe: globe.default,
  };
  loaded = true;
  return modules;
}

function latLngToCamera(
  lat: number,
  lng: number,
  dist: number,
): [number, number, number] {
  const phi = (90 - lat) * (Math.PI / 180);
  const theta = (lng + 180) * (Math.PI / 180);
  return [
    -(dist * Math.sin(phi) * Math.cos(theta)),
    dist * Math.cos(phi),
    dist * Math.sin(phi) * Math.sin(theta),
  ];
}

export default function GlobeCanvas({
  countries,
  selected,
  onSelect,
  onHover,
}: {
  countries: CountrySummary[];
  selected: string | null;
  onSelect: (code: string) => void;
  onHover: (country: CountrySummary | null) => void;
}) {
  const [ready, setReady] = useState(loaded);

  useEffect(() => {
    if (!ready) {
      loadModules().then(() => setReady(true));
    }
  }, [ready]);

  if (!ready || !modules) return null;

  const { Canvas, OrbitControls, R3fGlobe } = modules;

  const pointsData = countries.filter((c) => c.lat != null && c.lng != null);

  const ringsData = selected
    ? countries.filter((c) => c.code === selected && c.lat != null)
    : [];

  return (
    <Canvas flat camera={{ fov: 50, position: latLngToCamera(48, -60, 220) }}>
      <OrbitControls
        enablePan={false}
        minDistance={101}
        maxDistance={1e4}
        dampingFactor={0.1}
        zoomSpeed={0.3}
        rotateSpeed={0.3}
      />
      <color attach="background" args={["#0a0a0b"]} />
      <ambientLight color={0xcccccc} intensity={Math.PI} />
      <directionalLight intensity={0.6 * Math.PI} />
      <R3fGlobe
        globeImageUrl="//cdn.jsdelivr.net/npm/three-globe/example/img/earth-night.jpg"
        showAtmosphere={true}
        atmosphereColor="#1a3a6a"
        atmosphereAltitude={0.15}
        pointsData={pointsData}
        pointLat="lat"
        pointLng="lng"
        pointColor={(d: any) => scoreHex(d.avgScore)}
        pointAltitude={(d: any) => (d.code === selected ? 0.06 : 0.02)}
        pointRadius={(d: any) => (d.code === selected ? 0.8 : 0.4)}
        pointResolution={16}
        ringsData={ringsData}
        ringLat="lat"
        ringLng="lng"
        ringColor={() => scoreHex(ringsData[0]?.avgScore)}
        ringMaxRadius={2}
        ringPropagationSpeed={1}
        ringRepeatPeriod={1200}
        onClick={(_layer: string, d: any) => {
          if (d?.code) onSelect(d.code);
        }}
        onHover={(_layer: string | undefined, d: any) => {
          onHover(d ?? null);
        }}
      />
    </Canvas>
  );
}
