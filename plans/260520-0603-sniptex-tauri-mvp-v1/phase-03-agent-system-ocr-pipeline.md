---
phase: 3
title: "Agent System & OCR Pipeline"
status: complete
completed: 2026-05-22
priority: P1
effort: "3d"
dependencies: [2]
---

# Phase 3: Agent System & OCR Pipeline

## Overview

Implement the core BYOA + BYOK brain: agent registry (**Codex CLI default + Gemini CLI experimental + Gemini Vision API cloud fallback**), cross-platform binary detection (CLI agents) + secure API-key storage (cloud agent), async dispatch with timeout, post-processor that strips LLM preambles/fences/sign-offs, smart formatter that classifies output (EQUATION_ONLY / TABLE_ONLY / MIXED), and a fallback chain across all three. Validate end-to-end via a CLI test harness before wiring it to the UI.

<!-- Updated: Validation Session 2 - Codex restored to v1 scope (reversal of Session 1) -->
<!-- Updated: Validation Session 3 (2026-05-21) - Path C hybrid: cloud_gemini_api.rs adapter added; Codex is the default; Gemini CLI gated behind --approval-mode plan; master prompt mirrors Session-3 patches -->

## Key Insights

- Master prompt finalized in `plans/test-prompt.sh` (Session-3 patches: category-label silence + table-cell math-scope rule). **Single source of truth** lives in `src-tauri/src/ocr/prompt.rs`; the bash script and Rust module must stay in sync — including the Session-3 wording.
- v1 ships **3 adapters**: `codex.rs` (default), `gemini_cli.rs` (experimental, gated), `cloud_gemini_api.rs` (BYOK direct API call). Adapter pattern in `agents/registry.rs` makes adding more (Claude Code, OpenCode) a v1.x mechanical task.
- Detection must search PATH + platform-specific extra dirs (Homebrew, mise, npm-global, AppData/Roaming/npm) for CLI agents. Cloud-API path requires no binary detection — just key presence in OS keychain.
- Codex image input syntax verified Session 3: `codex exec --skip-git-repo-check --image <FILE> --output-last-message <FILE> -- "<prompt>"`. `--skip-git-repo-check` is REQUIRED when CWD isn't a git repo (true for snip staging dir). `--output-last-message` writes clean assistant-only output.
- Gemini CLI gating: Phase 1 surfaced a systemic failure mode where Gemini's agent loop tries `read_file ~/.claude/.ck.json` when image has no surrounding text. **Mitigation:** spawn Gemini with `--approval-mode plan` (read-only) or a `--policy` that disables `read_file`/`write_file` tools. Confirm in this phase via cli_test.
- Gemini Vision API path uses `gemini-2.0-flash-exp` model (free tier 15 RPM / 1500 RPD), endpoint `https://generativelanguage.googleapis.com/v1beta/models/...:generateContent`. Same MASTER_PROMPT, image sent as inline base64 data part.

## Requirements

**Functional**
- `detect_installed_agents()` returns list of `AgentInfo` with binary path + version.
- `run_ocr(agent, image_path)` returns cleaned text within 30s or returns typed `DispatchError`.
- `post_process(raw)` strips preambles (EN + VN), code fences, sign-offs.
- `detect_type(output)` returns one of `EquationOnly`, `TableOnly`, `Mixed` using heuristics from `replan.md` §5.
- `run_with_fallback(&[agent], image)` tries agents in order, returns first success.

**Non-functional**
- All async via Tokio; no blocking calls in dispatcher.
- Timeout enforced via `tokio::time::timeout` + `kill_on_drop` so spawned processes never leak.

## Architecture

```
src-tauri/src/
├── ocr/
│   ├── mod.rs
│   ├── prompt.rs              (MASTER_PROMPT const — single source of truth; mirrors plans/test-prompt.sh post-Session-3)
│   ├── dispatcher.rs          (run_ocr, run_with_fallback, DispatchError)
│   ├── postprocess.rs         (post_process — strips preambles, fences, sign-offs, and category-label leak as defense in depth)
│   └── smart_format.rs        (detect_type, DetectedType enum)
├── agents/
│   ├── mod.rs                 (detect_installed_agents, is_executable, detect_version)
│   ├── registry.rs            (AGENTS const, AgentSpec, AgentInfo, build_command_args, AgentKind { CliBin, CloudApi })
│   ├── codex.rs               (DEFAULT — Codex-specific arg builder with --skip-git-repo-check + --output-last-message + -- separator)
│   ├── gemini_cli.rs          (experimental secondary — gated with --approval-mode plan to prevent tool-loop divergence)
│   ├── cloud_gemini_api.rs    (NEW Session 3 — HTTP client for Gemini Vision API; reads key from keychain)
│   └── keychain.rs            (NEW Session 3 — secure API-key storage via `keyring` crate; service="com.sniptex", account="gemini-api-key")
└── commands.rs                (Tauri commands: detect_agents, test_agent, run_snip [stub], set_api_key, has_api_key)
```

