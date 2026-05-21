---
phase: 15
title: "Package Managers & Soft Launch"
status: pending
priority: P1
effort: "3d"
dependencies: [14]
---

# Phase 15: Package Managers & Soft Launch

## Overview

Submit Homebrew Cask + Winget manifests, tag and publish `v1.0.0` via the CI pipeline from Phase 12, soft-launch across the target channels (FB groups, Reddit, HackerNews, ProductHunt, Tinhte, Voz, X/Twitter), set up funding pages (Open Collective + GitHub Sponsors), and monitor first-week metrics to triage urgent bugs. Launch messaging uses the **"BYOA or BYOK — your choice"** framing locked in Validation Session 3.

<!-- Updated: Validation Session 3 (2026-05-21) - Path C hybrid: launch posts mention 3 OCR paths (Codex CLI default, Gemini CLI experimental, Gemini API cloud BYOK); sub-5s response is a marketing talking point for cloud mode -->


## Key Insights

- Homebrew Cask requires the formula to point to **stable, versioned** release URLs — don't push Cask PR until `v1.0.0` is publicly released.
- Winget manifest auto-generated via `wingetcreate` from MSI URL; submit via `winget-pkgs` repo PR.
- Soft launch order matters: niche communities first (validates messaging) → ProductHunt + HN (mass reach) → press if traction.
- Launch day = highest bug-report volume; have a triage system ready (GitHub Issues templates, response SLAs).

## Requirements

**Functional**
- `v1.0.0` tag pushed; CI pipeline produces and publishes the release
- Homebrew Cask PR open and merged → `brew install --cask sniptex` works
- Winget manifest PR open and merged → `winget install SnipTeX` works
- Landing page live with working downloads
- Open Collective + GitHub Sponsors set up with goals
- Launch posts published on at least 7 of the planned channels
- Issue templates ready: bug report, feature request, agent-not-detected, install-failure
- **Discord** community channel created and linked from README + landing footer + Settings → About

**Non-functional**
- Triage response within 24h during launch week
- Track first-week metrics: downloads, GitHub stars, signups, install funnel completion

## Architecture

Launch channels (from `replan.md` §9 Week 5):

```
Order        Channel                          Audience                 Effort
1            FB "Giáo viên Toán THPT"         VN teachers              Low
2            r/LaTeX                          LaTeX power users        Low
3            r/macapps                        Mac early adopters       Low
4            r/Windows / r/software           Windows users            Low
5            HackerNews "Show HN"             Tech crowd               Medium
6            ProductHunt                      Maker community          High
7            Tinhte                           VN tech enthusiasts      Low
8            Voz                              VN forum                 Low
9            Twitter/X #LaTeX #OCR            Niche + amplification    Low
10 (later)   Press: TechCrunch / Genk         If traction              High
```

Funding setup:

```
Open Collective: collective.sniptex (open transparent)
GitHub Sponsors: github.com/sponsors/<maintainer>
Goal: $99 first tier → Apple Developer Program (notarize)
```

## Related Code Files

- Create: `Casks/sniptex.rb` PR to `Homebrew/homebrew-cask`
- Create: Winget manifest YAMLs via `wingetcreate`, PR to `microsoft/winget-pkgs`
- Modify: `README.md` — add badges (stars, downloads, license), install commands at top
- Create: `.github/ISSUE_TEMPLATE/` templates (bug-report.yml, feature-request.yml, agent-not-detected.yml, install-failure.yml)
- Create: `assets/marketing/launch-posts/` — drafts per channel
- Modify: `docs/src/components/DonateBadges.astro` — point to live OC + Sponsors

## Implementation Steps

1. Final QA pass — go through every phase's success criteria one more time on Mac + Windows fresh installs.
2. Tag `v1.0.0` from main: `git tag v1.0.0 && git push --tags`. CI runs Phase 12 release workflow.
3. After CI uploads draft release, manually publish on GitHub.
4. Update `Casks/sniptex.rb` with real `sha256` values from published DMGs (one per arch). Submit PR to `Homebrew/homebrew-cask`.
5. Generate Winget manifest: `wingetcreate new` against published MSI URL. Submit PR to `microsoft/winget-pkgs`. Note: review can take 1-3 days.
6. Set up Open Collective: register collective, link bank, write mission ("Build free OCR snip tool for teachers and students"), set Apple Developer Program $99 as first goal.
7. Set up GitHub Sponsors: enable on profile, create tier $5/$10/$25.
8. Update `README.md`:
   - Badges row (stars, downloads, license, OC, Sponsors)
   - Install commands at top (`brew install --cask sniptex`, `winget install SnipTeX`, direct DMG/MSI links)
   - Demo GIF embed
   - Vietnamese section
   - Contributing link
