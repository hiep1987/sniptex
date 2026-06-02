import { create } from "zustand";
import { emit } from "@tauri-apps/api/event";
import {
  tauri,
  type AppSettings,
  type SettingsPatch,
  type ThemeMode,
  type OutputFormat,
  type HistorySizeOption,
} from "@/lib/invoke";

export const SETTINGS_CHANGED_EVENT = "settings-changed";

export type { ThemeMode, OutputFormat, HistorySizeOption };

type SettingsState = AppSettings & {
  loaded: boolean;
  fetch: () => Promise<void>;
  patch: (p: SettingsPatch) => Promise<void>;
};

const DEFAULTS: AppSettings = {
  hotkey: "Command+Shift+M",
  agent_priority: ["codex", "cloud-gemini", "cloud-mistral"],
  default_format: "smart",
  copy_as_formats: ["plain", "smart", "inline", "display", "markdown"],
  history_size: "one_hundred",
  preview_duration_ms: 3000,
  sound_on_success: true,
  launch_at_login: false,
  theme: "system",
  onboarding_completed: false,
  cloud_mode_enabled: false,
  local_ocr_enabled: false,
  local_ocr_url: "http://127.0.0.1:8765",
  local_ocr_formula_enabled: true,
  local_ocr_text_enabled: true,
};

export const useSettingsStore = create<SettingsState>((set) => ({
  ...DEFAULTS,
  loaded: false,

  fetch: async () => {
    try {
      const s = await tauri.getSettings();
      set({ ...s, loaded: true });
    } catch (e) {
      console.error("[settings] fetch failed", e);
    }
  },

  patch: async (p) => {
    try {
      const updated = await tauri.updateSettings(p);
      set(updated);
      emit(SETTINGS_CHANGED_EVENT).catch(() => {});
    } catch (e) {
      console.error("[settings] patch failed", e);
    }
  },
}));
