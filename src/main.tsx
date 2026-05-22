import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App";
import CaptureOverlayWindow from "./windows/capture-overlay-window";
import "./styles/globals.css";

const label = (() => {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return "main";
  }
})();

const isOverlay = label === "overlay";
const Root = isOverlay ? CaptureOverlayWindow : App;

if (isOverlay) {
  // Defeat the default light/dark body background so the captured
  // backdrop image is the only thing visible behind the selection.
  document.documentElement.classList.add("overlay-window");
  document.body.classList.add("overlay-window");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
