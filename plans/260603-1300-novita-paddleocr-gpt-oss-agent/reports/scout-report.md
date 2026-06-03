---
title: "Scout Report - Novita Hybrid Agent"
created: 2026-06-03
---

# Scout Report

## Existing Novita Support

- `src-tauri/src/agents/cloud_novita_api.rs`
  - Existing Novita OpenAI-compatible adapter.
  - Current model: `deepseek/deepseek-ocr-2`.
  - Sends image as data URI through `/openai/v1/chat/completions`.
  - Has parsing, redaction, mime tests inline.

- `src-tauri/src/agents/keychain.rs`
  - Already has `NOVITA_ACCOUNT`.
  - Already has `has/get/set_novita_api_key`.

- `src-tauri/src/commands.rs`
  - `set_api_key`, `has_api_key`, `delete_api_key` already accept provider `"novita"`.
  - `test_api_key` already routes provider `"novita"` to `cloud_novita_api`.

- `src-tauri/src/agents/registry.rs`
  - Existing `CLOUD_NOVITA_ID = "cloud-novita"`.
  - Existing display: `DeepSeek-OCR 2`.
  - Existing fallback includes `cloud-novita`.

- `src-tauri/src/ocr/dispatcher.rs`
  - `run_cloud_agent` already has a `CLOUD_NOVITA_ID` branch.
  - Cloud output runs through shared `post_process`.

- `src/windows/settings/agents-tab.tsx`
  - `cloud-novita` already appears in settings.
  - Uses provider key `"novita"`.

- `src/stores/settings-store.ts`
  - Default priority includes `cloud-novita`.

## Recommended Integration Shape

Create a new agent ID:

```rust
pub const CLOUD_NOVITA_HYBRID_ID: &str = "cloud-novita-hybrid";
```

Display name:

```text
Novita OCR + GPT OSS
```

Do not replace `cloud-novita` because it is already a working DeepSeek-OCR 2 path.

## Files To Modify

- `src-tauri/src/agents/registry.rs`
- `src-tauri/src/agents/mod.rs`
- `src-tauri/src/ocr/dispatcher.rs`
- `src-tauri/src/commands.rs`
- `src/windows/settings/agents-tab.tsx`
- `src/stores/settings-store.ts`
- `src-tauri/Cargo.toml`

## Files To Create

- `src-tauri/src/agents/cloud-novita-hybrid-api.rs` is NOT valid Rust module name.
- Use `src-tauri/src/agents/cloud_novita_hybrid_api.rs`.
- Use `src-tauri/tests/rust/cloud_novita_hybrid_api_test.rs`.
- Optional: `src-tauri/src/bin/novita_hybrid_smoke.rs`.

## Constraints

- Keep files under 200 lines where feasible.
- If hybrid adapter grows, split:
  - `cloud_novita_hybrid_api.rs` public orchestration
  - `novita_hybrid_contract.rs` parser/redaction contract
  - `novita_gpt_oss.rs` GPT cleanup request/response if needed
- Do not store API keys in settings JSON.
- Do not log raw OCR content if it may contain user-sensitive screenshots.
