import { useEffect } from "react";
import { useSettingsStore } from "@/stores/settings-store";

export function useTheme() {
  const theme = useSettingsStore((s) => s.theme);

  useEffect(() => {
    const root = document.documentElement;

    const apply = (dark: boolean) => {
      root.classList.toggle("dark", dark);
    };

    // Persist the user's choice so the inline script in index.html can
    // pre-apply the dark class on the next cold start and avoid the
    // light-theme flash while the settings IPC is still in flight.
    try {
      localStorage.setItem("sniptex-theme", theme);
    } catch (_) {
      // localStorage blocked — that's fine, runtime still works.
    }

    if (theme === "dark") {
      apply(true);
      return;
    }
    if (theme === "light") {
      apply(false);
      return;
    }

    // "system" — follow OS preference and listen for changes.
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    apply(mq.matches);

    const handler = (e: MediaQueryListEvent) => apply(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);
}
