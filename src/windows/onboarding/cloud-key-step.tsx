import { useState, useEffect } from "react";
import { ExternalLink, ShieldCheck } from "lucide-react";
import { tauri } from "@/lib/invoke";
import ApiKeyInput from "@/components/api-key-input";

const PROVIDERS: Array<{
  id: string;
  label: string;
  url: string;
  urlLabel: string;
  linkLabel?: string;
}> = [
  {
    id: "gemini",
    label: "Google Gemini API",
    url: "https://aistudio.google.com/api-keys",
    urlLabel: "Google AI Studio",
  },
  {
    id: "mistral",
    label: "Mistral Vision API",
    url: "https://admin.mistral.ai",
    urlLabel: "Mistral Admin",
    // Mistral has no free tier — make the CTA honest.
    linkLabel: "Get a paid key",
  },
];

export default function CloudKeyStep() {
  const [keys, setKeys] = useState<Record<string, string>>({ gemini: "", mistral: "" });
  const [saved, setSaved] = useState<Record<string, boolean>>({ gemini: false, mistral: false });

  useEffect(() => {
    (async () => {
      const g = await tauri.hasApiKey("gemini");
      const m = await tauri.hasApiKey("mistral");
      setSaved({ gemini: g, mistral: m });
    })();
  }, []);

  const save = async (provider: string) => {
    const key = keys[provider]?.trim();
    if (!key) return;
    await tauri.setApiKey(provider, key);
    setSaved((s) => ({ ...s, [provider]: true }));
    setKeys((k) => ({ ...k, [provider]: "" }));
    await tauri.updateSettings({ cloud_mode_enabled: true });
  };

  return (
    <div className="max-w-lg space-y-4">
      <h2 className="text-lg font-semibold">
        Bring Your Own Key{" "}
        <span className="text-sm font-normal text-slate-500">(optional)</span>
      </h2>
      <p className="text-sm text-slate-600 dark:text-slate-300">
        Want sub-5-second response? Add a cloud API key. You can set one,
        both, or skip this entirely.
      </p>

      <div className="space-y-4">
        {PROVIDERS.map((p) => (
          <div
            key={p.id}
            className="rounded-lg border border-slate-200 p-3 dark:border-slate-700"
          >
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">{p.label}</span>
              {saved[p.id] && (
                <span className="rounded bg-green-100 px-1.5 py-0.5 text-[10px] text-green-700 dark:bg-green-900 dark:text-green-300">
                  Key saved
                </span>
              )}
            </div>

            {!saved[p.id] ? (
              <div className="mt-2 space-y-2">
                <ApiKeyInput
                  value={keys[p.id]}
                  onChange={(v) => setKeys((k) => ({ ...k, [p.id]: v }))}
                  placeholder={`Paste ${p.urlLabel} API key`}
                />
                <div className="flex items-center gap-3">
                  <button
                    type="button"
                    onClick={() => save(p.id)}
                    disabled={!keys[p.id]?.trim()}
                    className="rounded-md bg-slate-900 px-3 py-1 text-xs font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
                  >
                    Save key
                  </button>
                  <a
                    href={p.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={(e) => {
                      e.preventDefault();
                      void tauri.openExternal(p.url);
                    }}
                    className="text-xs text-blue-600 hover:underline dark:text-blue-400"
                  >
                    {p.linkLabel ?? "Get a free key"} <ExternalLink className="ml-0.5 inline size-3" />
                  </a>
                </div>
              </div>
            ) : (
              <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                Key is stored in your OS keychain.
              </p>
            )}
          </div>
        ))}
      </div>

      <div className="flex items-start gap-2 rounded-md bg-slate-50 p-3 text-xs text-slate-500 dark:bg-slate-900 dark:text-slate-400">
        <ShieldCheck className="mt-0.5 size-4 shrink-0" />
        <p>
          Your key is stored in the OS keychain (macOS Keychain / Windows
          Credential Manager). Cloud mode sends the captured image to the
          provider's servers for OCR.
        </p>
      </div>
    </div>
  );
}
