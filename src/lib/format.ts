// Output-format conversions for the "Copy as…" menu.
//
// The OCR pipeline returns text already shaped by the master prompt
// (see `src-tauri/src/ocr/prompt.rs`):
//   EQUATION_ONLY → raw LaTeX with NO outer $…$ delimiters
//   TABLE_ONLY    → Markdown table with inline `$…$` math
//   MIXED         → Markdown with `$…$` and/or `$$…$$` blocks
//
// Conversions therefore operate on the master-prompt convention.

import { tauri, type DetectedType } from "./invoke";

// Export modes the user can pick from "Copy as…". Kept intentionally
// small so every entry has a *meaningful* transformation today; the
// upstream OCR agent (Codex / Gemini) already decides inline vs
// display vs Markdown-mixed semantics, so we don't expose redundant
// delimiter-only variants until Phase 9 wires real conversion.
export type FormatKind = "raw" | "tex" | "plain" | "markdown" | "mathml";

export type FormatOption = {
  kind: FormatKind;
  label: string;
};

/**
 * Choose a sensible default copy format for the given detected type.
 * Equation-only snips ship as TeX (paste-into-.tex friendly);
 * everything else flows through as Markdown.
 */
export function defaultFormatFor(detected: DetectedType | null): FormatKind {
  if (detected === "EQUATION_ONLY") return "tex";
  return "markdown";
}

/**
 * Produce the text payload for a given format. Pure function — does not
 * touch the clipboard. The Preview Window wraps this in a Tauri
 * clipboard-manager `writeText` call.
 *
 * `mathml` is async because it needs MathJax loaded; the others are
 * sync. We keep the signature uniformly async so callers don't branch.
 */
export async function formatOutput(
  text: string,
  detected: DetectedType | null,
  kind: FormatKind,
): Promise<string> {
  switch (kind) {
    case "raw":
      return text;
    case "tex":
      // EQUATION_ONLY: master prompt emits paste-ready LaTeX already,
      // nothing to convert. TABLE_ONLY / MIXED: flip GitHub Markdown
      // tables to `\begin{tabular}` so the paste lands cleanly in a
      // `.tex` file. Inline `$…$` math passes through untouched.
      if (detected === "EQUATION_ONLY") return text;
      try {
        return await tauri.convertToTex(text);
      } catch (err) {
        // Backend conversion is best-effort; fall back to the raw
        // Markdown so the user always gets *something* on the
        // clipboard rather than a silent failure.
        console.warn("[format] convert_to_tex failed, falling back", err);
        return text;
      }
    case "plain":
      return toPlain(text);
    case "markdown":
      return text;
    case "mathml":
      return await toMathML(text, detected);
  }
}

function toPlain(text: string): string {
  // Strip the common LaTeX-math delimiters so users pasting into a
  // notes app get a readable approximation. Tables degrade to their
  // Markdown form minus the `$…$` wrappers.
  return text
    .replace(/\$\$([\s\S]*?)\$\$/g, "$1")
    .replace(/\$([^$\n]+)\$/g, "$1")
    .trim();
}

async function toMathML(
  text: string,
  detected: DetectedType | null,
): Promise<string> {
  // MathML conversion only makes sense for raw equations.
  // mathjax-full 3.x exposes MathML output through the internal
  // SerializedMmlVisitor on the parsed MML tree rather than as a
  // top-level OutputJax. Phase 9 (Format Toggle scope) wires the full
  // pipeline; for now we surface a clearly-flagged fallback so the
  // menu item still does something useful instead of erroring.
  if (detected !== "EQUATION_ONLY") return toPlain(text);
  const { mathjax } = await import("mathjax-full/js/mathjax.js");
  const { TeX } = await import("mathjax-full/js/input/tex.js");
  const { liteAdaptor } = await import(
    "mathjax-full/js/adaptors/liteAdaptor.js"
  );
  const { RegisterHTMLHandler } = await import(
    "mathjax-full/js/handlers/html.js"
  );
  const { AllPackages } = await import(
    "mathjax-full/js/input/tex/AllPackages.js"
  );
  const { SerializedMmlVisitor } = await import(
    "mathjax-full/js/core/MmlTree/SerializedMmlVisitor.js"
  );

  const adaptor = liteAdaptor();
  RegisterHTMLHandler(adaptor);
  const tex = new TeX({ packages: AllPackages });
  const doc = mathjax.document("", { InputJax: tex });
  const mathItem = doc.convert(text.trim(), {
    display: true,
    end: 2, // stop after the MML compile pass; we don't need an output jax.
  }) as unknown as { root: unknown };
  const visitor = new SerializedMmlVisitor();
  // visitor expects an MmlNode; the TeX compile pass produces one.
  return visitor.visitTree(mathItem.root as never);
}

export const COPY_AS_OPTIONS: FormatOption[] = [
  { kind: "raw", label: "Raw OCR text" },
  { kind: "tex", label: "TeX" },
  { kind: "plain", label: "Plain text" },
  { kind: "markdown", label: "Markdown" },
  { kind: "mathml", label: "MathML" },
];
