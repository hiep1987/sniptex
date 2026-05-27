import { CheckCircle2, Keyboard } from "lucide-react";
import { useSettingsStore } from "@/stores/settings-store";

export default function ReadyStep() {
  const hotkey = useSettingsStore((s) => s.hotkey);
  const display = hotkey
    .replace("Command", "⌘")
    .replace("Control", "Ctrl")
    .replace("Shift", "⇧")
    .replace("Alt", "⌥");

  return (
    <div className="max-w-lg space-y-5">
      <div className="flex items-center gap-3">
        <CheckCircle2 className="size-8 text-green-600" />
        <h2 className="text-xl font-semibold">You're Ready!</h2>
      </div>

      <p className="text-sm text-slate-600 dark:text-slate-300">
        SnipTeX is set up and running in your menu bar. Here's what you
        need to know:
      </p>

      <div className="rounded-lg border border-slate-200 bg-slate-50 p-4 dark:border-slate-700 dark:bg-slate-900">
        <div className="flex items-center gap-3">
          <Keyboard className="size-5 text-slate-500" />
          <div>
            <p className="text-sm font-medium">
              Press{" "}
              <kbd className="rounded bg-slate-200 px-1.5 py-0.5 font-mono text-xs dark:bg-slate-700">
                {display}
              </kbd>{" "}
              anywhere
            </p>
            <p className="text-xs text-slate-500 dark:text-slate-400">
              Drag to select a region → OCR runs → result is copied to
              clipboard
            </p>
          </div>
        </div>
      </div>

      <ul className="space-y-1.5 text-sm text-slate-600 dark:text-slate-300">
        <li>• Access settings and history from the menu bar icon</li>
        <li>• Right-click the preview to copy in different formats</li>
        <li>• History keeps all your past snips searchable</li>
      </ul>

      <p className="text-xs text-slate-400 dark:text-slate-500">
        Click <strong>Finish</strong> to close this guide and start
        snipping.
      </p>
    </div>
  );
}
