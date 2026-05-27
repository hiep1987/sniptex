---
phase: 3
title: "Tauri Command + Dispatcher Wiring"
status: pending
priority: P1
effort: "2h"
dependencies: [1, 2]
---

# Phase 3: Tauri Command + Dispatcher Wiring

## Overview

Add a `run_pdf_ocr` Tauri command that accepts a PDF file path, routes to the correct adapter (cloud API direct vs CLI-via-rendered-images), concatenates results, copies to clipboard, saves to history, and emits events for the preview window.

## Requirements

- Functional: `run_pdf_ocr(pdf_path, agent_id?)` → same `SnipResult` shape as `run_snip`
- Functional: fallback chain respects settings agent_priority (same as snip)
- Functional: result saved to history with `source: "pdf"` distinction
- Non-functional: progress events emitted per page so frontend can show "Page 2/5…"

## Architecture

```
run_pdf_ocr(pdf_path, agent_id?)
  → detect agents
  → read settings priority
  → pick agent (explicit or fallback)
  → if CloudApi:
      call_with_pdf_path(pdf_path, prompt, api_key)
  → if CliBin:
      pdf_render::render_pages_to_pngs(pdf_path)
      for each page:
        emit("pdf-progress", { page, total })
        run_ocr(agent, page_png)
      concatenate results
  → post_process(text)
  → copy to clipboard
  → persist to history (image_path = pdf_path, thumb = first page render)
  → emit("snip-complete", result)
  → return SnipResult
```

## Related Code Files

- Modify: `src-tauri/src/commands.rs` — add `run_pdf_ocr` command
- Modify: `src-tauri/src/lib.rs` — register `run_pdf_ocr` in invoke_handler
- Modify: `src/lib/invoke.ts` — add `runPdfOcr` method
- Modify: `src-tauri/tauri.conf.json` — add `.pdf` to asset protocol scope if needed

## Implementation Steps

1. Add `run_pdf_ocr` command in `commands.rs`:
   - Accept `pdf_path: String`, `agent_id: Option<String>`
   - Open file dialog validation (check file exists, ends with .pdf)
   - Route by `AgentKind`: cloud → `call_with_pdf_path`, CLI → render + per-page OCR
   - Emit `pdf-progress` event with `{ page: usize, total: usize }` for CLI path
   - Run `post_process` on concatenated output
   - Copy to clipboard via `tauri-plugin-clipboard-manager`
   - Save to history: store original PDF path as `image_path`, render first page as thumb
2. Register command in `lib.rs` invoke_handler.
3. Add `runPdfOcr` to `invoke.ts`.
4. Add PDF prompt variant in `ocr/prompt.rs` — same master prompt but with "this is a multi-page document" prefix.

## Success Criteria

- [ ] `run_pdf_ocr` compiles and is registered
- [ ] Cloud API path sends PDF directly without rendering
- [ ] CLI path renders pages and OCRs each
- [ ] Progress events emitted for CLI multi-page path
- [ ] Result saved to history
- [ ] Existing `run_snip` unaffected

## Risk Assessment

- **Large PDFs (50+ pages) may hit timeout or memory limits** — Mitigation: emit progress per page; 30s timeout per page for CLI; cloud APIs handle internally. Consider page-count warning in frontend (Phase 4).
