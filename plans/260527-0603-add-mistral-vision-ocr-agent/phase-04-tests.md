---
phase: 4
title: "Tests"
status: completed
priority: P2
effort: "30m"
dependencies: [1, 2, 3]
---

# Phase 4: Tests

## Overview

Add unit tests for the Mistral adapter (response parsing, error mapping, key redaction) and registry integration (argv builder, fallback chain). No live API calls -- test the parsing/mapping layer only. Validation completed with the tester-run locked test sweep before the redaction follow-up and a local full test pass after it.

This phase does not cover end-to-end app validation with a real Mistral API key.

## Related Code Files

- Create: `src-tauri/tests/rust/cloud_mistral_api_test.rs`
- Modify: `src-tauri/tests/rust/agent_registry_argv_test.rs`
- Modify: `src-tauri/Cargo.toml` (add `[[test]]` entry)

## Implementation Steps

1. Create `tests/rust/cloud_mistral_api_test.rs`:
   - `test_redact_strips_bearer_token` -- verify API key redaction in error strings
   - `test_mime_resolution` -- png/jpg/jpeg/webp mapping
   - `test_parse_success_response` -- valid OCR JSON `pages[].markdown` -> extracted text
   - `test_parse_empty_pages` -- empty page list -> EmptyResponse error
   - `test_parse_null_markdown` -- null markdown content -> EmptyResponse error

2. Add `[[test]]` entry to `Cargo.toml`:
   ```toml
   [[test]]
   name = "cloud_mistral_api"
   path = "tests/rust/cloud_mistral_api_test.rs"
   ```

3. Extend `agent_registry_argv_test.rs`:
   - `test_cloud_mistral_returns_empty_args` -- `build_command_args("cloud-mistral", ...)` returns `Vec::new()`
   - `test_fallback_chain_includes_mistral` -- `CLOUD_MISTRAL_ID` is in `DEFAULT_FALLBACK_CHAIN`
   - `test_spec_by_id_finds_mistral` -- `spec_by_id("cloud-mistral")` returns Some

4. Add inline `#[cfg(test)] mod tests` in `cloud_mistral_api.rs`:
   - `test_redact_bearer_token`
   - `test_mime_for_common_extensions`

5. Run `cargo test` to verify all pass.

## Success Criteria

- [x] `cargo test` passes with 0 failures
- [x] Mistral adapter parsing tests cover success + error paths
- [x] Registry tests verify Mistral presence in catalogue and fallback chain
- [x] No live API calls in test suite
- [ ] Live app test with a real Mistral API key is completed

## Validation

```bash
cd src-tauri
cargo check --tests --locked
cargo test --tests --locked
```

Tester run passed with 83 tests before the redaction follow-up. Local full validation then passed with:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

That final run passed with 85 tests.

Live app validation remains pending because no real Mistral API key has been added in this environment.

## Risk Assessment

- Tests are offline/unit only. No flakiness risk from network.
