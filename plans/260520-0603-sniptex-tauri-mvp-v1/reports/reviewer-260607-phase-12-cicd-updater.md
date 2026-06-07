# Code Review: Phase 12 — CI/CD Release Workflow & Auto-Updater

**Reviewer:** code-reviewer
**Date:** 2026-06-07
**Scope:** 8 files listed in user prompt. No scope expansion.
**Verdict:** **DONE_WITH_CONCERNS** — score **6.5 / 10**
Two Critical findings (macos-13 retired, `active: false` is a no-op) block a clean tag push as-is. Everything else is straightforward.

---

## CRITICAL

### C1. `macos-13` runner is RETIRED — Intel matrix entry will fail
**File:** `.github/workflows/release.yml:20`
**Finding:** `macos-13` was deprecated 2025-09-22 and fully unsupported by 2025-12-08 per GitHub Actions schedule ([changelog](https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/)). We are 6+ months past EoL. Jobs queued against `macos-13` will fail to start.
**Walkthrough of user's red-team angle #4:** The "should we add a comment?" question understates the severity — this is not a soon-to-deprecate warning, it is a runner that no longer exists. The Intel build will never run.
**Recommendation:** Either
  - (a) **drop Intel build** (drop the matrix entry, document Apple Silicon-only in `install-guide.md` + Cask `depends_on arch: :arm64`), or
  - (b) **swap to `macos-15-intel`** (the only Intel-native runner left; per GitHub, the last Intel runner, retiring Fall 2027), or
  - (c) **cross-compile on `macos-14`** with `rustup target add x86_64-apple-darwin` + tauri-action `args: --target x86_64-apple-darwin` (universal2 via Tauri's bundler).
  Option (b) is the smallest diff and matches the user's "Mac Intel runner: separate matrix entry" intent. `docs/releasing.md:139` already lists the symptom in the troubleshooting table — promote it to a code change.

### C2. `plugins.updater.active = false` is a NO-OP in Tauri v2 — updater is fully active
**File:** `src-tauri/tauri.conf.json:141`
**Finding:** Tauri v1 had a `tauri.updater.active` flag. **Tauri v2's `plugins.updater` config dropped it.** The v2 [`Config` struct](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/config.rs) only declares: `dangerous_insecure_transport_protocol`, `dangerous_accept_invalid_certs`, `dangerous_accept_invalid_hostnames`, `endpoints`, `pubkey`, `windows`. The struct uses a manual `Deserialize` impl with **no `deny_unknown_fields`** — so `active: false` is silently dropped at config parse time. The plugin is registered at `src-tauri/src/lib.rs:66` unconditionally and `updater:default` is granted in `src-tauri/capabilities/default.json:30`.
**Walkthrough of red-team angle #1:** "What does `check()` do?" — it calls the configured endpoint `https://github.com/hiep1987/sniptex/releases/latest/download/latest.json`. Until a release tag is published, GitHub will return a 404 (or redirect to a 404). `check()` will resolve to either `null` (if it tolerates 404 gracefully) or **throw** (network/parse error). The UpdateDialog hook returns `kind: "error"` in the throw case, surfacing **"Update check unavailable"** to the user — incorrect copy: it implies a config problem, when actually the feature is simply not yet released.
**Impact:** Not a security hole (the pubkey check still gates installs), but the user's stated mental model — "disabled until activated" — is wrong. The plugin is on, hits the network on every click, and produces a misleading error.
**Recommendation:** Pick one:
  - **(a) Truly disable** — remove the updater plugin registration behind a Cargo feature (`#[cfg(feature = "updater-active")]` around lib.rs:66 + capabilities entry), default off. Activation = enable the feature in Cargo, not edit JSON.
  - **(b) Accept the no-op + fix the copy + delete the `active` field** so it doesn't mislead future maintainers. Change `about-tab.tsx:21` toast to "No update available yet" for the error branch when the endpoint 404s, OR detect 404 explicitly and surface that.
  - **(c) Bake activation into the tag** — gate the JS `runCheck()` call on a build-time flag (Vite env var `VITE_UPDATER_ACTIVE`). Tag pushes that bake the flag get a working updater; otherwise the button shows "Updates managed via Releases page".
  Also remove the `active: false` line from `tauri.conf.json` and update `docs/releasing.md:25-29` so the rotation guide doesn't tell users to flip a flag that does nothing.

---

## HIGH

### H1. `gh release download` on a DRAFT release — undocumented but works only with elevated token scope
**File:** `.github/workflows/release.yml:80`
**Finding:** The `checksums` job depends on `build`, which creates a **draft** release. `gh release download "${{ github.ref_name }}"` against a draft works on a workflow with `contents: write` (which the workflow has at line 8), but the gh CLI has known quirks around drafts ([cli/cli#9076](https://github.com/cli/cli/issues/9076), [cli/cli#5252](https://github.com/cli/cli/issues/5252)) — `gh release list` doesn't show drafts in Actions, though `download` by tag name does work since drafts are tagged. The `--skip-existing` flag helps if the checksums file is later re-uploaded.
**Recommendation:** Add a smoke check before the download: `gh release view "${{ github.ref_name }}" --json isDraft,assets >/dev/null` to fail loudly with a clear message if the draft isn't visible to this token. Optional: pin `gh` version with `actions/setup-gh` to avoid CLI drift.

### H2. `tauri-action@v0` is a moving tag — pin to a release SHA for reproducibility
**File:** `.github/workflows/release.yml:51`
**Finding:** `@v0` floats over the entire v0.x line. A future breaking change (e.g., `includeUpdaterJson` rename, `tagName` semantics) will silently break release tags. Industry standard for release-critical actions is SHA pinning.
**Recommendation:** Replace with `tauri-apps/tauri-action@<full-sha>` and add a Dependabot config to bump it intentionally. At minimum pin to a specific `@v0.5.x` release.

### H3. Updater plugin order in `lib.rs` — registered before `single-instance`
**File:** `src-tauri/src/lib.rs:66` (already-existing, but newly relevant once H1/C2 resolved)
**Finding:** Per Tauri docs, `single-instance` plugin should be the **first** plugin registered (it intercepts the second-instance handoff). The updater plugin runs after sql/store/dialog/etc. — fine — but verify `single-instance` is first if it's registered at all. (Cargo.toml:150 shows the crate is present; you should check registration ordering.)
**Recommendation:** Out-of-scope for this phase but flag for follow-up. Not a regression introduced by Phase 12.

### H4. `pnpm build` already runs `tsc` — `typecheck` step is redundant
**File:** `.github/workflows/ci.yml:32-36`
**Finding:** `package.json:10` defines `"build": "tsc && vite build"`. The `typecheck` step at line 33 runs `pnpm tsc --noEmit`; the next step runs the same `tsc` (with emit) inside `pnpm build`. Double-typecheck wastes ~10s. Not wrong, but YAGNI.
**Recommendation:** Drop the `typecheck` step; rely on `pnpm build` to catch type errors. OR keep typecheck and change build to `vite build` only (skip the inner tsc). I'd keep the explicit `tsc --noEmit` — it gives a cleaner failure signal in PR checks — and drop the inner tsc from `build` (rename to `vite build`). Pure cleanup; either is fine.

---

## MEDIUM

### M1. UpdateDialog: no Escape-key handler, no focus trap
**File:** `src/components/update-dialog.tsx:40-48`
**Finding:** `aria-modal="true"` is set but no Escape handler and no focus management. Backdrop click only closes during `phase === "idle"` — once download starts, user can only wait (no cancel). Clipboard / hotkey handlers in main window may still fire because focus isn't trapped.
**Recommendation:**
  - Add a `useEffect` on the dialog: `keydown` listener that calls `onDismiss()` on Escape if `phase === "idle"`.
  - Use `<dialog>` element or a focus-trap library (radix-dialog / @react-aria/overlays already partial in shadcn ecosystem) for proper a11y.
  - Add a "Cancel download" affordance when phase=downloading (Tauri 2 updater's `downloadAndInstall` callback can be aborted via AbortController if you split it into `download` + `install`).

### M2. UpdateDialog: copy is wrong about restart
**File:** `src/components/update-dialog.tsx:80`
**Finding:** "Quit and reopen SnipTeX to use the new version." — Tauri v2's updater on macOS replaces the `.app` in-place and **the running app needs to relaunch**; on Windows the MSI installer relaunches itself. The current text instructs manual restart on both. User noted `@tauri-apps/plugin-process` not installed; that's the gap.
**Recommendation:** Two paths:
  - **Cheap:** install `@tauri-apps/plugin-process`, call `relaunch()` after `downloadAndInstall` succeeds.
  - **Cheapest:** call `getCurrentWindow().close()` plus tray quit via `tauri.exit()` from `@tauri-apps/api/app`. You already have `tauri.openExternal` plumbing — adding an exit IPC handler is one line.
  Either way, this is UX polish, not blocking.

### M3. Concurrency expression doesn't actually do what comment says
**File:** `.github/workflows/ci.yml:9-11`
**Finding:** The expression `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` correctly evaluates to `true` for PRs and `false` for pushes. Walkthrough confirms user's claim — push-to-main is NOT cancellable. **Logic is correct.** No change needed; informational only — moving it to MEDIUM because the user explicitly asked for a walkthrough.

### M4. Updater hook state-vs-result split: `update` state may lag the tagged result
**File:** `src/components/update-dialog.tsx:128-148` + `src/windows/settings/about-tab.tsx:15-23`
**Finding:** `runCheck()` returns the tagged union (correct). But the dialog renders from `update` state (line 62 in about-tab). After `runCheck()` resolves with `kind: "available"`, React batches the `setUpdate(result)` call — the dialog renders on the next tick. The component never reads `update` to **branch**, only to render. Stale-closure-on-error is avoided ✅, but there is still a tiny gap: between the `await runCheck()` and the re-render, if the user clicks the button again, `runCheck()` runs twice in parallel — both will call `setUpdate`, the second overwrites the first.
**Recommendation:** Either:
  - Guard with `if (checking) return;` in `onCheckClick` (`about-tab.tsx:15`).
  - Or use a ref-based mutex inside `runCheck`.
  Low impact; only matters under double-click.

### M5. Cargo `autobins = false` may surprise test discovery
**File:** `src-tauri/Cargo.toml:9`
**Finding:** `autobins = false` disables auto-discovery of `src/bin/*.rs` ONLY for binaries; **`[[test]]` entries are explicitly declared at lines 105-143**, so `cargo test` discovery is fine. User's verification (`cargo test --no-run` — all 13 test binaries compile) confirms this. ✅ No issue, just documenting the analysis.

### M6. `clippy --workspace --all-targets` with no `-D warnings` — advisory, fine; but `--all-targets` includes the dev-bins
**File:** `.github/workflows/ci.yml:74`
**Finding:** `cargo clippy --workspace --all-targets` clippy-checks the lib + main bin + tests. **It does NOT include the dev-bins** because `required-features = ["dev-bins"]` excludes them when the feature flag is off. ✅ Correct. Informational.
**However:** when you eventually need to lint the dev-bins, you'll need a second clippy step `--features dev-bins`. Add to phase-12 follow-ups if dev-bins should be lint-clean.

---

## LOW

### L1. `docs/releasing.md` step 4 lists `latest.json` as expected artifact, but `tauri-action` only emits it with `includeUpdaterJson: true`
**File:** `docs/releasing.md:86`
**Finding:** Correct as written (workflow does set the flag at release.yml:65), but if a future maintainer flips it, the doc is misleading. Already covered by troubleshooting row line 141. ✅ Fine.

### L2. `generate-checksums.sh:46` doesn't include `*.sig` files
**File:** `scripts/generate-checksums.sh:46`
**Finding:** The script hashes `.dmg / .msi / .app.tar.gz / .msi.zip / .nsis.zip` but skips `.sig` files. Tauri updater `.sig` files are tiny base64-encoded sigs — debatable whether to hash them. Most projects include them in `checksums.txt` for completeness.
**Recommendation:** Add `*.sig` to the glob if you want users to verify the sig file's integrity end-to-end. Low priority.

### L3. `generate-checksums.sh` uses `cd "$DIR"` but writes `$OUT = $DIR/checksums.txt` (absolute) — works, slightly confusing
**File:** `scripts/generate-checksums.sh:38,45`
**Finding:** Style nit. After `cd "$DIR"`, all glob/output paths are relative; `OUT` still uses `$DIR/checksums.txt`. Works because the trap also uses absolute path. Consider redefining `OUT="checksums.txt"` after the `cd` for consistency.

### L4. `about-tab.tsx` text "Bring your own agent or API key" — Phase 6 onboarding already says this, minor duplication
**File:** `src/windows/settings/about-tab.tsx:37`
Informational; not Phase 12 scope.

### L5. `release.yml` releaseBody hardcodes "Auto-generated build" — maintainer probably wants to edit before publishing draft (which is the point)
**File:** `.github/workflows/release.yml:60`
**Finding:** OK as the user manually edits the draft before publishing. Already covered in `docs/releasing.md` step 4. ✅ Fine.

---

## INFO

### I1. Bundle size estimate
- Current DMG (Phase 11, dev-bins inside): **19 MB** (`src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/SnipTeX_0.1.0_aarch64.dmg` = 20,059,149 bytes).
- New release bin (no dev-bins): `sniptex` = **23 MB stripped** uncompressed. The Phase 11 build also included 6 dev bins inside the .app; with feature-gating off, those binaries are not produced. **Expected DMG size: ~15-16 MB** depending on compression of remaining resources. A ~3-4 MB shrink is consistent with what feature-gating ~5 trivial smoke binaries would save (most weight is in shared deps already in the main bin).

### I2. Cask hash interpretation
**File:** `Casks/sniptex.rb:3`
- Cask currently SHA256s the **OLD Phase 11 local DMG (with dev-bins)**, NOT the CI-produced DMG (which doesn't exist yet).
- User's stated interpretation is correct: **the Cask hash will need a one-time update when v0.1.0 is cut via CI**, because the CI rebuild will produce a different SHA256.
- Recommendation: when you push the v0.1.0 tag, after the draft release is finalized, update `Casks/sniptex.rb:3` to the SHA256 of the **CI-built** `SnipTeX_0.1.0_aarch64.dmg`. Do not publish the Cask to a tap until then.

### I3. `nicekid1` URL audit — clean
- Grep for `nicekid1` across .rs / .tsx / .ts / .json / .md / .yml / .toml / .rb: **zero matches**.
- All `hiep1987` URLs in README, install-guide, releasing.md, tauri.conf.json, about-tab.tsx are consistent. ✅ No regression.
- Note: `nicekid1` was almost certainly a placeholder/typo from earlier scaffolding — no evidence of a real upstream fork to preserve attribution.

### I4. Cargo feature-gating verification
Ran `cargo build --release --manifest-path src-tauri/Cargo.toml --bins` from a clean state. Output: **only `sniptex` was built**; the 6 dev-bins are absent. ✅ `[features] dev-bins = []` + `required-features = ["dev-bins"]` works exactly as intended. `cargo test --no-run` (user's verification) compiles all 13 test binaries because tests don't need the feature gate.

### I5. Workflow YAML / shell syntax
Both workflows are valid YAML (user verified). `generate-checksums.sh` passes `bash -n`. ✅ Nothing to add.

### I6. Permissions on `~/.tauri/sniptex.key`
Out of scope for this review (filesystem state, not committed). User claims `mode 600` which is correct.

### I7. `pnpm tsc --noEmit` invocation works
Verified locally — `pnpm tsc --version` resolves the binary correctly. No need for `pnpm exec`. ✅ Red-team angle #3 resolved.

---

## POSITIVE OBSERVATIONS

- Clean separation of `js` and `rust` matrix jobs in CI — fast feedback for frontend changes without waiting for Rust toolchain.
- `Swatinem/rust-cache@v2` correctly scoped to `workspaces: src-tauri` — avoids accidental cache poisoning from sibling Cargo workspaces.
- `fail-fast: false` in both matrices — one runner failing doesn't kill the rest. Important for the partial-release scenario in release.yml.
- `docs/releasing.md` is genuinely useful — pre-req checklist, step-by-step, key rotation, and a troubleshooting table. Better than what most Tauri projects ship.
- Feature-gating dev bins via Cargo features is the correct idiomatic solution (vs. moving files or splitting into a sub-crate).
- `UpdateCheckResult` tagged-union refactor IS a real improvement over the original stale-closure pattern. Good defensive refactor.
- `cancel-in-progress` PR-only expression — exactly right; protects push-to-main from race-y cancellation.

---

## RECOMMENDED ACTIONS (Prioritized)

1. **C1** — Replace `macos-13` with `macos-15-intel` (or drop Intel). Without this, every release tag push will fail on the Intel job before anything else.
2. **C2** — Decide updater activation strategy. Options ranked by my preference: (b) accept no-op + delete `active` field + fix the error toast copy; OR (a) cfg-gate the plugin in `lib.rs`. Do NOT keep the current state where the JSON field is a lie.
3. **H1** — Add a `gh release view` smoke check before `gh release download` in `checksums` job.
4. **H2** — Pin `tauri-action@v0` to a SHA.
5. **M1** — Add Escape handler to UpdateDialog. Cheap, improves a11y.
6. **M2** — Either install `@tauri-apps/plugin-process` + call `relaunch()`, or change the post-install copy to be platform-accurate.
7. **M4** — Disable the "Check for updates" button while `checking === true`.
8. **L2** — Optionally include `.sig` files in checksums.
9. **I2** — Plan a one-time Cask SHA256 update post-v0.1.0 release tag.

---

## METRICS

- Files reviewed: 8 (all in scope)
- LOC reviewed: ~470
- Findings: 2 Critical, 4 High, 6 Medium, 5 Low, 7 Info
- Type Coverage: clean (`pnpm tsc --noEmit` passes)
- Linting: 13 pre-existing clippy warnings (already known, advisory)
- Build sanity: ✅ release build produces only `sniptex` bin (24 MB stripped)
- Test sanity: ✅ all 13 test binaries compile

---

## UNRESOLVED QUESTIONS

1. **Cask publication timing:** does the Cask live in a personal tap or homebrew-cask? Personal tap → user controls when to update. homebrew-cask main → PR after v0.1.0 release. Affects urgency of I2.
2. **Intel build truly necessary?** Apple Silicon shipping date is now ~5 years old; Intel Mac install base is shrinking. Dropping Intel might be the right call (and would resolve C1 with zero workflow churn).
3. **Updater rollout strategy:** does the user want updater to ship working in v0.1.0 (then C2 path (b) + activate + push secrets pre-tag), or stay dark until v0.2.0 (then C2 path (a) — cfg-gate the plugin off, no surprise network calls)?
4. **Windows code signing:** workflow doesn't reference an Authenticode cert. Phase 11 deferred signing — confirm this is still intentional and document the SmartScreen UX in `docs/releasing.md` step 4 or `install-guide.md`.
5. **`tauri-plugin-single-instance` registration order:** out-of-scope for Phase 12, but verify it's registered first in `lib.rs` since it's already in Cargo.toml.

---

**Status:** DONE_WITH_CONCERNS
**Summary:** Phase 12 artifacts are 80% solid (clean YAML, sound Cargo gating, useful docs, good React refactor), but two findings — retired `macos-13` runner and the no-op `active: false` field — break the user's stated scope ("scaffold + disabled, works when activated"). Both are cheap to fix. Score 6.5/10.
**Concerns/Blockers:** C1 + C2 should be addressed before pushing any v0.1.0 tag; H1 + H2 should be addressed before promoting workflows to "release-quality".
