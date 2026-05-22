import { useEffect, useMemo, useState } from "react";

type Props = {
  source: string;
};

// markdown-it-katex bundles its own KaTeX import; we also need the
// KaTeX stylesheet so rendered formulas pick up their font + layout.
// Mount the sheet once per window via a top-level dynamic import inside
// the component effect so windows that never render Markdown don't pay
// the cost.

let mdInstance: import("markdown-it").default | null = null;
let stylesMounted = false;

async function getMarkdownIt(): Promise<import("markdown-it").default> {
  if (mdInstance) return mdInstance;
  // `@vscode/markdown-it-katex` is the actively-maintained KaTeX
  // bridge that ships with current KaTeX; the older `markdown-it-katex`
  // bundles KaTeX 0.5.x and mis-parses modern `x^{n}` superscripts as
  // subscripts.
  const [{ default: MarkdownIt }, { default: mdKatex }] = await Promise.all([
    import("markdown-it"),
    import("@vscode/markdown-it-katex"),
  ]);
  // `html: false` blocks raw HTML from the LLM output reaching the DOM
  // (XSS guard). `linkify: true` auto-detects bare URLs in MIXED snips.
  // `breaks: true`: OCR output preserves visual line breaks (e.g. multi-
  // choice answers A) / B) / C) on separate lines).
  mdInstance = new MarkdownIt({
    html: false,
    linkify: true,
    breaks: true,
  });
  mdInstance.use(mdKatex, { throwOnError: false, errorColor: "#cc0000" });
  return mdInstance;
}

async function ensureKatexStyles() {
  if (stylesMounted) return;
  // KaTeX CSS is plain — Vite handles the side-effect import.
  await import("katex/dist/katex.min.css");
  stylesMounted = true;
}

export function MarkdownRenderer({ source }: Props) {
  const [html, setHtml] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  const trimmed = useMemo(() => source.trim(), [source]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await ensureKatexStyles();
        const md = await getMarkdownIt();
        if (cancelled) return;
        setHtml(md.render(trimmed));
        setError(null);
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [trimmed]);

  if (error) {
    return (
      <div className="rounded border border-rose-500/40 bg-rose-500/10 p-3 font-mono text-xs text-rose-700 dark:text-rose-300">
        <div className="mb-1 font-semibold">Markdown render error</div>
        <div className="whitespace-pre-wrap">{error}</div>
      </div>
    );
  }

  return (
    <div
      className="markdown-renderer max-w-none overflow-auto text-sm text-slate-900 dark:text-slate-100"
      // markdown-it output has been sanitized (html:false) — safe to inject.
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
