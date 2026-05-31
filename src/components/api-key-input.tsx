import { useState } from "react";
import { Eye, EyeOff, ClipboardPaste, Loader2 } from "lucide-react";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { cn } from "@/lib/cn";

type TestOutcome = { ok: true; preview?: string } | { ok: false; error: string };

type Props = {
  value: string;
  onChange: (key: string) => void;
  onTest?: () => Promise<TestOutcome>;
  placeholder?: string;
  className?: string;
};

export default function ApiKeyInput({
  value,
  onChange,
  onTest,
  placeholder = "Paste your API key",
  className,
}: Props) {
  const [visible, setVisible] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestOutcome | null>(null);

  const handlePaste = async () => {
    try {
      const text = await readText();
      if (text) onChange(text.trim());
    } catch (e) {
      console.error("[api-key] clipboard read failed", e);
    }
  };

  const handleTest = async () => {
    if (!onTest) return;
    setTesting(true);
    setTestResult(null);
    try {
      const outcome = await onTest();
      setTestResult(outcome);
    } catch (e) {
      setTestResult({ ok: false, error: String(e) });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-center gap-1.5">
        <div className="relative flex-1">
          <input
            type={visible ? "text" : "password"}
            value={value}
            onChange={(e) => {
              onChange(e.target.value);
              setTestResult(null);
            }}
            placeholder={placeholder}
            autoComplete="off"
            spellCheck={false}
            className="w-full rounded-md border border-slate-300 bg-white py-1.5 pr-8 pl-3 font-mono text-sm text-slate-700 placeholder:text-slate-400 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200 dark:placeholder:text-slate-500"
          />
          <button
            type="button"
            onClick={() => setVisible((v) => !v)}
            className="absolute top-1/2 right-2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
          >
            {visible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
          </button>
        </div>

        <button
          type="button"
          onClick={handlePaste}
          title="Paste from clipboard"
          className="rounded-md border border-slate-300 p-1.5 text-slate-500 hover:bg-slate-50 dark:border-slate-600 dark:text-slate-400 dark:hover:bg-slate-700"
        >
          <ClipboardPaste className="size-4" />
        </button>

        {onTest && (
          <button
            type="button"
            onClick={handleTest}
            disabled={!value || testing}
            className="rounded-md bg-slate-900 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-white"
          >
            {testing ? (
              <Loader2 className="size-3 animate-spin" />
            ) : (
              "Test"
            )}
          </button>
        )}
      </div>

      {testResult !== null && (
        <div
          className={cn(
            "space-y-1 text-xs",
            testResult.ok
              ? "text-green-600 dark:text-green-400"
              : "text-red-600 dark:text-red-400",
          )}
        >
          <p>
            {testResult.ok
              ? "Key is valid — OCR returned a response."
              : "Key test failed."}
          </p>
          {testResult.ok && testResult.preview && (
            <p className="font-mono text-[11px] text-slate-500 dark:text-slate-400">
              {testResult.preview}
            </p>
          )}
          {!testResult.ok && (
            <p className="font-mono text-[11px] break-words text-red-500 dark:text-red-400">
              {testResult.error}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
