import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Toaster } from "sonner";
import { useHistoryStore } from "@/stores/history-store";
import { HistoryRow } from "@/components/history-row";
import { strings } from "@/strings";

export default function HistoryWindow() {
  const items = useHistoryStore((s) => s.items);
  const search = useHistoryStore((s) => s.search);
  const setSearch = useHistoryStore((s) => s.setSearch);
  const loading = useHistoryStore((s) => s.loading);
  const error = useHistoryStore((s) => s.error);
  const load = useHistoryStore((s) => s.load);
  const remove = useHistoryStore((s) => s.remove);
  const rerun = useHistoryStore((s) => s.rerun);

  // Initial load — when the webview first mounts.
  useEffect(() => {
    void load();
  }, [load]);

  // Tauri spawns each window as its own webview with isolated JS state.
  // The preview-window's store can't update this window's store directly,
  // so we refetch from SQLite whenever Rust emits `snip-complete` (a fresh
  // record just landed) and whenever this window becomes visible again
  // (the close button hides it instead of destroying it, so the mount
  // effect doesn't re-run on next show).
  useEffect(() => {
    let cancelled = false;
    let offSnip: UnlistenFn | undefined;
    let offFocus: UnlistenFn | undefined;

    listen("snip-complete", () => {
      if (!cancelled) void load();
    })
      .then((fn) => {
        if (cancelled) fn();
        else offSnip = fn;
      })
      .catch((err) =>
        console.error("[history] snip-complete listen failed", err),
      );

    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused && !cancelled) void load();
      })
      .then((fn) => {
        if (cancelled) fn();
        else offFocus = fn;
      })
      .catch((err) =>
        console.error("[history] focus listen failed", err),
      );

    return () => {
      cancelled = true;
      offSnip?.();
      offFocus?.();
    };
  }, [load]);

  const listRef = useRef<HTMLDivElement | null>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 96,
    overscan: 6,
  });

  return (
    <main className="flex h-dvh w-dvw flex-col bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <header className="border-b border-slate-200 p-4 dark:border-slate-800">
        <h1 className="mb-2 text-sm font-semibold tracking-tight">
          {strings.history.title}
        </h1>
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={strings.history.searchPlaceholder}
          className="w-full rounded-md border border-slate-300 bg-slate-50 px-3 py-1.5 text-sm placeholder:text-slate-400 focus:border-slate-500 focus:outline-none dark:border-slate-700 dark:bg-slate-900 dark:placeholder:text-slate-500"
        />
        {error && (
          <p className="mt-2 text-xs text-red-600 dark:text-red-400">
            {error}
          </p>
        )}
      </header>

      {loading && items.length === 0 ? (
        <p className="p-6 text-sm text-slate-500 dark:text-slate-400">
          Loading history…
        </p>
      ) : items.length === 0 ? (
        <p className="p-6 text-sm text-slate-500 dark:text-slate-400">
          {strings.history.empty}
        </p>
      ) : (
        <div ref={listRef} className="flex-1 overflow-auto">
          <div
            style={{
              height: virtualizer.getTotalSize(),
              position: "relative",
              width: "100%",
            }}
          >
            {virtualizer.getVirtualItems().map((row) => {
              const item = items[row.index];
              return (
                <div
                  key={item.id}
                  className="absolute inset-x-0"
                  style={{
                    transform: `translateY(${row.start}px)`,
                    height: row.size,
                  }}
                >
                  <HistoryRow
                    item={item}
                    onDelete={remove}
                    onRerun={rerun}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      <Toaster richColors closeButton position="bottom-right" />
    </main>
  );
}
