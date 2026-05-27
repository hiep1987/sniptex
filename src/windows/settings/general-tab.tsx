import { useSettingsStore, type ThemeMode } from "@/stores/settings-store";
import { tauri } from "@/lib/invoke";

const THEMES: { value: ThemeMode; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export default function GeneralTab() {
  const { theme, launch_at_login, sound_on_success, preview_duration_ms, patch } =
    useSettingsStore();

  return (
    <div className="max-w-xl space-y-6">
      <h2 className="text-lg font-semibold">General</h2>

      <Field label="Theme">
        <div className="flex gap-2">
          {THEMES.map((t) => (
            <button
              key={t.value}
              type="button"
              onClick={() => patch({ theme: t.value })}
              className={pill(theme === t.value)}
            >
              {t.label}
            </button>
          ))}
        </div>
      </Field>

      <Toggle
        label="Launch at login"
        description="Start SnipTeX when you log in."
        checked={launch_at_login}
        onChange={(v) => {
          patch({ launch_at_login: v });
          tauri.setLaunchAtLogin(v).catch(console.error);
        }}
      />

      <Toggle
        label="Sound on success"
        description="Play a sound when OCR completes."
        checked={sound_on_success}
        onChange={(v) => patch({ sound_on_success: v })}
      />

      <Field label="Preview auto-hide">
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={1000}
            max={10000}
            step={500}
            value={preview_duration_ms}
            onChange={(e) =>
              patch({ preview_duration_ms: Number(e.target.value) })
            }
            className="flex-1"
          />
          <span className="w-12 text-right text-xs text-slate-500 tabular-nums">
            {(preview_duration_ms / 1000).toFixed(1)}s
          </span>
        </div>
      </Field>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <label className="block text-sm font-medium">{label}</label>
      {children}
    </div>
  );
}

function Toggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-start gap-3 cursor-pointer">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-0.5 size-4 rounded border-slate-300 accent-slate-900 dark:accent-slate-100"
      />
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-slate-500 dark:text-slate-400">{description}</p>
      </div>
    </label>
  );
}

function pill(active: boolean) {
  return [
    "rounded-md px-3 py-1.5 text-xs font-medium transition",
    active
      ? "bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900"
      : "border border-slate-300 text-slate-600 hover:bg-slate-100 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-800",
  ].join(" ");
}
