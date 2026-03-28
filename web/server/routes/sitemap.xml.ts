import { defineEventHandler, setHeader } from "vinxi/http";

const API_URL = process.env.VITE_API_URL || "http://localhost:3001";
const API_KEY = process.env.VITE_API_KEY || "";
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
  if (!res.ok) throw new Error(`API ${res.status}`);
  return res.json();
}

export default defineEventHandler(async (event) => {
  setHeader(event, "content-type", "application/xml; charset=utf-8");
  setHeader(event, "cache-control", "public, max-age=3600, s-maxage=3600");

  try {
    const [airports, operators, countries] = await Promise.all([
      apiFetch<AirportListItem[]>("/api/airports"),
      apiFetch<OperatorListItem[]>("/api/operators"),
      apiFetch<CountrySummary[]>("/api/countries"),
    ]);

    const today = new Date().toISOString().split("T")[0];

    const urls = [
      // Static pages
      { loc: SITE_URL, priority: "1.0", changefreq: "daily" },
      { loc: `${SITE_URL}/rankings`, priority: "0.9", changefreq: "weekly" },
      { loc: `${SITE_URL}/operators`, priority: "0.8", changefreq: "weekly" },
      { loc: `${SITE_URL}/countries`, priority: "0.8", changefreq: "weekly" },
      // Airport pages
      ...airports.map((a) => ({
        loc: `${SITE_URL}/airport/${a.iataCode}`,
        priority: "0.7",
        changefreq: "weekly",
      })),
      // Operator pages
      ...operators.map((o) => ({
        loc: `${SITE_URL}/operators/${o.id}`,
        priority: "0.6",
        changefreq: "monthly",
      })),
      // Country pages
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

    return xml;
  } catch (error) {
    console.error("Sitemap generation failed:", error);
    return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>${SITE_URL}</loc>
    <priority>1.0</priority>
  </url>
</urlset>`;
  }
});
