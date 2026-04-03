import { Link, useRouterState } from "@tanstack/react-router";
import { Button } from "~/components/ui/button";
import { LogOut } from "lucide-react";
import { useAuthStore } from "~/stores/admin";

const NAV_ITEMS = [
  { to: "/admin", label: "Dashboard" },
  { to: "/admin/jobs", label: "Jobs" },
  { to: "/admin/airports", label: "Airports" },
  { to: "/admin/operators", label: "Operators" },
  { to: "/admin/data-gaps", label: "Gaps" },
] as const;

export function AdminLayout({
  children,
  title,
  actions,
}: {
  children: React.ReactNode;
  title: string;
  actions?: React.ReactNode;
}) {
  const { location } = useRouterState();
  const logout = useAuthStore((s) => s.logout);

  return (
    <div className="min-h-screen bg-background text-foreground">
      {/* Top nav bar */}
      <nav className="sticky top-0 z-40 border-b border-border bg-background/95 backdrop-blur-sm">
        <div className="max-w-7xl mx-auto px-6 flex h-12 items-center justify-between">
          <div className="flex items-center gap-6">
            <Link
              to="/"
              className="text-sm font-semibold text-muted-foreground hover:text-foreground transition-colors"
            >
              airports.report
            </Link>
            <div className="w-px h-4 bg-border" />
            <div className="flex items-center gap-1">
              {NAV_ITEMS.map((item) => {
                const isActive = location.pathname === item.to;
                return (
                  <Link key={item.to} to={item.to}>
                    <Button
                      variant={isActive ? "secondary" : "ghost"}
                      size="sm"
                      className="text-xs"
                    >
                      {item.label}
                    </Button>
                  </Link>
                );
              })}
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => { logout(); window.location.href = "/admin"; }}
            className="text-xs text-muted-foreground"
          >
            <LogOut className="size-3" />
            Logout
          </Button>
        </div>
      </nav>

      {/* Page content */}
      <div className="max-w-7xl mx-auto px-6 py-6">
        <div className="flex items-center justify-between mb-6">
          <h1 className="font-grotesk text-lg font-bold">{title}</h1>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
        {children}
      </div>
    </div>
  );
}
