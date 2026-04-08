import { NavLink } from "react-router-dom";
import { Mouse, MonitorSmartphone, Settings, Sun, Moon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { useTheme } from "@/components/theme-provider";
import { cn } from "@/lib/utils";

const navItems = [
  { to: "/", icon: Mouse, label: "Devices" },
  { to: "/flow", icon: MonitorSmartphone, label: "Flow", disabled: true },
  { to: "/settings", icon: Settings, label: "Settings" },
] as const;

export function Sidebar() {
  const { resolvedTheme, setTheme } = useTheme();

  return (
    <aside className="flex h-full w-[72px] flex-col border-r border-sidebar-border bg-sidebar-background transition-all duration-200 hover:w-[220px] group/sidebar">
      <div className="flex h-14 items-center justify-center px-4">
        <span className="text-lg font-bold text-primary">L</span>
        <span className="hidden group-hover/sidebar:inline ml-1 text-sm font-semibold text-foreground">
          Logi Linux
        </span>
      </div>

      <nav className="flex-1 space-y-1 px-2 pt-4">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.disabled ? "#" : item.to}
            onClick={item.disabled ? (e) => e.preventDefault() : undefined}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors relative",
                isActive && !item.disabled
                  ? "bg-primary/10 text-primary before:absolute before:left-0 before:top-1/2 before:-translate-y-1/2 before:h-5 before:w-[3px] before:rounded-r before:bg-primary"
                  : "text-muted-foreground hover:bg-secondary hover:text-foreground",
                item.disabled && "opacity-50 cursor-not-allowed",
              )
            }
          >
            <item.icon className="h-5 w-5 shrink-0" />
            <span className="hidden group-hover/sidebar:flex items-center gap-2 whitespace-nowrap">
              {item.label}
              {item.disabled && (
                <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                  Soon
                </Badge>
              )}
            </span>
          </NavLink>
        ))}
      </nav>

      <div className="p-2 mb-2">
        <button
          onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
          className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors"
        >
          {resolvedTheme === "dark" ? (
            <Sun className="h-5 w-5 shrink-0" />
          ) : (
            <Moon className="h-5 w-5 shrink-0" />
          )}
          <span className="hidden group-hover/sidebar:inline whitespace-nowrap">
            {resolvedTheme === "dark" ? "Light mode" : "Dark mode"}
          </span>
        </button>
      </div>
    </aside>
  );
}
