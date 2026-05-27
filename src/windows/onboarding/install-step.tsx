import { useEffect, useState, useCallback } from "react";
import { RefreshCw, CheckCircle2, XCircle } from "lucide-react";
import { tauri, type AgentInfo } from "@/lib/invoke";
import { cn } from "@/lib/cn";

const IS_MAC = navigator.platform.includes("Mac");

const INSTALL_COMMANDS = {
  codex: {
    mac: "npm install -g @openai/codex",
    win: "npm install -g @openai/codex",
    note: "Recommended default — most reliable OCR results.",
  },
  "gemini-cli": {
    mac: "brew install google/gemini-cli/gemini-cli",
    win: "npm install -g @google/gemini-cli",
    note: "Experimental secondary — may fall back on some content types.",
  },
};

export default function InstallStep() {
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [scanning, setScanning] = useState(true);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      setAgents(await tauri.detectAgents());
    } catch (e) {
      console.error("[onboarding] detect failed", e);
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => { scan(); }, [scan]);

  const hasCodex = agents.some((a) => a.spec.id === "codex");
  const hasGemini = agents.some((a) => a.spec.id === "gemini-cli");
  const hasAnyCli = hasCodex || hasGemini;

  return (
    <div className="max-w-lg space-y-4">
      <h2 className="text-lg font-semibold">Install an OCR Agent</h2>
      <p className="text-sm text-slate-600 dark:text-slate-300">
        SnipTeX sends captured images to a CLI agent for OCR. Install at
        least one:
      </p>

      <div className="space-y-3">
        {(["codex", "gemini-cli"] as const).map((id) => {
          const installed = agents.some((a) => a.spec.id === id);
          const cmds = INSTALL_COMMANDS[id];
          const cmd = IS_MAC ? cmds.mac : cmds.win;

          return (
            <div
              key={id}
              className="rounded-lg border border-slate-200 p-3 dark:border-slate-700"
            >
              <div className="flex items-center gap-2">
                {installed ? (
                  <CheckCircle2 className="size-4 text-green-600" />
                ) : (
                  <XCircle className="size-4 text-slate-400" />
                )}
                <span className="text-sm font-medium">
                  {id === "codex" ? "OpenAI Codex" : "Gemini CLI"}
                </span>
                <span
                  className={cn(
                    "rounded px-1.5 py-0.5 text-[10px]",
                    installed
                      ? "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300"
                      : "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
                  )}
                >
                  {installed ? "Installed" : "Not found"}
                </span>
              </div>
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                {cmds.note}
              </p>
              {!installed && (
                <code className="mt-2 block rounded bg-slate-100 px-3 py-2 font-mono text-xs text-slate-700 select-all dark:bg-slate-800 dark:text-slate-300">
                  {cmd}
                </code>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={scan}
          disabled={scanning}
          className="inline-flex items-center gap-1.5 rounded-md border border-slate-300 px-2.5 py-1 text-xs text-slate-600 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-800"
        >
          <RefreshCw className={cn("size-3", scanning && "animate-spin")} />
          Re-scan
        </button>
        {hasAnyCli && (
          <p className="text-xs text-green-600 dark:text-green-400">
            You have a CLI agent — you can proceed!
          </p>
        )}
      </div>

      <p className="text-xs text-slate-400 dark:text-slate-500">
        No CLI? That's ok — the next step lets you add a cloud API key
        instead.
      </p>
    </div>
  );
}
