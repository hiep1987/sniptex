import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Toaster } from "sonner";
import { cn } from "@/lib/cn";
import { strings } from "@/strings";
import { useSettingsStore } from "@/stores/settings-store";
import GeneralTab from "./settings/general-tab";
import AgentsTab from "./settings/agents-tab";
import HotkeysTab from "./settings/hotkeys-tab";
import FormatsTab from "./settings/formats-tab";
import AboutTab from "./settings/about-tab";

type TabKey = "general" | "agents" | "hotkeys" | "formats" | "about";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: strings.settings.tabs.general },
  { key: "agents", label: strings.settings.tabs.agents },
  { key: "hotkeys", label: strings.settings.tabs.hotkeys },
  { key: "formats", label: strings.settings.tabs.formats },
  { key: "about", label: strings.settings.tabs.about },
];

const TAB_COMPONENTS: Record<TabKey, React.ComponentType> = {
  general: GeneralTab,
  agents: AgentsTab,
  hotkeys: HotkeysTab,
  formats: FormatsTab,
  about: AboutTab,
};

export default function SettingsWindow() {
  const [active, setActive] = useState<TabKey>("general");
  const fetch = useSettingsStore((s) => s.fetch);

  useEffect(() => { fetch(); }, [fetch]);

  useEffect(() => {
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

  const TabContent = TAB_COMPONENTS[active];

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
        <TabContent />
      </section>
      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}
