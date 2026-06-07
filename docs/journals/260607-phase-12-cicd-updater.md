# Phase 12: CI/CD Release Workflow & Auto-Updater — Scaffolded, 2 Critical Findings Caught + Fixed Pre-Commit

**Date**: 2026-06-07
**Component**: CI/CD Pipeline, Tauri Auto-Updater, Cargo Build Hygiene
**Status**: Scaffolded; both Critical review findings fixed in-session and shipped in commits

## What Happened

Shipped the full Phase 12 scaffold: GitHub Actions CI + tag-triggered release workflows, Tauri auto-updater config (endpoint + ed25519 pubkey), `UpdateDialog` React component + AboutTab "Check for updates" button, SHA256 checksums script, dev/test binary feature-gate, and maintainer release docs. Code review caught 2 Critical + 4 High findings; verified each claim inline (curl on GitHub runner-images repo for C1; grep on installed Tauri updater source for C2), then fixed C1 + C2 + 3 of 4 High before commit. The 4 shipped commits already reflect every applied fix.

## The Two Critical Findings (Both Fixed)

### C1 — `macos-13` Retired Dec 2025
- Reviewer flagged the matrix runner. Verified by `curl` to `https://raw.githubusercontent.com/actions/runner-images/main/images/macos/macos-13-Readme.md` → HTTP 404. Confirmed retired.
- **Applied fix**: swapped to `macos-15-arm64` (ARM) + `macos-15` (Intel) in both `ci.yml` and `release.yml`. Verified GitHub still serves both README files. `macos-14` (the original ARM choice) is also on a deprecation clock (fully unsupported Nov 2026) — switched away from it too.

### C2 — Tauri 2 Updater Has No `active` Field
- Reviewer claimed `plugins.updater.active: false` is silently dropped by Tauri 2's `Config` deserializer.
- Verified by `grep -A 80 "pub struct Config" ~/.cargo/registry/src/.../tauri-plugin-updater-2.10.1/src/config.rs`. The Config struct fields are: `dangerous_insecure_transport_protocol`, `dangerous_accept_invalid_certs`, `dangerous_accept_invalid_hostnames`, `endpoints`, `pubkey`, `windows`. No `active`. No `dialog`. The `Deserialize` impl doesn't use `deny_unknown_fields`, so both fields are silently dropped.
- **Applied fix**: removed `active: false` and `dialog: false` from `tauri.conf.json`. Updated `docs/releasing.md` to clarify the updater is "always live" once endpoint + pubkey are set; the AboutTab button gracefully shows "Update check unavailable" until the first release tag publishes a real `latest.json` (acceptable pre-launch state).

## Other Fixes Applied Pre-Commit

- **H1** — SHA-pinned `tauri-action@v0` to `tauri-apps/tauri-action@84b9d35b5fc46c1e45415bdb6144030364f7ebc5` (v0.6.2). Reproducible releases.
- **H2** — dropped redundant `pnpm tsc --noEmit` step from `ci.yml`. `pnpm build` already runs `tsc && vite build` per `package.json`.
- **H3** — added Escape-to-dismiss + focus-on-mount to `UpdateDialog`. Escape is locked during the `downloading` phase to prevent mid-download cancel.

## What Stayed Deferred (Intentional, Tracked in Phase Doc)

- **GitHub Secrets setup** (`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) — must be added by user via GitHub web UI from the keypair at `~/.tauri/sniptex.key`. Process documented in `docs/releasing.md`.
- **24h periodic update check on launch** — plan scope item; deferred until the manual button is proven via first release.
- **End-to-end tag-push smoke test** — `gh auth status` returned 401 mid-session; user must `gh auth login` then push a test tag to verify CI matrix actually queues + builds + publishes draft. Not blocking commits.
- **`gh release download` from draft + concurrency guard on Check-for-updates double-click** — single remaining High; small, deferred.

## Adjacent Wins

- **Dev-bin cleanup from Phase 11 review carried over.** Phase 11 flagged 5 dev/test binaries shipping inside `.app/Contents/MacOS/`. Phase 12 added a 6th on disk (`tabular_e2e_smoke`). Applied `autobins = false` + `[features] dev-bins = []` + explicit `[[bin]] sniptex` for main + `required-features = ["dev-bins"]` on all 6 dev bins. Verified `cargo check` (default features) only resolves the main `sniptex` bin; `cargo check --features dev-bins` resolves all 6. Tauri's bundler uses default features, so release `.app` will no longer carry the dev surface.
- **Stale `nicekid1/sniptex` URLs** in `about-tab.tsx` fixed to `hiep1987/sniptex` as a free side-fix in the updater commit. Grep confirmed zero remaining references.

## Commits

- `f6565be` feat(ci): add CI + tag-triggered release workflows with checksums
- `32cdde9` feat(updater): wire endpoint + pubkey, ship UpdateDialog + AboutTab check
- `d7aeafd` refactor(bins): feature-gate dev binaries behind 'dev-bins' to shrink release
- `8c6fbca` docs(plan): close phase 12 with scaffolded CI/CD + updater status

Branch now 7 commits ahead of `origin/main` (3 from Phase 11 + 4 from Phase 12). No push.

## Lessons Worth Keeping

1. **Verify external tool schemas against installed source, not docs.** Phase 12 plan referenced Tauri 1 updater fields (`active`, `dialog`). Both gone in v2. Caught only because the reviewer read the actual `config.rs` and I re-verified. Future versions of any plugin: grep `~/.cargo/registry/src/.../<plugin>/src/config.rs` before trusting that a config knob exists.
2. **GitHub Actions runner labels churn on published schedules.** `macos-13` was always going to retire; we just didn't track it. For release-critical workflows: explicit version-tagged runners (not `macos-latest`), and a quarterly runner-changelog sweep.
3. **`pnpm build` is canonical, not a sub-step.** When a script already chains `tsc && vite build`, separate "typecheck" steps in CI are duplicated work. Trust the package.json contract.

## Unresolved Questions

1. **Test-tag push timing**: cut a `v0.0.1-test` to validate the CI matrix end-to-end, or wait for the real v0.1.0 release?
2. **Intel Mac scope**: keep `macos-15` Intel build in the matrix (~doubles release time) or drop Intel and ship aarch64-only for v0.x?
3. **Updater UX before first release**: leave the "Update check unavailable" toast as-is until first release, or soften copy to "No release published yet — watch GitHub for v0.1.0"?
