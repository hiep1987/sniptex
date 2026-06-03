import { invoke } from "@tauri-apps/api/core";

export type HelloReply = {
  message: string;
  version: string;
};

export type DetectedType = "EQUATION_ONLY" | "TABLE_ONLY" | "MIXED";

export type SnipResult = {
  status: "ok" | "cancelled";
  text: string | null;
  detected: DetectedType | null;
  agent: string | null;
  image_path: string | null;
  record_id: number | null;
};

export type HistoryRecord = {
  id: number;
  uuid: string;
  created_at: number; // unix epoch seconds
  agent: string;
  text: string;
  detected: DetectedType;
  image_path: string;
  thumb_path: string;
  latency_ms: number;
};

export type ExportFormat = "latex" | "markdown" | "plain";

export type ThemeMode = "system" | "light" | "dark";
export type OutputFormat =
  | "smart"
  | "inline"
  | "display"
  | "plain"
  | "markdown"
  | "math_ml"
  | "unicode_pretty";
export type HistorySizeOption =
  | "fifty"
  | "one_hundred"
  | "five_hundred"
  | "unlimited";

export type AppSettings = {
  hotkey: string;
  agent_priority: string[];
  default_format: OutputFormat;
  copy_as_formats: OutputFormat[];
  history_size: HistorySizeOption;
  preview_duration_ms: number;
  sound_on_success: boolean;
  launch_at_login: boolean;
  theme: ThemeMode;
  onboarding_completed: boolean;
  cloud_mode_enabled: boolean;
};

export type SettingsPatch = Partial<AppSettings>;

export type AgentKind = "CliBin" | "CloudApi";
export type AgentInfo = {
  spec: {
    id: string;
    display_name: string;
    binary_names: string[];
    supports_vision: boolean;
    kind: AgentKind;
  };
  binary_path: string;
  version: string | null;
};

export type WindowLabel =
  | "preview"
  | "settings"
  | "history"
  | "onboarding"
  | "overlay";

export const tauri = {
  hello: (name?: string) =>
    invoke<HelloReply>("hello", { name: name ?? null }),
  runSnip: (agentId?: string) =>
    invoke<SnipResult>("run_snip", { agentId: agentId ?? null }),
  runPdfOcr: (pdfPath: string, agentId?: string) =>
    invoke<SnipResult>("run_pdf_ocr", { pdfPath, agentId: agentId ?? null }),
  cancelPdfOcr: () => invoke<void>("cancel_pdf_ocr"),
  showWindow: (label: WindowLabel) =>
    invoke<void>("show_window", { label }),
  hideWindow: (label: WindowLabel) =>
    invoke<void>("hide_window", { label }),
  openExternal: (url: string) => invoke<void>("open_external", { url }),

  // Phase 7: SQLite history
  getHistory: (limit = 100) =>
    invoke<HistoryRecord[]>("get_history", { limit }),
  searchHistory: (query: string, limit = 100) =>
    invoke<HistoryRecord[]>("search_history", { query, limit }),
  deleteRecord: (id: number) =>
    invoke<void>("delete_record", { id }),
  rerunSnip: (recordId: number, agentId: string) =>
    invoke<HistoryRecord>("rerun_snip", { recordId, agentId }),
  exportRecord: (id: number, format: ExportFormat) =>
    invoke<string>("export_record", { id, format }),

  // API key management
  setApiKey: (provider: string, key: string) =>
    invoke<void>("set_api_key", { provider, key }),
  hasApiKey: (provider: string) =>
    invoke<boolean>("has_api_key", { provider }),
  deleteApiKey: (provider: string) =>
    invoke<void>("delete_api_key", { provider }),
  testApiKey: (provider: string, key: string) =>
    invoke<{ ok: boolean; char_count: number; preview: string }>(
      "test_api_key",
      { provider, key },
    ),

  // Agent detection
  detectAgents: () => invoke<AgentInfo[]>("detect_agents"),
  testAgent: (agentId: string, imagePath: string) =>
    invoke<{ ok: boolean; detected: DetectedType; char_count: number; preview: string }>(
      "test_agent",
      { agentId, imagePath },
    ),

  // Settings
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (patch: SettingsPatch) =>
    invoke<AppSettings>("update_settings", { patch }),
  rebindHotkey: (newShortcut: string) =>
    invoke<void>("rebind_hotkey", { newShortcut }),
  setLaunchAtLogin: (enabled: boolean) =>
    invoke<void>("set_launch_at_login", { enabled }),

  // Format conversion (Phase 9: Copy as TeX uses this to turn
  // Markdown tables into \begin{tabular} blocks).
  convertToTex: (text: string) => invoke<string>("convert_to_tex", { text }),
};
