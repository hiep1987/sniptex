---
title: "PDF to TeX Upload"
description: "Add PDF file upload → TeX conversion. Cloud APIs receive PDF directly; CLI agents receive rendered page images. Output is concatenated TeX for all pages."
status: pending
priority: P1
branch: "main"
tags: ["pdf", "ocr", "upload", "tex"]
blockedBy: []
blocks: []
created: "2026-05-27"
createdBy: "ck:plan"
source: skill
---

# PDF to TeX Upload

## Overview

Add an "Open PDF" button to the main window. User picks a .pdf file, SnipTeX sends it through the OCR pipeline (cloud APIs receive PDF bytes directly; CLI agents receive per-page PNG renders via pdf.js), and returns concatenated TeX/Markdown copied to clipboard. Result saved to history.

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Cloud API PDF adapters](./phase-01-cloud-api-pdf-adapters.md) | Done |
| 2 | [CLI agent PDF-via-image fallback](./phase-02-cli-agent-pdf-via-image.md) | Done |
| 3 | [Tauri command + dispatcher wiring](./phase-03-tauri-command-dispatcher.md) | Pending |
| 4 | [Frontend: file picker + PDF preview + progress](./phase-04-frontend-pdf-ui.md) | Pending |

## Key Dependencies

- `pdfjs-dist` (npm) — render PDF pages to canvas → PNG in frontend
- Existing cloud API adapters (`cloud_gemini_api.rs`, `cloud_mistral_api.rs`)
- Existing dispatcher + history pipeline

## Architecture

```
User clicks "Open PDF"
  → Tauri file dialog picks .pdf
  → Frontend reads file, sends path to Rust
  → Rust dispatcher checks agent type:
      Cloud API → read PDF bytes, send as application/pdf base64
      CLI agent → frontend renders pages via pdf.js → temp PNGs → CLI per page
  → Concatenate all page results
  → Copy to clipboard + save to history + show preview
```
