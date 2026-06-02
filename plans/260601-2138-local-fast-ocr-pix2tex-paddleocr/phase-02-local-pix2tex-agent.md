---
phase: 2
title: "local-pix2tex Agent"
status: complete
priority: P1
effort: 1d
dependencies: [1]
---

# Phase 2: local-pix2tex Agent

## Context Links

- `src-tauri/src/ocr/dispatcher.rs`
- `src-tauri/src/ocr/postprocess.rs`
- `src-tauri/src/ocr/smart_format.rs`
- `src-tauri/src/agents/local_ocr_api.rs`
- `src-tauri/src/agents/local_ocr_pix2tex.rs`

## Overview

Add a local formula OCR path for equation-only snips. It should be the fastest path for isolated formulas and should not run on text-heavy images unless explicitly selected.

## Key Insights

- `pix2tex` is best for formula images, not paragraph OCR.
- Output should be raw LaTeX without `$...$` for equation-only.
- Daemon warm state is mandatory for speed.

## Requirements

Functional:
- `local-pix2tex` calls `/ocr/pix2tex`.
- Post-process removes fences/preambles using existing OCR cleanup.
- Reject empty, unreadable, or very low-confidence output.
- Direct rerun with `local-pix2tex` works from History.

Non-functional:
- Target hot latency: under 1s for small equation snips on M1 Pro.
- Timeout: 10s with clean fallback error.

## Architecture

```text
run_ocr(local-pix2tex)
  -> local_ocr_pix2tex::pix2tex(base_url, image_path)
  -> post_process
  -> return raw TeX
```

## Related Code Files

- Modify: `src-tauri/src/ocr/dispatcher.rs` — route `LocalHttp`.
- Add: `src-tauri/src/agents/local_ocr_pix2tex.rs` — pix2tex request/response adapter.
- Modify: `src-tauri/src/agents/local_ocr_api.rs` — keep daemon health/capability discovery.
- Add tests: `src-tauri/tests/rust/local_ocr_api_test.rs`.

## Implementation Steps

1. Add `LOCAL_PIX2TEX_ID`.
2. Implement multipart request to `/ocr/pix2tex`.
3. Map daemon errors:
   - connection refused -> agent unavailable
   - 408/timeout -> timeout
   - 422 unsupported -> bad request
   - empty text -> empty output
4. Apply existing `post_process`.
5. Keep output unwrapped; formatting remains `copyOutput` responsibility.
6. Add tests for response parsing and error mapping.

## Todo List

- [x] Agent id and display name added.
- [x] Adapter request implemented.
- [x] Error mapping tests added.
- [x] Direct rerun path verified structurally through shared `run_ocr_for_path` / History rerun path.

## Success Criteria

- Adapter returns raw TeX from a valid `/ocr/pix2tex` daemon response.
- Timeout, unsupported, empty, low-confidence, and parse errors map to dispatcher errors so fallback can continue.
- Live latency benchmark is deferred until Phase 6 provides the reference daemon; record p95 in `docs/local-fast-ocr.md` before declaring the full local-fast OCR plan complete.

## Risk Assessment

- Risk: pix2tex returns plausible junk for normal text. Mitigation: auto router only chooses it for equation-like images.

## Security Considerations

- Send only image bytes to localhost.
- Do not execute arbitrary local commands from app.

## Next Steps

- Phase 3 adds PaddleOCR text path.
