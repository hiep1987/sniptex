import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";

const VIEWPORT_GAP = 8;
const MENU_GAP = 4;
const MIN_MENU_WIDTH = 176;

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
  anchorEl: HTMLElement | null;
  currentAgent: string;
  onClose: () => void;
  onPick: (agentId: string) => void | Promise<void>;
};

export function RerunMenu({ anchorEl, currentAgent, onClose, onPick }: Props) {
  const [agents, setAgents] = useState<AgentInfoDto[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState<{
    top: number;
    left: number;
    maxHeight: number;
  } | null>(null);

  useLayoutEffect(() => {
    if (!anchorEl) return;

    const updatePosition = () => {
      const rect = anchorEl.getBoundingClientRect();
      const menuHeight = rootRef.current?.offsetHeight ?? 0;
      const menuWidth = rootRef.current?.offsetWidth ?? MIN_MENU_WIDTH;
      const spaceBelow =
        window.innerHeight - rect.bottom - MENU_GAP - VIEWPORT_GAP;
      const spaceAbove = rect.top - MENU_GAP - VIEWPORT_GAP;
      const opensBelow = menuHeight <= spaceBelow || spaceBelow >= spaceAbove;
      const availableHeight = Math.max(
        VIEWPORT_GAP,
        opensBelow ? spaceBelow : spaceAbove,
      );
      const top = opensBelow
        ? rect.bottom + MENU_GAP
        : Math.max(
            VIEWPORT_GAP,
            rect.top - MENU_GAP - Math.min(menuHeight, availableHeight),
          );
      const maxLeft = Math.max(
        VIEWPORT_GAP,
        window.innerWidth - menuWidth - VIEWPORT_GAP,
      );
      const left = Math.min(
        Math.max(VIEWPORT_GAP, rect.right - menuWidth),
        maxLeft,
      );

      setPos({ top, left, maxHeight: availableHeight });
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [anchorEl, agents, error]);

  useEffect(() => {
    invoke<AgentInfoDto[]>("detect_agents")
      .then(setAgents)
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (!rootRef.current) return;
      if (!rootRef.current.contains(e.target as Node)) onClose();
    };
    const t = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener("mousedown", handler);
    };
  }, [onClose]);

  if (!pos) return null;

  const menu = (
    <div
      ref={rootRef}
      style={{
        position: "fixed",
        top: pos.top,
        left: pos.left,
        maxHeight: pos.maxHeight,
      }}
      className="z-50 min-w-44 overflow-y-auto rounded-md border border-slate-200 bg-white p-1 text-xs shadow-lg dark:border-slate-700 dark:bg-slate-900"
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
            className="flex w-full cursor-pointer items-center justify-between rounded px-2 py-1.5 text-left text-slate-700 hover:bg-slate-100 disabled:cursor-not-allowed disabled:opacity-50 dark:text-slate-200 dark:hover:bg-slate-800"
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

  return createPortal(menu, document.body);
}
