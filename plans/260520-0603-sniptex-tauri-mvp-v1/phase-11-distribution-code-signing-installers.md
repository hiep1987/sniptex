---
phase: 11
title: "Distribution: Code Signing & Installers"
status: complete-arm
priority: P2
effort: "2d"
dependencies: [10]
completed: "2026-06-06"
---

## Status — 2026-06-06

✅ **Mac ARM**: build + ad-hoc sign + DMG bundle + Cask formula + install-guide
✅ **Code review** by `code-reviewer` agent: 5 High findings, all resolved
   (see `reports/reviewer-260606-phase-11-distribution.md`)
⏸ **Mac Intel** (`x86_64-apple-darwin`): deferred to Phase 12 CI (`macos-latest` runner)
⏸ **Windows MSI**: deferred to Phase 12 CI (`windows-latest` runner)
⏸ **Live `brew install --cask` smoke test**: requires v0.1.0 release tag — runs in Phase 12

**Artifacts produced this round:**
- `src-tauri/tauri.conf.json` — added `bundle.macOS.{minimumSystemVersion: "12.0", signingIdentity: "-"}`
- `scripts/sign-mac.sh` — ad-hoc codesign + verify (also covered by signingIdentity flag at build time)
- `Casks/sniptex.rb` — Cask formula for `hiep1987/sniptex` v0.1.0, zap covers both `com.sniptex.app` (Tauri default) and `com.sniptex` (keychain fallback)
- `docs/install-guide.md` — Mac (Monterey–Sequoia) + Windows (SmartScreen + Smart App Control)
- `README.md` — minimal project README with install pointer to install-guide
- DMG built: `SnipTeX_0.1.0_aarch64.dmg` — 19 MB, SHA256 `13a5ea48b26fea2e5aba14bade0ef0c833c52e4f5bc1d8425e2e3e13e3515124`

# Phase 11: Distribution: Code Signing & Installers

## Overview

Prepare distribution artifacts for Mac and Windows. Mac: ad-hoc code sign the `.app` bundle, create DMG with drag-to-Applications layout, write Homebrew Cask formula. Windows: build MSI installer, document SmartScreen workaround. Apple Developer Program notarization is **deferred until donations cover $99/yr** (confirmed Session 1 Q7). Windows code signing is also deferred (~$200/yr).

## Key Insights

- **Mac ad-hoc signing**: `codesign --sign - --deep --force SnipTeX.app` — removes the "damaged app" dialog but does NOT bypass Gatekeeper. Users installing via DMG still need "Right-click → Open" or `xattr -cr`.
- **Homebrew Cask is the primary Mac distribution channel** — Cask installs bypass Gatekeeper entirely, making the unsigned-app issue invisible to brew users.
- **Windows MSI**: Tauri builds MSI via WiX; unsigned MSI triggers SmartScreen warning ("Windows protected your PC"). User clicks "More info → Run anyway". Reputation builds after ~3,000 downloads.
- **DMG layout**: background image + app icon + Applications shortcut. Tauri supports DMG via `tauri build --bundles dmg`.
- **Homebrew Cask PR**: submit to `homebrew/homebrew-cask` repo. Requires: stable release tag, DMG SHA256, formula file.

## Requirements

**Functional**
- Mac DMG installer with drag-to-Applications layout
- Mac app bundle ad-hoc signed (`codesign --sign -`)
- Homebrew Cask formula for `brew install --cask sniptex`
- Windows MSI installer (from Phase 10 build)
- README/docs: clear Gatekeeper workaround instructions (Mac)
- README/docs: clear SmartScreen workaround instructions (Windows)

**Non-functional**
- DMG size < 25MB
- MSI size < 20MB
- Cask formula passes `brew audit --cask sniptex`

## Architecture

### Distribution Matrix

| Platform | Format | Signing | Channel |
|----------|--------|---------|---------|
| Mac (ARM) | `.dmg` | Ad-hoc | GitHub Releases + Homebrew Cask |
| Mac (Intel) | `.dmg` | Ad-hoc | GitHub Releases + Homebrew Cask |
| Windows (x64) | `.msi` | Unsigned | GitHub Releases |

### Homebrew Cask Formula

```ruby
cask "sniptex" do
  version "1.0.0"
  sha256 "SHA256_OF_DMG"

  url "https://github.com/USER/sniptex/releases/download/v#{version}/SnipTeX_#{version}_aarch64.dmg"
  name "SnipTeX"
  desc "Free OCR snip tool for LaTeX and Markdown"
  homepage "https://github.com/USER/sniptex"

  depends_on macos: ">= :monterey"

  app "SnipTeX.app"

  zap trash: [
    "~/Library/Application Support/com.sniptex.app",
    "~/Library/Preferences/com.sniptex.app.plist",
  ]
end
```

## Related Code Files

