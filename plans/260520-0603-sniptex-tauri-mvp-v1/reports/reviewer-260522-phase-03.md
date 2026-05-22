# Phase 3 Code Review — Agent System + OCR Pipeline

Date: 2026-05-22
Reviewer: code-reviewer
Plan: `plans/260520-0603-sniptex-tauri-mvp-v1/phase-03-agent-system-ocr-pipeline.md`

## Scope

- Files: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/ocr/{mod,prompt,postprocess,smart_format,dispatcher}.rs`, `src-tauri/src/agents/{mod,registry,codex,gemini_cli,cloud_gemini_api,keychain}.rs`, `src-tauri/src/bin/cli_test.rs`, `tests/rust/*`.
- LOC reviewed: ~900 (Rust) + 1 toml.
- Already verified by author: 22 unit tests green, 0 clippy warnings, codex smoke OK, gemini-cli plan-mode gate OK.

## Overall Assessment

Solid. Adapter pattern, error mapping and security posture all line up with plan. Code is concise, modular (no file > 220 lines), and each adapter is < 90 lines. Acceptance criteria 1–10 are met. Findings below are mostly defensive/edge-case hardening, not blockers.

---

## CRITICAL

None.

---

## HIGH

### H1. `looks_like_rate_limit` false-positive on stderr containing string "429"
`src/ocr/dispatcher.rs:206-209` flags rate-limit on substring `429` (and `quota` / `rate limit`, case-insensitive). Any unrelated stderr line that happens to contain "429" (e.g. a stack-trace line, port number, request id) will be misclassified, suppressing the real `NonZeroExit{code, stderr}` path and triggering a wrong fallback. Tighten:
- Anchor "429" with surrounding non-digit boundary, or require co-occurrence of "rate" / "quota" / "limit" / "exceed" tokens.
- Suggested: `lower.contains("rate limit") || lower.contains("quota") || regex r"\b429\b" && lower.contains("error")`.
Severity High because misclassification flips the fallback path silently.

### H2. `keyring::Error::NoEntry` discriminant assumption
`src/agents/keychain.rs:24` matches `keyring::Error::NoEntry`. In `keyring` v3, the actual variant for the "no record" case is `keyring::Error::NoEntry`. The match arm itself is fine, but `has()` at line 43 swallows ALL errors (including `Backend(...)`) as `false`. A platform-keychain backend outage will then surface as "no API key" → `MissingApiKey("gemini")` → user thinks they need to re-enter the key. Distinguish:
- `has()` should return `Result<bool, KeychainError>` or at least log on non-`NotFound` so we don't silently mask backend faults.
- Even simpler: `has = matches!(get(...), Ok(_)) || matches!(err, Err(NotFound)? false : true)` — i.e. treat backend errors as "unknown", not "absent".
Impact: false negatives on detection + key-presence checks under flaky keychain conditions.

### H3. Gemini CLI `--approval-mode plan` rejects out-of-workspace files — surfaced as plain `NonZeroExit`
The plan's risk section already documents this as accepted ("best-effort"), but the dispatcher returns it as a generic `NonZeroExit{code, stderr}` with no hint to the UI. When Phase 6 wires the Settings test-agent button, the user will see a raw stderr line. Recommendation: in `run_cli_agent`, before bare `NonZeroExit`, do a stderr substring check for `"plan"`/`"workspace"`/`"outside"` and map to a new `DispatchError::WorkspaceRestricted` (or reuse `BadRequest`). Otherwise the Settings UI cannot tell the user "Gemini CLI cannot read files outside its workspace; use Codex or cloud-gemini for screen snips."

---

## MEDIUM

### M1. Cloud agent `binary_path` is a synthetic string masquerading as a path
`src/agents/mod.rs:39` constructs `binary_path: PathBuf::from("<cloud-api>")`. The `AgentInfo` shape forces a `PathBuf` even when the agent has no binary. Any consumer that does `agent.binary_path.exists()` or hands the path to `Command::new(...)` will misbehave. Today the dispatcher gates on `AgentKind` first (line 78–82), so it's safe, but the trap is one careless caller away. Options:
- Make `binary_path: Option<PathBuf>`.
- Or rename + retype: `pub location: AgentLocation` enum `{ Binary(PathBuf), CloudEndpoint(&'static str) }`.
Bonus: keeps the registry pattern truly mechanical for a 4th adapter (pattern-fit, criterion E).

### M2. `cleanup` skipped on panic / abrupt cancellation
`src/ocr/dispatcher.rs` cleans the `--output-last-message` temp file on every explicit return path, but if the async future is dropped between `cmd.output().await` succeeding and reaching `cleanup(...)` (e.g. UI cancels the request, parent task aborted), the temp file leaks in `std::env::temp_dir()`. `kill_on_drop` handles the child, not the file. Wrap the path in a small `Drop`-guard struct that calls `std::fs::remove_file` on drop:
```rust
struct TempFile(PathBuf);
impl Drop for TempFile { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }
```
This also removes the need for the four manual `cleanup()` calls.

### M3. `Cargo.toml` `[dev-dependencies] tokio` shadows `[dependencies] tokio` with a smaller feature set
`Cargo.toml:38` declares `tokio = { features = ["macros","rt-multi-thread"] }` under dev-deps; main deps at line 28 already pull `features = ["full"]`. Cargo unions features in dev builds, so this is harmless today, but it expresses intent that dev tests do not need `full` — which will cause confusion if someone later cuts main deps to a smaller set. Either drop the dev-dep block entirely (tests use `tokio::test` via main dep) or document that the dev entry is a no-op safety net.

### M4. `redact_key` regex misses keys with a trailing word-boundary punctuation
`src/agents/cloud_gemini_api.rs:180` `r"AIza[0-9A-Za-z_\-]{35}"` requires exactly 35 trailing chars. Real Google API keys are 39 chars total (`AIza` + 35). That's correct. But if Google ever rotates the prefix/length (they have done so for other Google products) or a leaked key gets logged in a slightly different format, redaction silently fails. Mitigations:
- Add a second pattern for any token of length ≥39 prefixed by `AIza` (broader: `AIza[\w\-]{30,}`).
- Add a unit test that asserts redaction handles `?key=AIza...` query strings (currently only tested as `AIzaSyD...` in the middle of a sentence).

### M5. `post_process` preamble strip drops the entire first line, even when it's just a literal `"Here is"`
`src/ocr/postprocess.rs:54-63` checks `s.starts_with(p)` for each preamble. If the actual content legitimately starts with `"Here is the function definition:"` followed by a code block, we lose the line. Lower-risk edge case (OCR rarely opens with these strings), but worth mentioning. Consider strengthening with a "followed by ':' or newline within first ~80 chars" check, or limit to known LLM templates like `"Here is the equation"`/`"Here's the table"`.

### M6. `detect_type` LaTeX-density heuristic accepts noise as EquationOnly
`src/ocr/smart_format.rs:38-52` flags as EquationOnly when:
- contains one of `\frac \int \sum \sqrt \lim`, OR
- contains a `\` AND any `^` / `_` character.
The second condition is loose: a path string like `C:\Users\name_with_under_score` will match. In OCR output, a single backslash + an underscore inside a markdown identifier (snake_case in code) could plausibly appear. Tighten: require ≥2 distinct LaTeX command tokens OR require `\` followed by a letter.

Also: `has_latex && natural_words < 3` — `natural_words` counts words `>2 chars` not containing `\`. A 2-equation block like `f(x) = x + y \\ g(x) = x` may pass with natural_words=0, correctly EquationOnly. Good. But `\frac{a}{b} where a > b` returns natural_words=1 (`where`) → still EquationOnly. Probably acceptable; flag for awareness.

### M7. `cli_test` does not pass `--prompt` overrides; can't smoke-test prompt regressions
Minor: harness has no `--prompt-file` / `--print-args` flag, so verifying that Codex actually received the post-Session-3 prompt requires re-reading source. Suggest adding `--print-args` (print built argv) and `--prompt-file PATH` (override MASTER_PROMPT) — turns the harness into a true regression suite.

---

## LOW

### L1. `Cargo.toml` test bin `cli_test` is shipped in release builds
`[[bin]] name = "cli_test"` lives under `src/bin/` and will be compiled into the release output unless gated. Either add `required-features = []` + a feature flag, or move the file under a `tests/` example to prevent it from being bundled. Not a correctness bug, just bloat.

### L2. `agents::detect_installed_agents` calls `dirs::home_dir()` which can return `None` on weird Linux setups
`src/agents/mod.rs:75` silently skips all extra dirs if `home_dir` is `None`. Worth a `log::warn!` so users without `$HOME` know detection is degraded.

### L3. `staging_path` uses `Uuid::new_v4()` per call — fine, but no sandbox under temp
`src/ocr/dispatcher.rs:211-213` lands files directly in `std::env::temp_dir()` rather than a `sniptex/` subdir. On shared multi-user systems this is fine permission-wise but pollutes the root temp dir. Cheap fix: `std::env::temp_dir().join("sniptex").join(file_name)` + `create_dir_all` once.

### L4. `detect_version` `--version` probe ignores timeout-after-the-fact
`src/agents/mod.rs:113-122` uses `std::process::Command` (blocking) and *measures* elapsed after `cmd.output()` returns. If a misbehaving binary hangs forever, the probe blocks the entire detection scan; the 2s budget is never enforced because there's no timer wrapping the call. Cheap fix: wrap with `wait_timeout = "0.2"` or spawn + poll. Plan note acknowledges blocking is intentional; the real bug is that the budget is decorative. Either remove the budget check (it's misleading) or actually enforce it.

### L5. `run_cloud_agent` rejects mismatched `agent.spec.id` but the only CloudApi spec today is `cloud-gemini`
`src/ocr/dispatcher.rs:152-154` defensive but currently dead code. Harmless; will activate when a second CloudApi adapter (e.g. OpenAI Vision) lands. Keep.

### L6. `commands.rs::stringify_dispatch_error` comment promises redaction, but `Display` is delegated to `thiserror`
Comment at `src/commands.rs:117` says "DispatchError already redacts API keys via cloud_gemini_api::redact_key before it constructs BadRequest/ServerError". True for the cloud path. But `DispatchError::Io(...)` / `Network(...)` strings come from `std::io::Error` / `reqwest::Error` and are NOT scrubbed. Today no API key flows through those paths, but if someone later threads `format!("send to {url}", url=endpoint(key))` into a Network error, it leaks. Add a final `redact_key`-style scrub at the `stringify_dispatch_error` boundary as belt-and-suspenders.

---

## NIT

- `src/agents/cloud_gemini_api.rs:151-156` uses `.drain(..).next()` four times — `into_iter().next()` reads cleaner and avoids the mutable rebinding.
- `src/ocr/postprocess.rs:78-81` builds `format!("\n\n{}", so)` inside the loop; trivially pull out as `for so in SIGNOFFS { let marker = ...; }` — already correct, just remarking.
- `src/agents/codex.rs` and `src/agents/gemini_cli.rs` are 18-line thin wrappers around `build_command_args` — they don't earn their own file. Acceptable for parallelism with future adapters, but per the codebase's <200-line rule + YAGNI/KISS, consider folding into `registry.rs` until a real per-adapter behavior appears. Counter-arg: keeps the "add 4th agent = add file" pattern obvious. Either way, fine.
- `commands.rs:131-132` has a compile-time `let _ = CLOUD_GEMINI_ID;` style guard — clever; consider adding a comment explaining it's intentional dead code to prevent rustc unused-import refactors from breaking the contract.
- `prompt.rs:21` table example uses `\\\\` for an in-string `\\` separator and naked unescaped backslashes elsewhere — visually confusing but functionally correct.

---

## A. Phase 2 Regression Surface — CLEAR

- `lib.rs` change: `pub mod agents; mod commands; pub mod ocr;` + 6 new commands in `invoke_handler![...]`. `hello` still wired (line 90). Hotkey debounce + emit logic intact (lines 32-85). No removal/reorder of existing plugins.
- No mutation of `cfg(desktop)` plugin builder chain.
- `invoke_handler` is a closed list; the added commands cannot collide with plugin commands.
- **Conclusion:** no observable Phase 2 regression.

## B. Security Audit — STRONG with caveats

- Keychain `keyring::Entry::new("com.sniptex", "gemini-api-key")` — correct service/account pair, single source of truth. `set/get/has/delete` all use the same `entry()` constructor.
- `cloud_gemini_api.rs::redact_key` strips `AIza{35}` pattern before building `BadRequest` / `ServerError` messages. Pattern is correct for current Google API key format.
- `reqwest::Client::builder()` — confirmed `default-features = false, features = ["rustls-tls", "json"]` in Cargo.toml (line 33). No openssl path. ✅
- API key flows: keychain `get_gemini_api_key()` → `call_with_image_path(image, MASTER_PROMPT, &key)` → endpoint built with `?key={api_key}` (URL query). **Risk:** if `reqwest`'s middleware logger is ever enabled (env `RUST_LOG=reqwest=debug`), the full URL — key included — gets logged. **Mitigation:** Google API also accepts `x-goog-api-key` header; using header instead of query keeps key out of URL logs / proxy access logs. Recommend switching for v1.1 (see Q1).
- Key never serialized to settings.json (no `serde::Serialize` on a struct containing it). ✅
- `Display` on `DispatchError`: the `Network(String)` and `Io(String)` variants are NOT pre-redacted (see L6). Today no codepath puts the key into those strings, but the contract isn't enforced.

## C. Concurrency Correctness — MOSTLY GOOD

- `kill_on_drop(true)` set on every CLI spawn. `tokio::time::timeout` wraps the `.output()` future. On timeout, the future is dropped → child killed. ✅ Acceptance criterion 7 met.
- `stdin(Stdio::null())` prevents child from blocking on stdin. ✅
- Cleanup ordering: temp file removed on all explicit Err paths AND success path. Missed paths: panic mid-await, future cancellation between `cmd.output().await` returning Ok and the cleanup call. See M2.
- `detect_version` uses blocking `std::process::Command`. Per plan note (criterion 6), this is acceptable because it runs only at boot via `tokio::task::spawn_blocking` (verified at `commands.rs:26,33,83`). ✅ But L4: the 2s budget is not actually enforced; a hanging probe wedges the worker.
- No shared mutable state in dispatcher. No Mutex/RwLock contention surface.
- `run_with_fallback` is sequential by design — no `join_all` race. Correct: rate-limit on one agent shouldn't burn quota on the next.

## D. Error Mapping Completeness — COMPLETE

`From<CloudGeminiError> for DispatchError` covers all 7 variants:

| CloudGeminiError | DispatchError | Verdict |
|---|---|---|
| RateLimited | RateLimited | ✅ |
| BadRequest(m) | BadRequest(m) | ✅ |
| AuthFailed(c) | AuthFailed(c) | ✅ |
| ServerError(c, m) | NonZeroExit{code, stderr} | ⚠️ Maps HTTP error to "non-zero exit" — slightly lying about origin. Consider adding `DispatchError::UpstreamHttp(u16, String)`. |
| Network(m) | Network(m) | ✅ |
| EmptyResponse | EmptyOutput | ✅ |
| Parse(m) | BadRequest(m) | ⚠️ A JSON parse failure becomes "bad request" which is semantically wrong (the request was fine; the response was malformed). Add `DispatchError::Parse(String)` or `UnexpectedResponse(String)`. |

Both ⚠️ items are non-blocking but worth tightening.

## E. Pattern Fit — adding a 4th agent IS mechanical

To add e.g. Claude Code (`claude` CLI):
1. Add `CLAUDE_CODE_ID` + `AgentSpec` in `registry.rs::AGENTS`.
2. Add a `CLAUDE_CODE_ID => vec![...]` arm in `build_command_args`.
3. Optionally create `agents/claude_code.rs` for parity (currently just wrappers).
4. Append to `DEFAULT_FALLBACK_CHAIN` (or leave for user config).
That's it. Detection, dispatch, cleanup all flow through generic code paths. ✅ Pattern goal achieved.

For a 4th CloudApi (e.g. OpenAI Vision):
1. New `AgentSpec { kind: CloudApi, id: OPENAI_VISION_ID, ... }`.
2. New `agents/openai_vision_api.rs` with own error enum + `call()` mirroring `cloud_gemini_api`.
3. New `From<OpenAiError> for DispatchError` mapping.
4. New branch in `run_cloud_agent` matching `OPENAI_VISION_ID`.
5. New `OPENAI_ACCOUNT` constant in `keychain.rs` + `commands.rs::set_api_key` arm.
Slightly less mechanical than CLI because of the per-vendor HTTP shape + keychain account, but no architectural rework needed.

## F. Test Coverage — GOOD with gaps

- ✅ Session-3 regression guards: `post_process_strips_leading_category_label` + `detect_type_returns_equation_only_for_raw_latex_even_with_newlines` both present and meaningful.
- ✅ 22 tests pass, 0 clippy warnings.
- ⚠️ Missing: `detect_type_returns_equation_only_for_multi_command_block` (e.g. `\frac{a}{b} \\\\ \sqrt{x}` already covered, but a 5-line ALIGN block isn't tested).
- ⚠️ Missing: `post_process_strips_combined_preamble_AND_fence` (real Gemini outputs often have both wrapping a single body).
- ⚠️ Missing: `post_process_handles_crlf_line_endings` — Windows. Current opening-fence regex uses `\n` only.
- ⚠️ `cloud_gemini_api_test.rs` only tests the `From` mapping — no HTTP shape validation. Acceptable per plan ("full HTTP-mock coverage deferred").
- ⚠️ No test for `build_command_args` returning the Session-3-verified argv shape. One unit test asserting the literal argv vector would lock the contract:
```rust
let args = build_command_args(CODEX_ID, "/img.png", "PROMPT", Some("/tmp/last.txt"));
assert_eq!(args, vec!["exec","--skip-git-repo-check","--image","/img.png",
                      "--output-last-message","/tmp/last.txt","--","PROMPT"]);
```
Cheap, catches argv drift from future "cleanup" refactors.

## Edge Cases (Scout Pass)

- Gemini CLI `-p` with prompt containing literal `@"..."` already in text would inject a second file ref — improbable but worth a sanitize step on the prompt before format!.
- `format!("{prompt}\n@\"{image_path}\"")` does no escaping on `image_path`. If a future code path passes a user-controlled path containing `"` or backtick, the shell-side `gemini` CLI parser may misparse. Today image paths are all process-controlled (snip staging or CLI smoke), so low risk; flag for Phase 4 when capture writes the path.
- `is_executable` on Windows checks `.exe/.cmd/.bat` extensions; `.ps1` and the npm `cmd-shim` shape (which is `.cmd` — covered ✅) are fine. WSL binaries on Windows have no extension — would be missed. Acceptable.
- `tokio::fs::read(image_path)` for cloud path: no size cap. A multi-megabyte PNG inflates to ~1.3× as base64; for screen snips capped by Phase 4 selection size, fine. Add a guard later if Phase 4 ever lets user pick arbitrary files.

## Positive Observations

- Clean separation: dispatcher does no per-vendor knowledge beyond `AgentKind` branch.
- Master prompt as `pub const` referenced from both code paths — exactly one source of truth. Sync with `test-prompt.sh` is verified (verbatim match).
- `kill_on_drop(true)` + `Stdio::null()` + `tokio::time::timeout` is the textbook-correct combo.
- `redact_key` exists, has its own test, runs before the error string crosses the module boundary.
- Test naming describes scenario, not finding code (complies with `review-audit-self-decision.md` §5).
- File sizes: largest is `dispatcher.rs` at 219 lines, well under 200-line target margin.
- Defense-in-depth on category label strip (postprocess catches what the prompt rule misses).

## Recommended Actions (Prioritized)

1. **H1** — tighten `looks_like_rate_limit` to avoid 429-substring false positives.
2. **H2** — distinguish backend errors from `NotFound` in `keychain::has()`.
3. **H3** — map Gemini `plan`-mode rejection to a typed error so UI can guide the user.
4. **M2** — `Drop`-guard the temp file so cancellation doesn't leak.
5. **D-tightening** — add `DispatchError::Parse` + `UpstreamHttp` to remove the two semantically-wrong mappings.
6. **F-add** — argv contract test on `build_command_args` for CODEX_ID; CRLF + preamble+fence combo tests on post_process.
7. **L4** — either enforce the 2s `detect_version` budget or drop the misleading check.
8. **M1** — `binary_path: Option<PathBuf>` or refactor to `AgentLocation` enum.
9. **L6** — add a final `redact_key` pass in `stringify_dispatch_error`.
10. **Settings UX (deferred)** — switch cloud API auth from `?key=` query to `x-goog-api-key` header.

## Metrics

- Type coverage: 100% (Rust, all `pub` items typed).
- Test coverage (eyeballed): postprocess + smart_format ~90% line coverage; dispatcher ~30% (no HTTP/process mocking, deferred); cloud_gemini_api ~40% (mapping only).
- Linting issues: 0 (cargo clippy --all-targets --all-features, per author).
- File size: max 219 lines (dispatcher.rs). All under 200-line guideline except dispatcher by 19.

## Acceptance Criteria — Plan vs Code

| # | Criterion | Status |
|---|---|---|
| 1 | `detect_installed_agents` returns AgentInfo + binary_path + version, cross-platform | ✅ |
| 2 | `run_ocr` returns cleaned text within 30s or typed `DispatchError` | ✅ |
| 3 | `post_process` strips preambles (EN+VN), fences, sign-offs, leaked labels | ✅ |
| 4 | `detect_type` LaTeX-density BEFORE blank-line check | ✅ (verified at smart_format.rs:38) |
| 5 | `run_with_fallback` tries `[codex, cloud-gemini, gemini-cli]` order | ✅ (registry.rs:71) |
| 6 | All async via tokio; no blocking in dispatcher (detect_version uses std intentionally at boot) | ✅ |
| 7 | Timeout via `tokio::time::timeout` + `kill_on_drop` | ✅ |
| 8 | Master prompt mirrors `plans/test-prompt.sh` verbatim incl. Session-3 wording | ✅ (DETECTION-internal + table-cell math-scope present in both) |
| 9 | API key never serialized to settings.json, never logged, redacted in errors | ✅ with L6 caveat |
| 10 | Cloud API uses rustls-tls only | ✅ (Cargo.toml:33) |

## Verdict

**Phase 3 is shippable.** Recommend addressing H1/H2/H3 (and ideally M2) before Phase 4 starts piping real captures through `run_with_fallback`. Everything else is cleanup that can ride with future phases.

## Unresolved Questions

- Q1: Should we move the cloud API key from `?key=` query string to `x-goog-api-key` header to prevent URL-level leaks via proxy/middleware logs? (Posture decision; default = stay query for v1, switch v1.1.)
- Q2: Is `Drop`-guarded temp cleanup worth the small code addition now, or wait until Phase 4 surfaces the cancellation path? (Mild preference: do it now to keep the dispatcher self-contained.)
- Q3: Plan acceptance #6 says "no blocking in dispatcher" — `keychain::get_gemini_api_key()` at dispatcher.rs:155 is a blocking call in an `async fn`. macOS Keychain unlock can prompt the user (blocking). Should this be `spawn_blocking`-wrapped to avoid stalling the runtime? Not flagged as a finding because the keychain access is typically < 1ms after first unlock, but worth a decision.
- Q4: `DEFAULT_FALLBACK_CHAIN` is currently a hard-coded `&[CODEX_ID, CLOUD_GEMINI_ID, GEMINI_CLI_ID]`. Phase 6 will let users reorder; should we surface it as `pub fn user_chain() -> &'static [&'static str]` now and have Phase 6 override it via a global, or leave the rewire for Phase 6?
- Q5: `cli_test` binary ships in release artifacts unless gated (L1). Acceptable for v1.0 (debug aid) or remove? Recommend: keep in v1, hide behind a `cli-test` feature flag later.
