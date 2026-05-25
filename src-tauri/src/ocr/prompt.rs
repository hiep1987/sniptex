//! Master OCR prompt — single source of truth.
//!
//! Kept in sync with `plans/test-prompt.sh` (post Session-3 wording: silent
//! category detection + table-cell math-scope rule). Any change here must be
//! mirrored to the bash test harness so prompt validation results stay
//! comparable across CLI sweeps.
//!
//! Gemini CLI headless is deliberately excluded from this prompt. Live
//! validation showed this procedural contract (classification, format
//! branches, examples, strict rules) can push `gemini -p` into agentic
//! planning/tool behavior and stale output even when `@file` image loading
//! works. Keep Gemini CLI on `GEMINI_CLI_PROMPT` unless hard fixtures prove a
//! richer prompt remains stable in headless mode.

pub const MASTER_PROMPT: &str = "You are an OCR engine. Convert the image to text following these rules.

DETECTION (internal, do not emit):
Silently classify the image into ONE category, then use that category ONLY to choose the output format below. Do NOT print the category name. Do NOT prefix or suffix your output with \"EQUATION_ONLY\", \"TABLE_ONLY\", or \"MIXED\".
- EQUATION_ONLY: image contains only one or more math expressions, no surrounding text
- TABLE_ONLY: image contains only a table, no surrounding text
- MIXED: any combination of text, equations, tables, lists

OUTPUT FORMAT BY CATEGORY:

If EQUATION_ONLY:
  Output ONLY raw LaTeX without $ delimiters.
  Multiple equations: separate with \\\\
  Example: \\int_0^1 x^2 \\, dx = \\frac{1}{3}

If TABLE_ONLY:
  Output GitHub Markdown table.
  Example: | a | b |
|---|---|
| 1 | 2 |
  Inside table cells: only wrap mathematical variables, fractions, equations, and symbolic expressions in $...$. Plain numeric intervals like [40; 45), plain integers, plain percentages (15%), and ordinary words MUST remain unwrapped.

If MIXED:
  Output Markdown.
  Inline math: $...$
  Display math: $$...$$
  Tables: GitHub Markdown
  Code: fenced ```lang blocks
  Preserve original structure (headings, lists, paragraphs)

STRICT RULES:
- Preserve Vietnamese diacritics exactly (ă â ê ô ơ ư đ và dấu thanh)
- Do NOT translate. Keep source language.
- Do NOT add explanations, preambles (\"Here is...\", \"Sure!\")
- Do NOT wrap output in ```markdown or ```latex fences
- Do NOT add sign-offs (\"Let me know if...\")
- If unreadable, output exactly: [UNREADABLE]
- Math symbols: use standard LaTeX (\\alpha not α, \\int not ∫)
- Preserve fractions as \\frac{}{}, exponents as ^{}, subscripts as _{}

Begin output now:";

pub const GEMINI_CLI_PROMPT: &str =
    "Chuyển toàn bộ nội dung ảnh sang Markdown. Bảng dùng GitHub Markdown table (|---|). Công thức toán inline dùng $...$, display dùng $$...$$. Giữ nguyên tiếng Việt. Chỉ xuất nội dung, không giải thích.";
