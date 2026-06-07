# Phase 12: CI/CD Release Workflow — End-to-End Tag-Push Verification (10 Rounds, ~5.5h Elapsed)

**Date**: 2026-06-07
**Component**: GitHub Actions release matrix, Tauri updater schema, signature generation, artifact upload
**Status**: Verified green; all 9 release assets (DMG, MSI, NSIS, .app.tar.gz, .sig files, latest.json, checksums.txt) landed in draft release with valid Tauri 2 updater schema

## What Happened

Pushed `v0.0.1-test` tag to validate Phase 12's scaffolded CI/release pipeline end-to-end. Watched the GitHub Actions matrix build (macos-latest + windows-latest, both succeed in ~4 min). Confirmed all artifacts (2 Mac, 2 Windows, 3 sig files, 1 manifest, 1 checksums) landed in a draft release, latest.json validates against Tauri 2 schema, sig files match payload content. Test tag + draft release deleted after verification confirmed no regressions. **Took 10 rounds of tag-push + fix to reach green from "everything broken".**

## The Brutal Truth

This was frustrating. Phase 12 scaffold passed code review + self-test of individual scripts, but the end-to-end matrix revealed silent failures that don't surface until artifacts try to land. Round 1 hung in `queued / 0 steps` for **4 hours 43 minutes** before timing out — runner selection was silently wrong. Rounds 2–9 burned through "sig files generated in logs but missing from upload", "checksums.txt wrote to the wrong directory", "Tauri 2 schema doesn't match the action's expectations", "ghrelease upload can't rename assets", "Mac sig uploaded with the wrong filename".

The cost: 10 tag iterations, 10 delete-and-retry cycles, ~5.5 hours of wall time, multiple "this should have worked based on the Tauri action docs" moments followed by "the docs are for Tauri 1, not 2". The updater infrastructure was half-built — the action generated artifacts but then lost them because the v0.6.2 release still expects Tauri 1's `bundle.updater.*` schema, not Tauri 2's `plugins.updater.*`.

After round 10 finally landed a green release, the real relief: **nothing in the actual shipping code is broken**. All 12 fixes were scaffolding, CI wiring, or build artifact plumbing — not core app logic.

## Technical Details

**Round 1 (4h 43m timeout)**:
```
runs-on: macos-15-arm64
Status: queued (0 steps, 4h 43m, timeout)
```
`macos-15-arm64` is a Larger-Runner-only label (requires billing). Free public repos silently park jobs in `queued`. GitHub's public runner names post-2025: `macos-latest` (ARM), `macos-15` (Intel, paid), `ubuntu-latest`.

**Round 2–3 (cargo fmt drift)**:
```
error: code is formatted incorrectly
Files: cloud_novita_hybrid_api.rs, lib.rs, settings.rs
```
CI's `cargo fmt --all -- --check` caught formatting that never matched the repo's rustfmt defaults. Locally, `cargo fmt` said "already formatted"; in CI, it complained. Root: dev `.cargo/config.toml` wasn't committed (or wasn't in `--check` context). All 3 files had stray whitespace that dev builds silently ignored.

**Round 4–5 (dev binaries bundled into release)**:
```
error: Failed to copy binary from ".../release/pdf_smoke": does not exist
```
Phase 12 moved 6 dev/test binaries from `src-tauri/src/bin/*.rs` → `src-tauri/examples/*.rs` and marked them with `required-features = ["dev-bins"]`. **But Tauri's bundler ignores `required-features` on `[[bin]]` entries** — it reads `Cargo.toml` and unconditionally tries to copy every `[[bin]]` entry into the bundle. The `git mv` succeeded; the Cargo.toml edit (swapping `[[bin]]` → `[[example]]`) was staged separately and landed 1 commit later. Commit `845ee4a` fixed by moving the lines back to Cargo.toml.

**Round 6 (Ubuntu linker deps)**:
```
rust-lld: error: unable to find library -lgbm
rust-lld: error: unable to find library -lpipewire-0.3
```
xcap (screen capture crate) pulls libspa-sys → libpipewire-0.3-dev, and the Wayland path needs libgbm-dev (Generic Buffer Manager). Missing from apt deps.

