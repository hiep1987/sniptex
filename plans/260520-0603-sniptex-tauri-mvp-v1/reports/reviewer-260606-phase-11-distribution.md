# Reviewer Report — Phase 11 Distribution

**Date:** 2026-06-06
**Reviewer:** code-reviewer
**Scope:** 6 files (tauri.conf.json diff, scripts/sign-mac.sh, Casks/sniptex.rb, docs/install-guide.md, README.md, .claude/.ckignore)
**Acceptance gate:** Phase 11 success criteria + 10 red-team angles in delegation prompt.

---

## Summary

Phase 11 artifacts are mostly correct and the build/sign chain is verified end-to-end. Three real bugs found that affect end users: a wrong hotkey in README, a Mac Intel install path that 404s, and missing keychain-fallback path in Cask zap. macOS 15 / Win11 Smart App Control gaps in install guide are also worth fixing before launch. The dev-binary bloat is pre-existing tech debt — flag but don't block Phase 11.

**Verdict:** DONE_WITH_CONCERNS — fix the High items before tagging v0.1.0. Medium/Low can ship.

**Score:** 7 / 10

---

## Findings

### Critical
_None._

### High

**H1. README.md:51 — wrong default hotkey advertised**
- README states `Cmd+Shift+S` (Mac) / `Ctrl+Shift+S` (Win).
- Actual default: `Cmd+Shift+M` / `Ctrl+Shift+M`. Verified at `src-tauri/src/hotkey.rs:30-40` (`Code::KeyM`) and confirmed in `plans/.../phase-08-...md:65` (`hotkey: String = "CommandOrControl+Shift+M"`).
- Impact: users press the documented combo and nothing happens; first-impression bug, especially during demo video / launch.
- Fix: change README.md:51 to `Cmd+Shift+M` / `Ctrl+Shift+M`.

**H2. Casks/sniptex.rb — Intel Mac install path is broken**
- URL is hard-coded to `SnipTeX_#{version}_aarch64.dmg`. No `on_arch :intel` / `on_arm` branch.
- `brew install --cask sniptex` on an Intel Mac fetches the ARM DMG → either 404 (if you later upload only `_x64.dmg`) or "Bad CPU type" at launch (current state).
- README.md:20 advertises "macOS 12+ Intel — `.dmg` (ad-hoc signed)" as a supported row; install-guide.md:33 references `SnipTeX_<version>_x64.dmg`. Cask doesn't deliver it.
- Fix options:
  - (a) Ship a universal binary (`--target universal-apple-darwin`) and rename Cask URL to a single `_universal.dmg`; OR
  - (b) Add `on_arch` block:
    ```ruby
    on_arm do
      url ".../SnipTeX_#{version}_aarch64.dmg"
      sha256 "..."
    end
    on_intel do
      url ".../SnipTeX_#{version}_x64.dmg"
      sha256 "..."
    end
    ```
- Until either lands, narrow README/install-guide to "Apple Silicon only in v0.1.0".

**H3. Casks/sniptex.rb:15-20 — zap misses the keychain file-backed fallback**
- `agents/keychain.rs:42-47` writes `dirs::data_dir()/com.sniptex/api-keys.json`. On macOS that resolves to `~/Library/Application Support/com.sniptex/api-keys.json` — note `com.sniptex`, **not** `com.sniptex.app`.
- Zap only trashes `~/Library/Application Support/com.sniptex.app`. The cleartext API-key fallback file survives `brew uninstall --zap`.
- Privacy / hygiene issue: BYOK keys persist on disk after explicit user "zap"-uninstall.
- Fix: add `"~/Library/Application Support/com.sniptex"` to the zap trash list. Optionally also note that OS Keychain entries under service `com.sniptex` (4 accounts: `gemini-api-key`, `mistral-api-key`, `novita-api-key`, `cloud-goclaw-api-key`) cannot be removed by Cask zap — document as residual cleanup or add a CLI command.

