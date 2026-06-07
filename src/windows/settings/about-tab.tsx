import { useEffect, useState } from "react";
import { ExternalLink, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { tauri } from "@/lib/invoke";
import { UpdateDialog, useUpdateCheck } from "@/components/update-dialog";

export default function AboutTab() {
  const [version, setVersion] = useState("…");
  const { update, checking, runCheck, dismiss } = useUpdateCheck(false);

  useEffect(() => {
    tauri.hello().then((r) => setVersion(r.version)).catch(() => {});
  }, []);

  async function onCheckClick() {
    const result = await runCheck();
    if (result.kind === "available") return; // dialog renders below
    if (result.kind === "none") {
      toast.success("You're on the latest version");
    } else {
      toast.error("Update check unavailable", { description: result.message });
    }
  }

  return (
    <div className="max-w-xl space-y-6">
      <h2 className="text-lg font-semibold">About SnipTeX</h2>

      <div className="rounded-lg border border-slate-200 bg-white p-5 dark:border-slate-700 dark:bg-slate-900">
        <p className="text-2xl font-semibold tracking-tight">SnipTeX</p>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          Version {version}
        </p>
        <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">
          Free, open-source OCR snip tool for LaTeX and Markdown.
          <br />
          Bring your own agent or API key.
        </p>
        <button
          type="button"
          onClick={onCheckClick}
          disabled={checking}
          className="mt-4 inline-flex items-center gap-1.5 rounded border border-slate-300 px-3 py-1.5 text-xs text-slate-700 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:text-slate-200 dark:hover:bg-slate-800"
        >
          <RefreshCw className={checking ? "size-3.5 animate-spin" : "size-3.5"} />
          {checking ? "Checking…" : "Check for updates"}
        </button>
      </div>

      <div className="space-y-2">
        <ExtLink href="https://github.com/hiep1987/sniptex">
          GitHub Repository
        </ExtLink>
        <ExtLink href="https://github.com/hiep1987/sniptex/issues">
          Report an Issue
        </ExtLink>
        <ExtLink href="https://github.com/sponsors/hiep1987">
          Sponsor on GitHub
        </ExtLink>
      </div>

      {update && <UpdateDialog update={update} onDismiss={dismiss} />}

      <div className="space-y-1 text-xs text-slate-400 dark:text-slate-500">
        <p>MIT License · Copyright © 2026 SnipTeX contributors</p>
        <p>
          Built with Tauri, React, and Rust.
        </p>
      </div>

      <button
        type="button"
        onClick={() => tauri.showWindow("onboarding")}
        className="text-xs text-blue-600 hover:underline dark:text-blue-400"
      >
        Replay onboarding
      </button>
    </div>
  );
}

function ExtLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        e.preventDefault();
        void tauri.openExternal(href);
      }}
      className="flex items-center gap-1.5 text-sm text-slate-600 hover:text-slate-900 hover:underline dark:text-slate-300 dark:hover:text-white"
    >
      <ExternalLink className="size-3.5" />
      {children}
    </a>
  );
}
