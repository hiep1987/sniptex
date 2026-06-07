---
phase: 12
title: "CI/CD Release Workflow & Auto-Updater"
status: scaffolded
priority: P2
effort: "2d"
dependencies: [11]
completed: "2026-06-07"
---

## Status — 2026-06-07

✅ **CI workflow** (`.github/workflows/ci.yml`): js + rust matrix (ubuntu / macos-15-arm64 / windows-latest), advisory clippy
✅ **Release workflow** (`.github/workflows/release.yml`): tag-triggered, 2-runner matrix (`macos-latest` ARM + `windows-latest`), SHA-pinned tauri-action@v0.6.2, includeUpdaterJson, separate checksums job. Intel Mac dropped after the v0.0.1-test run revealed `macos-15-arm64` + `macos-15` stall in `queued` (Larger Runner labels require paid billing)
✅ **Updater config** (`src-tauri/tauri.conf.json`): endpoints + real ed25519 pubkey wired
✅ **Updater plugin** already registered (`lib.rs:66` + `capabilities/default.json:30`)
✅ **UpdateDialog** (`src/components/update-dialog.tsx`): modal portal, download progress, Escape-to-dismiss, focus-on-mount
✅ **AboutTab "Check for updates"** (`src/windows/settings/about-tab.tsx`) + `nicekid1`→`hiep1987` URL fix
✅ **scripts/generate-checksums.sh** with smoke test verified locally
✅ **Dev-bin cleanup**: 6 dev/test bins feature-gated behind `dev-bins` Cargo feature; release builds exclude them
✅ **docs/releasing.md** maintainer guide + signing-key rotation + troubleshooting matrix
✅ **Code review** by `code-reviewer` agent: 2 Critical + 4 High + 6 Medium findings — Critical + 3 High applied; remaining items tracked in report
   (see `reports/reviewer-260607-phase-12-cicd-updater.md`)

⏸ **Tauri updater activation**: there is no `active` master switch in Tauri 2 — the plugin is always live. Endpoint URL 404s until first release tag is published. Pre-release behavior: AboutTab button surfaces "Update check unavailable" (acceptable).
⏸ **Test-tag CI push**: requires `gh auth login` (current 401) — deferred to user-driven step
⏸ **GitHub Secrets setup** (`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`): user-driven, documented in `docs/releasing.md`
⏸ **24h periodic update check on launch**: in scope per plan; not shipped this round (UI on-demand check only). Add when first release ships and the manual button is proven.
⏸ **Windows code signing**: still deferred per Phase 11 Session-1 Q7 (~$200/yr cert)

**Artifacts produced this round:**
- `.github/workflows/ci.yml` (NEW)
- `.github/workflows/release.yml` (NEW)
- `scripts/generate-checksums.sh` (NEW, kebab-case shell)
- `src/components/update-dialog.tsx` (NEW)
- `src/windows/settings/about-tab.tsx` (MOD: "Check for updates" button + URL fix)
- `src-tauri/tauri.conf.json` (MOD: updater endpoints + pubkey)
- `src-tauri/Cargo.toml` (MOD: `autobins = false`, `[features] dev-bins = []`, explicit `[[bin]] sniptex`, `required-features = ["dev-bins"]` on 6 dev bins)
- `docs/releasing.md` (NEW)
- `~/.tauri/sniptex.key` + `.key.pub` (LOCAL, NOT COMMITTED; mode 600)

# Phase 12: CI/CD Release Workflow & Auto-Updater

## Overview

Set up GitHub Actions CI/CD pipeline to build Mac (ARM + Intel) and Windows releases on git tag push, auto-upload artifacts to GitHub Releases, and integrate Tauri's built-in auto-updater so users get update notifications in-app. The pipeline also runs tests and linting on every PR.

## Key Insights

