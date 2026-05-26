---
phase: 3
title: "Keychain and Commands Wiring"
status: completed
priority: P1
effort: "20m"
dependencies: [1]
---

# Phase 3: Keychain and Commands Wiring

## Overview

Add Mistral API key storage/retrieval helpers to `keychain.rs` and extend the Tauri `set_api_key` / `has_api_key` / `delete_api_key` commands to accept `"mistral"` as a provider.

## Related Code Files

- Modify: `src-tauri/src/agents/keychain.rs`
- Modify: `src-tauri/src/commands.rs`

## Implementation Steps

### keychain.rs

1. Add constant: `pub const MISTRAL_ACCOUNT: &str = "mistral-api-key";`

2. Add helper functions (mirror Gemini pattern):
   ```rust
   pub fn has_mistral_api_key() -> bool {
       has(MISTRAL_ACCOUNT)
   }

   pub fn get_mistral_api_key() -> Result<String, KeychainError> {
       get(MISTRAL_ACCOUNT)
   }

   pub fn set_mistral_api_key(key: &str) -> Result<(), KeychainError> {
       set(MISTRAL_ACCOUNT, key)
   }
   ```

### commands.rs

3. Extend `set_api_key` match:
   ```rust
   "mistral" => keychain::set_mistral_api_key(&key).map_err(|e| e.to_string()),
   ```

4. Extend `has_api_key` match:
   ```rust
   "mistral" => Ok(keychain::has_mistral_api_key()),
   ```

5. Extend `delete_api_key` match:
   ```rust
   "mistral" => keychain::delete(keychain::MISTRAL_ACCOUNT).map_err(|e| e.to_string()),
   ```

## Success Criteria

- [x] `cargo check` passes
- [x] `set_api_key("mistral", "test-key")` stores in OS keychain
- [x] `has_api_key("mistral")` returns true after set
- [x] `delete_api_key("mistral")` removes key
- [x] Frontend `detect_agents` now returns Mistral when key is set

## Validation

- `cargo check --manifest-path src-tauri/Cargo.toml`

## Risk Assessment

- Pure additive change to existing match arms. No risk to Gemini or Codex paths.
