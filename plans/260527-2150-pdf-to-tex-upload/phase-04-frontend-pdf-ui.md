---
phase: 4
title: "Frontend: File Picker + PDF Preview + Progress"
status: pending
priority: P1
effort: "3h"
dependencies: [3]
---

# Phase 4: Frontend — File Picker, PDF Preview, Progress

## Overview

Add "Open PDF" button to main window, file picker dialog (`.pdf` filter), processing progress bar for multi-page CLI path, and route the result to the existing preview window + clipboard flow.

## Requirements

- Functional: "Open PDF" button opens native file dialog filtered to `.pdf`
- Functional: during processing, show progress ("Processing page 2 of 5…")
- Functional: result appears in preview window same as a snip result
- Functional: result saved to history (visible in history window)
- Non-functional: button disabled while processing (same pattern as "Snip now")

## Architecture

Main window gets a new "Open PDF" button next to "Snip now". Click → `tauri-plugin-dialog` file picker → returns path → call `tauri.runPdfOcr(path)`. Listen for `pdf-progress` events to show page progress. On completion, `snip-complete` event fires and preview window shows result (existing flow).

No pdf.js needed in v1 — the preview window already renders TeX/Markdown output. PDF page preview (showing the original PDF) is a nice-to-have for later.

## Related Code Files

- Modify: `src/App.tsx` — add "Open PDF" button, progress state, pdf-progress listener
- Modify: `src/lib/invoke.ts` — already has `runPdfOcr` from Phase 3
- Modify: `src/strings.ts` — add PDF-related strings

## Implementation Steps

1. Add strings to `strings.ts`: `pdf.open`, `pdf.processing`, `pdf.pageProgress`.
2. In `App.tsx`:
   - Add "Open PDF" button in the actions grid (next to History/Settings)
   - Import `open` from `@tauri-apps/plugin-dialog` for file picker
   - On click: `open({ filters: [{ name: "PDF", extensions: ["pdf"] }] })`
   - If path returned, call `tauri.runPdfOcr(path)`
   - Listen for `pdf-progress` event → update progress text "Processing page N of M…"
   - Disable both "Snip now" and "Open PDF" while processing
   - Error handling via toast (same as snip errors)
3. Preview window already handles `snip-complete` → no changes needed.
4. History window already shows all records → PDF results appear automatically.

## Success Criteria

- [ ] "Open PDF" button visible in main window
- [ ] File picker only shows .pdf files
- [ ] Progress text updates per page during CLI processing
- [ ] Result appears in preview window after completion
- [ ] Result appears in history
- [ ] Button disabled during processing
- [ ] Error toast on failure

## Risk Assessment

- **File picker may not filter on some Linux DEs** — Mitigation: validate extension in Rust before processing; reject non-PDF with clear error.
