import { useEffect, useState } from "react";
import { ExternalLink } from "lucide-react";
import { tauri } from "@/lib/invoke";

export default function AboutTab() {
  const [version, setVersion] = useState("…");

  useEffect(() => {
    tauri.hello().then((r) => setVersion(r.version)).catch(() => {});
  }, []);

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
      </div>

      <div className="space-y-2">
        <ExtLink href="https://github.com/nicekid1/sniptex">
          GitHub Repository
        </ExtLink>
        <ExtLink href="https://github.com/nicekid1/sniptex/issues">
          Report an Issue
        </ExtLink>
        <ExtLink href="https://github.com/sponsors/nicekid1">
          Sponsor on GitHub
        </ExtLink>
      </div>

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