**H4. docs/install-guide.md — macOS 15 (Sequoia) Right-click→Open no longer works on first launch**
- Steps 4-5 in §"Option B — DMG download" rely on right-click → Open. Sequoia 15.x changed Gatekeeper: for unsigned/unidentified-developer apps, the first launch now requires System Settings → Privacy & Security → "Open Anyway" (right-click → Open is silently denied with the unidentified-developer block; only "Move to Bin" / "Done" buttons appear).
- Apple confirms Settings-path as the official procedure for Sonoma 14 + Sequoia 15.
- Fix: add Sequoia-specific subsection: "On macOS 15: open System Settings → Privacy & Security, scroll to Security, click **Open Anyway** next to the SnipTeX block, enter password. The button appears for ~1 hour after a failed launch attempt."

**H5. docs/install-guide.md — Windows Smart App Control (SAC) not addressed**
- Guide covers SmartScreen (clickable "More info → Run anyway"). Windows 11 22H2+ ships with **Smart App Control** enabled by default on clean installs, which blocks unsigned MSIs with **no click-through** — the only path is to disable SAC (one-way until very recent builds; permanent on most current installs).
- Suggested "disable Reputation-based protection" step actually addresses SmartScreen, not SAC.
- Real impact: a non-trivial slice of Windows 11 users on factory-fresh hardware cannot install at all by following the current guide.
- Fix: add a §"If 'More info' is missing" branch that distinguishes (a) Reputation-based-protection disable for SmartScreen vs (b) Smart App Control check (Settings → Privacy & Security → Windows Security → App & browser control → Smart App Control). Note that disabling SAC is a one-time action that may require a clean Windows reinstall to re-enable, so users should accept that trade-off knowingly.

### Medium

**M1. README.md:20 — Mac Intel row + Win x64 row over-promise current build state**
- Table claims four shipped formats. Verified-built today: aarch64 DMG only. Win x64 + Mac Intel cross-builds are deferred to Phase 12 CI (per phase-10 / phase-12 docs).
- Recommend either: mark non-built rows as "Phase 12" / "planned for v0.1.x" OR remove until cross-builds land, to avoid the H2-class confusion. Same applies to the Windows ARM64 row — installer was build-validated in Phase 10 but no release artifact is uploaded yet.

**M2. .claude/.ckignore — `!target` is too permissive**
- `src-tauri/target` is 34 GB on this host. A bare `!target` re-allows hooks/grep/glob to walk the entire dir, risking context blowup and accidental ingestion of `.fingerprint/*.json` metadata.
- Stated reason ("sign + checksum bundle artifacts") only needs the bundle output paths.
- Fix: replace with narrower allowlist, e.g.:
  ```
  !target/release/bundle/**
  !target/*/release/bundle/**
  ```
  Covers both `target/release/bundle/...` and per-target paths like `target/aarch64-apple-darwin/release/bundle/...` without exposing build intermediates.