Reference code in `replan.md` §5–§6 — copy structures verbatim then adapt.

## Related Code Files

- Create: `src-tauri/src/ocr/{mod,prompt,dispatcher,postprocess,smart_format}.rs`
- Create: `src-tauri/src/agents/{mod,registry,codex,gemini_cli,cloud_gemini_api,keychain}.rs`
- Modify: `src-tauri/src/commands.rs` — add `detect_agents`, `test_agent`, `set_api_key`, `has_api_key`, stub `run_snip`
- Modify: `src-tauri/Cargo.toml` — add `tokio` (full), `regex`, `thiserror`, `uuid`, `dirs`, `serde`, `reqwest` (rustls-tls, json), `base64`, `keyring`
- Create: `src-tauri/src/bin/cli_test.rs` — `cargo run --bin cli_test -- --image test.png --agent codex` (also supports `--agent cloud-gemini`)
- Create: `tests/rust/ocr_postprocess_test.rs`, `tests/rust/ocr_smart_format_test.rs`, `tests/rust/cloud_gemini_api_test.rs` (mocked HTTP)

## Implementation Steps

1. Add Rust crate deps: `tokio = { version = "1", features = ["full"] }`, `regex`, `thiserror`, `uuid`, `dirs`, `serde`, `serde_json`, `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }`, `base64`, `keyring = "3"`.
2. Write `ocr/prompt.rs` with `pub const MASTER_PROMPT: &str = "..."` — copy the post-Session-3 wording from `test-prompt.sh` verbatim, including `DETECTION (internal, do not emit):` heading and the table-cell math-scope rule. Add a doc comment linking to `plans/test-prompt.sh` and noting both must stay in sync.
3. Implement `ocr/postprocess.rs` per `replan.md` §5 — strip preambles (`Here is`, `Sure!`, `Đây là`, ...), strip ```markdown / ```latex fences, strip sign-offs (`Let me know`, `Hope this helps`), AND defensively strip leading `^(MIXED|EQUATION_ONLY|TABLE_ONLY)\s*$` lines (belt-and-suspenders against any residual category-label leak — see Session-3 Critical Finding #6).
4. Implement `ocr/smart_format.rs` per `replan.md` §5 — `detect_type` with 3 heuristics. **Fix the Session-3-surfaced bug**: check LaTeX-density BEFORE blank-line presence, otherwise multi-line equations get misclassified as MIXED.
5. Write unit tests in `tests/rust/`:
   - `post_process_strips_preamble_then_returns_body`
   - `post_process_strips_fenced_latex_block`
   - `post_process_strips_leading_category_label` (Session-3 regression guard)
   - `detect_type_returns_table_only_for_pure_markdown_table`
   - `detect_type_returns_equation_only_for_raw_latex_even_with_newlines` (Session-3 regression guard)
   - `detect_type_returns_mixed_for_text_with_inline_math`
6. Implement `agents/registry.rs` with `AGENTS` const containing **3 specs**: Codex (default), Gemini CLI (experimental, gated), Cloud Gemini API (BYOK). `AgentKind` enum distinguishes `CliBin` from `CloudApi`. `build_command_args(agent_id, image, prompt)` matches Session-3-verified syntax:
   - **Codex** (default): `exec --skip-git-repo-check --image {image} --output-last-message {last_msg_file} -- "{prompt}"`
   - **Gemini CLI** (experimental): `-p "{prompt}\n@\"{image}\"" --yolo --approval-mode plan` (the `--approval-mode plan` gates the tool-loop and is REQUIRED per Session 3)
   - **Cloud Gemini API**: not a CLI; uses `cloud_gemini_api::call(image, prompt, api_key)` directly.
7. Implement `agents/mod.rs::detect_installed_agents()`:
   - Parse `PATH` with platform-correct separator (`:` Unix, `;` Windows)
   - Append extra dirs: `~/.local/bin`, `~/.local/share/mise/installs`, `~/.bun/bin`, `~/.cargo/bin`, `~/AppData/Roaming/npm` (Win), `/opt/homebrew/bin` + `/usr/local/bin` (Mac)
   - Apply `.exe` suffix on Windows
   - For each candidate path, call `is_executable(path)` (platform-conditional impl)
   - On hit, run `<binary> --version` with 2s timeout to capture version string
   - Additionally check keychain via `keychain::has_gemini_api_key()` — if present, surface `cloud-gemini` as an installed "agent" too.
8. Implement `agents/keychain.rs`: thin wrapper over `keyring::Entry::new("com.sniptex", "gemini-api-key")` with `get`, `set`, `has`, `delete` methods. Surface errors as typed `KeychainError`.
9. Implement `agents/cloud_gemini_api.rs::call(image_bytes, prompt, api_key) -> Result<String, DispatchError>`:
   - POST to `https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key=<api_key>`
   - Body: `{contents: [{parts: [{text: <prompt>}, {inline_data: {mime_type: "image/png", data: <base64>}}]}]}`
   - Parse `candidates[0].content.parts[0].text`
   - Map HTTP 429 → `DispatchError::RateLimited`, 400 → `DispatchError::BadRequest`, network errors → `DispatchError::Network`
   - 30s timeout via `reqwest::Client::builder().timeout(...)`
10. Implement `ocr/dispatcher.rs::run_ocr`:
    - Branch on `AgentKind`: `CliBin` → spawn process path (existing); `CloudApi` → call `cloud_gemini_api::call`
    - CLI path: spawn `tokio::process::Command` with `kill_on_drop(true)` and `stdin(null)`; `tokio::time::timeout(Duration::from_secs(30), cmd.output()).await`
    - Branch on exit code, detect rate-limit by scanning stderr for "rate limit" / "429"
    - Both paths: pipe output through `post_process`; reject empty / `[UNREADABLE]` as `DispatchError::EmptyOutput`
11. Implement `run_with_fallback(&[AgentInfo], image)` — iterate per user-configured priority, return first `Ok`, accumulate last error. Default priority = `[codex, cloud-gemini, gemini-cli]` (Codex first; cloud preferred over CLI Gemini due to Session-3 EQ_ONLY collapse).
12. Wire Tauri commands in `commands.rs`:
    - `#[tauri::command] async fn detect_agents() -> Result<Vec<AgentInfo>, String>`
    - `#[tauri::command] async fn test_agent(agent_id: String) -> Result<bool, String>` (runs OCR on a bundled tiny test image)
    - `#[tauri::command] async fn set_api_key(provider: String, key: String) -> Result<(), String>` (provider = "gemini")
    - `#[tauri::command] async fn has_api_key(provider: String) -> Result<bool, String>`
    - Stub `run_snip(agent_id: Option<String>)` to be filled in Phase 4 once capture exists
13. Create `src-tauri/src/bin/cli_test.rs`: parse `--image PATH` + optional `--agent ID` (`codex`|`gemini-cli`|`cloud-gemini`), run dispatcher, print result. Use as smoke test before UI exists.
14. Run unit tests: `cargo test --manifest-path src-tauri/Cargo.toml`. All green.
15. Manual smoke on all 3 paths:
    - `cargo run --bin cli_test -- --image fixtures-extra/equation-only/<sample>.png --agent codex`
    - `cargo run --bin cli_test -- --image fixtures-extra/equation-only/<sample>.png --agent cloud-gemini` (after `set_api_key`)
    - `cargo run --bin cli_test -- --image fixtures-extra/equation-only/<sample>.png --agent gemini-cli` (verify `--approval-mode plan` prevents the `read_file ~/.claude/.ck.json` failure mode)

## Todo List

- [x] Add tokio/regex/thiserror/uuid/dirs/reqwest/base64/keyring deps
- [x] Write `ocr/prompt.rs` mirroring post-Session-3 `plans/test-prompt.sh` (DETECTION-internal + table-cell math-scope rule)
- [x] Implement `post_process` with all preamble/fence/sign-off strips + category-label defense
- [x] Implement `detect_type` 3-heuristic classifier (LaTeX-density check BEFORE blank-line check — Session-3 bug fix)
- [x] Write 6+ unit tests for postprocess + smart_format (incl. Session-3 regression guards) — 29 tests total
- [x] Implement `agents/registry.rs` with 3 specs: codex (default), gemini-cli (gated), cloud-gemini (BYOK)
- [x] Implement `agents/keychain.rs` with `keyring` crate (service=com.sniptex) — incl. `has_detailed` to distinguish NotFound from backend faults
- [x] Implement `agents/cloud_gemini_api.rs` HTTP client to generativelanguage.googleapis.com (rustls-tls only, key redaction in errors)
- [x] Implement cross-platform `detect_installed_agents` + executable check + cloud-key presence check
- [x] Implement `run_ocr` dual-path (CliBin + CloudApi) with 30s timeout + rate-limit detection (tightened: word-boundary 429 + context tokens)
- [x] Implement `run_with_fallback` chain (default priority: codex → cloud-gemini → gemini-cli)
- [x] Expose 5 Tauri commands (`detect_agents`, `test_agent`, `set_api_key`, `has_api_key`, stub `run_snip`) + `delete_api_key`
- [x] Build CLI test binary supporting all 3 agent IDs
- [x] Run cargo test — all green (29 tests pass, 0 clippy warnings)
- [x] Smoke test cli_test on real fixture image — codex ✅ (clean LaTeX), gemini-cli ✅ (plan-mode gate verified), cloud-gemini deferred (requires user API key)
- [x] Verify Gemini CLI `--approval-mode plan` prevents `read_file ~/.claude/.ck.json` failure mode — confirmed: plan-mode refuses out-of-workspace reads instead of looping into tool calls. Trade-off (workspace restriction) is the documented "best-effort" mitigation per Risk Assessment.

## Success Criteria

- [ ] `cli_test --image <real-fixture> --agent codex` produces cleaned LaTeX/Markdown on stdout
- [ ] `cli_test --image <real-fixture> --agent cloud-gemini` (after `set_api_key`) produces cleaned output in ≤5 s p95
- [ ] `cli_test --image <real-fixture> --agent gemini-cli` succeeds with `--approval-mode plan` flag — no `read_file ~/.claude/.ck.json` failures
- [ ] `cli_test` returns nonzero exit on timeout / rate limit / empty output
- [ ] `detect_installed_agents` finds Codex + Gemini CLI installed via npm-global on Mac and Windows; surfaces `cloud-gemini` when keychain has a key
- [ ] API key stored via `keyring` is readable on next process launch (cross-platform: macOS Keychain, Windows Credential Manager)
- [ ] Unit test coverage on `postprocess` + `smart_format` ≥85%
- [ ] No compile warnings in `cargo clippy --all-targets --all-features`

## Risk Assessment

- **Risk: Gemini CLI or Codex changes flag syntax in a future release** — Mitigation: adapter pattern isolates change to `agents/{gemini_cli,codex,cloud_gemini_api}.rs`; add CI smoke that runs against latest installed CLI versions weekly.
- **Risk: Gemini API key leaks** — Mitigation: stored via OS keychain only (`keyring` crate). Never logged. Never serialized to settings.json. Never printed in error messages (sanitize via `Display` impl on `DispatchError`).
- **Risk: Gemini API free-tier rate limit (15 RPM / 1500 RPD) hit during user session** — Mitigation: `DispatchError::RateLimited` falls back to next agent in chain; surface "Daily limit reached, falling back to CLI" toast.
- **Risk: Gemini API endpoint changes / model deprecation** — Mitigation: pin model name in const `CLOUD_GEMINI_MODEL: &str = "gemini-2.0-flash-exp"`; document upgrade procedure.
- **Risk: `--approval-mode plan` flag doesn't actually prevent Gemini-CLI tool-loop** — Mitigation: verify in this phase via `cli_test --agent gemini-cli`; if flag is insufficient, also use `--policy` to disable `read_file`/`write_file`; if that's still insufficient, downgrade Gemini-CLI to "best-effort" with documented failure rate.
- **Risk: Codex image input absent/changed** — Mitigation: ~~Phase 1 verifies syntax~~. Verified Session 3 (working). If a future Codex release breaks it, adapter pattern isolates the change.
- **Risk: Rate-limit detection brittle (string match in stderr)** — Mitigation: pattern matches both word "rate limit" and HTTP 429; extend list if other phrasings discovered.
- **Risk: Timeout kills mid-stream output user wanted** — Mitigation: 30s is generous; expose timeout in settings later (Phase 8).

## Security Considerations

- Image written to `std::env::temp_dir()` — system handles permissions. Delete after dispatch on success and failure (use `Drop` guard or explicit cleanup).
- Spawned CLI inherits user env — do not leak `PATH` mutations from the app to other processes.
- Never log image contents or full LLM output in production builds; gate behind `RUST_LOG=debug`.
- **Gemini API key** (Session-3 addition): stored exclusively via OS keychain (`keyring`). NEVER written to settings.json, NEVER printed in logs/errors. `Display` impl on `DispatchError` redacts any string that looks like a key (`re::is_match(r"AIza[0-9A-Za-z_-]{35}")`).
- **Cloud-API request body** contains image bytes — sent over TLS only (`reqwest` with `rustls-tls` features). Disclose in privacy section of landing page that cloud mode sends image to Google.

## Next Steps

- Phase 4 (Screen Capture) builds the image source that feeds `run_snip`.
- Phase 6 (React UI) consumes `detect_agents` + `test_agent` for Settings UI.

## Open Questions

- Cache `detect_installed_agents` result for session, or re-scan each time? Default: cache + manual "Re-scan" button in Settings.
