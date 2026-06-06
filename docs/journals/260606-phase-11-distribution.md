# Phase 11 Shipped: macOS ARM Distribution Locked

**Date**: 2026-06-06 23:47
**Severity**: Low
**Component**: Distribution, code signing, installer workflow
**Status**: Complete (ARM-only v0.1.0)

## What Happened

Shipped macOS ARM (Apple Silicon) distribution for v0.1.0: Tauri build-time signing + DMG bundle + Homebrew Cask formula + install-guide with macOS 12–Sequoia walkthroughs. Scope cut to ARM-only this round per user confirmation; x86_64 + Windows MSI land in Phase 12 CI (Phase 11 focus was feasibility of the signing pipeline, not multi-arch breadth).

Code review caught 5 High findings (all doc/cask-layer, no runtime issues) — fixed in-session before commit. DMG verified: 19 MB, ad-hoc signed, satisfies Designated Requirement.

## The Brutal Truth

Frustrated initially by the scope cut (local x86_64 target not available on the dev machine) — felt like a regression until realizing the CI multirunner approach is actually cleaner. Moving Mac Intel + Windows to Phase 12 CI eliminates manual-build-per-platform toil and keeps release workflow deterministic. The real win: Tauri's `signingIdentity: "-"` field eliminates a post-build sign step entirely. That simplification alone pays for itself in CI.

The code reviewer's finding about keychain fallback paths was the scariest miss. The `keyring` crate writes API keys to TWO paths: Tauri's standard `~/Library/Application Support/com.sniptex.app` AND a fallback path `~/Library/Application Support/com.sniptex` (note: no `.app`). Without both in the Cask zap block, users who uninstall with `--zap` leave credentials behind. Easy fix once found, but means the uninstall workflow is fragile unless we document this split path behavior everywhere. It's in the code (`src-tauri/src/agents/keychain.rs:42-47`) but not obvious.

## Technical Details

**Signing path consolidation:**
- Originally: sign-mac.sh as canonical signer post-build
- Discovered: Tauri 2 has `bundle.macOS.signingIdentity: "-"` in `tauri.conf.json`
- Outcome: Build-time signing is the canonical path; sign-mac.sh kept only for local re-signing arbitrary .app bundles

**Code review findings (5 High, all fixed):**
1. README hotkey wrong: listed Cmd+Shift+S, actual is Cmd+Shift+M (verified `src-tauri/src/hotkey.rs:39`)
2. Cask had no `on_arch` block; README promised Intel but only ARM DMG existed — scoped all three docs (README, install-guide, Cask) to ARM-only for v0.1.0
3. Cask zap missing keychain fallback path `com.sniptex` — added both `com.sniptex` and `com.sniptex.app` to zap block
4. macOS Sequoia (15+) removed right-click → Open shortcut for unidentified apps; users now go System Settings → Privacy & Security → "Open Anyway" — install-guide now branches by macOS version
5. Windows 11 Smart App Control silently blocks unsigned MSIs with no SmartScreen dialog (SAC off-switch is one-way, can't re-enable without reinstall) — documented in install-guide under Windows section

**Artifacts:**
- `src-tauri/tauri.conf.json`: added `bundle.macOS.{minimumSystemVersion: "12.0", signingIdentity: "-"}`
- `scripts/sign-mac.sh`: 35 lines, codesign + verify for manual re-signing
- `Casks/sniptex.rb`: 23 lines, passes `brew style`, zap covers both keychain paths + Logs/Caches/Saved State
- `docs/install-guide.md`: 147 lines, macOS (A: Cask, B: DMG) + Windows (SmartScreen + SAC) + troubleshooting table
- `README.md`: 111 lines, minimal project summary + install pointer
- DMG: `SnipTeX_0.1.0_aarch64.dmg`, 19 MB, SHA256 `13a5ea48b26fea2e5aba14bade0ef0c833c52e4f5bc1d8425e2e3e13e3515124`, verified valid on disk

## What We Tried

1. **Separate post-build signing step** → superseded by Tauri's `signingIdentity: "-"` field in tauri.conf.json. Build-time signing is simpler, fewer CI dependencies.
2. **Single keychain path in Cask zap** → reviewer caught missing fallback, added both paths.
3. **Single install-guide for all macOS versions** → reviewer noted Sequoia behavior change (no right-click bypass), split into version branches.

## Root Cause Analysis

Why scope cut to ARM-only?
- Dev machine (Apple Silicon) has `aarch64` toolchain but not `x86_64-apple-darwin`.
- Cross-compiling x86_64 locally requires additional Rosetta 2 setup or separate CI step.
- User confirmed: CI multirunner is the right place for Intel + Windows; Phase 11 focus is signing pipeline feasibility, not breadth.

Why keychain path not obvious?
- `keyring` crate's file fallback path differs from Tauri's bundle ID path. Not documented in SnipTeX code comments.
- Uninstall workflows (Cask zap, manual cleanup) must hit both paths or leave artifacts.
- This is a real attack surface (API credentials) — required proactive zap design, not reactive debugging.

Why macOS Sequoia behavior change?
- Apple's security model for unsigned apps tightened in macOS 15.0. Right-click → Open was a usability affordance; Sequoia removed it.
- System Settings → Privacy & Security now the only bypass for first-launch.
- Windows Smart App Control is similar: reputation-based protection + mandatory SAC toggle for unsigned MSIs.

## Lessons Learned

1. **Check `keyring` fallback paths** in any Tauri + credential-management combo. The fallback write path is a second home for uninstall logic; document it visibly in code.
2. **Tauri 2 ad-hoc signing via `signingIdentity: "-"`** is the right default for unsigned (free) releases. Don't over-engineer post-build sign scripts; let the framework handle it.
3. **Branch install guides by OS version** (not just platform). macOS/Windows security models evolve; v0.1.0 steps won't be v1.0 steps.
4. **Scope cuts to CI are good.** Multirunner pipelines (macOS-latest + windows-latest) eliminate local build variance and keep release artifacts deterministic.
5. **Brew style lint caught nothing,** but code review caught real issues. Lint is a baseline; human review on uninstall + signing + credentials is non-negotiable.

## Deferred (Real Tech Debt, Not Blocking)

The .app bundle ships 5 dev/test binaries under `Contents/MacOS/` — `cli_test`, `novita_smoke`, `history_smoke`, `pdf_smoke`, `novita_hybrid_smoke` — from `[[bin]]` entries in Cargo.toml. Tauri signs them all (fine), but they're bundle bloat + attack surface in a release. Move to `examples/` or feature-gate before v1.0. Non-blocking for v0.1.0 (smoke test binaries are useful for debugging). Mark as Phase 13+ tech debt.

## Next Steps

1. **Phase 12**: Add Windows x64 MSI + macOS Intel x86_64 to CI (`windows-latest` + `macos-latest` runners). Test multirunner artifact publishing.
2. **Phase 12**: Tag v0.1.0 release, publish to GitHub Releases (with SHA256 checksums). Cask submission goes in Phase 15 (requires live smoke test).
3. **Phase 13+**: Feature-gate dev binaries or move to examples; thin release bundle before v1.0.
4. **Ongoing**: Document keychain dual-path design in code comments when touching keyring integration.

## Unresolved Questions

None. Scope clear (ARM-only for v0.1.0), architecture locked in (Tauri build-time signing), multiarch path deferred to CI (Phase 12). All findings resolved before commit.
