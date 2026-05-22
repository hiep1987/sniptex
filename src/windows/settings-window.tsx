import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { cn } from "@/lib/cn";
import { strings } from "@/strings";

type TabKey = "general" | "agents" | "hotkeys" | "formats" | "about";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: strings.settings.tabs.general },
  { key: "agents", label: strings.settings.tabs.agents },
  { key: "hotkeys", label: strings.settings.tabs.hotkeys },
  { key: "formats", label: strings.settings.tabs.formats },
  { key: "about", label: strings.settings.tabs.about },
];

export default function SettingsWindow() {
  const [active, setActive] = useState<TabKey>("general");

  useEffect(() => {
    // Tray's "About SnipTeX" jumps directly to the About tab.
    let cancelled = false;
    let off: UnlistenFn | undefined;

    listen("tray-about", () => {
      if (!cancelled) setActive("about");
    })
      .then((fn) => {
        if (cancelled) fn();
        else off = fn;
      })
      .catch((err) => console.error("[settings] listen failed", err));

    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  return (
    <main className="flex h-dvh w-dvw bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <aside className="w-44 shrink-0 border-r border-slate-200 bg-slate-50 p-2 dark:border-slate-800 dark:bg-slate-900">
        <h1 className="px-2 pb-3 pt-1 text-sm font-semibold tracking-tight">
          {strings.settings.title}
        </h1>
        <nav className="flex flex-col gap-0.5">
          {TABS.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActive(tab.key)}
              className={cn(
                "rounded-md px-2 py-1.5 text-left text-sm transition",
                active === tab.key
                  ? "bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
                  : "text-slate-600 hover:bg-slate-200 dark:text-slate-300 dark:hover:bg-slate-800",
              )}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </aside>
      <section className="flex-1 overflow-auto p-6">
        <SettingsTabPlaceholder tab={active} />
      </section>
    </main>
  );
}

function SettingsTabPlaceholder({ tab }: { tab: TabKey }) {
  const label = TABS.find((t) => t.key === tab)?.label ?? tab;
  return (
    <div className="max-w-xl">
      <h2 className="mb-2 text-lg font-semibold">{label}</h2>
      <p className="text-sm text-slate-500 dark:text-slate-400">
        {strings.settings.comingSoon}
      </p>
    </div>
  );
}
