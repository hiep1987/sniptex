import { useEffect, useState, useCallback } from "react";
import { GripVertical, RefreshCw, Trash2, ExternalLink } from "lucide-react";
import { tauri, type AgentInfo } from "@/lib/invoke";
import { useSettingsStore } from "@/stores/settings-store";
import ApiKeyInput from "@/components/api-key-input";
import { cn } from "@/lib/cn";

const CLOUD_PROVIDERS: Record<string, { keyLabel: string; getKeyUrl: string }> = {
  "cloud-gemini": {
    keyLabel: "Google AI Studio",
    getKeyUrl: "https://aistudio.google.com/apikey",
  },
  "cloud-mistral": {
    keyLabel: "Mistral Console",
    getKeyUrl: "https://console.mistral.ai/api-keys",
  },
};

export default function AgentsTab() {
  const { agent_priority, patch } = useSettingsStore();
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [scanning, setScanning] = useState(false);
  const [keyStates, setKeyStates] = useState<Record<string, boolean>>({});
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [keyDraft, setKeyDraft] = useState("");

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      const found = await tauri.detectAgents();
      setAgents(found);
      const ks: Record<string, boolean> = {};
      for (const id of ["gemini", "mistral"]) {
        ks[id] = await tauri.hasApiKey(id);
      }
      setKeyStates(ks);
    } catch (e) {
      console.error("[agents] scan failed", e);
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => { scan(); }, [scan]);

  const ALL_KNOWN = ["codex", "cloud-gemini", "cloud-mistral", "gemini-cli"];
  const allIds = [
    ...agent_priority.filter((id) => ALL_KNOWN.includes(id)),
    ...ALL_KNOWN.filter((id) => !agent_priority.includes(id)),
  ];

  const moveUp = (idx: number) => {
    if (idx === 0) return;
    const next = [...allIds];
    [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
    patch({ agent_priority: next });
  };

  const moveDown = (idx: number) => {
    if (idx >= allIds.length - 1) return;
    const next = [...allIds];
    [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
    patch({ agent_priority: next });
  };

  const saveKey = async (provider: string) => {
    if (!keyDraft.trim()) return;
    const providerKey = provider === "cloud-gemini" ? "gemini" : "mistral";
    await tauri.setApiKey(providerKey, keyDraft.trim());
    setKeyStates((s) => ({ ...s, [providerKey]: true }));
    setEditingKey(null);
    setKeyDraft("");
    scan();
  };

  const deleteKey = async (provider: string) => {
    const providerKey = provider === "cloud-gemini" ? "gemini" : "mistral";
    await tauri.deleteApiKey(providerKey);
    setKeyStates((s) => ({ ...s, [providerKey]: false }));
    scan();
  };

  return (
    <div className="max-w-xl space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Agents</h2>
        <button
          type="button"
          onClick={scan}
          disabled={scanning}
          className="inline-flex items-center gap-1.5 rounded-md border border-slate-300 px-2.5 py-1 text-xs text-slate-600 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-800"
        >
          <RefreshCw className={cn("size-3", scanning && "animate-spin")} />
          Re-scan
        </button>
      </div>

      <p className="text-xs text-slate-500 dark:text-slate-400">
        Drag to reorder fallback priority. Top agent is tried first.
      </p>

      <div className="space-y-2">
        {allIds.map((id, idx) => {
          const info = agents.find((a) => a.spec.id === id);
          const isCloud = id.startsWith("cloud-");
          const providerKey = id === "cloud-gemini" ? "gemini" : id === "cloud-mistral" ? "mistral" : null;
          const hasKey = providerKey ? keyStates[providerKey] ?? false : false;
          const installed = !!info;
          const cloud = CLOUD_PROVIDERS[id];

          return (
            <div
              key={id}
              className="rounded-lg border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-900"
            >
              <div className="flex items-center gap-2">
                <div className="flex flex-col gap-0.5">
                  <button
                    type="button"
                    onClick={() => moveUp(idx)}
                    disabled={idx === 0}
                    className="text-slate-400 hover:text-slate-600 disabled:opacity-30 dark:hover:text-slate-300"
                  >
                    <GripVertical className="size-3 rotate-180" />
                  </button>
                  <button
                    type="button"
                    onClick={() => moveDown(idx)}
                    disabled={idx >= allIds.length - 1}
                    className="text-slate-400 hover:text-slate-600 disabled:opacity-30 dark:hover:text-slate-300"
                  >
                    <GripVertical className="size-3" />
                  </button>
                </div>

                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">
                      {info?.spec.display_name ?? id}
                    </span>
                    <span
                      className={cn(
                        "rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase",
                        isCloud
                          ? "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300"
                          : "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400",
                      )}
                    >
                      {isCloud ? "Cloud" : "CLI"}
                    </span>
                    <span
                      className={cn(
                        "rounded px-1.5 py-0.5 text-[10px]",
                        installed || hasKey
                          ? "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300"
                          : "bg-red-100 text-red-600 dark:bg-red-900 dark:text-red-400",
                      )}
                    >
                      {isCloud
                        ? hasKey ? "Key set" : "No key"
                        : installed ? "Installed" : "Not found"}
                    </span>
                  </div>
                  {info?.version && (
                    <p className="text-[11px] text-slate-400">{info.version}</p>
                  )}
                </div>
              </div>

              {isCloud && cloud && (
                <div className="mt-2 space-y-2 border-t border-slate-100 pt-2 dark:border-slate-800">
                  {editingKey === id ? (
                    <div className="space-y-2">
                      <ApiKeyInput
                        value={keyDraft}
                        onChange={setKeyDraft}
                        placeholder={`Paste ${cloud.keyLabel} API key`}
                      />
                      <div className="flex gap-2">
                        <button
                          type="button"
                          onClick={() => saveKey(id)}
                          disabled={!keyDraft.trim()}
                          className="rounded-md bg-slate-900 px-3 py-1 text-xs font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
                        >
                          Save key
                        </button>
                        <button
                          type="button"
                          onClick={() => { setEditingKey(null); setKeyDraft(""); }}
                          className="rounded-md border border-slate-300 px-3 py-1 text-xs dark:border-slate-600"
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => { setEditingKey(id); setKeyDraft(""); }}
                        className="text-xs text-blue-600 hover:underline dark:text-blue-400"
                      >
                        {hasKey ? "Update API key" : "Set API key"}
                      </button>
                      {hasKey && (
                        <button
                          type="button"
                          onClick={() => deleteKey(id)}
                          className="text-xs text-red-500 hover:underline"
                        >
                          <Trash2 className="mr-0.5 inline size-3" />
                          Remove
                        </button>
                      )}
                      <a
                        href={cloud.getKeyUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs text-slate-500 hover:underline dark:text-slate-400"
                      >
                        Get a free key <ExternalLink className="ml-0.5 inline size-3" />
                      </a>
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <p className="text-xs text-slate-400 italic">
        More agents (Claude Code, OpenCode) coming in v1.x.
      </p>
    </div>
  );
}
