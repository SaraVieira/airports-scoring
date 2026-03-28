import {
  createRootRoute,
  Outlet,
  HeadContent,
  Scripts,
  Link,
  useRouterState,
} from "@tanstack/react-router";
import { AirportSearch } from "../components/airport-search";
import "../styles.css";

const SITE_NAME = "airports.report";
const SITE_DESC = "Opinionated scoring and intelligence for European airports. Delays, sentiment, connectivity, and more — backed by data, delivered with snark.";
const SITE_TITLE = "airports.report — European Airport Scores, Delays & Sentiment Rankings";
const SITE_URL = "https://airports.report";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: SITE_TITLE },
      { name: "description", content: SITE_DESC },
      { name: "theme-color", content: "#0a0a0b" },
      // Open Graph
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: SITE_NAME },
      { property: "og:title", content: SITE_NAME },
      { property: "og:description", content: SITE_DESC },
      { property: "og:url", content: SITE_URL },
      // Twitter
      { name: "twitter:card", content: "summary_large_image" },
      { name: "twitter:title", content: SITE_NAME },
      { name: "twitter:description", content: SITE_DESC },
    ],
    links: [
      { rel: "canonical", href: SITE_URL },
    ],
    scripts: [
      {
        type: "application/ld+json",
        children: JSON.stringify({
          "@context": "https://schema.org",
          "@type": "WebSite",
          name: SITE_NAME,
          url: SITE_URL,
          description: SITE_DESC,
          potentialAction: {
            "@type": "SearchAction",
            target: `${SITE_URL}/airport/{search_term_string}`,
            "query-input": "required name=search_term_string",
          },
        }),
      },
    ],
  }),
  component: RootComponent,
});

function RootComponent() {
  const { location } = useRouterState();
  const isAdmin = location.pathname.startsWith("/admin");

  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body className="dark bg-background text-foreground">
        {!isAdmin && (
          <nav className="fixed top-0 left-0 right-0 z-50 h-12 bg-[#0a0a0b]/90 backdrop-blur-sm border-b border-white/[0.05]">
            <div className="max-w-5xl mx-auto px-16 h-full flex items-center justify-between">
              <div className="flex items-center gap-6">
                <Link
                  to="/"
                  className="font-grotesk text-sm font-bold text-zinc-400 tracking-wider hover:text-zinc-100 transition-colors"
                >
                  airports.report
                </Link>
                <Link
                  to="/countries"
                  className="text-xs font-medium text-zinc-500 hover:text-zinc-200 transition-colors"
                >
                  Countries
                </Link>
                <Link
                  to="/rankings"
                  className="text-xs font-medium text-zinc-500 hover:text-zinc-200 transition-colors"
                >
                  All airports
                </Link>
                <Link
                  to="/operators"
                  className="text-xs font-medium text-zinc-500 hover:text-zinc-200 transition-colors"
                >
                  Operators
                </Link>
              </div>
              <AirportSearch compact />
            </div>
          </nav>
        )}
        <Outlet />
        <Scripts />
      </body>
    </html>
  );
}
