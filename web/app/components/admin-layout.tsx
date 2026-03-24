import { Link, useRouterState } from "@tanstack/react-router";

const NAV_ITEMS = [
  { to: "/admin", label: "Dashboard" },
  { to: "/admin/jobs", label: "Jobs" },
  { to: "/admin/airports", label: "Airports" },
] as const;

export function AdminLayout({
  children,
  title,
}: {
  children: React.ReactNode;
  title: string;
}) {
  const { location } = useRouterState();

  const handleLogout = () => {
    localStorage.removeItem("admin_password");
    window.location.reload();
  };

  return (
    <div className="min-h-screen bg-[#0a0a0b] text-zinc-100">
      <div className="max-w-5xl mx-auto px-16 pt-20 pb-8">
        <div className="flex items-center justify-between mb-8">
          <h1 className="font-grotesk text-xl font-bold">{title}</h1>
          <div className="flex items-center gap-4">
            {NAV_ITEMS.map((item) => {
              const isActive = location.pathname === item.to;
              return (
                <Link
                  key={item.to}
                  to={item.to}
                  className={`font-mono text-xs ${
                    isActive
                      ? "text-zinc-100"
                      : "text-zinc-400 hover:text-zinc-100"
                  }`}
                >
                  {item.label}
                </Link>
              );
            })}
            <button
              onClick={handleLogout}
              className="font-mono text-xs text-zinc-500 hover:text-zinc-300"
            >
              Logout
            </button>
          </div>
        </div>
        {children}
      </div>
    </div>
  );
}
