import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import {
  Camera,
  History as HistoryIcon,
  Keyboard,
  Settings as SettingsIcon,
} from "lucide-react";
import { tauri, type HelloReply, type SnipResult } from "@/lib/invoke";
import { useHotkeyStore } from "@/state/hotkey-store";
import { strings } from "@/strings";

/**
 * Main window — the user-visible "home" of SnipTeX. Provides
 * discoverable buttons for the four core actions plus a status line
 * showing the latest snip. The global hotkey + tray still drive the
 * snip pipeline (PreviewWindow owns those listeners); the buttons
 * here are a convenience entry point.
 */
type SnipStatus = "idle" | "capturing" | "processing" | "error";

export default function App() {
  const [hello, setHello] = useState<HelloReply | null>(null);
  const [lastSnip, setLastSnip] = useState<SnipResult | null>(null);
  // Driven by the Rust-emitted `snip-state` event so the button reflects
  // ANY snip trigger — button click, global hotkey, or tray menu — not
  // just clicks on this button.
  const [snipStatus, setSnipStatus] = useState<SnipStatus>("idle");
  const { pressCount, lastPressedAt } = useHotkeyStore();

  useEffect(() => {
    tauri.hello(strings.app.name).then(setHello).catch(console.error);
  }, []);

  useEffect(() => {
    let cancelled = false;
    let offComplete: UnlistenFn | undefined;
    let offState: UnlistenFn | undefined;

    listen<SnipResult>("snip-complete", (event) => {
      if (cancelled) return;
      setLastSnip(event.payload);
    })
      .then((fn) => {
        if (cancelled) fn();
        else offComplete = fn;
      })
      .catch((err) =>
        console.error("[main] snip-complete listen failed", err),
      );

    listen<SnipStatus>("snip-state", (event) => {
      if (cancelled) return;
      setSnipStatus(event.payload);
    })
      .then((fn) => {
        if (cancelled) fn();
        else offState = fn;
      })
      .catch((err) =>
        console.error("[main] snip-state listen failed", err),
      );

    return () => {
      cancelled = true;
      offComplete?.();
      offState?.();
    };
  }, []);

  const snipping = snipStatus !== "idle" && snipStatus !== "error";

  const handleSnip = async () => {
    if (snipping) return;
    try {
      const result = await tauri.runSnip();
      if (result.status === "cancelled") toast("Snip cancelled");
      // On success Rust emits `snip-complete` → PreviewWindow renders.
      // `snip-state` events handle button label across the whole flow.
    } catch (err) {
      toast.error("Snip failed", { description: String(err) });
    }
  };

  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col gap-6 px-6 py-10 text-center">
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight">
          {strings.app.name}
        </h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">
          {strings.app.tagline}
        </p>
      </header>

      <section className="rounded-lg border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-700 dark:bg-slate-900">
        <button
          type="button"
          onClick={handleSnip}
          disabled={snipping}
          className="mx-auto inline-flex items-center gap-2 rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white"
        >
          <Camera className="size-4" />
          {snipStatus === "capturing"
            ? "Capturing…"
            : snipStatus === "processing"
            ? "Processing…"
            : "Snip now"}
        </button>
        <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
          Or press{" "}
          <kbd className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[11px] dark:bg-slate-800">
            <Keyboard className="mr-1 inline size-3" />
            Cmd/Ctrl + Shift + M
          </kbd>{" "}
          anywhere.
        </p>
        <p className="mt-2 text-[11px] text-slate-400">
          Hotkey presses detected: <strong>{pressCount}</strong>
          {lastPressedAt && (
            <> · last {new Date(lastPressedAt).toLocaleTimeString()}</>
          )}
        </p>
      </section>

      <section className="grid grid-cols-2 gap-3">
        <button
          type="button"
          onClick={() => void tauri.showWindow("history")}
          className="flex items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 py-2 text-sm text-slate-700 transition hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:bg-slate-800"
        >
          <HistoryIcon className="size-4" /> Show history
        </button>
        <button
          type="button"
          onClick={() => void tauri.showWindow("settings")}
          className="flex items-center justify-center gap-2 rounded-md border border-slate-200 bg-white px-3 py-2 text-sm text-slate-700 transition hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:bg-slate-800"
        >
          <SettingsIcon className="size-4" /> Open settings
        </button>
      </section>

      {lastSnip?.status === "ok" && lastSnip.text && (
        <section className="rounded-md border border-slate-200 bg-white p-4 text-left dark:border-slate-700 dark:bg-slate-900">
          <div className="mb-2 flex items-center gap-2 text-[10px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
            <span>Latest snip</span>
            {lastSnip.agent && (
              <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono dark:bg-slate-800">
                {lastSnip.agent}
              </span>
            )}
            {lastSnip.detected && (
              <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono dark:bg-slate-800">
                {lastSnip.detected.replace("_", " ").toLowerCase()}
              </span>
            )}
          </div>
          <pre className="max-h-48 overflow-auto font-mono text-xs whitespace-pre-wrap text-slate-700 dark:text-slate-200">
            {lastSnip.text}
          </pre>
        </section>
      )}

      {hello && (
        <footer className="text-[11px] text-slate-400">
          {hello.message} v{hello.version}
        </footer>
      )}

      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}
