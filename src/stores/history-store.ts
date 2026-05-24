import { create } from "zustand";
import { tauri, type HistoryRecord } from "@/lib/invoke";

// Phase 7: SQLite-backed snip history.
//
// Frontend state machine:
//   * `items`         — current visible rows (already filtered or full list)
//   * `search`        — debounced search input
//   * `loading`       — true while a fetch / search round-trip is in flight
//   * `error`         — last error string (null on success)
//
// The store owns one debounce timer; new keystrokes reset it. The store
// also exposes `push()` for instant insert when PreviewWindow receives
// `snip-complete`, so a fresh snip lands in the list without a refetch.

const SEARCH_DEBOUNCE_MS = 200;
const DEFAULT_FETCH_LIMIT = 100;

export type HistoryItem = HistoryRecord;

let debounceHandle: ReturnType<typeof setTimeout> | null = null;

export type HistoryState = {
  items: HistoryItem[];
  search: string;
  loading: boolean;
  error: string | null;
  load: (limit?: number) => Promise<void>;
  setSearch: (q: string) => void;
  push: (item: HistoryItem) => void;
  remove: (id: number) => Promise<void>;
  rerun: (id: number, agentId: string) => Promise<HistoryItem>;
  clear: () => void;
};

export const useHistoryStore = create<HistoryState>((set, get) => ({
  items: [],
  search: "",
  loading: false,
  error: null,

  async load(limit = DEFAULT_FETCH_LIMIT) {
    set({ loading: true, error: null });
    try {
      const rows = await tauri.getHistory(limit);
      set({ items: rows, loading: false });
    } catch (err) {
      set({ loading: false, error: String(err) });
    }
  },

  setSearch(q: string) {
    set({ search: q });
    if (debounceHandle) clearTimeout(debounceHandle);
    debounceHandle = setTimeout(async () => {
      const query = get().search;
      set({ loading: true, error: null });
      try {
        const rows = query.trim()
          ? await tauri.searchHistory(query, DEFAULT_FETCH_LIMIT)
          : await tauri.getHistory(DEFAULT_FETCH_LIMIT);
        set({ items: rows, loading: false });
      } catch (err) {
        set({ loading: false, error: String(err) });
      }
    }, SEARCH_DEBOUNCE_MS);
  },

  push(item) {
    set((s) => ({
      items: [item, ...s.items.filter((it) => it.id !== item.id)].slice(0, 200),
    }));
  },

  async remove(id) {
    try {
      await tauri.deleteRecord(id);
      set((s) => ({ items: s.items.filter((it) => it.id !== id) }));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  async rerun(id, agentId) {
    const refreshed = await tauri.rerunSnip(id, agentId);
    set((s) => ({
      items: s.items.map((it) => (it.id === id ? refreshed : it)),
    }));
    return refreshed;
  },

  clear() {
    set({ items: [] });
  },
}));