**M3. .app bundle ships 5 dev/test binaries (pre-existing, but Phase 11 distribution makes it visible)**
- `Contents/MacOS/` contains: `cli_test`, `history_smoke`, `novita_smoke`, `pdf_smoke`, `novita_hybrid_smoke` alongside the main `sniptex` binary. `tabular_e2e_smoke` exists in `src/bin/` and is auto-discovered by cargo but didn't make it into the bundle in this build (Tauri filtered it for unknown reason — worth checking).
- Each adds ~2-4 MB to the .app and increases attack surface (extra code paths signed under the same identifier). DMG still under 25 MB target so not a sizing blocker.
- These are explicit `[[bin]]` entries in `src-tauri/Cargo.toml:63-77`. Tauri bundles every `bin` target by default unless excluded.
- Real fix (Phase 12 or earlier): move smoke binaries to `src-tauri/examples/*` (cargo doesn't bundle examples) OR gate them behind `required-features = ["dev-tools"]`. Either is a Cargo.toml-only change.
- Not blocking Phase 11 acceptance — flag for Phase 12 CI prep.

### Low

**L1. README.md:5-9 — claims 3 OCR paths; codebase exposes 5**
- README mentions Codex CLI / Gemini CLI / Gemini Vision API. `src-tauri/src/agents/mod.rs:15-16` exposes 5 cloud IDs: Gemini, Mistral, Novita, Novita-hybrid, Goclaw. Phase 8 settings UI surfaces a `provider` toggle for them.
- Either intentional marketing simplification (fine — note "MVP shows Gemini API; alpha-only Mistral/Novita") or stale (then README needs update). Recommend a one-liner: "Additional cloud providers (Mistral, Novita) available in experimental mode."

**L2. Casks/sniptex.rb:6 — `verified:` trailing slash style**
- Value is `"github.com/hiep1987/sniptex/"`. Cask Cookbook prefers no trailing slash for the smallest-uniquely-identifying portion, BUT trailing slash IS standard practice for GitHub-hosted Casks when verifying a URL prefix (anchors the prefix). `brew style` passes.
- Will likely be a non-issue for `brew audit --cask`. If audit warns, drop the trailing slash. Not blocking.

**L3. scripts/sign-mac.sh:26 — `--deep` is deprecated by Apple**
- Apple flags `codesign --deep` as deprecated in current Xcode (use per-binary signing instead). Still works in CI today, but Apple may break it.
- Also, since `tauri.conf.json:136` sets `"signingIdentity": "-"`, Tauri already ad-hoc signs each binary during `tauri build` (confirmed: `codesign -dvv` reports `Signature=adhoc`, `flags=0x10002(adhoc,runtime)`). The script is therefore redundant in the common case — useful only after manual `.app` modification.
- Recommendation: keep the script (useful for re-signing after modification) but add a comment noting the build already signs ad-hoc, and consider switching to per-binary signing if `--deep` ever stops working.

**L4. docs/install-guide.md:78 — "~3,000 downloads" SmartScreen reputation threshold is folklore**
- Microsoft doesn't publish a fixed threshold; it's algorithmic and depends on geographic spread, time, and signing. Either remove the number or hedge with "varies, typically several thousand".

### Info

**I1. tauri.conf.json:134-137 — clean isolated addition, no regressions**
- The new `bundle.macOS` block is isolated and doesn't shadow Windows/Linux bundle config (verified: no other `windows`/`linux` keys exist under `bundle`).
- `minimumSystemVersion: "12.0"` value format is valid (Tauri 2.x default is `"10.13"`, two-part).
- `signingIdentity: "-"` is a valid Tauri-utils field (`tauri-utils-2.9.2/src/config.rs:653`). Triggers ad-hoc sign during `tauri build`.
- `npx tauri build` succeeded (user-verified), produced `SnipTeX_0.1.0_aarch64.dmg`, SHA256 matches the value baked into Casks/sniptex.rb:3 (`13a5ea48...`). End-to-end pipeline integrity confirmed.

**I2. Cask URL pattern matches actual emitted filename**
- Verified: `find target -name "*.dmg"` returns `target/aarch64-apple-darwin/release/bundle/dmg/SnipTeX_0.1.0_aarch64.dmg` — matches the Cask's `SnipTeX_#{version}_aarch64.dmg` interpolation exactly.

**I3. Cask zap covers main `app_data_dir` correctly**
- Tauri's `app_data_dir()` on macOS resolves to `~/Library/Application Support/com.sniptex.app`. SQLite, images/, thumbs/, settings.json all land there. Zap entry 1 catches it.
- See H3 for the keychain-fallback gap (the other directory).

**I4. Autostart plist not in zap (low impact)**
- `tauri-plugin-autostart` is enabled (`Cargo.toml:122`, capability `autostart:default`). When the user opts in to "Launch at login", a plist is written under `~/Library/LaunchAgents/com.sniptex.app.plist` (auto_launch crate behavior).
- Zap doesn't trash `~/Library/LaunchAgents/com.sniptex.app.plist`. Most users won't enable autostart in MVP; cheap to add to zap proactively.
- Suggested addition to zap trash list: `"~/Library/LaunchAgents/com.sniptex.app.plist"`.

**I5. scripts/sign-mac.sh hardening is sufficient**
- `set -euo pipefail` + quoted `"$APP_PATH"` handle spaces and missing args correctly. `bash -n` passes. `codesign --timestamp=none` is correct for ad-hoc (no Apple timestamp server interaction needed). No injection risk in the single user-supplied arg (passed as positional, not eval'd).

**I6. No new public-contract regressions vs Phase 10**
- Window labels (`main`, `preview`, `settings`, `history`, `onboarding`, `overlay`) unchanged.
- App identifier (`com.sniptex.app`) unchanged.
- Asset-protocol scopes unchanged.
- No removed config keys. Adding `bundle.macOS` doesn't break Win MSI/NSIS targets in `bundle.targets`.

---

## Verification Trail

| Claim | Verified by | Result |
|---|---|---|
| `tauri.conf.json` is valid JSON | `python3 -c json.load` | OK |
| Cask Ruby syntax valid | `ruby -c` | OK |
| Cask passes `brew style` | `brew style Casks/sniptex.rb` | 0 offenses |
| `sign-mac.sh` bash syntax valid | `bash -n` | OK |
| DMG SHA256 matches Cask | `shasum -a 256` | `13a5ea48...` matches |
| `tauri build` ad-hoc signs at build | `codesign -dvv` on bundled .app | `Signature=adhoc, flags=0x10002(adhoc,runtime)` |
| Hotkey default really M not S | grep `src-tauri/src/hotkey.rs:30-40` | `Code::KeyM` |
| `com.sniptex` keychain fallback path | read `agents/keychain.rs:42-47` | `dirs::data_dir/com.sniptex/api-keys.json` |
| Dev bins in bundle | `ls Contents/MacOS/` | 5 extra binaries present |
| `signingIdentity` is a real Tauri key | grep `tauri-utils-2.9.2/src/config.rs:653` | confirmed |

---

## Recommended Actions (priority order)

1. **H1** — Fix hotkey in README.md:51 (Cmd+Shift+M / Ctrl+Shift+M). 1-line change.
2. **H2** — Decide universal binary vs `on_arch` Cask blocks; align README + install-guide with current reality.
3. **H3** — Add `~/Library/Application Support/com.sniptex` to Cask zap trash.
4. **H4** — Add macOS 15 "System Settings → Open Anyway" section to install-guide.
5. **H5** — Add Smart App Control branch to install-guide Windows section.
6. **M1** — Narrow README platform-support table to what's actually shipping in v0.1.0.
7. **M2** — Tighten `.ckignore` to `!target/*/release/bundle/**` and `!target/release/bundle/**`.
8. **M3** — Defer to Phase 12: move smoke binaries to `examples/` or feature-gate them.
9. **L1-L4, I4** — Polish before Phase 15 launch.

---

## Unresolved Questions

1. Is shipping aarch64-only acceptable for v0.1.0 (drop Intel Mac from README and install-guide), or is the plan to do a Mac Intel build before tagging?
2. Does the team want to invest in universal binaries (simpler Cask, larger DMG ~30 MB) vs maintaining two separate Cask URLs?
3. For the dev-bin bundle bloat (M3): is feature-gating with `required-features = ["dev-tools"]` acceptable, or should they move to `examples/`? Latter is cleaner but breaks `cargo run --bin novita_smoke` workflow if developers rely on it.
4. Should the Cask zap include the OS-Keychain cleanup via a `uninstall_postflight do ... end` hook running `security delete-generic-password`? Adds complexity but completes BYOK secret cleanup.
5. Phase 11 doc says "Cask formula passes `brew audit --cask`". User ran `brew style` (passes) but `brew audit` was not run. Worth running `brew audit --cask Casks/sniptex.rb` once the URL is reachable (post v0.1.0 release tag).

---

**Status:** DONE_WITH_CONCERNS
**Summary:** Phase 11 build + sign + Cask + docs chain is functional and verifiable; 5 High findings (wrong hotkey, Intel-Mac install break, missing zap path for keychain fallback, missing Sequoia/SAC install steps) should land before v0.1.0 tag.
**Concerns/Blockers:** The H-class items are user-visible regressions/footguns at launch. None are blocking the artifacts review itself — all are fixable in <30 min.
