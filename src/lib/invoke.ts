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
  showWindow: (label: WindowLabel) =>
    invoke<void>("show_window", { label }),
  hideWindow: (label: WindowLabel) =>
    invoke<void>("hide_window", { label }),

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
};
