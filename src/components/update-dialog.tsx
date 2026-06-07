import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { check, type Update } from "@tauri-apps/plugin-updater";

type Phase = "idle" | "downloading" | "ready" | "error";

type Props = {
  update: Update;
  onDismiss: () => void;
};

export function UpdateDialog({ update, onDismiss }: Props) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const primaryBtnRef = useRef<HTMLButtonElement | null>(null);

  // Escape closes the dialog while idle; locked during download to avoid
  // user dismissing mid-write.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && phase !== "downloading") onDismiss();
    };
    window.addEventListener("keydown", onKey);
    primaryBtnRef.current?.focus();
    return () => window.removeEventListener("keydown", onKey);
  }, [phase, onDismiss]);

  async function applyUpdate() {
    setPhase("downloading");
    setError(null);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          setTotal(event.data.contentLength ?? null);
          setDownloaded(0);
        } else if (event.event === "Progress") {
          setDownloaded((d) => d + event.data.chunkLength);
        }
      });
      setPhase("ready");
    } catch (err) {
      setPhase("error");
      setError(String(err));
    }
  }

  const pct = total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null;

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-dialog-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      onClick={(e) => {
        if (phase === "idle" && e.target === e.currentTarget) onDismiss();
      }}
    >
      <div className="w-full max-w-md rounded-lg border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900">
        <h2 id="update-dialog-title" className="text-lg font-semibold">
          Update available
        </h2>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          SnipTeX {update.version} is ready to install.
          {update.currentVersion ? ` You're on ${update.currentVersion}.` : ""}
        </p>

        {update.body && (
          <pre className="mt-3 max-h-40 overflow-y-auto whitespace-pre-wrap rounded bg-slate-50 p-3 text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
            {update.body}
          </pre>
        )}

        {phase === "downloading" && (
          <div className="mt-4">
            <div className="h-2 w-full overflow-hidden rounded bg-slate-200 dark:bg-slate-700">
              <div
                className="h-full bg-blue-500 transition-[width] duration-150"
                style={{ width: pct != null ? `${pct}%` : "0%" }}
              />
            </div>
            <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
              {pct != null ? `${pct}% downloaded` : "Downloading…"}
            </p>
          </div>
        )}

        {phase === "ready" && (
          <p className="mt-4 text-sm text-green-700 dark:text-green-400">
            Update installed. Quit and reopen SnipTeX to use the new version.
          </p>
        )}

        {phase === "error" && error && (
          <p className="mt-4 text-sm text-red-600 dark:text-red-400">
            Update failed: {error}
          </p>
        )}

        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onDismiss}
            disabled={phase === "downloading"}
            className="rounded border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:hover:bg-slate-800"
          >
            {phase === "ready" ? "Close" : "Later"}
          </button>
          {phase !== "ready" && (
            <button
              ref={primaryBtnRef}
              type="button"
              onClick={applyUpdate}
              disabled={phase === "downloading"}
              className="rounded bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {phase === "error" ? "Retry" : "Update now"}
            </button>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}

export type UpdateCheckResult =
  | { kind: "available"; update: Update }
  | { kind: "none" }
  | { kind: "error"; message: string };

// Convenience hook: kick off a check and return a tagged result so callers
// don't race React state updates. Owners decide what to render.
export function useUpdateCheck(autoRunOnMount: boolean) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [checking, setChecking] = useState(false);
  const [ranOnce, setRanOnce] = useState(false);

  const runCheck = async (): Promise<UpdateCheckResult> => {
    setChecking(true);
    try {
      const result = await check();
      setUpdate(result);
      return result ? { kind: "available", update: result } : { kind: "none" };
    } catch (err) {
      return { kind: "error", message: String(err) };
    } finally {
      setChecking(false);
      setRanOnce(true);
    }
  };

  useEffect(() => {
    if (autoRunOnMount && !ranOnce) {
      void runCheck();
    }
  }, [autoRunOnMount, ranOnce]);

  return { update, checking, ranOnce, runCheck, dismiss: () => setUpdate(null) };
}
