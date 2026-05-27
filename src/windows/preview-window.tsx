import { useCallback, useEffect, useMemo, useState } from "react";
import {
  getCurrentWebviewWindow,
  type WebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { cursorPosition } from "@tauri-apps/api/window";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Toaster, toast } from "sonner";
import {
  Check,
  ChevronDown,
  Copy,
  Pin,
  PinOff,
  X,
} from "lucide-react";
import { LatexRenderer } from "@/components/latex-renderer";
import { MarkdownRenderer } from "@/components/markdown-renderer";
import { COPY_AS_OPTIONS, defaultFormatFor, formatOutput, type FormatKind } from "@/lib/format";
import { useAutoHide } from "@/hooks/use-auto-hide";
import { useSnipResult } from "@/hooks/use-snip-result";
import { useSnipTrigger } from "@/hooks/use-snip-trigger";
import { useSettingsStore } from "@/stores/settings-store";
import { cn } from "@/lib/cn";
import { strings } from "@/strings";
import type { DetectedType } from "@/lib/invoke";

// Preview window opens at a small offset from the cursor so the user
// doesn't have to scan the full screen to find the result.
const CURSOR_OFFSET = { x: 16, y: 16 };

export default function PreviewWindow() {
  // Drives the global snip trigger from this always-running window.
  // Other windows don't mount this hook to avoid duplicate listeners.
  useSnipTrigger();

  const event = useSnipResult();
  const snip = event?.result ?? null;
  const autoHideMs = useSettingsStore((s) => s.preview_duration_ms);

  const [pinned, setPinned] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [copyState, setCopyState] = useState<"idle" | "copied">("idle");
  const [menuOpen, setMenuOpen] = useState(false);

  const handleHide = useCallback(() => {
    void hidePreviewWindow();
  }, []);

  // Only arm the auto-hide timer once a real snip has rendered.
  // Without this gate, the timer would fire 3s after window mount
  // (cold start, empty state) and try to hide a window that isn't
  // even visible — generating noisy permission errors and surprising
  // the user if the window were ever shown by other means.
  const hasResult = !!(snip && snip.status === "ok" && snip.text);

  const { bump } = useAutoHide({
    enabled: hasResult,
    durationMs: autoHideMs,
    pinned,
    hovered,
    // Sequence ticks on every backend emission, so even an identical
    // re-snip of the same equation restarts the timer cleanly.
    resetKey: event?.seq ?? null,
    onHide: handleHide,
  });

  // Surface backend snip-error events as a toast in the Preview Window
  // so the user sees them even when the trigger originated from the
  // tray (the trigger hook only catches the invoke-rejection path).
  useEffect(() => {
    let cancelled = false;
    let off: (() => void) | undefined;
    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen<string>("snip-error", (e) => {
          if (cancelled) return;
          toast.error("Snip failed", { description: e.payload });
        }),
      )
      .then((fn) => {
        if (cancelled) fn();
        else off = fn;
      })
      .catch((err) => console.warn("[preview] snip-error listen failed", err));
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  // Materialise the snip into history + auto-copy + show the window.
  // Keyed off `event.seq` so identical content still re-triggers.
  useEffect(() => {
    if (!snip || snip.status !== "ok" || !snip.text) return;

    console.info("[preview] snip-complete arrived", {
      seq: event?.seq,
      detected: snip.detected,
      agent: snip.agent,
      chars: snip.text.length,
    });

    // CRITICAL: show the window FIRST so even if anything downstream
    // throws (clipboard, MathJax, cursor positioning), the user still
    // sees that the snip completed. Subsequent work happens behind a
    // now-visible surface.
    void showPreviewNearCursor();

    setPinned(false);
    // Reset hovered: if the previous snip hid while mouse was over the
    // window, no mouseleave fires (hidden window receives no events) —
    // the state would otherwise stay `true` and suppress auto-hide on
    // the next snip.
    setHovered(false);
    setCopyState("idle");
    setMenuOpen(false);

    // History persistence happens entirely in Rust (`persist_to_history`
    // before the snip-complete event fires). HistoryWindow listens for
    // that same event in its own webview and refetches from SQLite —
    // pushing into a per-window Zustand store from here is a no-op
    // because each Tauri window has its own JS context.

    // Auto-copy the default format so the user can paste immediately
    // without needing to click anything. The visible window is a
    // confirmation, not the source of truth.
    void copyOutput(snip.text, snip.detected, defaultFormatFor(snip.detected))
      .then(() => setCopyState("copied"))
      .catch((err) => {
        console.warn("[preview] auto-copy failed", err);
      });
    // event.seq is the source of truth: identical snip text shouldn't
    // suppress the re-show.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event?.seq]);

  // Dev-only mount marker for the JS console — confirms the React tree
  // actually booted inside the (initially-hidden) Preview window.
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    console.info("[preview] window mounted, listening for snip-complete");
  }, []);

  if (!snip || snip.status !== "ok" || !snip.text) {
    // First-run / cold-start state. The window starts hidden in
    // tauri.conf.json, so this branch normally isn't seen.
    return (
      <main className="flex h-dvh w-dvw flex-col items-center justify-center bg-slate-900/85 px-6 text-center text-slate-100 backdrop-blur">
        <h1 className="text-base font-semibold">{strings.preview.emptyTitle}</h1>
        <p className="mt-1 text-xs text-slate-300">{strings.preview.emptyHint}</p>
        <Toaster richColors closeButton position="bottom-right" />
      </main>
    );
  }

  return (
    <main
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      className="flex h-dvh w-dvw flex-col overflow-hidden rounded-xl border border-slate-300/40 bg-white/95 text-slate-900 shadow-2xl backdrop-blur dark:border-slate-700/60 dark:bg-slate-900/95 dark:text-slate-100"
    >
      <PreviewToolbar
        agent={snip.agent}
        detected={snip.detected}
        copyState={copyState}
        pinned={pinned}
        menuOpen={menuOpen}
        onCopy={async () => {
          if (!snip.text) return;
          await copyOutput(
            snip.text,
            snip.detected,
            defaultFormatFor(snip.detected),
          );
          setCopyState("copied");
          // Keep the window alive a moment longer so user sees the tick.
          bump();
        }}
        onCopyAs={async (kind) => {
          if (!snip.text) return;
          await copyOutput(snip.text, snip.detected, kind);
          setCopyState("copied");
          setMenuOpen(false);
          bump();
          toast.success(`Copied as ${kind}`);
        }}
        onTogglePin={() => setPinned((p) => !p)}
        onToggleMenu={() => setMenuOpen((m) => !m)}
        onDismiss={handleHide}
      />
      <section className="flex-1 overflow-auto px-4 py-3">
        <PreviewBody text={snip.text} detected={snip.detected} />
      </section>
      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}

function PreviewBody({
  text,
  detected,
}: {
  text: string;
  detected: DetectedType | null;
}) {
  // EQUATION_ONLY → MathJax (heaviest fidelity for pure equations).
  // TABLE_ONLY / MIXED → markdown-it + KaTeX-for-inline-math.
  if (detected === "EQUATION_ONLY") {
    return <LatexRenderer latex={text} displayMode />;
  }
  return <MarkdownRenderer source={text} />;
}

type ToolbarProps = {
  agent: string | null;
  detected: DetectedType | null;
  copyState: "idle" | "copied";
  pinned: boolean;
  menuOpen: boolean;
  onCopy: () => void | Promise<void>;
  onCopyAs: (kind: FormatKind) => void | Promise<void>;
  onTogglePin: () => void;
  onToggleMenu: () => void;
  onDismiss: () => void;
};

function PreviewToolbar({
  agent,
  detected,
  copyState,
  pinned,
  menuOpen,
  onCopy,
  onCopyAs,
  onTogglePin,
  onToggleMenu,
  onDismiss,
}: ToolbarProps) {
  const detectedLabel = useMemo(() => {
    if (!detected) return "—";
    return detected.replace("_", " ").toLowerCase();
  }, [detected]);

  return (
    <header className="flex items-center gap-2 border-b border-slate-200/70 px-3 py-2 dark:border-slate-700/60">
      {/* The frameless Preview Window has no native titlebar, so this
          left-side region of the toolbar doubles as the drag handle.
          `data-tauri-drag-region` is recognised by Tauri's runtime and
          forwards mousedown to `window.startDragging()`. */}
      <div
        data-tauri-drag-region
        className="flex min-w-0 flex-1 cursor-grab items-center gap-2 text-xs text-slate-500 select-none dark:text-slate-400"
      >
        <span
          data-tauri-drag-region
          className="truncate font-medium text-slate-700 dark:text-slate-200"
        >
          {strings.app.name}
        </span>
        <span
          data-tauri-drag-region
          className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-slate-600 dark:bg-slate-800 dark:text-slate-300"
        >
          {detectedLabel}
        </span>
        {agent && (
          <span
            data-tauri-drag-region
            className="truncate text-[11px] text-slate-400"
          >
            via {agent}
          </span>
        )}
      </div>

      <button
        type="button"
        onClick={() => void onCopy()}
        className={cn(
          "inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition",
          copyState === "copied"
            ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/50 dark:text-emerald-200"
            : "bg-slate-100 text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700",
        )}
      >
        {copyState === "copied" ? (
          <>
            <Check className="size-3.5" /> {strings.preview.copied}
          </>
        ) : (
          <>
            <Copy className="size-3.5" /> {strings.preview.copy}
          </>
        )}
      </button>

      <div className="relative">
        <button
          type="button"
          onClick={onToggleMenu}
          className="inline-flex items-center gap-1 rounded-md bg-slate-100 px-2 py-1 text-xs font-medium text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
        >
          {strings.preview.copyAs}
          <ChevronDown className="size-3.5" />
        </button>
        {menuOpen && (
          <ul className="absolute right-0 z-10 mt-1 w-56 overflow-hidden rounded-md border border-slate-200 bg-white shadow-lg dark:border-slate-700 dark:bg-slate-900">
            {COPY_AS_OPTIONS.map((opt) => (
              <li key={opt.kind}>
                <button
                  type="button"
                  onClick={() => void onCopyAs(opt.kind)}
                  className="block w-full px-3 py-1.5 text-left text-xs text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800"
                >
                  {opt.label}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <button
        type="button"
        onClick={onTogglePin}
        className={cn(
          "inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition",
          pinned
            ? "bg-amber-100 text-amber-700 dark:bg-amber-900/50 dark:text-amber-200"
            : "bg-slate-100 text-slate-700 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700",
        )}
        title={pinned ? strings.preview.unpin : strings.preview.pin}
      >
        {pinned ? <PinOff className="size-3.5" /> : <Pin className="size-3.5" />}
        {pinned ? strings.preview.pinned : strings.preview.pin}
      </button>

      <button
        type="button"
        onClick={onDismiss}
        className="inline-flex items-center justify-center rounded-md p-1 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200"
        title={strings.preview.dismiss}
      >
        <X className="size-3.5" />
      </button>
    </header>
  );
}

// === window-side helpers ===

async function copyOutput(
  text: string,
  detected: DetectedType | null,
  kind: FormatKind,
): Promise<void> {
  const formatted = await formatOutput(text, detected, kind);
  await writeText(formatted);
}

let cachedWindow: WebviewWindow | null = null;
function previewWindow(): WebviewWindow {
  if (!cachedWindow) {
    cachedWindow = getCurrentWebviewWindow();
  }
  return cachedWindow;
}

async function hidePreviewWindow(): Promise<void> {
  try {
    await previewWindow().hide();
  } catch (err) {
    console.warn("[preview] hide failed", err);
  }
}

async function showPreviewNearCursor(): Promise<void> {
  const win = previewWindow();
  // Position first (best-effort), then show. If position fails we
  // still want the window visible — show() runs in its own try.
  try {
    const [cursor, scale] = await Promise.all([
      cursorPosition().catch((err) => {
        console.warn("[preview] cursorPosition failed", err);
        return null;
      }),
      win.scaleFactor().catch(() => 1),
    ]);
    if (cursor) {
      // cursorPosition returns physical pixels in OS global space;
      // divide by scale to convert to LogicalPosition.
      const x = cursor.x / scale + CURSOR_OFFSET.x;
      const y = cursor.y / scale + CURSOR_OFFSET.y;
      console.info("[preview] positioning at", { x, y, scale });
      await win.setPosition(new LogicalPosition(x, y));
    } else {
      console.info("[preview] cursor unknown — using last position");
    }
  } catch (err) {
    console.warn("[preview] setPosition failed", err);
  }

  try {
    await win.show();
    await win.setFocus();
    console.info("[preview] show + focus done");
  } catch (err) {
    console.warn("[preview] show failed", err);
  }
}
