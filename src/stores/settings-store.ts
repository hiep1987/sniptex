import { create } from "zustand";

// Phase 8 will replace this skeleton with persisted user preferences
// backed by tauri-plugin-store. For now, keep the in-memory shape so
// Settings/Preview windows can already wire their UI to it.

export type ThemePreference = "light" | "dark" | "system";
export type DefaultFormat = "auto" | "inline" | "display" | "markdown" | "plain";

export type SettingsState = {
  theme: ThemePreference;
  defaultFormat: DefaultFormat;
  /** Preview window auto-hide duration in milliseconds. */
  autoHideMs: number;
  /** Default Format Toggle for the LaTeX-tabular toggle (Phase 9). */
  preferLatexTables: boolean;
  setTheme: (t: ThemePreference) => void;
  setDefaultFormat: (f: DefaultFormat) => void;
  setAutoHideMs: (ms: number) => void;
  setPreferLatexTables: (v: boolean) => void;
};

export const useSettingsStore = create<SettingsState>((set) => ({
  theme: "system",
  defaultFormat: "auto",
  autoHideMs: 3000,
  preferLatexTables: false,
  setTheme: (theme) => set({ theme }),
  setDefaultFormat: (defaultFormat) => set({ defaultFormat }),
  setAutoHideMs: (autoHideMs) => set({ autoHideMs }),
  setPreferLatexTables: (preferLatexTables) => set({ preferLatexTables }),
}));
