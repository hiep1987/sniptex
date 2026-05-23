import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type AgentInfoDto = {
  spec: {
    id: string;
    display_name: string;
    kind: "CliBin" | "CloudApi";
  };
  binary_path: string;
  version: string | null;
};

type Props = {
  currentAgent: string;
  onClose: () => void;
  onPick: (agentId: string) => void | Promise<void>;
};

export function RerunMenu({ currentAgent, onClose, onPick }: Props) {
  const [agents, setAgents] = useState<AgentInfoDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    invoke<AgentInfoDto[]>("detect_agents")
      .then(setAgents)
      .catch((err) => setError(String(err)));
  }, []);

  // Close on outside click — defensive UX so the floating menu doesn't
  // strand itself when the user clicks somewhere else in the list.
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (!rootRef.current) return;
      if (!rootRef.current.contains(e.target as Node)) onClose();
    };
    // Schedule attach so the click that opened the menu doesn't
    // immediately close it.
    const t = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  return (
    <div
      ref={rootRef}
      className="absolute right-0 top-9 z-10 min-w-44 rounded-md border border-slate-200 bg-white p-1 text-xs shadow-lg dark:border-slate-700 dark:bg-slate-900"
    >
      <p className="px-2 py-1 text-[10px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
        Rerun with
      </p>
      {!agents && !error && (
        <p className="px-2 py-1 text-slate-500">Detecting…</p>
      )}
      {error && <p className="px-2 py-1 text-red-600">{error}</p>}
      {agents && agents.length === 0 && (
        <p className="px-2 py-1 text-slate-500">No agents installed.</p>
      )}
      {agents?.map((a) => {
        const disabled = a.spec.id === currentAgent;
        return (
          <button
            key={a.spec.id}
            type="button"
            disabled={disabled}
            onClick={() => void onPick(a.spec.id)}
            className="flex w-full items-center justify-between rounded px-2 py-1.5 text-left text-slate-700 hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-50 dark:text-slate-200 dark:hover:bg-slate-800"
            title={disabled ? "Already used for this snip" : ""}
          >
            <span>{a.spec.display_name}</span>
            {a.version && (
              <span className="ml-2 truncate font-mono text-[10px] text-slate-400">
                {a.version}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
