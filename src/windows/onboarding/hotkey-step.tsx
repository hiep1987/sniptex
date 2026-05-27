import { useState } from "react";
import { tauri } from "@/lib/invoke";
import { useSettingsStore } from "@/stores/settings-store";
import HotkeyInput from "@/components/hotkey-input";

export default function HotkeyStep() {
  const { hotkey, patch } = useSettingsStore();
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const handleChange = async (combo: string) => {
    setError(null);
    setSuccess(false);
    try {
      await tauri.rebindHotkey(combo);
      patch({ hotkey: combo });
      setSuccess(true);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="max-w-lg space-y-4">
      <h2 className="text-lg font-semibold">Pick Your Hotkey</h2>
      <p className="text-sm text-slate-600 dark:text-slate-300">
        This keyboard shortcut triggers a screen capture from anywhere.
        Click the box to change it.
      </p>

      <div className="space-y-2">
        <label className="block text-sm font-medium">Capture shortcut</label>
        <HotkeyInput value={hotkey} onChange={handleChange} />

        {error && (
          <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
        )}
        {success && (
          <p className="text-xs text-green-600 dark:text-green-400">
            Hotkey updated! Try pressing it now.
          </p>
        )}
      </div>

      <p className="text-xs text-slate-400 dark:text-slate-500">
        You can always change this later in Settings → Hotkeys.
      </p>
    </div>
  );
}
