---
phase: 1
title: "Mistral Cloud API Adapter"
status: completed
priority: P1
effort: "1h"
dependencies: []
---

# Phase 1: Mistral Cloud API Adapter

## Overview

Create `src-tauri/src/agents/cloud_mistral_api.rs` -- an HTTP adapter that sends a base64-encoded image + OCR prompt to Mistral's chat completions endpoint and returns the text response. Follows the exact same pattern as `cloud_gemini_api.rs`.

## Architecture

Mistral uses OpenAI-compatible chat completions format:

```
POST https://api.mistral.ai/v1/chat/completions
Authorization: Bearer <api_key>
Content-Type: application/json

{
  "model": "mistral-small-latest",
  "messages": [{
    "role": "user",
    "content": [
      { "type": "text", "text": "<prompt>" },
      { "type": "image_url", "image_url": "data:image/png;base64,<b64>" }
    ]
  }],
  "max_tokens": 4096
}
```

Response: `choices[0].message.content` contains the OCR text.

## Related Code Files

- Create: `src-tauri/src/agents/cloud_mistral_api.rs`
- Read for reference: `src-tauri/src/agents/cloud_gemini_api.rs`

## Implementation Steps

1. Create `cloud_mistral_api.rs` with:
   - `pub const CLOUD_MISTRAL_MODEL: &str = "mistral-small-latest";`
   - `const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);`
   - `CloudMistralError` enum mirroring `CloudGeminiError` variants (RateLimited, BadRequest, AuthFailed, ServerError, Network, EmptyResponse, Parse)
   - Request structs: `ChatCompletionRequest`, `Message`, `ContentPart` (text variant + image_url variant)
   - Response structs: `ChatCompletionResponse` with `choices[].message.content`
   - `fn endpoint() -> &'static str` returning `"https://api.mistral.ai/v1/chat/completions"`
   - `pub async fn call(image_bytes, mime_type, prompt, api_key) -> Result<String, CloudMistralError>`:
     - Build `reqwest::Client` with 30s timeout
     - Base64-encode image, format as `data:{mime_type};base64,{encoded}`
     - POST with `Authorization: Bearer {api_key}` header
     - Map HTTP errors: 429 -> RateLimited, 400 -> BadRequest, 401/403 -> AuthFailed, 5xx -> ServerError
     - Parse response, extract `choices[0].message.content`
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

- Mistral API format is OpenAI-compatible, well-documented. Low risk.
- `max_tokens: 4096` sufficient for even complex LaTeX output.
