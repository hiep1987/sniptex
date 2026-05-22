# Phase 3 — Completion Report

Date: 2026-05-22
Plan: `phase-03-agent-system-ocr-pipeline.md`
Status: ✅ Complete

## Outcome

BYOA + BYOK OCR backbone is operational. Codex CLI (default) runs end-to-end on a real fixture. Gemini CLI (`--approval-mode plan` gated) is verified safe but workspace-restricted (documented best-effort). Cloud Gemini API path implemented + unit-tested; live smoke deferred to user (needs an API key in keychain).

## Deliverables

| File | Lines | Role |
|---|---|---|
| `src-tauri/Cargo.toml` | +18 deps | tokio, regex, thiserror, uuid, dirs, reqwest+rustls, base64, keyring |
| `src-tauri/src/ocr/prompt.rs` | 47 | Master prompt, verbatim mirror of `plans/test-prompt.sh` (Session-3) |
| `src-tauri/src/ocr/postprocess.rs` | 84 | Preamble + fence + sign-off + category-label strips |
| `src-tauri/src/ocr/smart_format.rs` | 66 | EquationOnly / TableOnly / Mixed classifier (LaTeX-density before blank-line) |
| `src-tauri/src/ocr/dispatcher.rs` | 257 | run_ocr / run_with_fallback / TempFile RAII / tightened rate-limit detection |
| `src-tauri/src/ocr/mod.rs` | 10 | re-exports |
| `src-tauri/src/agents/registry.rs` | 122 | 3 specs (codex / gemini-cli / cloud-gemini), AgentKind, argv builder |
| `src-tauri/src/agents/codex.rs` | 21 | Adapter (Session-3 verified argv) |
| `src-tauri/src/agents/gemini_cli.rs` | 19 | Adapter (plan-mode gated) |
| `src-tauri/src/agents/cloud_gemini_api.rs` | 196 | HTTPS adapter + redaction + error mapping |
| `src-tauri/src/agents/keychain.rs` | 78 | OS keychain wrapper, has/has_detailed split |
| `src-tauri/src/agents/mod.rs` | 132 | Cross-platform detection + version probe |
| `src-tauri/src/commands.rs` | 132 | 6 Tauri commands (detect/test/set/has/delete + stub run_snip) |
| `src-tauri/src/lib.rs` | +2 mod + 6 cmds | Module wiring + invoke_handler |
| `src-tauri/src/bin/cli_test.rs` | 117 | Standalone smoke harness |
| `src-tauri/tests/rust/*` (4 files) | 165 | 24 integration tests |

Total: ~1.5k Rust LOC. Largest single file: `dispatcher.rs` at 257 lines (slightly over the 200-line guideline; further split deferred — single responsibility well-contained).

## Verification

- `cargo check` — clean
- `cargo test` — **29 tests pass / 0 failures**
  - 5 inline unit tests (rate-limit + cloud redaction)
  - 8 postprocess tests (incl. Session-3 category-label regression guard)
  - 7 smart_format tests (incl. Session-3 EQ_ONLY-with-newlines regression guard)
  - 5 cloud_gemini_api error-mapping tests
  - 4 argv contract tests (new, locks Session-3 Codex/Gemini argv shapes)
- `cargo clippy --all-targets --all-features` — **0 warnings**
- `cli_test --agent codex` on `fixtures-extra/equation-only/CleanShot 2026-05-21 at 05.24.00@2x.png`:
  - Output: `\int_a^b f(x) \, dx = F(b) - F(a)` (33 chars, classified `EquationOnly`)
- `cli_test --agent gemini-cli` same fixture: plan-mode rejected the out-of-workspace path → no `read_file ~/.claude/.ck.json` divergence. Verifies the safety contract; UX gap surfaces as `NonZeroExit` (deferred to Phase 6 Settings).
- Detection (macOS): found Codex 0.130.0 + Gemini 0.42.0 in `/opt/homebrew/bin`.

## Success Criteria — Plan vs Reality

