---
phase: 2
title: "CLI Agent PDF-via-Image Fallback"
status: done
priority: P1
effort: "3h"
dependencies: [1]
---

# Phase 2: CLI Agent PDF-via-Image Fallback

## Overview

CLI agents (Codex, Gemini CLI) only accept image files. When the user uploads a PDF and the selected agent is a CLI type, render each page to a PNG via a Rust PDF library, then run OCR on each page image sequentially and concatenate results.

## Requirements

- Functional: given a PDF path + CLI agent, produce per-page PNGs in temp dir, OCR each, return concatenated TeX
- Non-functional: temp PNGs cleaned up after processing; 30s timeout per page (not per PDF)

## Architecture

Use the `pdfium-render` crate (Rust bindings to PDFium) or `pdf` + `image` crates to render pages. Each page → temp PNG → existing `run_ocr(agent, png_path)` → collect results → join with `\n\n% --- Page N ---\n\n`.

Fallback: if no PDF renderer available at compile time, return error "PDF upload requires a cloud API agent (Gemini or Mistral). CLI agents cannot process PDF directly."

## Related Code Files

- Create: `src-tauri/src/ocr/pdf_render.rs` — render PDF pages to temp PNGs
- Modify: `src-tauri/Cargo.toml` — add PDF rendering dependency
- Modify: `src-tauri/src/ocr/mod.rs` — re-export pdf_render module

## Implementation Steps

1. Evaluate PDF rendering options: `pdfium-render` (needs bundled PDFium binary) vs `lopdf` + `image` (pure Rust but limited rendering) vs `mupdf` bindings. Recommend `mupdf` crate for quality + no external binary.
2. Add dependency to `Cargo.toml`.
3. Create `pdf_render.rs` with `render_pages_to_pngs(pdf_path: &str, temp_dir: &Path, dpi: u32) -> Result<Vec<PathBuf>>`. Default DPI: 200 (good balance of quality vs size for OCR).
4. Add `run_ocr_pdf_cli(agent: &AgentInfo, pdf_path: &str) -> Result<String>` that:
   - Creates temp dir
   - Renders all pages to PNGs
   - Calls `run_ocr(agent, page_png)` for each page sequentially
   - Concatenates results with page separator
   - Cleans up temp dir (RAII guard)
5. Add test with a small 2-page fixture PDF.

## Success Criteria

- [x] `render_pages_to_pngs` produces readable PNGs from a test PDF
- [x] CLI agent OCR on rendered page images produces valid TeX (via `run_pdf_cli`)
- [x] Temp files cleaned up on success and failure paths (RAII `TempDir`)
- [x] Existing image OCR tests unaffected

## Risk Assessment

- **PDF rendering crate adds binary size** — Mitigation: `mupdf` adds ~5MB; acceptable for a desktop app. Can make feature-gated if needed.
- **Complex PDFs (vector graphics, custom fonts) may render poorly** — Mitigation: 200 DPI default is sufficient for text/math; user can always use cloud API for better quality.