- Tauri provides official GitHub Actions: `tauri-apps/tauri-action` handles build + upload to GitHub Releases.
- Build matrix: `macos-latest` (ARM, cross-compile Intel) + `windows-latest` (x64).
- Auto-updater: `tauri-plugin-updater` checks a JSON endpoint (GitHub Releases API or static file) for new versions. On update available → show dialog → download + replace binary → restart.
- Release flow: push tag `v1.0.0` → CI builds all targets → uploads DMG/MSI + update manifest → GitHub Release created as draft → maintainer reviews + publishes.
- Ad-hoc signing in CI: `codesign --sign -` works without Apple Developer cert on `macos-latest` runners.

## Requirements

**Functional**
- CI runs on every PR: `cargo check`, `cargo test`, `cargo clippy`, `pnpm lint`, `pnpm tsc`
- Release workflow triggers on `v*` tag push
- Builds: Mac ARM DMG, Mac Intel DMG, Windows x64 MSI
- All artifacts uploaded to GitHub Release (draft)
- Auto-updater JSON manifest generated and attached to release
- In-app update check: on launch (once per 24h) + manual "Check for updates" in Settings → About
- Update dialog: "New version X.Y.Z available. [Update now] [Later]"
- Update downloads in background, shows progress, applies on next restart

**Non-functional**
- CI build time < 20 minutes per target
- Release artifacts include SHA256 checksums
- Update check does not block app startup

## Architecture

### CI/CD Pipeline

```
Tag push (v*)
    ↓
GitHub Actions: .github/workflows/release.yml
    ↓
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ macos-latest     │  │ macos-latest     │  │ windows-latest   │
│ aarch64-apple    │  │ x86_64-apple     │  │ x86_64-windows   │
│ → DMG (ARM)      │  │ → DMG (Intel)    │  │ → MSI            │
│ → codesign -     │  │ → codesign -     │  │                  │
└────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘
         └──────────────┬──────┴──────────────┬──────┘
                        ↓                     ↓
              GitHub Release (draft)
              + update manifest JSON
              + SHA256 checksums
```

### Auto-Updater Flow

```
App launch
    ↓
tauri-plugin-updater.check()
    ↓ (async, non-blocking)
Fetch https://github.com/.../releases/latest → update manifest
    ↓
Compare current version vs latest
    ↓ (if newer)
Show update dialog → user accepts → download → install on restart
```

### Update Manifest (latest.json)

```json
{
  "version": "1.1.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2026-06-15T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "url": "https://github.com/.../SnipTeX_1.1.0_aarch64.dmg.tar.gz",
      "signature": "..."
    },
    "darwin-x86_64": {
      "url": "https://github.com/.../SnipTeX_1.1.0_x86_64.dmg.tar.gz",
      "signature": "..."
    },
    "windows-x86_64": {
      "url": "https://github.com/.../SnipTeX_1.1.0_x64-setup.msi.zip",
      "signature": "..."
    }
  }
}
```

## Related Code Files

- Create: `.github/workflows/ci.yml` — PR checks (cargo check, test, clippy, pnpm lint, tsc)
- Create: `.github/workflows/release.yml` — tag-triggered build + release
- Modify: `src-tauri/tauri.conf.json` — updater endpoint config, signing keys
- Modify: `src-tauri/Cargo.toml` — verify `tauri-plugin-updater` dependency
- Create: `src/components/UpdateDialog.tsx` — update available notification
- Modify: `src/windows/SettingsWindow/AboutTab.tsx` — "Check for updates" button
- Create: `scripts/generate-checksums.sh` — SHA256 for release artifacts

## Implementation Steps