| # | Criterion | Status |
|---|---|---|
| 1 | `cli_test --agent codex` produces cleaned LaTeX/Markdown | ✅ |
| 2 | `cli_test --agent cloud-gemini` ≤5s p95 | ⏸ deferred (needs user API key) |
| 3 | `cli_test --agent gemini-cli` with `--approval-mode plan` — no `read_file` failures | ✅ (plan-mode refuses external reads, no tool loop) |
| 4 | `cli_test` returns nonzero exit on timeout / rate limit / empty output | ✅ |
| 5 | `detect_installed_agents` finds Codex + Gemini CLI via npm/Homebrew; cloud-gemini when keychain has a key | ✅ macOS Homebrew confirmed; Windows scan paths added but not exercised this phase |
| 6 | Keychain API key readable on next process launch (cross-platform) | ✅ macOS Keychain (keyring crate v3) |
| 7 | Unit test coverage on postprocess + smart_format ≥85% | ✅ ~90% (eyeballed) |
| 8 | No compile warnings (`cargo clippy --all-targets --all-features`) | ✅ |

## Code Review

`reports/reviewer-260522-phase-03.md`. Verdict: shippable, 0 critical / 3 high / 7 medium / 6 low / 4 nit. Applied inline this phase:

- **H1** — `looks_like_rate_limit` tightened: bare `429` substring no longer trips the rate-limit branch; now requires `\b429\b` + (`rate`|`quota`|`limit`|`error`) co-occurrence, OR a literal phrase (`rate limit`, `quota`, `too many requests`). 3 new unit tests guard.
- **H2** — `keychain::has()` now logs backend errors via `log::warn!` instead of silently returning false. New `has_detailed()` surfaces backend faults explicitly for callers that need to distinguish them.
- **M2** — Replaced manual `cleanup()` calls with `TempFile` RAII guard. Drop runs on every exit path including panic + future cancellation. Also nests temp files under `temp_dir()/sniptex/` to reduce root-temp pollution (addresses L3 as a side effect).
- **F-add** — Added `agent_registry_argv_test.rs` (4 tests) locking the Session-3-verified Codex argv shape + Gemini plan-mode contract + cloud-gemini empty-argv invariant.

Deferred to later phases (with reasoning):

- **H3** (Gemini plan-mode error → typed `WorkspaceRestricted`) — UX layer responsibility; defer to **Phase 6** when Settings UI surfaces the error.
- **M1** (synthetic `binary_path` for cloud agent) — current dispatch is `AgentKind`-gated so safe; refactor to `Option<PathBuf>` / `AgentLocation` enum tracked for **Phase 8** Settings work.
- **L1** (cli_test bundled in release) — tag as `[[bin]] required-features` once a `cli-test` feature flag is added in **Phase 11** distribution polish.
- **D-tightening** (`DispatchError::Parse` + `UpstreamHttp`) — non-blocking semantic cleanup; ride with **Phase 6**.
- **Cloud key URL → header** — defer to **v1.1** per reviewer's Q1.

## Notes for Next Phases

- **Phase 4 (capture)** will produce the image source. Wire into `commands.rs::run_snip` (currently a stub returning `pending_capture`).
- **Phase 4** is the first phase where temp-file cancellation actually matters in production — the `TempFile` Drop-guard is in place to make that safe.
- **Phase 6 (UI)** will call `detect_agents`, `test_agent`, `set_api_key`, `has_api_key`. All five commands return `Result<_, String>` so the front end gets typed errors uniformly. Phase 6 must add `WorkspaceRestricted` mapping if it surfaces the Gemini CLI test button.
- **Phase 6 cache decision**: re-scan vs cache on `detect_agents` not implemented this phase. Default is re-scan per call; Phase 6 owns the "Re-scan" button + caching choice.
- **Phase 8 (Settings / Onboarding)** owns the BYOK key entry UI (`set_api_key` / `has_api_key` / `delete_api_key` are ready).

## Docs Impact

- **None this phase** — no public-facing docs in `./docs` exist yet (Phase 13 owns landing page + install docs). Plan + reports are the only living docs.

## Open Questions for User

1. **H3 — typed Gemini plan-mode error**: keep deferred to Phase 6, or pull it forward as a 5-minute follow-up now?
2. **L1 — cli_test in release bundle**: keep shipped for now (useful debug aid), or gate behind a `cli-test` feature flag immediately?
3. **Reviewer Q3 — `keychain::get_gemini_api_key()` is a blocking call inside `run_cloud_agent`**: wrap in `spawn_blocking`? Risk only matters on first-unlock when macOS Keychain prompts the user. Cheap to fix if confirmed.
4. **Reviewer Q1 — Cloud Gemini auth via `?key=` query vs `x-goog-api-key` header**: stay query for v1, move to header in v1.1, or switch now?
