---
phase: 3
title: "Unsafe Output Guard and Fallback Policy"
status: complete
priority: P1
effort: "0.5d"
---

# Phase 03: Unsafe Output Guard and Fallback Policy

## Context Links

- [OCR post-process](../../src-tauri/src/ocr/postprocess.rs)
- [OCR dispatcher](../../src-tauri/src/ocr/dispatcher.rs)
- [History store rerun method](../../src/stores/history-store.ts)

## Overview

Prevent Gemini CLI tool errors, thinking transcripts, empty responses, or unrelated OCR text from being written into history as successful OCR output.

## Key Insights

- Current post-process cleans many known Gemini artifacts, but bad outputs can still look non-empty.
- Gemini can return plausible OCR-like text from the wrong source; this must be rejected before the history row is updated.
- For explicit rerun with a selected agent, fallback should not silently switch to Codex because the UI says "rerun with Gemini".
- The safe behavior is: selected Gemini either returns clean output or the existing record remains unchanged and the UI shows an error.

## Requirements

- Reject response text containing known tool execution failures before `post_process` or immediately after parsing.
- If JSON stats indicate tool calls, reject or at least reject failed/nonzero tool-call cases.
- For Gemini CLI history rerun, compare output against the previous row and reject mismatched problem labels or low significant-token overlap.
- Keep selected-agent rerun strict: do not fallback to another agent for `rerun_snip(record_id, agent_id)`.
- Ensure `history_repo::update_output` runs only after validated OCR text exists.
- Frontend should preserve old row on rerun error and expose the error state.

## Architecture

```text
Gemini JSON response
  -> parse `.response`
  -> inspect stats/tools and known error markers
  -> post_process
  -> validate non-empty and not [UNREADABLE]
  -> rerun consistency check against previous row
  -> update history
```

## Related Code Files

- Related: `src-tauri/src/agents/registry.rs`
- Modify: `src-tauri/src/ocr/dispatcher.rs`
- Modify: `src-tauri/src/ocr/consistency.rs`
- Related: `src-tauri/src/ocr/gemini_workspace.rs`
- Related: `src-tauri/src/ocr/mod.rs`
- Modify: `src-tauri/src/ocr/postprocess.rs` only if a narrowly scoped marker belongs there
- Modify: `src/stores/history-store.ts` only if current error handling is insufficient
- Modify tests under: `src-tauri/tests/rust/`

## Implementation Steps

1. Add `looks_like_gemini_tool_error(response: &str) -> bool`.
2. Reject common markers:
   - `Error executing tool`
   - `Path not in workspace`
   - `default_api_`
   - tool names when they appear in an error context
3. If JSON stats expose tool calls, only tolerate successful `read_file` calls inside the isolated Gemini workspace and reject every other tool use.
4. Confirm `rerun_snip` updates DB only after dispatcher returns `Ok`.
5. Keep frontend behavior simple: catch rerun failure and set store error.
6. Add Gemini rerun consistency validation before DB update so unrelated but plausible OCR text fails closed.

## Todo List

- [x] Add Gemini unsafe-output detector.
- [x] Reject tool-call JSON stats for OCR-only Gemini calls.
- [x] Add tests for tool error text and stats-based rejection.
- [x] Add tests for unrelated-output rejection.
- [x] Verify History row remains unchanged on rerun error.

## Completion Notes

- Implemented 2026-05-24.
- Gemini CLI JSON parser now rejects structured errors, non-read-file tool stats, failed tool calls, known CLI tool-error markers, empty responses, and malformed JSON.
- Tool-call stats support both `totalCalls` and `total_calls`.
- Unsafe output guard is Gemini-CLI-only and uses CLI-specific phrases to reduce false positives.
- Added Gemini-only rerun consistency validation in `rerun_snip`; it rejects mismatched problem labels such as `Câu 9` -> `Bài 5` and low-overlap unrelated OCR text before any DB update.
- Explicit History rerun remains strict: selected agent calls `run_ocr` directly, not fallback.
- History row preservation on rerun failure was verified by flow inspection: backend updates DB only after validated OCR text returns `Ok`, and the store only replaces a row after `tauri.rerunSnip` resolves. This repo has no frontend test runner configured, so no automated store test was added.
- Reviewer/debugger re-check found no remaining confirmed Phase 03 issues.

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml --test gemini_cli_output` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml ocr::consistency --lib` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `pnpm exec tsc --noEmit` passed.
- `git diff --check` passed.
- `cargo fmt` was not run because `rustfmt` is not installed for the active toolchain.

## Success Criteria

- History never gets overwritten with `Error executing tool...`.
- History never gets overwritten with plausible Gemini OCR text from a different question.
- User sees a rerun error instead of corrupted OCR text.
- Codex and Cloud Gemini behavior remains unchanged.

## Risk Assessment

- **Risk:** Rejecting any tool call may be too strict for future Gemini CLI versions.
  **Mitigation:** Keep the guard Gemini-CLI-only and revisit after fixture validation.
- **Risk:** Error marker false positive inside a real document.
  **Mitigation:** Match full phrases that are strongly CLI-specific.

## Security Considerations

- Tool calls in OCR mode are unnecessary and expand local filesystem exposure.
- Prefer fail-closed for Gemini CLI because it is already marked experimental.

## Next Steps

- Phase 04 validates behavior with unit tests and real fixture rerun checks.
