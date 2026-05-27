import { useState, useCallback, useRef } from "react";
import { cn } from "@/lib/cn";

type Props = {
  value: string;
  onChange: (shortcut: string) => void;
  className?: string;
};

export default function HotkeyInput({ value, onChange, className }: Props) {
  const [capturing, setCapturing] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const inputRef = useRef<HTMLDivElement>(null);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!capturing) return;
      e.preventDefault();
      e.stopPropagation();

      const parts: string[] = [];
      if (e.metaKey) parts.push("Command");
      if (e.ctrlKey) parts.push("Control");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");

      const key = e.key;
      const isModifier = ["Meta", "Control", "Alt", "Shift"].includes(key);
      if (isModifier) {
        setPreview(parts.join("+") + "+…");
        return;
      }

      const keyName = key.length === 1 ? key.toUpperCase() : key;
      parts.push(keyName);

      if (parts.length < 2) return;

      const combo = parts.join("+");
      setPreview(null);
      setCapturing(false);
      onChange(combo);
    },
    [capturing, onChange],
  );

  const handleKeyUp = useCallback(
    (e: React.KeyboardEvent) => {
      if (!capturing) return;
      const isModifier = ["Meta", "Control", "Alt", "Shift"].includes(e.key);
      if (isModifier && preview) {
        setPreview(null);
      }
    },
    [capturing, preview],
  );

  const startCapture = () => {
    setCapturing(true);
    setPreview(null);
    inputRef.current?.focus();
  };

  const cancelCapture = () => {
    setCapturing(false);
    setPreview(null);
  };

  return (
    <div
      ref={inputRef}
      tabIndex={0}
      role="button"
      onKeyDown={handleKeyDown}
      onKeyUp={handleKeyUp}
      onBlur={cancelCapture}
      onClick={startCapture}
      className={cn(
        "inline-flex min-w-[200px] cursor-pointer items-center justify-center rounded-md border px-3 py-2 text-sm font-mono transition select-none",
        capturing
          ? "border-blue-500 bg-blue-50 text-blue-700 ring-2 ring-blue-200 dark:border-blue-400 dark:bg-blue-950 dark:text-blue-300 dark:ring-blue-800"
          : "border-slate-300 bg-white text-slate-700 hover:bg-slate-50 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700",
        className,
      )}
    >
      {capturing
        ? preview ?? "Press a key combo…"
        : formatDisplay(value)}
    </div>
  );
}

function formatDisplay(shortcut: string): string {
  return shortcut
    .replace("Command", "⌘")
    .replace("Control", "Ctrl")
    .replace("Shift", "⇧")
    .replace("Alt", "⌥");
}
