import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { tauri, type HelloReply } from "@/lib/invoke";
import { useHotkeyStore } from "@/state/hotkey-store";

export default function App() {
  const [hello, setHello] = useState<HelloReply | null>(null);
  const { pressCount, lastPressedAt, recordPress } = useHotkeyStore();

  useEffect(() => {
    tauri.hello("SnipTeX").then(setHello).catch(console.error);
  }, []);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    listen("hotkey-pressed", () => {
      recordPress();
      toast("Hotkey received", {
        description: "Cmd/Ctrl+Shift+M roundtripped from Rust.",
      });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(console.error);

    return () => {
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
        to test the global hotkey roundtrip.
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

      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}
