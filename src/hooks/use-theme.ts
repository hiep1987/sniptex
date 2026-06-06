import { useEffect } from "react";
import { useSettingsStore } from "@/stores/settings-store";

export function useTheme() {
  const theme = useSettingsStore((s) => s.theme);
  const loaded = useSettingsStore((s) => s.loaded);

  useEffect(() => {
    // The inline pre-mount script in index.html has already applied the
    // right class from localStorage based on the user's previous session.
    // If we run before the backend fetch has landed (loaded=false), the
    // store still holds the in-memory defaults (theme="system"), and
    // touching the DOM here would strip the pre-applied dark class and
    // re-introduce the flash we just removed. Wait until the real value
    // is in the store before mirroring anything back.
    if (!loaded) return;

    const root = document.documentElement;

    const apply = (dark: boolean) => {
      root.classList.toggle("dark", dark);
    };

    // Persist the user's choice so the inline script in index.html can
    // pre-apply the dark class on the next cold start.
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
  }, [theme, loaded]);
}
