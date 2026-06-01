// Output-format conversions for the "Copy as…" menu.
//
// The OCR pipeline returns text already shaped by the master prompt
// (see `src-tauri/src/ocr/prompt.rs`):
//   EQUATION_ONLY → raw LaTeX with NO outer $…$ delimiters
//   TABLE_ONLY    → Markdown table with inline `$…$` math
//   MIXED         → Markdown with `$…$` and/or `$$…$$` blocks
//
// Conversions therefore operate on the master-prompt convention.

import { tauri, type DetectedType, type OutputFormat } from "./invoke";

export type FormatOption = {
  kind: OutputFormat;
  label: string;
};

const FORMAT_LABELS: Record<OutputFormat, string> = {
  smart: "Smart",
  inline: "Inline LaTeX",
  display: "Display LaTeX",
  plain: "Plain LaTeX",
  markdown: "Markdown",
  math_ml: "MathML",
  unicode_pretty: "Unicode",
};

export const FORMAT_ORDER: OutputFormat[] = [
  "plain",
  "smart",
  "inline",
  "display",
  "markdown",
  "math_ml",
  "unicode_pretty",
];

/**
 * Resolve the Smart setting to the master-prompt-native output for the
 * detected type: equations are already raw TeX, tables/mixed are Markdown.
 */
export function defaultFormatFor(detected: DetectedType | null): OutputFormat {
  if (detected === "EQUATION_ONLY") return "plain";
  return "markdown";
}

export function labelForFormat(kind: OutputFormat): string {
  return FORMAT_LABELS[kind];
}

export function sortFormats(kinds: OutputFormat[]): OutputFormat[] {
  const enabled = new Set(kinds);
  return FORMAT_ORDER.filter((kind) => enabled.has(kind));
}

export function copyAsOptions(kinds: OutputFormat[]): FormatOption[] {
  return sortFormats(kinds).map((kind) => ({ kind, label: labelForFormat(kind) }));
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
  kind: OutputFormat,
): Promise<string> {
  const resolved = kind === "smart" ? defaultFormatFor(detected) : kind;
  switch (kind) {
    case "smart":
      return formatOutput(text, detected, resolved);
    case "inline":
      if (detected !== "EQUATION_ONLY") return text;
      return `$${stripOuterMathDelimiters(text)}$`;
    case "display":
      if (detected !== "EQUATION_ONLY") return text;
      return `$$\n${stripOuterMathDelimiters(text)}\n$$`;
    case "plain":
      if (detected === "EQUATION_ONLY") return stripOuterMathDelimiters(text);
      return await markdownWithLatexTables(text);
    case "markdown":
      if (detected === "EQUATION_ONLY") {
        return `$$\n${stripOuterMathDelimiters(text)}\n$$`;
      }
      return text;
    case "math_ml":
      return await toMathML(text, detected);
    case "unicode_pretty":
      return toUnicodePretty(text);
  }
}

function stripOuterMathDelimiters(text: string): string {
  const trimmed = text.trim();
  if (trimmed.startsWith("$$") && trimmed.endsWith("$$")) {
    return trimmed.slice(2, -2).trim();
  }
  if (trimmed.startsWith("$") && trimmed.endsWith("$")) {
    return trimmed.slice(1, -1).trim();
  }
  return trimmed;
}

function toPlainText(text: string): string {
  // Strip the common LaTeX-math delimiters so users pasting into a
  // notes app get a readable approximation. Tables degrade to their
  // Markdown form minus the `$…$` wrappers.
  return text
    .replace(/\$\$([\s\S]*?)\$\$/g, "$1")
    .replace(/\$([^$\n]+)\$/g, "$1")
    .trim();
}

async function markdownWithLatexTables(text: string): Promise<string> {
  try {
    return await tauri.convertToTex(text);
  } catch (err) {
    console.warn("[format] convert_to_tex failed, falling back", err);
    return text;
  }
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
  if (detected !== "EQUATION_ONLY") return toPlainText(text);
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

function toUnicodePretty(text: string): string {
  return toPlainText(text)
    .replace(/\\alpha/g, "α")
    .replace(/\\beta/g, "β")
    .replace(/\\gamma/g, "γ")
    .replace(/\\Delta/g, "Δ")
    .replace(/\\delta/g, "δ")
    .replace(/\\theta/g, "θ")
    .replace(/\\lambda/g, "λ")
    .replace(/\\mu/g, "μ")
    .replace(/\\pi/g, "π")
    .replace(/\\sigma/g, "σ")
    .replace(/\\omega/g, "ω")
    .replace(/\\infty/g, "∞")
    .replace(/\\leq/g, "≤")
    .replace(/\\geq/g, "≥")
    .replace(/\\neq/g, "≠")
    .replace(/\\times/g, "×")
    .replace(/\\cdot/g, "·")
    .replace(/\\pm/g, "±")
    .replace(/\\to/g, "→");
}
