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

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "airports.report" },
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
