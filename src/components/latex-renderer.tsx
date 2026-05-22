import { useEffect, useRef, useState } from "react";
import { ensureMathJaxStylesMounted, loadMathJax } from "@/lib/mathjax-loader";

type Props = {
  latex: string;
  displayMode?: boolean;
  onError?: (err: unknown) => void;
};

/**
 * Renders a raw TeX string via MathJax 3 CommonHTML output.
 * MathJax is lazy-loaded the first time this component mounts in a
 * given window; subsequent renders re-use the cached adapter.
 */
export function LatexRenderer({ latex, displayMode = true, onError }: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        await ensureMathJaxStylesMounted();
        const adapter = await loadMathJax();
        if (cancelled || !hostRef.current) return;
        const node = adapter.tex2chtml(latex, { display: displayMode });
        hostRef.current.replaceChildren(node);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        console.error("[latex-renderer] failed", err);
        setError(msg);
        onError?.(err);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [latex, displayMode, onError]);

  if (error) {
    return (
      <div className="rounded border border-rose-500/40 bg-rose-500/10 p-3 font-mono text-xs text-rose-700 dark:text-rose-300">
        <div className="mb-1 font-semibold">MathJax error</div>
        <div className="whitespace-pre-wrap">{error}</div>
      </div>
    );
  }

  return (
    <div
      ref={hostRef}
      className="latex-renderer overflow-auto text-slate-900 dark:text-slate-100"
    />
  );
}
