// Lazy MathJax 3 loader. The full MathJax-CHTML bundle is ~500KB
// before gzip, so we dynamic-import it on first call and cache the
// resulting adapter promise. Only Preview Window reaches this path;
// the other windows never pay the cost.
//
// Implementation note: MathJax's "building-blocks" API requires a
// MathDocument that owns parsing + output. Calling `doc.convert(tex)`
// returns a styled CHTML node that's *already typeset* — we just need
// to (1) make sure CHTML's stylesheet is in <head> and (2) ensure the
// CHTML output jax has measured the page metrics at least once so
// glyph sizing is correct.

type MathJaxAdapter = {
  /**
   * Render a TeX string and return a *fresh* DOM node. The node is
   * detached — the caller must append it to the live DOM.
   */
  tex2chtml: (input: string, options: { display: boolean }) => HTMLElement;
  /** Re-mount the CHTML stylesheet if it was evicted. */
  ensureStyles: () => void;
};

let pending: Promise<MathJaxAdapter> | null = null;

export function loadMathJax(): Promise<MathJaxAdapter> {
  if (pending) return pending;

  pending = (async () => {
    const [
      { mathjax },
      { TeX },
      { CHTML },
      { browserAdaptor },
      { RegisterHTMLHandler },
      { AllPackages },
    ] = await Promise.all([
      import("mathjax-full/js/mathjax.js"),
      import("mathjax-full/js/input/tex.js"),
      import("mathjax-full/js/output/chtml.js"),
      import("mathjax-full/js/adaptors/browserAdaptor.js"),
      import("mathjax-full/js/handlers/html.js"),
      import("mathjax-full/js/input/tex/AllPackages.js"),
    ]);

    const adaptor = browserAdaptor();
    RegisterHTMLHandler(adaptor);

    const tex = new TeX({ packages: AllPackages });
    // Use jsdelivr-hosted woff2 fonts for now. Phase 11 bundles them
    // locally so the production build works offline.
    const chtml = new CHTML({
      fontURL:
        "https://cdn.jsdelivr.net/npm/mathjax@3/es5/output/chtml/fonts/woff-v2",
    });

    const doc = mathjax.document(document, { InputJax: tex, OutputJax: chtml });

    const ensureStyles = () => {
      // CHTML emits its own <style> element; mount once into <head>.
      const styles = chtml.styleSheet(doc) as unknown as HTMLStyleElement;
      if (styles && !document.head.contains(styles)) {
        document.head.appendChild(styles);
      }
    };

    return {
      tex2chtml(input, { display }) {
        // `doc.convert` parses the TeX, runs the CHTML output jax, and
        // returns a fully-typeset node. We do NOT call updateDocument
        // here — that would re-process all math on the page, which is
        // both wasteful and resets sizing for previously rendered nodes.
        ensureStyles();
        return doc.convert(input, { display }) as HTMLElement;
      },
      ensureStyles,
    };
  })();

  return pending;
}

export async function ensureMathJaxStylesMounted(): Promise<void> {
  const adapter = await loadMathJax();
  adapter.ensureStyles();
}
