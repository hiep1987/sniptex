---
title: "Phase 02 - Hybrid Backend Adapter"
status: complete
priority: P1
effort: 3h
created: 2026-06-03
---

# Phase 02 - Hybrid Backend Adapter

## Context Links

- [Phase 01](./phase-01-api-contract-and-cost-controls.md)
- Existing Novita adapter: `src-tauri/src/agents/cloud_novita_api.rs`
- Dispatcher: `src-tauri/src/ocr/dispatcher.rs`

## Overview

Implement a new adapter module that orchestrates DeepSeek-OCR 2 then GPT OSS 120B.

## Key Insights

- Do not merge this into `cloud_novita_api.rs`; that file already owns DeepSeek-OCR 2.
- Keep public surface equivalent to other cloud adapters.
- Split helper functions if file approaches 200 lines.

## Requirements

- Functional:
  - `call_with_image_path(image_path, prompt, api_key) -> Result<String, CloudNovitaHybridError>`.
  - `call(image_bytes, mime_type, prompt, api_key)`.
  - `parse_gpt_oss_response`.
  - `redact_key`.
- Non-functional:
  - Timeouts.
  - Typed errors.
  - Tests do not hit network.

## Architecture

```rust
pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaHybridError> {
    let bytes = tokio::fs::read(image_path).await?;
    call(&bytes, mime_for(image_path), prompt, api_key).await
}
```

## Related Code Files

- Create: `src-tauri/src/agents/cloud_novita_hybrid_api.rs`.
- Modify: `src-tauri/src/agents/mod.rs`.
- Modify: `src-tauri/src/ocr/dispatcher.rs`.
- Modify: `src-tauri/Cargo.toml` for test target only if needed.

## Implementation Steps

1. Add `CloudNovitaHybridError` mirroring other cloud adapter error enums.
2. Implement mime handling from existing Novita adapter.
3. Call existing `cloud_novita_api::call` for DeepSeek-OCR 2 markdown.
4. Implement `normalize_intermediate_markdown`:
   - Trim.
   - Collapse excessive blank lines.
   - Strip obvious OCR metadata.
   - Cap length.
5. Implement `should_call_gpt_cleanup`:
   - MVP: always call GPT unless markdown is empty.
   - Future: skip GPT for clean short LaTeX.
6. Implement `call_gpt_oss_cleanup` using OpenAI-compatible chat completions.
7. Return cleaned GPT output to dispatcher; dispatcher handles shared `post_process`.

## Todo List

- [x] Create adapter module.
- [x] Add parse helpers.
- [x] Add redaction helper.
- [x] Map errors into `DispatchError`.
- [x] Add no-network tests.

## Success Criteria

- Adapter compiles.
- Unit tests cover parser happy path, empty response, redaction, and error mapping.
- Live HTTP status handling remains validated through targeted smoke once endpoint is available.
- DeepSeek errors map cleanly into hybrid errors.

## Risk Assessment

- Risk: GPT cleanup hallucinates missing math.
  - Mitigation: strict prompt, `[UNREADABLE]` contract, consistency checks in tests.
- Risk: Intermediate markdown too large.
  - Mitigation: cap and surface truncation metric in smoke output.

## Security Considerations

- Redact API keys and bearer headers.
- Do not include raw image bytes in errors.
- Do not write intermediate markdown to disk unless user enables debug.

## Next Steps

- Phase 03 exposes the agent in registry and settings.
