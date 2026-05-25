---
phase: 1
title: "Gemini CLI JSON Output Contract"
status: complete
priority: P1
effort: "0.5d"
---

# Phase 01: Gemini CLI JSON Output Contract

## Context Links

- [Agent registry](../../src-tauri/src/agents/registry.rs)
- [OCR dispatcher](../../src-tauri/src/ocr/dispatcher.rs)
- Official Gemini CLI docs: headless mode supports `--output-format json`; parse `.response` for scripting.

## Overview

Stop treating Gemini CLI stdout as the OCR body. Request JSON output and parse only the final response field, similar in spirit to Codex's `--output-last-message`.

## Key Insights

- Codex is clean because it writes assistant-only output to a temp file.
- Gemini CLI has no exact equivalent, but JSON output is the closest supported automation contract.
- Gemini JSON may include stats and metadata; OCR should consume only the answer text.

## Requirements

- Add `--output-format json` to Gemini CLI argv.
- Add `-e none` to disable extensions for this OCR-only call.
- Parse Gemini stdout as JSON when `agent.spec.id == GEMINI_CLI_ID`.
- Return a clear `DispatchError::BadRequest` or `DispatchError::EmptyOutput` when JSON is malformed or `.response` is missing/empty.

## Architecture

```text
gemini stdout JSON
  -> GeminiCliResponse { response, stats? }
  -> response string
  -> post_process()
  -> OCR text
```

## Related Code Files

- Modify: `src-tauri/src/agents/registry.rs`
- Modify: `src-tauri/src/ocr/dispatcher.rs`
- Modify: `src-tauri/tests/rust/agent_registry_argv_test.rs`
- Create if needed: `src-tauri/tests/rust/gemini_cli_output_test.rs`

## Implementation Steps

1. Extend Gemini argv builder with `--output-format json` and `-e none`.
2. Add a small `GeminiCliJsonResponse` struct in dispatcher or a focused helper module.
3. Parse stdout only for Gemini CLI; keep Codex behavior unchanged.
4. Feed parsed `.response` into existing `post_process`.
5. Add unit tests for valid JSON, missing response, empty response, and non-JSON stdout.

## Todo List

- [x] Update Gemini argv contract.
- [x] Add JSON parser helper.
- [x] Preserve Codex raw/last-message path.
- [x] Add parser and argv tests.

## Completion Notes

- Implemented 2026-05-24.
- Gemini CLI argv now requests JSON output and disables extensions for OCR-only calls.
- Dispatcher parses Gemini CLI stdout as JSON and consumes only `.response`.
- Structured Gemini JSON `error` values are surfaced as `DispatchError::BadRequest`.
- Codex `--output-last-message` behavior is unchanged.
- Reviewer re-check found no remaining Phase 01 blockers.

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml --test agent_registry_argv --test gemini_cli_output --test ocr_postprocess` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `pnpm exec tsc --noEmit` passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml` was not run because `rustfmt` is not installed for the active toolchain.

## Success Criteria

- Gemini CLI no longer stores JSON metadata or CLI banners as OCR text.
- Existing Codex tests still pass.
- Malformed Gemini output fails fast with actionable error.

## Risk Assessment

- **Risk:** Gemini CLI version lacks `--output-format json`.
  **Mitigation:** Surface non-zero/malformed output clearly; keep Gemini experimental.
- **Risk:** JSON schema changes.
  **Mitigation:** Parse only the stable `.response` field and ignore unknown fields.

## Security Considerations

- Do not log full OCR content or prompt.
- Redact paths only if error messages are surfaced outside dev logs.

## Next Steps

- Phase 02 isolates Gemini's workspace so JSON parsing is not forced to handle avoidable tool-loop noise.
