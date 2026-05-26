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
| 4 | [Tests](./phase-04-tests.md) | Completed |

## Dependencies

- Depends on existing agent infrastructure from MVP Phase 3 (complete).
- Phase 8 (Settings UI) will later add a Mistral API key input field, but backend works independently via keychain CLI or `set_api_key` command.

## Key Decisions

- **Model choice:** `mistral-small-latest` -- vision-capable, fast, cheap. User can override later via settings (Phase 8/9 scope).
- **Fallback chain:** Append `cloud-mistral` after `cloud-gemini` in `DEFAULT_FALLBACK_CHAIN` so it serves as an additional fallback when Gemini fails.
- **Error mapping:** Mistral returns standard HTTP status codes (429 rate limit, 401/403 auth, 400 bad request) -- map to existing `DispatchError` variants.

## Validation

Completed implementation and verification with:

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
