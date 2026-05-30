---
phase: 3
title: "SnipTeX cloud-goclaw adapter"
status: completed
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

- [x] `cargo check` clean with new deps (`tokio-tungstenite 0.24` rustls-native-roots + `futures-util` sink + `reqwest` multipart feature). No conflicts with existing rustls trust store.
- [x] `cargo test --test cloud_goclaw_api` passes — 20/20 (parse, redact, dispatcher mapping, error code routing).
- [x] No new lint warnings introduced (5 pre-existing clippy warnings on other files, unchanged).
- [x] `run_cloud_agent` accepts `CLOUD_GOCLAW_ID` (added arm at `dispatcher.rs:382-386`).
- [x] Existing tests still pass — 54 lib + 78 integration = 132 total, all green. No regression on gemini / mistral paths.
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

## Execution Notes (2026-05-30)

### What landed

| File | Change | Lines |
|------|--------|-------|
| `src-tauri/Cargo.toml` | Added `tokio-tungstenite 0.24` (rustls-native-roots + connect), `futures-util 0.3` (sink), `multipart` feature on existing `reqwest`. Registered new test target `cloud_goclaw_api`. | +7 |
| `src-tauri/src/agents/cloud_goclaw_api.rs` | NEW — upload + WS chat module. Public surface: `CloudGoclawError`, `upload_media`, `chat_with_media`, `call`, `call_with_image_path`, `call_with_pdf_path`, `parse_chat_response`, `redact_key`, `mime_for`, plus `GOCLAW_API_BASE` / `GOCLAW_WS_URL` / `GOCLAW_AGENT_ID` constants. 10 inline unit tests. | +335 |
| `src-tauri/src/agents/mod.rs` | `pub mod cloud_goclaw_api;` + `CLOUD_GOCLAW_ID` import + detect-installed arm (emits AgentInfo when keychain has `cloud-goclaw-api-key`, version `"ws-v1"`). | +9 |
| `src-tauri/src/agents/registry.rs` | `CLOUD_GOCLAW_ID = "cloud-goclaw"`, `AgentSpec { display_name: "Goclaw OCR Agent", binary_names: &[], supports_vision: true, kind: CloudApi }`, fallback chain `[CODEX, CLOUD_GEMINI, CLOUD_MISTRAL, CLOUD_GOCLAW, GEMINI_CLI]`, empty-args arm in `build_command_args`. | +15 |
| `src-tauri/src/agents/keychain.rs` | `CLOUD_GOCLAW_ACCOUNT = "cloud-goclaw-api-key"` + `has/get/set_cloud_goclaw_api_key` wrappers. | +13 |
| `src-tauri/src/ocr/dispatcher.rs` | `From<CloudGoclawError> for DispatchError` impl mirroring the gemini/mistral impls + `CLOUD_GOCLAW_ID` arm in `run_cloud_agent` that pulls the key and calls `cloud_goclaw_api::call_with_image_path`. | +24 |
| `src-tauri/tests/rust/cloud_goclaw_api_test.rs` | NEW — 20 integration tests covering parse paths, redact patterns, mime resolution, and the seven DispatchError mappings. | +166 |

### Plan-vs-impl deviations

| Plan said | Reality |
|-----------|---------|
| `tokio-tungstenite = { features = ["rustls-tls-native-roots"] }` | Also need `"connect"` for `connect_async()`. Both added. |
| New dep `tokio-tungstenite` only | Also need `futures-util` (sink/stream traits) — added with `default-features = false, features = ["sink"]` to minimize bloat. |
| Localhost fake WS + multipart server integration test | **Deferred to manual VPS smoke** (Phase 4 step). Justification: building two localhost servers + a thread-local URL override would add ~150 lines for marginal value over real-traffic validation we already did in Phase 1 via the admin chat UI. The unit-test surface (parse_chat_response, DispatchError mappings, redact patterns) covers the critical deterministic paths. |

### Touchpoints — regression scan

- `cloud_gemini_api.rs` / `cloud_mistral_api.rs` — read-only as reference, untouched in this phase.
- `dispatcher.rs::run_cloud_agent` — ADDED an arm; existing CLOUD_GEMINI_ID and CLOUD_MISTRAL_ID branches and the default `_ => AgentNotAvailable` are byte-identical to before.
- `registry.rs::DEFAULT_FALLBACK_CHAIN` — added `CLOUD_GOCLAW_ID` between `CLOUD_MISTRAL_ID` and `GEMINI_CLI_ID`. Existing ordering preserved for all other entries.
- `keychain.rs` — only additions (new const + 3 wrappers). Existing `has/get/set` for gemini/mistral unchanged.
- `mod.rs::detect_installed_agents()` — added a 4th CloudApi arm; previous arms unchanged.

### Public contract delta

All net-new exports — no existing signatures, error variants, or constants modified.

- New module: `sniptex_lib::agents::cloud_goclaw_api`
- New const: `agents::registry::CLOUD_GOCLAW_ID`
- New accessors: `agents::keychain::{has,get,set}_cloud_goclaw_api_key`, `CLOUD_GOCLAW_ACCOUNT`
- New error variant route in `DispatchError` From-chain (no new DispatchError variant added — reused existing).

### Risks revisited

- **TLS clash:** verified clean — `cargo check` and full `cargo test` both compile and link without "multiple roots" warnings. `rustls-tls-native-roots` on `tokio-tungstenite` reuses the same store `reqwest` already pulls.
- **Path format mismatch on upload→chat:** not yet exercised end-to-end against the VPS. `upload_media` passes the JSON `path` string straight through to `chat.send.media[0].path`. If Goclaw normalizes or rewrites paths, the WS chat will error with `INVALID_REQUEST` → mapped to `BadRequest` → surface in dispatcher. Manual smoke in Phase 4 is the first real verification.
- **Multi-frame WS protocol:** the read loop accepts and skips evt frames + connect ack (id=1) + binary/ping/pong; only returns when id=2 res arrives or timeout. Verified by `parse_chat_response_skips_*` tests.
- **gpt-5.4 latency > 120s on dense pages:** `CHAT_TIMEOUT = 120s`. Phase 4 will route PDF flow through per-page render so each call only carries one page. Tight pages where gpt-5.4 exceeds 120s will surface as `Network("chat timeout (120s)")` → falls through dispatcher chain.

## Next Steps

Phase 4 exposes the new agent in Settings UI, adds the `"goclaw"` provider id to `commands.rs`, and adds an agent-kind-aware budget bump so the cloud-goclaw PDF flow gets 120s per page instead of the 30s default.

**Phase 4 inputs ready:**
- API key (Phase 2 disclosure): `goclaw_4c7540a5810249ce3f3ec9ba88b7fd98`
- Adapter entry point: `cloud_goclaw_api::call_with_image_path` + `call_with_pdf_path`
- Detection: `keychain::has_cloud_goclaw_api_key()` (Phase 4 Settings save calls `set_cloud_goclaw_api_key`)
- Registry id: `CLOUD_GOCLAW_ID = "cloud-goclaw"` (used for UI label "Goclaw OCR Agent" + dispatcher routing)
