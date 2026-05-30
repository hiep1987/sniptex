---
phase: 3
title: "SnipTeX cloud-goclaw adapter"
status: pending
priority: P1
effort: "5h"
dependencies: [2]
---

# Phase 3: SnipTeX cloud-goclaw Adapter

## Overview

Implement the Rust adapter that talks to the Phase 2 Goclaw agent. Two transports per OCR call:
1. **HTTPS multipart POST** to `/api/v1/media/upload` to push the image bytes (reuses existing `reqwest` w/ rustls).
2. **WebSocket** to `/ws` for `connect` + `chat.send` referencing the uploaded path.

Mirrors the public surface of `cloud_gemini_api` / `cloud_mistral_api` so it slots into the dispatcher and PDF routing without special-casing. New crate dep: `tokio-tungstenite` for async WS.

## Requirements

- Functional: `call_with_image_path(image_path, _prompt, api_key) -> Result<String, CloudGoclawError>` — uploads file, opens WS, sends `connect`, sends `chat.send` with one media item attached, awaits the `res`, closes WS, returns `payload.content`.
- Functional: `call_with_pdf_path(pdf_path, _prompt, api_key)` — routes through the existing per-page render pipeline (same as cloud-gemini); each page = one full upload + chat.send round-trip.
- Non-functional: 30s timeout on each HTTPS upload; 120s timeout on the WS chat exchange (gpt-5.4 OCR latency ≈ codex CLI per VPS measurement).
- Non-functional: API key (`goclaw_xxx`) redacted from every error string surfaced to logs.
- Non-functional: one fresh WS connection per call. Matches the production `chatbot_goclaw.py` reference. No connection pool in v1.

## Architecture

```
call(image_bytes, mime, _prompt, api_key)
  │
  ├── 1. upload_media(image_bytes, mime, api_key) ──→ uploaded_path
  │      POST https://goclaw.tikz2svg.com/api/v1/media/upload
  │      Headers: Authorization: Bearer goclaw_xxx
  │      Body (multipart/form-data): file=<bytes>
  │      Response 200: { path: "<absolute>", mime_type: "image/png" }
  │
  └── 2. chat_with_media(uploaded_path, api_key) ──→ content
         wss://goclaw.tikz2svg.com/ws
         frame 1: { type: req, id: "1", method: "connect",
                    params: { token: "goclaw_xxx", user_id: "sniptex-<uuid>" } }
         frame 2: { type: req, id: "2", method: "chat.send",
                    params: { agentId: "tex-ocr",
                              sessionKey: "tex-ocr:<uuid>",
                              message: "",
                              media: [{ path: "<uploaded_path>",
                                        filename: "<basename>" }] } }
         read frames until { type: res, id: "2", ok: true, payload.content: "..." }
         close
```

Verified against Goclaw source at `/opt/goclaw/internal/gateway/methods/chat.go::parseMedia` (2026-05-30 VPS scout):

```go
type chatMediaItem struct {
    Path     string `json:"path"`
    Filename string `json:"filename,omitempty"`
}
type chatSendParams struct {
    Message    string          `json:"message"`
    AgentID    string          `json:"agentId"`
    SessionKey string          `json:"sessionKey"`
    Stream     bool            `json:"stream"`
    Media      json.RawMessage `json:"media,omitempty"` // []string OR []chatMediaItem
}
```

Each OCR call creates a fresh `sessionKey` (`tex-ocr:{uuid}`) — SnipTeX has no use for conversation history.

Error mapping → `DispatchError`:

| Source | Condition | DispatchError variant |
|--------|-----------|----------------------|
| HTTPS upload | DNS / TLS fail | `Network` |
| HTTPS upload | 401 / 403 | `AuthFailed(401)` |
| HTTPS upload | 413 (file too large) | `BadRequest` |
| HTTPS upload | 429 | `RateLimited` |
| HTTPS upload | 5xx | `NonZeroExit { code, stderr }` |
| HTTPS upload | timeout 30s | `Timeout(30)` |
| WS connect | TLS / DNS / refused | `Network` |
| WS res frame | `error.code == "UNAUTHORIZED"` | `AuthFailed(401)` |
| WS res frame | `error.code == "NOT_FOUND"` (agent missing) | `BadRequest` |
| WS res frame | `error.code == "RATE_LIMITED"` | `RateLimited` |
| WS res frame | `error.code == "INVALID_REQUEST"` | `BadRequest` |
| WS res frame | `payload.content` empty | `EmptyOutput` |
| WS overall | timeout 120s | `Timeout(120)` |

## Related Code Files

**Work context: `/Users/hieplequoc/Projects/sniptex`**

