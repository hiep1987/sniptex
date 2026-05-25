---
phase: 4
title: "Tests and Live Rerun Validation"
status: automated-complete-gui-smoke-pending
priority: P1
effort: "0.5d"
---

# Phase 04: Tests and Live Rerun Validation

## Context Links

- [CLI smoke harness](../../src-tauri/src/bin/cli_test.rs)
- [Phase 7 plan](../260520-0603-sniptex-tauri-mvp-v1/phase-07-sqlite-history-with-fts5-search.md)
- [Fixture set](../../fixtures-extra/)

## Overview

Validate that Gemini CLI rerun is cleaner without regressing Codex or existing history behavior.

## Key Insights

- Unit tests can prove parsing and rejection behavior.
- Live Gemini CLI behavior still needs manual fixture checks because latency, rate limits, and CLI tool-loop behavior are external.
- Phase 7 completion should still use Codex as the pass baseline.

## Requirements

- Run focused Rust tests after implementation.
- Run `cli_test` against at least one equation-only and one table-only fixture for both Codex and Gemini CLI if local agent access permits.
- Run live app history rerun smoke test:
  - create or use an existing history record
  - rerun with Codex
  - rerun with Gemini CLI
  - confirm failed Gemini does not overwrite old output

## Related Code Files

- Modify: `src-tauri/tests/rust/agent_registry_argv_test.rs`
- Add/modify: `src-tauri/tests/rust/ocr_postprocess.rs` or `gemini_cli_output.rs`
- No production UI change unless store error handling is insufficient.

## Implementation Steps

1. Run focused tests:
   ```bash
   cargo test --manifest-path src-tauri/Cargo.toml --test agent_registry_argv --test ocr_postprocess
   ```
2. Run new Gemini JSON/parser tests.
3. Run full Rust test suite if focused tests pass:
   ```bash
   cargo test --manifest-path src-tauri/Cargo.toml
   ```
4. Run TypeScript compile:
   ```bash
   pnpm exec tsc --noEmit
   ```
5. Optional live CLI fixture checks:
   ```bash
   cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/equation-only/CleanShot 2026-05-21 at 05.24.00@2x.png" --agent gemini-cli
   cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/table-only/CleanShot 2026-05-20 at 15.01.25@2x.png" --agent gemini-cli
   ```

## Todo List

- [x] Add regression tests for JSON parsing.
- [x] Add regression tests for unsafe-output rejection.
- [x] Run focused Rust tests.
- [x] Run full Rust tests.
- [x] Run TypeScript compile.
- [x] Run live Gemini fixture checks if agent quota/network allows.
- [x] Update Phase 7 notes with automated and CLI fixture result.
- [ ] Run full GUI History rerun smoke in the app.

## Completion Notes

- Implemented and validated 2026-05-24.
- Added `--skip-trust` to the Gemini CLI argv after live validation hit a trusted-folder failure in the local headless environment.
- Codex live fixture checks passed:
  - equation-only fixture returned raw LaTeX and detected `EquationOnly`
  - table-only fixture returned a Markdown table and detected `TableOnly`
- Gemini CLI live fixture checks:
  - equation-only fixture was rejected at dispatcher level with `gemini-cli used tools during OCR-only call`
  - table fixture returned OCR text successfully and detected `Mixed`
- Additional live validation on app-data image `52f65375-3607-4fb0-a5be-fd25d0b3ddd3.png` showed the file path was correct. Gemini CLI headless only became stable after matching the successful TUI-like shape: text output, one-line prompt with `@path`, no model pin, no plan mode, neutral cwd, and include-dir scoped to the image parent.
- Root cause correction: the final culprit was not Gemini CLI generally, `@file`, agent path, or model version. It was the app's procedural `MASTER_PROMPT` contract in headless mode. The classification tree, format branches, examples, and strict rules pushed `gemini -p` away from plain transcription. Gemini CLI now uses a separate minimal prompt.
- Contract caveat: Codex and Gemini Vision API still use `MASTER_PROMPT`, while Gemini CLI uses the minimal prompt. Before treating Gemini CLI as equivalent on harder SGK images, validate cross-agent consistency on tables, geometry diagrams, axes, and mixed text/math images.
- Decision: keep Gemini CLI out of automatic fallback, but allow manual History rerun behind consistency guard. Keep Codex and Gemini Vision API as the safer default routes.
- `history_smoke` passed, including rerun-style `update_output` and FTS resync.
- Full Tauri UI rerun smoke was not executed in this non-GUI validation pass; Phase 7 live app smoke remains the final manual check.
- Gemini rerun consistency guard tests now include same-label unrelated math rejection and same-content-without-label acceptance.

## Validation

- `cargo test --manifest-path src-tauri/Cargo.toml --test agent_registry_argv --test ocr_postprocess --test gemini_cli_output` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml ocr::consistency --lib` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `pnpm exec tsc --noEmit` passed.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin history_smoke` passed.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/equation-only/CleanShot 2026-05-21 at 05.24.00@2x.png" --agent codex` passed outside sandbox.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/table-only/CleanShot 2026-05-20 at 15.01.25@2x.png" --agent codex` passed outside sandbox.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/equation-only/CleanShot 2026-05-21 at 05.24.00@2x.png" --agent gemini-cli` failed safely outside sandbox after detecting tool use.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "fixtures-extra/table-only/CleanShot 2026-05-20 at 15.01.25@2x.png" --agent gemini-cli` passed outside sandbox.
- `cargo run --manifest-path src-tauri/Cargo.toml --bin cli_test -- --image "/Users/hieplequoc/Library/Application Support/com.sniptex.app/images/52f65375-3607-4fb0-a5be-fd25d0b3ddd3.png" --agent gemini-cli` passed with the final text-mode one-line prompt contract.
- `cargo fmt` was not run because `rustfmt` is not installed for the active toolchain.

## Success Criteria

- All local compile/tests pass.
- Codex rerun remains unchanged.
- Gemini CLI does not corrupt history: it is removed from automatic fallback and manual rerun remains protected by consistency validation.
- Phase 7 live smoke-test checklist can be updated with explicit Gemini caveat.

## Risk Assessment

- **Risk:** Gemini CLI network/quota prevents live validation.
  **Mitigation:** Document as blocked; unit tests still validate local safeguards.
- **Risk:** Full test suite is slow.
  **Mitigation:** Run focused tests first, then full suite before final handoff.

## Security Considerations

- Do not commit API keys, user history DBs, screenshots from app data, or local Gemini settings.
- Test fixtures under repo are safe to use.

## Next Steps

- Run the remaining GUI History rerun smoke from the Phase 7 plan in a live app session.
