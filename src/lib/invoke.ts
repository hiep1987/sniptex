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
  showWindow: (label: WindowLabel) =>
    invoke<void>("show_window", { label }),
  hideWindow: (label: WindowLabel) =>
    invoke<void>("hide_window", { label }),
};
