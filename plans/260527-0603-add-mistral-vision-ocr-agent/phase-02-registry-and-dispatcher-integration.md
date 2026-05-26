---
phase: 2
title: "Registry and Dispatcher Integration"
status: completed
priority: P1
effort: "30m"
dependencies: [1]
---

# Phase 2: Registry and Dispatcher Integration

## Overview

Register the Mistral agent in the static catalogue (`registry.rs`), wire it into the dispatcher (`dispatcher.rs`), and add it to the agent detection logic (`mod.rs`).

## Related Code Files

- Modify: `src-tauri/src/agents/registry.rs`
- Modify: `src-tauri/src/ocr/dispatcher.rs`
- Modify: `src-tauri/src/agents/mod.rs`

## Implementation Steps

### registry.rs

1. Add constant: `pub const CLOUD_MISTRAL_ID: &str = "cloud-mistral";`

2. Add entry to `AGENTS` array:
   ```rust
   AgentSpec {
       id: CLOUD_MISTRAL_ID,
       display_name: "Mistral Vision API",
       binary_names: &[],
       supports_vision: true,
       kind: AgentKind::CloudApi,
   },
   ```

3. Append `CLOUD_MISTRAL_ID` to `DEFAULT_FALLBACK_CHAIN` (after `CLOUD_GEMINI_ID`):
   ```rust
   pub const DEFAULT_FALLBACK_CHAIN: &[&str] = &[CODEX_ID, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID];
   ```

4. Add `CLOUD_MISTRAL_ID => Vec::new()` arm in `build_command_args` match.

### dispatcher.rs

5. Add `use crate::agents::cloud_mistral_api::{self, CloudMistralError};`

6. Add `impl From<CloudMistralError> for DispatchError` block (same mapping pattern as `CloudGeminiError`).

7. Extend `run_cloud_agent` to handle `CLOUD_MISTRAL_ID`:
   ```rust
   async fn run_cloud_agent(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
       match agent.spec.id {
           CLOUD_GEMINI_ID => { /* existing gemini logic */ },
           CLOUD_MISTRAL_ID => {
               let key = keychain::get_mistral_api_key()
                   .map_err(|_| DispatchError::MissingApiKey("mistral"))?;
               let raw = cloud_mistral_api::call_with_image_path(image_path, MASTER_PROMPT, &key).await?;
               let cleaned = post_process(&raw);
               if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
                   return Err(DispatchError::EmptyOutput);
               }
               Ok(cleaned)
           },
           _ => Err(DispatchError::AgentNotAvailable(agent.spec.id.to_string())),
       }
   }
   ```

### mod.rs (agent detection)

8. Import `CLOUD_MISTRAL_ID` from registry.

9. Add detection branch in `detect_installed_agents`:
   ```rust
   AgentKind::CloudApi => {
       if spec.id == CLOUD_GEMINI_ID && keychain::has_gemini_api_key() {
           // existing...
       }
       if spec.id == CLOUD_MISTRAL_ID && keychain::has_mistral_api_key() {
           results.push(AgentInfo {
               spec: spec.clone(),
               binary_path: PathBuf::from("<cloud-api>"),
               version: Some("v1".to_string()),
           });
       }
   }
   ```

## Success Criteria

- [x] `cargo check` passes after all three files modified
- [x] `CLOUD_MISTRAL_ID` exported from registry
- [x] Mistral appears in `DEFAULT_FALLBACK_CHAIN`
- [x] `detect_installed_agents` returns Mistral when API key present
- [x] Dispatcher routes `cloud-mistral` to Mistral adapter

## Validation

- `cargo check --manifest-path src-tauri/Cargo.toml`

## Risk Assessment

- Pattern is identical to existing Cloud Gemini wiring. Mechanical change, low risk.
