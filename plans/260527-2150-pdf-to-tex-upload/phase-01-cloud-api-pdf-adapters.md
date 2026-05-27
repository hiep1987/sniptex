---
phase: 1
title: "Cloud API PDF Adapters"
status: done
priority: P1
effort: "3h"
dependencies: []
---

# Phase 1: Cloud API PDF Adapters

## Overview

Extend Gemini and Mistral cloud API adapters to accept PDF bytes directly (no image rendering needed). Both APIs support PDF as a document input type.

## Requirements

- Functional: `call_with_pdf_path(pdf_path, prompt, api_key)` returns concatenated TeX for all pages
- Non-functional: Same 30s timeout; PDF size limit logged but not enforced (API will reject oversized files)

## Architecture

**Gemini Vision API** — `inline_data` with `mime_type: "application/pdf"`. Same request shape as images; only the mime changes. Gemini processes all pages and returns a single response.

**Mistral OCR API** — Change `doc_type` from `"image_url"` to `"document_url"` when input is PDF. Use `data:application/pdf;base64,{encoded}` URI. Mistral OCR returns `pages[].markdown` — concatenate all pages.

## Related Code Files

- Modify: `src-tauri/src/agents/cloud_gemini_api.rs` — add `call_with_pdf_path`, extend `mime_for` to handle `.pdf`
- Modify: `src-tauri/src/agents/cloud_mistral_api.rs` — add `call_with_pdf_path`, add `document_url` doc type variant

## Implementation Steps

1. `cloud_gemini_api.rs`: extend `mime_for()` to return `"application/pdf"` for `.pdf` extension
2. Add `call_with_pdf_path()` — reads file, calls `call()` with `application/pdf` mime. Gemini returns all pages in one response.
3. `cloud_mistral_api.rs`: add `OcrDocumentPdf` struct with `doc_type: "document_url"` and `document_url: "data:application/pdf;base64,..."` 
4. Add `call_with_pdf_path()` — reads PDF, builds document_url request. Mistral returns `pages[]` — concatenate all `page.markdown` with `\n\n` separator.
5. Update `parse_response()` to join ALL pages, not just the first one.
6. Add unit tests for mime detection and response parsing with multi-page fixtures.

## Success Criteria

- [x] `mime_for("doc.pdf")` returns `"application/pdf"` (Gemini)
- [x] `call_with_pdf_path` compiles and type-checks for both adapters
- [x] Mistral `parse_response` concatenates all pages, not just first
- [x] Existing image tests still pass

## Risk Assessment

- **Gemini free tier may have PDF size limits** — Mitigation: 30s timeout covers API rejection; surface error message to user.
- **Mistral OCR endpoint may require different doc_type for PDF vs image** — Mitigation: check API docs; `document_url` type handles both URL and data URI per Mistral docs.
