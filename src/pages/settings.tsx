import { useTheme } from "@/components/theme-provider";
import { cn } from "@/lib/utils";

const themes = ["light", "dark", "system"] as const;

export function SettingsPage() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-muted-foreground">App preferences</p>
      </div>
      <div className="space-y-4">
        <h2 className="text-lg font-medium">Appearance</h2>
        <div className="flex gap-3">
          {themes.map((t) => (
            <button
              key={t}
              onClick={() => setTheme(t)}
              className={cn(
                "rounded-lg border px-4 py-2 text-sm capitalize transition-colors",
                theme === t
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:border-foreground/20",
              )}
            >
              {t}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
