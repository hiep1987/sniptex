import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App";
import CaptureOverlayWindow from "./windows/capture-overlay-window";
import PreviewWindow from "./windows/preview-window";
import SettingsWindow from "./windows/settings-window";
import HistoryWindow from "./windows/history-window";
import OnboardingWindow from "./windows/onboarding-window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { tauri } from "./lib/invoke";
import { useTheme } from "./hooks/use-theme";
import { useSettingsStore, SETTINGS_CHANGED_EVENT } from "./stores/settings-store";
import "./styles/globals.css";

// Expose tauri invoke helpers on window for devtools console access.
if (import.meta.env.DEV) {
  (window as any).tauri = tauri;
}

const label = (() => {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return "preview";
  }
})();

const Root = pickRoot(label);

if (label === "overlay") {
  // Defeat the default light/dark body background so the captured
  // backdrop image is the only thing visible behind the selection.
  document.documentElement.classList.add("overlay-window");
  document.body.classList.add("overlay-window");
}

if (label === "preview") {
  // The Preview Window is frameless + transparent; flag the body so
  // CSS can drop the opaque background and keep the rounded shell
  // riding on top of whatever's underneath.
  document.documentElement.classList.add("preview-window");
  document.body.classList.add("preview-window");
}

function ThemeProvider({ children }: { children: React.ReactNode }) {
  const fetch = useSettingsStore((s) => s.fetch);

  React.useEffect(() => { fetch(); }, [fetch]);

  React.useEffect(() => {
    let off: UnlistenFn | undefined;
    listen(SETTINGS_CHANGED_EVENT, () => { fetch(); })
      .then((fn) => { off = fn; })
      .catch(() => {});
    return () => { off?.(); };
  }, [fetch]);

  useTheme();
  return <>{children}</>;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <Root />
    </ThemeProvider>
  </React.StrictMode>,
);

function pickRoot(windowLabel: string): React.ComponentType {
  switch (windowLabel) {
    case "main":
      return App;
    case "overlay":
      return CaptureOverlayWindow;
    case "preview":
      return PreviewWindow;
    case "settings":
      return SettingsWindow;
    case "history":
      return HistoryWindow;
    case "onboarding":
      return OnboardingWindow;
    default:
      // Fallback for dev / unrecognised labels.
      return App;
  }
}
