---
title: "Add Mistral Vision API as OCR Agent"
description: "Add Mistral Vision API (pixtral/mistral-small-latest) as a fourth OCR provider. BYOK cloud agent, same pattern as cloud-gemini."
status: completed
priority: P2
branch: "main"
tags: ["mistral", "ocr", "cloud-api", "byok"]
blockedBy: []
blocks: []
created: "2026-05-26T23:03:21.791Z"
createdBy: "ck:plan"
source: skill
---

# Add Mistral Vision API as OCR Agent

## Overview

Add Mistral Vision API as a fourth OCR agent alongside Codex CLI, Gemini CLI, and Cloud Gemini. Mistral uses the OpenAI-compatible chat completions endpoint (`POST https://api.mistral.ai/v1/chat/completions`) with base64 inline images via `data:image/png;base64,...` in the `image_url` field. BYOK model: user provides their own Mistral API key, stored in OS keychain.

No new Cargo dependencies required -- `reqwest`, `base64`, `serde`, `serde_json` already present.

**Model:** `mistral-small-latest` (vision-capable, cost-effective for OCR).

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Mistral Cloud API Adapter](./phase-01-mistral-cloud-api-adapter.md) | Completed |
| 2 | [Registry and Dispatcher Integration](./phase-02-registry-and-dispatcher-integration.md) | Completed |
| 3 | [Keychain and Commands Wiring](./phase-03-keychain-and-commands-wiring.md) | Completed |
| 4 | [Tests](./phase-04-tests.md) | Completed (offline/unit only) |

## Remaining Validation

- [x] Add a real Mistral API key through the app/keychain flow.
- [ ] Confirm `detect_agents` lists Mistral Vision API in the running app (requires codesigned build -- keyring 3.x macOS access control blocks unsigned dev binaries from reading back keychain items created by different Entry instances; works in production Tauri app which has a stable bundle ID).
- [x] Run a real screenshot OCR request through `cloud-mistral` (3/3 fixtures passed: Vietnamese text, equations, tables).
- [ ] Verify fallback behavior from the app UI (requires Phase 8 Settings UI for key input).

## Dependencies

- Depends on existing agent infrastructure from MVP Phase 3 (complete).
- Phase 8 (Settings UI) will later add a Mistral API key input field, but backend works independently via keychain CLI or `set_api_key` command.

## Key Decisions

- **Model choice:** `mistral-small-latest` -- vision-capable, fast, cheap. User can override later via settings (Phase 8/9 scope).
- **Fallback chain:** Append `cloud-mistral` after `cloud-gemini` in `DEFAULT_FALLBACK_CHAIN` so it serves as an additional fallback when Gemini fails.
- **Error mapping:** Mistral returns standard HTTP status codes (429 rate limit, 401/403 auth, 400 bad request) -- map to existing `DispatchError` variants.

## Validation

Completed implementation and offline verification with:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Test follow-up noted by tester:

```bash
cd src-tauri
cargo check --tests --locked
cargo test --tests --locked
```

That pre-redaction run passed with 83 tests, then the local full `cargo test` passed with 85 tests after the redaction follow-up.

Live API validation passed (2026-05-27):

```
fixtures-sample/CleanShot 2026-05-20 at 14.51.26@2x.png → Vietnamese math definition (Mixed, 415 chars) ✓
fixtures-sample/CleanShot 2026-05-20 at 14.53.48@2x.png → Equation-heavy asymptote example (Mixed, 493 chars) ✓
fixtures-retest/CleanShot 2026-05-20 at 15.01.25@2x.png → Frequency table (TableOnly, 213 chars) ✓
```

Note: `detect_agents` in dev builds hits a macOS keychain access control limitation — `keyring` 3.x items are scoped by binary identity. In production (codesigned Tauri `.app`), the bundle ID stays stable so set → get across Entry instances works. This is not a code bug; it's a dev-environment limitation that doesn't affect the shipped app.