9. Create issue templates (.github/ISSUE_TEMPLATE/*.yml) with structured fields.
10. Create **Discord server** for community (decision locked in Validation Session 1); link from README + Settings → About + landing footer.
11. Draft launch posts per channel (English + Vietnamese where appropriate). Common framing: **"BYOA or BYOK — your choice"** — Codex CLI (default, privacy-first) / Gemini CLI (experimental) / Gemini API key (sub-5s cloud).
    - HN Show HN: title "Show HN: SnipTeX – free OCR snip tool for LaTeX & Markdown (Mac + Windows)"; first comment positions the 3-path choice (privacy-first BYOA vs sub-5s cloud BYOK) as the differentiator vs Mathpix.
    - ProductHunt: tagline "Free Mathpix alternative · BYOA or BYOK · Mac + Windows · MIT"; first comment expands on the privacy story.
    - r/LaTeX: lead with use case + GIF; surface "no API key needed if you already have Codex/Gemini CLI installed".
    - r/macapps & r/Windows: lead with hotkey-to-clipboard speed in the cloud-mode GIF (≤5s response).
    - FB VN: VN copy targeting SGK teacher pain point; surface **hai lựa chọn** (CLI riêng tư vs API key nhanh ~5s).
12. Stagger publish order across 3 days: niche communities Day 1, ProductHunt + HN Day 2, broader Day 3.
13. Monitor + triage:
    - Pin a launch issue summarizing known issues
    - Respond to top comments + DMs within 24h
    - Daily download count tracking (GitHub Releases API)
    - Crash report monitoring (if Tauri telemetry enabled later)
14. Post-launch retro after Day 7 — write `plans/260520-0603-sniptex-tauri-mvp-v1/reports/launch-retro.md` capturing what worked, what didn't, top user requests, and roadmap for v1.1.

## Todo List

- [ ] Final QA on Mac + Windows fresh installs
- [ ] Tag v1.0.0, run CI release, publish GitHub Release
- [ ] Update Cask formula with real sha256 + PR to homebrew-cask
- [ ] Generate Winget manifest + PR to winget-pkgs
- [ ] Set up Open Collective with $99 Apple goal
- [ ] Set up GitHub Sponsors tiers
- [ ] Update README with badges, install, demo GIF, VN section
- [ ] Create 4 issue templates
- [ ] Create Discord community server, link from README + landing footer + Settings About
- [ ] Draft 9 launch posts (EN + VN where applicable)
- [ ] Publish per channel order, staggered across 3 days
- [ ] Triage user reports within 24h SLA
- [ ] Track first-week metrics
- [ ] Write launch-retro.md after Day 7

## Success Criteria

- [ ] v1.0.0 published on GitHub Releases with Mac + Windows artifacts
- [ ] `brew install --cask sniptex` works post-PR merge
- [ ] `winget install SnipTeX` works post-PR merge
- [ ] Launch posts published on ≥7 of 9 planned channels
- [ ] Open Collective + GH Sponsors live with $99 goal visible
- [ ] First-week metrics tracked (downloads, stars, issues, signups)
- [ ] Top 5 user-reported issues acknowledged in GitHub within 48h

## Risk Assessment

- **Risk: HN/ProductHunt traffic overwhelms support** — Mitigation: launch issue with known issues pinned; canned responses for top patterns.
- **Risk: Homebrew or Winget PR rejected on formatting** — Mitigation: pre-validate (`brew style sniptex.rb`, `winget validate`); keep our own tap as fallback for Mac.
- **Risk: A blocker bug surfaces on launch day** — Mitigation: CI release pipeline can ship `v1.0.1` patch within 30 minutes via Phase 12 workflow.
- **Risk: Negative reception around BYOA model** — Mitigation: lead messaging with "free, privacy-first, open source"; BYOA is the why, not the headline. For users put off by CLI install friction, surface the BYOK cloud-API path (sub-5s, just paste a free Google AI Studio key) as the easier on-ramp. The "or" in "BYOA or BYOK" defuses the friction objection.

## Security Considerations

- Final binary scan with `clamav` or VirusTotal before publishing to verify no false positives in fresh AV definitions.
- Make sure no API keys or secrets bundled in release artifacts.

## Next Steps

- After Day 7: write launch-retro.md and plan v1.1 (Claude Code agent, OpenCode, additional formats, Linux build)
- File Apple Developer Program enrollment when OC reaches $99
- Track community contributions: triage PRs, label issues, recognize contributors in README

## Open Questions

- Reach out to Vietnamese edtech press (Genk, Tinhte editorial) Day 4 or wait for traction signal? Default: wait for ≥1k downloads then pitch.

<!-- Updated: Validation Session 1 - Discord locked as v1 community channel -->
