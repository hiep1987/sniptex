import { useState } from "react";
import { toast } from "sonner";
import { tauri } from "@/lib/invoke";
import { useSettingsStore } from "@/stores/settings-store";
import HotkeyInput from "@/components/hotkey-input";

const DEFAULT_HOTKEY =
  navigator.platform.includes("Mac") ? "Command+Shift+M" : "Control+Shift+M";

export default function HotkeysTab() {
  const { hotkey, patch } = useSettingsStore();
  const [error, setError] = useState<string | null>(null);

  const handleChange = async (combo: string) => {
    setError(null);
    try {
      await tauri.rebindHotkey(combo);
      patch({ hotkey: combo });
      toast.success(`Hotkey changed to ${combo}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleReset = async () => {
    setError(null);
    try {
      await tauri.rebindHotkey(DEFAULT_HOTKEY);
      patch({ hotkey: DEFAULT_HOTKEY });
      toast.success("Hotkey reset to default");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="max-w-xl space-y-6">
      <h2 className="text-lg font-semibold">Hotkeys</h2>

      <div className="space-y-3">
        <div>
          <label className="mb-1.5 block text-sm font-medium">
            Capture shortcut
          </label>
          <HotkeyInput value={hotkey} onChange={handleChange} />
        </div>

        {error && (
          <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
        )}

        <button
          type="button"
          onClick={handleReset}
          className="text-xs text-slate-500 hover:text-slate-700 hover:underline dark:text-slate-400 dark:hover:text-slate-200"
        >
          Reset to default ({DEFAULT_HOTKEY.replace("Command", "⌘").replace("Control", "Ctrl").replace("Shift", "⇧")})
        </button>
      </div>

      <p className="text-xs text-slate-400 dark:text-slate-500">
        Click the box above, then press your desired key combination.
        A modifier (⌘/Ctrl/Alt) plus a letter or number is required.
      </p>
    </div>
  );
}
