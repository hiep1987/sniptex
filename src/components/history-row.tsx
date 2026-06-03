import { useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { Copy, Loader2, RotateCw, Trash2 } from "lucide-react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { toast } from "sonner";
import type { HistoryItem } from "@/stores/history-store";
import type { SnipResult } from "@/lib/invoke";
import { RerunMenu } from "./rerun-menu";

type Props = {
  item: HistoryItem;
  onDelete: (id: number) => void | Promise<void>;
  onRerun: (id: number, agentId: string) => Promise<HistoryItem>;
};

export function HistoryRow({ item, onDelete, onRerun }: Props) {
  const [rerunOpen, setRerunOpen] = useState(false);
  const [rerunning, setRerunning] = useState(false);
  const rerunBtnRef = useRef<HTMLButtonElement | null>(null);
  const thumbSrc = useMemo(
    () => (item.thumb_path ? convertFileSrc(item.thumb_path) : null),
    [item.thumb_path],
  );
  const relativeTime = useMemo(
    () => formatRelative(item.created_at),
    [item.created_at],
  );

  const handleCopy = async () => {
    try {
      await writeText(item.text);
      toast.success("Copied to clipboard");
    } catch (err) {
      toast.error("Copy failed", { description: String(err) });
    }
  };

  const handleOpenPreview = async () => {
    const payload: SnipResult = {
      status: "ok",
      text: item.text,
      detected: item.detected,
      agent: item.agent,
      image_path: item.image_path,
      record_id: item.id,
    };

    try {
      await emit("history-preview-open", payload);
    } catch (err) {
      toast.error("Preview failed", { description: String(err) });
    }
  };

  return (
    <article
      data-testid="history-row"
      className="group flex gap-3 border-b border-slate-100 px-4 py-3 dark:border-slate-800"
    >
      {thumbSrc ? (
        <img
          src={thumbSrc}
          alt=""
          loading="lazy"
          className="size-16 shrink-0 rounded border border-slate-200 object-contain dark:border-slate-700"
        />
      ) : (
        <div className="size-16 shrink-0 rounded border border-dashed border-slate-200 dark:border-slate-700" />
      )}

      <button
        type="button"
        onClick={() => void handleOpenPreview()}
        className="min-w-0 flex-1 cursor-pointer rounded-sm text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
        title="Open in preview"
        aria-label="Open history item in preview"
      >
        <span className="mb-1 flex flex-wrap items-center gap-2 text-[10px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
          <span>{relativeTime}</span>
          <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300">
            {item.agent}
          </span>
          <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] text-slate-600 dark:bg-slate-800 dark:text-slate-300">
            {item.detected.replace("_", " ").toLowerCase()}
          </span>
          {item.latency_ms > 0 && (
            <span className="text-slate-400">{item.latency_ms} ms</span>
          )}
        </span>
        <span className="line-clamp-2 font-mono text-xs text-slate-700 dark:text-slate-300">
          {item.text}
        </span>
      </button>

      <div className={`relative flex shrink-0 flex-col items-end gap-1 transition focus-within:opacity-100 ${rerunOpen || rerunning ? "opacity-100" : "opacity-0 group-hover:opacity-100"}`}>
        <button
          type="button"
          onClick={handleCopy}
          title="Copy text"
          className="inline-flex size-7 items-center justify-center rounded text-slate-500 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
        >
          <Copy className="size-3.5" />
        </button>
        <button
          ref={rerunBtnRef}
          type="button"
          onClick={() => setRerunOpen((v) => !v)}
          disabled={rerunning}
          title={rerunning ? "Rerunning…" : "Rerun with another agent"}
          className="inline-flex size-7 items-center justify-center rounded text-slate-500 hover:bg-slate-100 hover:text-slate-900 disabled:cursor-wait dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
        >
          {rerunning ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <RotateCw className="size-3.5" />
          )}
        </button>
        <button
          type="button"
          onClick={() => void onDelete(item.id)}
          title="Delete"
          className="inline-flex size-7 items-center justify-center rounded text-slate-500 hover:bg-red-100 hover:text-red-700 dark:text-slate-400 dark:hover:bg-red-950 dark:hover:text-red-300"
        >
          <Trash2 className="size-3.5" />
        </button>

        {rerunOpen && (
          <RerunMenu
            anchorEl={rerunBtnRef.current}
            currentAgent={item.agent}
            onClose={() => setRerunOpen(false)}
            onPick={async (agentId) => {
              setRerunOpen(false);
              setRerunning(true);
              try {
                const result = await onRerun(item.id, agentId);
                if (result) {
                  toast.success(`Rerun complete with ${result.agent}`);
                }
              } catch (err) {
                toast.error("Rerun failed", { description: String(err) });
              } finally {
                setRerunning(false);
              }
            }}
          />
        )}
      </div>
    </article>
  );
}

function formatRelative(unixSeconds: number): string {
  const now = Date.now() / 1000;
  const diff = Math.max(0, now - unixSeconds);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
  const days = Math.floor(diff / 86400);
  if (days < 30) return `${days} d ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString();
}