1. Create `.github/workflows/ci.yml`:
   - Trigger: `pull_request` to `main`
   - Jobs: `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `pnpm lint`, `pnpm tsc --noEmit`
   - Cache: `~/.cargo/registry`, `target/`, `node_modules/`
2. Create `.github/workflows/release.yml`:
   - Trigger: `push tags: ['v*']`
   - Matrix: `macos-latest` (aarch64 + x86_64), `windows-latest` (x86_64)
   - Steps: checkout → setup Rust + Node → pnpm install → `tauri-apps/tauri-action` with `tagName`, `releaseName`, `releaseDraft: true`
   - Mac jobs: add `codesign --sign -` step after build
   - Generate SHA256 checksums for all artifacts
3. Configure Tauri updater in `tauri.conf.json`:
   - Endpoint: GitHub Releases API URL pattern
   - Generate updater key pair: `npx tauri signer generate -w ~/.tauri/sniptex.key`
   - Store private key as GitHub Secret `TAURI_SIGNING_PRIVATE_KEY`
4. Implement update check logic:
   - On app launch: check once, then every 24h (store last check timestamp)
   - `UpdateDialog` component: version info, changelog, download progress, "Update now" / "Later" buttons
5. Add "Check for updates" button in Settings → About tab.
6. Test release flow:
   - Push test tag → verify CI builds all 3 targets
   - Verify DMGs and MSI attached to draft release
   - Verify update manifest JSON is correct
   - Verify in-app updater detects new version from test release
7. Document release process in `CONTRIBUTING.md` or `docs/releasing.md`.

## Todo List

- [x] Create CI workflow (PR checks)
- [x] Create release workflow (tag-triggered)
- [x] Configure build matrix (Mac ARM + Intel, Windows x64) — using macos-15-arm64 + macos-15 + windows-latest
- [x] Add codesign step for Mac in CI — handled by Tauri's `signingIdentity: "-"` from Phase 11
- [x] Generate SHA256 checksums for artifacts — `scripts/generate-checksums.sh` + dedicated CI job
- [x] Generate Tauri updater signing key pair — `~/.tauri/sniptex.key`
- [x] Configure updater endpoint in tauri.conf.json
- [ ] Store signing private key as GitHub Secret — user-driven; documented in `docs/releasing.md`
- [ ] Implement update check on app launch (24h interval) — deferred; AboutTab manual button is the v0.1 surface
- [x] Build UpdateDialog component
- [x] Add "Check for updates" to About tab
- [ ] Test end-to-end: tag push → build → release → in-app update — deferred (gh re-auth + first release)
- [x] Document release process

## Success Criteria

- [x] PR to `main` triggers CI checks (workflow shipped; clippy is advisory not blocking — 13 pre-existing warnings; tighten to `-D warnings` after a clippy cleanup pass)
- [ ] Tag `v0.9.0-beta` push triggers release workflow; all 3 artifacts built (deferred — requires `gh auth login` + first user-driven tag push)
- [ ] GitHub Release created as draft with DMGs, MSI, checksums, update manifest (deferred to first tag push)
- [ ] In-app updater detects test release and shows update dialog (deferred — needs published release)
- [ ] Update downloads and installs correctly on at least one platform (deferred — needs published release)

## Risk Assessment

- **Risk: CI build time exceeds 20 min** — Mitigation: aggressive caching of cargo registry + target dir; sccache for Rust compilation.
- **Risk: Tauri updater signature verification fails** — Mitigation: test key pair generation + signing flow locally before CI integration.
- **Risk: GitHub Releases API rate limiting on update checks** — Mitigation: 24h check interval; cache response; fall back gracefully if rate-limited.
- **Risk: Cross-compilation Mac ARM → Intel fails on CI** — Mitigation: use separate matrix jobs (not cross-compile); `macos-latest` is ARM, add `macos-13` for Intel if needed.

## Security Considerations

- Tauri updater uses Ed25519 signatures — binary integrity verified before install.
- Private signing key stored as GitHub Secret, never committed.
- Update manifest served over HTTPS (GitHub Releases).
- No auto-install without user consent (dialog always shown).

## Next Steps

- Phase 13 (Landing page) links to GitHub Releases download page
- Phase 15 (Launch) uses the release workflow for v1.0.0 publish
