import { writeFileSync } from "fs";
import { resolve } from "path";

const API_URL = process.env.VITE_PUBLIC_API_URL || "http://localhost:3001";
const API_KEY = process.env.API_KEY || "";
const SITE_URL = "https://airports.report";

interface AirportListItem {
  iataCode: string;
}

interface OperatorListItem {
  id: number;
}

interface CountrySummary {
  code: string;
}

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    headers: API_KEY ? { "X-API-Key": API_KEY } : {},
  });
  if (!res.ok) throw new Error(`API ${res.status}: ${path}`);
  return res.json();
}

async function main() {
  console.log(`Generating sitemap from API at ${API_URL}...`);

  const [airports, operators, countries] = await Promise.all([
    apiFetch<AirportListItem[]>("/api/airports"),
    apiFetch<OperatorListItem[]>("/api/operators"),
    apiFetch<CountrySummary[]>("/api/countries"),
  ]);

  const today = new Date().toISOString().split("T")[0];

  const urls = [
    { loc: SITE_URL, priority: "1.0", changefreq: "daily" },
    { loc: `${SITE_URL}/rankings`, priority: "0.9", changefreq: "weekly" },
    { loc: `${SITE_URL}/operators`, priority: "0.8", changefreq: "weekly" },
    { loc: `${SITE_URL}/countries`, priority: "0.8", changefreq: "weekly" },
    ...airports.map((a) => ({
      loc: `${SITE_URL}/airport/${a.iataCode}`,
      priority: "0.7",
      changefreq: "weekly",
    })),
    ...operators.map((o) => ({
      loc: `${SITE_URL}/operators/${o.id}`,
      priority: "0.6",
      changefreq: "monthly",
    })),
    ...countries.map((c) => ({
      loc: `${SITE_URL}/countries/${c.code}`,
      priority: "0.6",
      changefreq: "monthly",
    })),
  ];

  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls
  .map(
    (u) => `  <url>
    <loc>${u.loc}</loc>
    <lastmod>${today}</lastmod>
    <changefreq>${u.changefreq}</changefreq>
    <priority>${u.priority}</priority>
  </url>`,
  )
  .join("\n")}
</urlset>`;

  const outPath = resolve(import.meta.dirname, "../public/sitemap.xml");
  writeFileSync(outPath, xml, "utf-8");
  console.log(
    `Sitemap written to public/sitemap.xml (${urls.length} URLs)`,
  );
}

main().catch((err) => {
  console.error("Sitemap generation failed:", err);
  process.exit(1);
});
