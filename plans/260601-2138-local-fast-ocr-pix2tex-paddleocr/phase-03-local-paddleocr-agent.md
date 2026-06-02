---
phase: 3
title: "local-paddleocr Agent"
status: complete
priority: P1
effort: 1d
dependencies: [1]
---

# Phase 3: local-paddleocr Agent

## Context Links

- `src-tauri/src/ocr/dispatcher.rs`
- `src-tauri/src/ocr/tabular.rs`
- `src-tauri/src/ocr/smart_format.rs`
- `src-tauri/src/agents/local_ocr_api.rs`
- `src-tauri/src/agents/local_ocr_client.rs`
- `src-tauri/src/agents/local_ocr_paddleocr.rs`

## Overview

Add a local text OCR path for Vietnamese **paragraph text**. This is the fast path for question/paragraph images where formula OCR is not needed. **Tables (simple or complex) are out of scope for v1** — PaddleOCR returns line-with-bbox output without row/column grouping, so any table reconstruction is deferred to v1.x. The auto router (Phase 4) sends every table-classified image to cloud-mistral.

## Key Insights

- PaddleOCR handles Vietnamese text better than pix2tex.
- It does not solve LaTeX formula conversion.
- Tables are deferred: PaddleOCR bbox clustering → Markdown table is non-trivial (cluster by y for rows, then by x for columns, infer column count) and benchmarking that against the 9 TABLE_ONLY fixtures is its own work item. Defer to v1.x.

## Requirements

Functional:
- `local-paddleocr` calls `/ocr/paddleocr`.
- Output is plain text (lines joined with `\n`) compatible with existing smart format flow.
- Preserve Vietnamese diacritics.
- Low-confidence results should fail so fallback can run.
- If daemon returns content the post-process classifier identifies as `TABLE_ONLY` (e.g. Markdown table markers leaked in by PaddleOCR), this adapter MUST return `BadRequest("local does not support tables")` so dispatcher falls through to cloud-mistral.

Non-functional:
- Target hot latency: under 1s for small text snips.
- Timeout: 10s.

## Architecture

```text
run_ocr(local-paddleocr)
  -> local_ocr_paddleocr::paddleocr(base_url, image_path)
  -> normalize lines / plain text
  -> post_process
  -> detect_type
```

## Related Code Files

- Modify: `src-tauri/src/agents/local_ocr_api.rs`.
- Add: `src-tauri/src/agents/local_ocr_paddleocr.rs`.
- Add: `src-tauri/src/agents/local_ocr_client.rs`.
- Modify: `src-tauri/src/ocr/dispatcher.rs`.
- Add tests: `src-tauri/tests/rust/local_ocr_api_test.rs`.

## Implementation Steps

1. Add `LOCAL_PADDLEOCR_ID`.
2. Implement `/ocr/paddleocr` request.
3. Map daemon HTTP responses:
   - 200 → parse text blocks (lines joined with `\n`)
   - 422 `unsupported_table` → `BadRequest("local does not support tables")` so dispatcher falls through to cloud-mistral
   - 408 / timeout → timeout
   - connection refused → agent unavailable
   - empty text → empty output
4. Enforce `confidence >= threshold` when provided.
5. Do not try to infer LaTeX formula from PaddleOCR text.
6. Defense-in-depth: even on a 200, detect leaked table markers (`|...|` rows) in the response and reject with `BadRequest("local does not support tables")` — guards against daemon bugs where the classifier let a table slip through.
7. Add tests:
   - Vietnamese diacritics preserved.
   - low confidence maps to fallback-compatible error.
   - daemon 422 maps to fallback-compatible BadRequest.
   - 200 with leaked table markers triggers defense-in-depth rejection.

## Todo List

- [x] Agent id and display name added.
- [x] PaddleOCR response parser implemented (paragraph text only).
- [x] Vietnamese fixture test added.
- [x] Table-content rejection test added.
- [x] `auto-local-fast` remains hidden from detection until Phase 4 router is implemented.
- [x] URL validation hardened against userinfo host-bypass values such as `http://localhost:8765@evil.test`.

## Success Criteria

- Vietnamese paragraph fixture returns clean text locally.
- Formula-only fixture is not routed here by auto router because the auto router is still Phase 4.
- Table fixture triggers explicit fallback (no silent flattened output).
- Local HTTP clients bypass proxies and only accept parsed loopback URLs.

## Risk Assessment

- Risk: text OCR loses math symbols. Mitigation: router sends formula-heavy images to pix2tex or cloud.

## Security Considerations

- Same localhost restrictions as Phase 1, enforced with URL parsing rather than prefix matching.
- Local OCR HTTP clients use `no_proxy()` to avoid sending local image uploads through environment proxies.

## Next Steps

- Phase 4 chooses between local agents automatically.