- Create: `src-tauri/src/agents/cloud_goclaw_api.rs` — the adapter (uploader + WS client).
- Modify: `src-tauri/src/agents/mod.rs` — `pub mod cloud_goclaw_api;` + cloud-detect entry (key presence → installed) alongside the other CloudApi branches.
- Modify: `src-tauri/src/agents/registry.rs` — add `CLOUD_GOCLAW_ID = "cloud-goclaw"`, spec entry, fallback chain placement.
- Modify: `src-tauri/src/agents/keychain.rs` — add `CLOUD_GOCLAW_ACCOUNT = "cloud-goclaw-api-key"` + has/get/set/delete accessors.
- Modify: `src-tauri/src/ocr/dispatcher.rs` — add `From<CloudGoclawError> for DispatchError` impl + `CLOUD_GOCLAW_ID` arm in `run_cloud_agent`.
- Modify: `src-tauri/Cargo.toml` — add `tokio-tungstenite = { version = "0.24", default-features = false, features = ["rustls-tls-native-roots"] }`; add `multipart` feature to existing `reqwest = { ..., features = [..., "multipart"] }`.

No `commands.rs` or frontend change in this phase — Phase 4 owns that.

## Implementation Steps

1. **Dep updates.**
   - `Cargo.toml`: add `tokio-tungstenite` with `rustls-tls-native-roots` to share TLS roots with the existing `reqwest` `rustls-tls` setup.
   - `reqwest`: enable `multipart` feature on the existing entry. Run `cargo check` to confirm clean resolve.
2. **Adapter file `cloud_goclaw_api.rs`** with the following Rust surface:
   ```rust
   pub const GOCLAW_API_BASE: &str = "https://goclaw.tikz2svg.com/api";
   pub const GOCLAW_WS_URL: &str = "wss://goclaw.tikz2svg.com/ws";
   pub const GOCLAW_AGENT_ID: &str = "tex-ocr";
   const UPLOAD_TIMEOUT: Duration = Duration::from_secs(30);
   const CHAT_TIMEOUT: Duration = Duration::from_secs(120);

   #[derive(Debug, thiserror::Error)]
   pub enum CloudGoclawError { /* RateLimited, BadRequest, AuthFailed(u16), NetworkError, ServerError, EmptyResponse, Parse */ }

   async fn upload_media(bytes: &[u8], mime_type: &str, filename: &str, api_key: &str)
     -> Result<String /* uploaded path */, CloudGoclawError>;

   async fn chat_with_media(uploaded_path: &str, basename: &str, api_key: &str)
     -> Result<String /* content */, CloudGoclawError>;

   pub async fn call(image_bytes: &[u8], mime_type: &str, prompt: &str, api_key: &str)
     -> Result<String, CloudGoclawError>;

   pub async fn call_with_image_path(image_path: &str, prompt: &str, api_key: &str)
     -> Result<String, CloudGoclawError>;

   pub async fn call_with_pdf_path(pdf_path: &str, prompt: &str, api_key: &str)
     -> Result<String, CloudGoclawError>;

   fn redact_key(s: &str) -> String;          // matches `cloud_gemini_api::redact_key` style
   pub fn parse_chat_response(text: &str) -> Result<String, CloudGoclawError>;
   ```
   Notes:
   - The `prompt` arg is ignored — Goclaw's `tex-ocr` Skill carries the prompt server-side.
   - `redact_key` strips `goclaw_[A-Za-z0-9_-]{20,}` from any error string.
   - `call_with_pdf_path` is just `call_with_image_path` looped per rendered page — same shape as the cloud-gemini adapter post-fix. Actual loop lives in `commands.rs::run_per_page_pdf_ocr`, the adapter itself is page-scoped.
3. **Registry wiring** (`registry.rs`):
   - `pub const CLOUD_GOCLAW_ID: &str = "cloud-goclaw";`
   - Add `AgentSpec { id: CLOUD_GOCLAW_ID, display_name: "Goclaw OCR Agent", binary_names: &[], supports_vision: true, kind: AgentKind::CloudApi }` to `AGENTS`.
   - Add to `DEFAULT_FALLBACK_CHAIN` after `CLOUD_MISTRAL_ID`: `[CODEX_ID, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID, CLOUD_GOCLAW_ID, GEMINI_CLI_ID]`.
   - Add `CLOUD_GOCLAW_ID` to the empty-args `match` arm in `build_command_args`.
4. **Keychain wiring** (`keychain.rs`):
   - `pub const CLOUD_GOCLAW_ACCOUNT: &str = "cloud-goclaw-api-key";`
   - `pub fn has/get/set_cloud_goclaw_api_key(...)` following the gemini/mistral pattern.
