---
title: "Phase 01 - API Contract and Cost Controls"
status: in-progress
priority: P1
effort: 2h
created: 2026-06-03
---

# Phase 01 - API Contract and Cost Controls

## Context Links

- [Novita research](./research/novita-model-research.md)
- [Scout report](./reports/scout-report.md)
- Existing adapter: `src-tauri/src/agents/cloud_novita_api.rs`

## Overview

Define the exact request/response contracts before coding. The main risk is DeepSeek-OCR 2 formatting artifacts that need GPT cleanup.

## Key Insights

- GPT OSS 120B is serverless and cheap per token.
- DeepSeek-OCR 2 is serverless but can emit malformed LaTeX/table artifacts.
- Cost control must focus on GPT token caps.

## Requirements

- Functional:
  - Define `GptOssCleanupResponse` parser for chat completions.
- Non-functional:
  - No API key logging.
  - No OCR content logging by default.
  - Default timeout: GPT <= 60s.

## Architecture

```text
input image bytes
  -> maybe_resize_for_ocr
  -> call_deepseek_ocr(key)
  -> normalize_intermediate_markdown
  -> should_call_gpt_cleanup
  -> call_gpt_oss_120b(markdown, key)
```

## Related Code Files

- Modify: `src-tauri/src/agents/cloud_novita_api.rs` only for shared helpers if useful.
- Create: `src-tauri/src/agents/cloud_novita_hybrid_api.rs`.
- No settings schema change required.

## Implementation Steps

1. Define constants:
   - `NOVITA_GPT_OSS_MODEL = "openai/gpt-oss-120b"`.
   - `NOVITA_GPT_OSS_ENDPOINT = "https://api.novita.ai/openai/v1/chat/completions"`.
2. Define token/output caps:
   - `GPT_MAX_INPUT_CHARS = 12000`.
   - `GPT_MAX_TOKENS = 4096`.
3. Define cleanup prompt:
   - Convert OCR markdown to final LaTeX/Markdown.
   - Preserve Vietnamese problem labels.
   - Do not invent missing content.
   - Return `[UNREADABLE]` if source is insufficient.

## Todo List

- [x] Document endpoint auth method.
- [x] Lock model ID strings.
- [x] Decide env var vs settings storage.
- [x] Add cost cap constants.

## Success Criteria

- No implementation ambiguity remains for Phase 02.
- No extra endpoint state is required.
- Estimated GPT token cost can be computed from input/output char counts.

## Risk Assessment

- Risk: GPT cleanup may over-correct OCR output.
  - Mitigation: keep hybrid manual until benchmark.

## Security Considerations

- Use existing Novita keychain access.
- Redact `Bearer ...` and `sk_...` in every error path.
- Avoid logging raw OCR text by default.

## Next Steps

- Phase 02 implements adapter and unit tests for cleanup parsing/redaction.