- Create: `scripts/sign-mac.sh` — ad-hoc codesign + verify script
- Create: `scripts/build-dmg.sh` — build DMG with custom layout (or rely on Tauri's built-in DMG)
- Create: `Casks/sniptex.rb` — Homebrew Cask formula (submitted to homebrew-cask repo)
- Modify: `src-tauri/tauri.conf.json` — DMG settings (background, icon size, window position)
- Create: `docs/install-guide.md` — Gatekeeper + SmartScreen workaround instructions
- Modify: `README.md` — installation section with platform-specific instructions

## Implementation Steps

1. Configure Tauri DMG settings in `tauri.conf.json`:
   - Background image (1x + 2x)
   - Icon size, window dimensions
   - Applications shortcut position
2. Build Mac release: `npx tauri build --target aarch64-apple-darwin` and `--target x86_64-apple-darwin`.
3. Ad-hoc sign: `codesign --sign - --deep --force target/release/bundle/macos/SnipTeX.app`.
4. Verify: `codesign --verify --verbose SnipTeX.app` — should show "valid on disk, satisfies its Designated Requirement".
5. Create `scripts/sign-mac.sh` automating steps 3-4.
6. Test DMG: mount, drag to Applications, launch, verify Gatekeeper behavior (Right-click → Open works).
7. Write Homebrew Cask formula with correct SHA256 and URL pattern.
8. Test Cask locally: `brew install --cask ./Casks/sniptex.rb`, verify install + launch + uninstall.
9. Build Windows MSI: `npx tauri build --target x86_64-pc-windows-msvc` (from Phase 10).
10. Test MSI: install on clean Windows, verify SmartScreen dialog, document click-through steps.
11. Write `docs/install-guide.md` with screenshots of Gatekeeper and SmartScreen dialogs + workaround steps.
12. Update README.md installation section.

## Todo List

- [x] Configure DMG layout in tauri.conf.json (minimumSystemVersion + signingIdentity)
- [x] Build Mac ARM DMG (Intel deferred to Phase 12 CI)
- [x] Ad-hoc sign Mac app bundle (via Tauri `signingIdentity: "-"` + sign-mac.sh)
- [x] Verify codesign on Mac (`valid on disk, satisfies its Designated Requirement`)
- [x] Create sign-mac.sh script
- [x] Test DMG install flow + Gatekeeper workaround (mount + layout verified; first-launch user steps documented)
- [x] Write Homebrew Cask formula (`brew style` passes; zap covers both bundle ID + keychain fallback)
- [x] Test Cask install locally (static: brew style + ruby syntax; live install needs release tag → Phase 12)
- [ ] Build Windows MSI (deferred to Phase 12 CI windows-latest runner)
- [ ] Test MSI install + SmartScreen workaround (deferred; flow documented in install-guide.md)
- [x] Write install-guide.md (no screenshots yet — text-only walkthroughs for Mac Monterey–Sequoia + Win 11 SmartScreen + Smart App Control)
- [x] Update README.md installation section (created from scratch — root README didn't exist)

## Success Criteria

- [x] Mac DMG mounts, drag-to-Applications shortcut present, `.app` inside signed (live drag-to-launch not exercised this round — defer to Phase 12 release smoke)
- [x] `codesign --verify` passes on ad-hoc signed bundle (`valid on disk, satisfies its Designated Requirement`)
- [ ] `brew install --cask ./sniptex.rb` installs and launches successfully (deferred — requires v0.1.0 release tag + tap; static formula validation passes today)
- [ ] Windows MSI installs on clean Windows, app runs after SmartScreen click-through (deferred to Phase 12 CI)
- [x] Install guide covers both platforms with clear step-by-step (screenshots deferred to Phase 13 landing-page work)

## Risk Assessment

- **Risk: Homebrew Cask PR rejected due to low download count** — Mitigation: self-host tap (`homebrew-sniptex`) initially; submit to official cask after reaching minimum download threshold.
- **Risk: Mac ad-hoc sign insufficient for some corporate environments** — Mitigation: document `xattr -cr` fallback; notarization deferred to post-donation milestone.
- **Risk: Tauri DMG builder doesn't support custom background** — Mitigation: use `create-dmg` npm package as fallback.

## Security Considerations

- Ad-hoc signing provides no identity verification — users must trust the GitHub release.
- Document SHA256 checksums in GitHub Release notes for manual verification.
- Homebrew Cask auto-verifies SHA256 on install.
- **Apple Developer Program deferred** — donation goal: $99/yr. Once funded: notarize + staple for seamless Gatekeeper experience.
- **Windows code signing deferred** — ~$200/yr for EV cert. SmartScreen reputation builds organically with downloads.

## Next Steps

- Phase 12 (CI/CD) automates the build + sign + release process
- Phase 15 (Launch) submits Homebrew Cask PR to official repo
