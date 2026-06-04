import { useSettingsStore, type OutputFormat } from "@/stores/settings-store";
import { sortFormats } from "@/lib/format";
import { cn } from "@/lib/cn";

const FORMATS: { value: OutputFormat; label: string; desc: string }[] = [
  { value: "plain", label: "Plain LaTeX", desc: "Raw TeX / tabular, no delimiters" },
  { value: "smart", label: "Smart", desc: "Auto-detect equation vs table vs mixed" },
  { value: "inline", label: "Inline LaTeX", desc: "Wrap in $…$" },
  { value: "display", label: "Display LaTeX", desc: "Wrap in $$…$$" },
  { value: "markdown", label: "Markdown", desc: "Markdown with math fences" },
  { value: "math_ml", label: "MathML", desc: "XML math markup" },
  { value: "unicode_pretty", label: "Unicode", desc: "Unicode math symbols" },
];

type FormatRadioGroupProps = {
  name: string;
  value: OutputFormat;
  onChange: (value: OutputFormat) => void;
};

function FormatRadioGroup({ name, value, onChange }: FormatRadioGroupProps) {
  return (
    <div className="space-y-1.5">
      {FORMATS.map((f) => (
        <label
          key={f.value}
          className={cn(
            "flex cursor-pointer items-center gap-3 rounded-md border px-3 py-2 transition",
            value === f.value
              ? "border-slate-900 bg-slate-50 dark:border-slate-100 dark:bg-slate-900"
              : "border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800",
          )}
        >
          <input
            type="radio"
            name={name}
            checked={value === f.value}
            onChange={() => onChange(f.value)}
            className="accent-slate-900 dark:accent-slate-100"
          />
          <div>
            <span className="text-sm font-medium">{f.label}</span>
            <span className="ml-2 text-xs text-slate-500 dark:text-slate-400">
              {f.desc}
            </span>
          </div>
        </label>
      ))}
    </div>
  );
}

export default function FormatsTab() {
  const { default_format, history_copy_format, copy_as_formats, patch } =
    useSettingsStore();

  const toggleCopyAs = (fmt: OutputFormat) => {
    const current = new Set(copy_as_formats);
    if (current.has(fmt)) {
      if (current.size <= 1) return;
      current.delete(fmt);
    } else {
      current.add(fmt);
    }
    patch({ copy_as_formats: sortFormats([...current]) });
  };

  return (
    <div className="max-w-xl space-y-6">
      <h2 className="text-lg font-semibold">Formats</h2>

      <div className="space-y-3">
        <label className="block text-sm font-medium">Default output format</label>
        <FormatRadioGroup
          name="default-format"
          value={default_format}
          onChange={(value) => patch({ default_format: value })}
        />
      </div>

      <div className="space-y-3">
        <label className="block text-sm font-medium">History row copy format</label>
        <FormatRadioGroup
          name="history-copy-format"
          value={history_copy_format}
          onChange={(value) => patch({ history_copy_format: value })}
        />
      </div>

      <div className="space-y-3">
        <label className="block text-sm font-medium">
          Show in "Copy as…" menu
        </label>
        <div className="space-y-1">
          {FORMATS.map((f) => (
            <label
              key={f.value}
              className="flex cursor-pointer items-center gap-3 px-1 py-1 text-sm"
            >
              <input
                type="checkbox"
                checked={copy_as_formats.includes(f.value)}
                onChange={() => toggleCopyAs(f.value)}
                className="size-4 rounded border-slate-300 accent-slate-900 dark:accent-slate-100"
              />
              {f.label}
            </label>
          ))}
        </div>
      </div>
    </div>
  );
}
