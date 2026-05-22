import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useHistoryStore } from "@/stores/history-store";
import { strings } from "@/strings";

export default function HistoryWindow() {
  const items = useHistoryStore((s) => s.items);
  const search = useHistoryStore((s) => s.search);
  const setSearch = useHistoryStore((s) => s.setSearch);

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const q = search.toLowerCase();
    return items.filter((it) => it.text.toLowerCase().includes(q));
  }, [items, search]);

  const listRef = useRef<HTMLDivElement | null>(null);
  const virtualizer = useVirtualizer({
    count: filtered.length,
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
      </header>

      {filtered.length === 0 ? (
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
              const item = filtered[row.index];
              return (
                <article
                  key={item.id}
                  className="absolute inset-x-0 border-b border-slate-100 px-4 py-3 dark:border-slate-800"
                  style={{
                    transform: `translateY(${row.start}px)`,
                    height: row.size,
                  }}
                >
                  <div className="mb-1 flex items-center gap-2 text-[10px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                    <span>{new Date(item.createdAt).toLocaleString()}</span>
                    {item.agent && (
                      <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                        {item.agent}
                      </span>
                    )}
                    {item.detected && (
                      <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                        {item.detected.replace("_", " ").toLowerCase()}
                      </span>
                    )}
                  </div>
                  <p className="line-clamp-2 font-mono text-xs text-slate-700 dark:text-slate-300">
                    {item.text}
                  </p>
                </article>
              );
            })}
          </div>
        </div>
      )}
    </main>
  );
}