5. **Module wiring** (`agents/mod.rs`):
   - `pub mod cloud_goclaw_api;`
   - `use registry::{..., CLOUD_GOCLAW_ID};`
   - `detect_installed_agents()`: add an arm for `spec.id == CLOUD_GOCLAW_ID && keychain::has_cloud_goclaw_api_key()` → emit `AgentInfo` with `binary_path: "<cloud-api>"`, `version: "ws-v1"`.
6. **Dispatcher wiring** (`ocr/dispatcher.rs`):
   - Import `cloud_goclaw_api::{self, CloudGoclawError}`.
   - Add `From<CloudGoclawError> for DispatchError` mirroring the cloud-gemini impl (RateLimited / BadRequest / AuthFailed / Network / EmptyResponse / Parse).
   - Add a `CLOUD_GOCLAW_ID` arm in `run_cloud_agent` calling `cloud_goclaw_api::call_with_image_path(...)`.
7. **Unit tests** (mirror `cloud_gemini_api`'s test file):
   - `redact_strips_goclaw_key_pattern` — make sure `goclaw_xxx` never leaks.
   - `parse_chat_response_extracts_content` — happy path: synthetic `res` frame → `Ok("hello")`.
   - `parse_chat_response_propagates_error_frame` — `{ ok: false, error: { code: "RATE_LIMITED", message: "..." } }` → `Err(RateLimited)`.
   - `parse_chat_response_empty_content_returns_empty_response` — `payload.content == ""` → `Err(EmptyResponse)`.
8. **Integration test (local fake WS server)**:
   - Use `tokio-tungstenite::accept_async` to spin up a localhost WS server, respond to `connect` then `chat.send` with a canned `res` payload.
   - For the upload step, spin up a localhost `axum` (already in tree via tauri) or `wiremock` HTTP mock that returns `{ "path": "/temp/x.png", "mime_type": "image/png" }`.
   - Pass test base URLs via test-only constants (override `GOCLAW_API_BASE` / `GOCLAW_WS_URL` via env vars or a thread-local). Keeps real VPS out of CI.

## Success Criteria

- [ ] `cargo check` clean with new deps.
- [ ] `cargo test cloud_goclaw` passes including the local fake-server integration test.
- [ ] No new lint warnings.
- [ ] `run_cloud_agent` accepts `CLOUD_GOCLAW_ID` and returns `Ok(content)` against the fake server.
- [ ] Existing 43+ tests still pass (no regression on gemini / mistral paths).
- [ ] Manual VPS smoke (gated by Phase 4): set the real `goclaw_xxx` key in keychain, run `cargo run --bin cli_test -- --agent cloud-goclaw --image fixtures/sample.png` → real OCR output.

## Risk Assessment

- **Risk**: `tokio-tungstenite` rustls feature clashes with `reqwest`'s rustls setup (two roots stores compiled in, link errors).
  **Mitigation**: explicitly use `rustls-tls-native-roots` for `tokio-tungstenite` since `reqwest` already pulls native roots. Verify in step 1 before deeper work.
- **Risk**: upload step succeeds, returns a path, but `chat.send.media` rejects it because path format mismatches.
  **Mitigation**: log the path returned by upload (with `log::debug`) so first manual run surfaces the actual shape. Adjust the `media` JSON in step 2 if needed.
- **Risk**: Goclaw streams multiple frames (e.g. evt before res) even with `stream: false`.
  **Mitigation**: read frames until a `res` matching the sent `id` arrives or timeout elapses. Ignore `evt` frames in v1. Confirmed safe by `chat.go` source — `res` always carries `id` matching the request.
- **Risk**: gpt-5.4 OCR latency exceeds 120s on dense pages.
  **Mitigation**: per-page timeout in `run_per_page_pdf_ocr` (commands.rs) already routes CLI-class agents through `PDF_CLI_PAGE_TIMEOUT` (120s). Phase 4 adds `cloud-goclaw` to the cloud branch with its own 120s — not the 30s cloud default — because gpt-5.4 is CLI-class in latency.
- **Risk**: WS reconnect storm if the agent flakes — every retry opens a fresh connection.
  **Mitigation**: no automatic retry inside the adapter. The dispatcher's fallback chain handles "all agents failed".

## Security Considerations

- API key never logged in plain text — every error path runs through `redact_key`.
- Key stored via `keychain` (macOS Keychain + filesystem fallback) — same path as Gemini / Mistral.
- HTTPS/WSS only (reject `ws://` / `http://`).
- Multipart upload body is the image bytes only — no SnipTeX metadata leaks to the server beyond the file itself.

## Next Steps

Phase 4 exposes the new agent in Settings UI, adds the `"goclaw"` provider id to `commands.rs`, and adds an agent-kind-aware budget bump so the cloud-goclaw PDF flow gets 120s per page instead of the 30s default.