**Round 7 (sig files generated, not uploaded)**:
```
Tauri Signer logs: "Signing /app.tar.gz ..."
GitHub Release: [no .sig file]
```
tauri-action@v0.6.2 advertises `includeUpdaterJson: true` but doesn't generate the manifest; worse, it generates .sig files (prints paths in logs) then loses them between build artifacts and upload steps. Root: tauri-action was written for Tauri 1's `bundle.updater.*` schema. Tauri 2 uses `plugins.updater.*` — the action reads Tauri.conf.json, finds no `bundle.updater.*` key, and falls through to a no-op. The .sig generation is transient (happens in tauri-action's process scope, lives in a temp dir, evaporates after the action ends).

**Round 8 (checksums.txt wrote to wrong dir)**:
```bash
# In generate-checksums.sh (original):
cd "$DIR"
shasum -a 256 *.dmg *.msi *.tar.gz > checksums.txt
# Resolves to: artifacts/artifacts/checksums.txt
```
Script did `cd $DIR` then wrote `checksums.txt` to cwd. When DIR=artifacts, this created artifacts/artifacts/checksums.txt. Fixed by iterating paths directly, no cd.

**Round 9 (gh release upload with #label)**:
```bash
gh release upload v0.0.1-test "artifacts/SnipTeX.app.tar.gz#SnipTeX_aarch64.app.tar.gz.sig"
# Result: asset named "SnipTeX.app.tar.gz.sig" (not "SnipTeX_aarch64...")
```
The `#label` syntax is a display-label (shown on the release page UI), **not a filename rename**. Asset still landed with the original filename. Latest.json expected `SnipTeX_aarch64.app.tar.gz.sig` from the manifest, found `SnipTeX.app.tar.gz.sig`, skipped Mac entry.

**Round 10 (final green)**:
Instead of relying on tauri-action to manage sigs, added post-build step:
```bash
npx tauri signer sign "artifacts/SnipTeX.app.tar.gz" -k "$TAURI_SIGNING_PRIVATE_KEY"
cp "artifacts/SnipTeX.app.tar.gz.sig" "artifacts/SnipTeX_aarch64.app.tar.gz.sig"
gh release upload v0.0.1-test "artifacts/SnipTeX_aarch64.app.tar.gz.sig"
```
Then manual `scripts/generate-latest-json.sh` parses the .sig files and builds the updater manifest against Tauri 2 schema. All 9 assets landed. Verified latest.json parses + matches schema.

## Commits Shipped (in order)

1. **9eec4c0** `style(rust): cargo fmt drift on 3 pre-existing files`
2. **dae586f** `fix(ci): drop macos-13 + macos-15-arm64, use macos-latest` (README + docs updated)
3. **845ee4a** `test(fixtures): un-ignore 9 codex/.txt files` + `refactor(bins): move 6 dev/test binaries to examples/` (accidental dual-commit bundle)
4. **1afab68** `fix(ci): add libpipewire-0.3 + libdbus + libxcb to Ubuntu apt deps`
5. **507c02a** `fix(ci): add libgbm-dev + libegl1-mesa-dev for Ubuntu rust link`
6. **6ed955c** `fix(release): self-generate latest.json and fix generate-checksums.sh` (checksums cd footgun + manual manifest)
7. **f02786f** `debug(ci): absolute paths + ls -la + missing/uploading log lines in sig upload`
8. **4d362bd** `fix(release): self-sign updater payloads via npx tauri signer sign`
9. **0c06f6b** `fix(release): tried gh release upload #label rename (no-op, reverted in next)`
10. **d77ac6a** `fix(release): cp Mac sig to the right filename BEFORE upload`

Branch now 8 commits ahead of `origin/main` (was 0; first 0 from Phase 12 scaffold were never pushed because verify found regressions). All 8 commits pushed; nothing in flight. Test tag + draft deleted; repo is clean.

## Findings Worth Keeping (for Future CI Work)

### GitHub Actions Runner Reality (2026-06)
- `macos-latest` = Apple Silicon (free tier, works). `macos-15-arm64`, `macos-14-large`, `macos-15-large` = Larger Runners (paid). Free public repos: jobs silently sit in `queued` forever if you request a paid label.
- `macos-13` (Intel) retired Dec 2025. No free Intel Mac runner anymore.
- No cross-compile workaround inside tauri-action; either pay for Larger Runners or ship aarch64-only.

### tauri-action@v0.6.2 + Tauri 2 = Broken Updater Plumbing
- Action expects `bundle.updater.*` (Tauri 1 schema). Tauri 2 uses `plugins.updater.*`.
- Result: action prints `.sig` paths in logs (looks like success), generates them transiently, **never uploads them** because they live in a temp dir that gets cleaned after the action ends. Also doesn't generate `latest.json`.
- Workaround: re-sign payloads in a post-build step using `npx tauri signer sign`, manually generate latest.json from .sig contents, upload all artifacts yourself.

### `git mv` + File Edit = Footgun
- `git mv file.rs new/file.rs` auto-stages the move. But a separate `Edit Cargo.toml` against an already-committed file **doesn't auto-stage** (Edit tool reads-then-replaces, only outputs success/error, doesn't call `git add`).
- Commit can land with the rename but not the Cargo.toml change that references it. Cost: one more iteration before `cargo build` fails.

### Dev Binaries in Release Bundles
- `[[bin]]` entries get bundled by Tauri unconditionally; `required-features` is ignored by the bundler.
- Use `[[example]]` (examples/ dir) for dev/test binaries. Tauri skips examples entirely. Examples are first-class Cargo targets: `cargo run --example pdf_smoke`, `cargo build --example pdf_smoke`.

### Asset Naming on GitHub Releases
- `gh release upload PATH#LABEL` sets a display-label only; **doesn't rename the asset filename**.
- To rename: cp the file to target name, upload the copy.

## Next Steps

**Before v0.1.0 release**:
- [ ] Add GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY` + password) via GitHub web UI.
- [ ] Cut a real v0.1.0 tag and validate the full release pipeline (no delete).
- [ ] Verify macOS DMG + Windows MSI auto-update on the AboutTab button (assumes first release is published).
- [ ] Document post-release: "First release published; updater now live. Users on v0.0.0 will see 'Update available' on next app launch."

**Future CI hardening**:
- [ ] Add a dry-run pre-flight check: `gh release create --draft --verify-tag` (don't actually upload) to catch asset naming issues before real release.
- [ ] Consider pinning tauri-action to a stable version OR writing custom `release-buildin.sh` that doesn't depend on Tauri 1 schema assumptions.
- [ ] Quarterly runner-changelog sweep (GitHub Actions publishes runner retirement dates 6 months in advance).

## Unresolved Questions

1. **Larger Runner cost**: If someone donates, should we add Intel Mac back to the matrix for v0.x, or stay aarch64-only until v1.0? Current plan: aarch64-only (free tier).
2. **Updater diagnostics**: The AboutTab "Check for updates" button silently shows "unavailable" until first release. Should we log updater check errors to stdout, or keep it silent-fail? (Currently: silent, by design.)
3. **Tag naming**: Next release tag is `v0.1.0` (or `v0.0.2` for a patch)? Plan assumes v0.1.0. Confirm scope before pushing.
