import { create } from "zustand";
import type { DetectedType } from "@/lib/invoke";

// Phase 7 will replace this skeleton with SQLite-backed history
// (Tauri SQL plugin + FTS5). Keeping the shape here lets HistoryWindow
// already render a list and lets PreviewWindow push the latest snip
// into the in-memory cache for instant display.

export type HistoryItem = {
  id: string;
  text: string;
  detected: DetectedType | null;
  agent: string | null;
  imagePath: string | null;
  createdAt: number;
};

export type HistoryState = {
  items: HistoryItem[];
  search: string;
  setSearch: (q: string) => void;
  push: (item: HistoryItem) => void;
  clear: () => void;
};

export const useHistoryStore = create<HistoryState>((set) => ({
  items: [],
  search: "",
  setSearch: (search) => set({ search }),
  push: (item) =>
    set((s) => ({
      // Cap the in-memory cache so a long session doesn't grow without
      // bound; persisted history (Phase 7) is unaffected.
      items: [item, ...s.items].slice(0, 200),
    })),
  clear: () => set({ items: [] }),
}));
