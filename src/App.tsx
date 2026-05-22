import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { tauri, type HelloReply, type SnipResult } from "@/lib/invoke";
import { useHotkeyStore } from "@/state/hotkey-store";

export default function App() {
  const [hello, setHello] = useState<HelloReply | null>(null);
  const [lastSnip, setLastSnip] = useState<SnipResult | null>(null);
  const { pressCount, lastPressedAt, recordPress } = useHotkeyStore();
  // Guard against re-entrant snips: a second hotkey press while the overlay
  // is up would race two run_snip calls against the single overlay window.
  const snipInFlight = useRef(false);

  useEffect(() => {
    tauri.hello("SnipTeX").then(setHello).catch(console.error);
  }, []);

  useEffect(() => {
    // StrictMode double-invokes effects in dev; the listen() promise can resolve
    // after cleanup runs, so we guard with a cancelled flag and unlisten
    // immediately if the effect was already torn down.
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    listen("hotkey-pressed", async () => {
      recordPress();
      if (snipInFlight.current) return;
      snipInFlight.current = true;
      try {
        const result = await tauri.runSnip();
        setLastSnip(result);
        if (result.status === "ok" && result.text) {
          toast.success(`OCR via ${result.agent ?? "agent"}`, {
            description:
              result.text.length > 80
                ? result.text.slice(0, 80) + "…"
                : result.text,
          });
        } else if (result.status === "cancelled") {
          toast("Snip cancelled");
        }
      } catch (err) {
        toast.error("Snip failed", { description: String(err) });
      } finally {
        snipInFlight.current = false;
      }
    })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(console.error);

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [recordPress]);

  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col items-center justify-center gap-6 px-6 text-center">
      <h1 className="text-4xl font-semibold tracking-tight">SnipTeX</h1>
      <p className="text-slate-500 dark:text-slate-400">
        Free OCR snip tool for LaTeX and Markdown.
      </p>

      {hello ? (
        <p className="rounded-md border border-slate-200/60 bg-white/60 px-4 py-2 text-sm shadow-sm dark:border-slate-700/60 dark:bg-slate-900/60">
          {hello.message}{" "}
          <span className="text-slate-400">v{hello.version}</span>
        </p>
      ) : (
        <p className="text-sm text-slate-400">Booting Rust backend…</p>
      )}

      <div className="rounded-md border border-dashed border-slate-300 px-4 py-3 text-sm dark:border-slate-700">
        Press{" "}
        <kbd className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-xs dark:bg-slate-800">
          Cmd/Ctrl + Shift + M
        </kbd>{" "}
        to snip a region, OCR it, and see the result here.
        <div className="mt-2 text-xs text-slate-500">
          Detected presses: <strong>{pressCount}</strong>
          {lastPressedAt && (
            <>
              {" · last "}
              {new Date(lastPressedAt).toLocaleTimeString()}
            </>
          )}
        </div>
      </div>

      {lastSnip?.status === "ok" && lastSnip.text && (
        <pre className="mt-2 max-h-64 w-full max-w-2xl overflow-auto rounded-md border border-slate-200 bg-white/80 p-3 text-left font-mono text-xs whitespace-pre-wrap dark:border-slate-700 dark:bg-slate-900/80">
          {lastSnip.text}
        </pre>
      )}

      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}
