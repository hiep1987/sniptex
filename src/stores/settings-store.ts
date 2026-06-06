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
  agent_priority: ["codex", "cloud-gemini", "cloud-mistral", "cloud-novita"],
  default_format: "smart",
  history_copy_format: "smart",
  copy_as_formats: ["plain", "smart", "inline", "display", "markdown"],
  history_size: "one_hundred",
  preview_duration_ms: 3000,
  sound_on_success: true,
  launch_at_login: false,
  theme: "system",
  onboarding_completed: false,
  cloud_mode_enabled: false,
};

export const useSettingsStore = create<SettingsState>((set) => ({
  ...DEFAULTS,
  loaded: false,

  fetch: async () => {
    // The main webview starts running before setup() has finished
    // managing the SettingsStore on Windows (Win32 windows are created
    // synchronously while the Tauri setup hook is still racing through
    // storage::init + plugin init + tray::init). The first IPC calls
    // can therefore fail with "state not managed" — retry with a long
    // enough backoff to cover the slow first-run cold-start on Windows
    // ARM64 VMs (Parallels disk + SQLite migrations + FTS5). macOS
    // succeeds on attempt 0 and never advances past it.
    const delays = [0, 100, 300, 700, 1500, 3000];
    for (let attempt = 0; attempt < delays.length; attempt++) {
      if (delays[attempt] > 0) {
        await new Promise((r) => setTimeout(r, delays[attempt]));
      }
      try {
        const s = await tauri.getSettings();
        set({ ...s, loaded: true });
        if (attempt > 0) {
          console.info(`[settings] fetch succeeded on attempt ${attempt + 1}`);
        }
        return;
      } catch (e) {
        if (attempt === delays.length - 1) {
          console.error(`[settings] fetch failed after ${delays.length} attempts`, e);
        }
      }
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
