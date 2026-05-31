---
phase: 1
title: "Mistral OCR API Adapter"
status: completed
priority: P1
effort: "1h"
dependencies: []
---

# Phase 1: Mistral OCR API Adapter

## Overview

Create `src-tauri/src/agents/cloud_mistral_api.rs` -- an HTTP adapter that sends a base64-encoded image to Mistral's dedicated OCR endpoint and returns `pages[].markdown`.

**Supersedes original draft:** this phase originally mentioned Mistral chat completions. Runtime now uses OCR API only. Do not use Mistral Completion/chat models for `cloud-mistral`.

## Architecture

Mistral uses the dedicated OCR endpoint:

```
POST https://api.mistral.ai/v1/ocr
Authorization: Bearer <api_key>
Content-Type: application/json

{
  "model": "mistral-ocr-latest",
  "document": {
    "type": "image_url",
    "image_url": "data:image/png;base64,<b64>"
  }
}
```

Response: `pages[].markdown` contains the OCR text.

## Related Code Files

- Create: `src-tauri/src/agents/cloud_mistral_api.rs`
- Read for reference: `src-tauri/src/agents/cloud_gemini_api.rs`

## Implementation Steps

1. Create `cloud_mistral_api.rs` with:
   - `pub const CLOUD_MISTRAL_MODEL: &str = "mistral-ocr-latest";`
   - `const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);`
   - `CloudMistralError` enum mirroring `CloudGeminiError` variants (RateLimited, BadRequest, AuthFailed, ServerError, Network, EmptyResponse, Parse)
   - Request structs: OCR request with `document.type = "image_url"` or `"document_url"`
   - Response structs: OCR response with `pages[].markdown`
   - `fn endpoint() -> &'static str` returning `"https://api.mistral.ai/v1/ocr"`
   - `pub async fn call(image_bytes, mime_type, _prompt, api_key) -> Result<String, CloudMistralError>`:
     - Build `reqwest::Client` with 30s timeout
     - Base64-encode image, format as `data:{mime_type};base64,{encoded}`
     - POST with `Authorization: Bearer {api_key}` header
     - Map HTTP errors: 429 -> RateLimited, 400 -> BadRequest, 401/403 -> AuthFailed, 5xx -> ServerError
     - Parse response, extract non-empty `pages[].markdown`
   - `pub async fn call_with_image_path(image_path, prompt, api_key)` convenience wrapper (same as Gemini's)
   - `fn redact_key(s: &str) -> String` to strip `Bearer ...` tokens from error strings

2. Add `pub mod cloud_mistral_api;` to `src-tauri/src/agents/mod.rs`

## Success Criteria

- [x] `cloud_mistral_api.rs` compiles with `cargo check`
- [x] Module exported from `agents/mod.rs`
- [x] Error types implement `Display` and `Error` traits
- [x] API key never appears in error messages (redact_key)

## Validation

- `cargo check --manifest-path src-tauri/Cargo.toml`

## Risk Assessment

- Mistral OCR API returns flattened Markdown for some complex tables. Mitigation: local deterministic reconstruction in Phase 9 `tabular_complex_grid.rs`.
