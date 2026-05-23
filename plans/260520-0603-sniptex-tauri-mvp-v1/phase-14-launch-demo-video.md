---
phase: 14
title: "Demo Video & Marketing Assets"
status: pending
priority: P2
effort: "2d"
dependencies: [13]
---

# Phase 14: Demo Video & Marketing Assets

## Overview

Create a 60-second demo video showing the end-to-end SnipTeX workflow (hotkey → capture → OCR → preview → clipboard → paste into editor), plus marketing assets for launch: social media banners, Product Hunt assets, and screenshot gallery. The video serves as the hero element on the landing page (Phase 13) and primary content for social launch posts (Phase 15).

## Key Insights

- 60s demo format: 5s intro (logo + tagline) → 45s workflow demo → 10s CTA (download links + GitHub star).
- Record on Mac with clean desktop; show both equation-only and mixed-content snips.
- Use actual OCR output (not fake) — demonstrates real accuracy and speed.
- Product Hunt requires: 1270x760 gallery images, app icon (240x240), maker tagline.
- Social banners: Twitter/X (1200x675), Facebook/LinkedIn (1200x630), Reddit (no specific size, 16:9 preferred).
- Vietnamese community posts need Vietnamese captions/text overlays on separate assets.

## Requirements

**Functional**
- 60-second demo video (1080p, 30fps, MP4 + WebM)
- Video shows: hotkey press → region selector → capture → processing indicator → preview window with rendered LaTeX → auto-copy → paste into document editor
- At least 2 demo scenarios: (1) equation-only from SGK Toán, (2) mixed text+equation content
- Show cloud mode speed (~3s) as the "wow" moment
- Social media banners (Twitter, Facebook/LinkedIn, Reddit)
- Product Hunt gallery images (5 screenshots, 1270x760)
- Product Hunt icon (240x240, app icon on white background)
- GitHub README hero GIF (800px wide, <5MB, 15s loop of core workflow)
- Open Graph image for landing page (1200x630)

**Non-functional**
- Video file size < 15MB (MP4) for fast loading on landing page
- All assets use consistent brand colors and typography
- No watermarks, no paid tools required for reproduction

## Architecture

### Video Structure (60s)

```
[0-5s]   Logo animation + "SnipTeX — Free OCR for LaTeX & Markdown"
[5-15s]  Scenario 1: Equation-only snip
         - Show PDF/textbook with equation on screen
         - Press ⌘⇧M → crosshair appears
         - Drag region → processing → preview shows rendered LaTeX
         - Paste into Overleaf/LaTeX editor → compiles correctly
[15-30s] Scenario 2: Mixed content (Vietnamese SGK)
         - Show SGK page with text + equations + table
         - Snip → Markdown output with inline math
         - Paste into Notion/Obsidian → renders correctly
[30-40s] Settings quick tour
         - Agent selection, theme toggle, format options
[40-50s] Cloud mode demo
         - Toggle cloud mode → snip → result in ~3 seconds
         - "Sub-5-second response with your own API key"
[50-60s] CTA
         - Download buttons (Mac + Windows)
         - GitHub star count + "Free & open source"
         - "sniptex.github.io" or GitHub URL
```

### Asset Inventory

| Asset | Size | Format | Used In |
|-------|------|--------|---------|
| Demo video | 1920x1080 | MP4 + WebM | Landing page hero, YouTube, social |
| Hero GIF | 800xauto | GIF | GitHub README |
| OG image | 1200x630 | PNG | Landing page meta, link previews |
| Twitter banner | 1200x675 | PNG | Twitter/X launch post |
| FB/LinkedIn banner | 1200x630 | PNG | Facebook, LinkedIn posts |
| PH gallery | 1270x760 x5 | PNG | Product Hunt listing |
| PH icon | 240x240 | PNG | Product Hunt listing |
| App screenshots | 1280x800 x4 | PNG | Landing page, GitHub README |

## Related Code Files

- Create: `assets/video/demo-script.md` — scene-by-scene script with timings
- Create: `assets/video/demo-raw.mp4` — raw screen recording
- Create: `assets/video/demo-final.mp4` — edited 60s demo (1080p)
- Create: `assets/video/demo-final.webm` — WebM version for web
- Create: `assets/images/hero.gif` — 15s looping GIF for README
- Create: `assets/images/og-image.png` — Open Graph image
- Create: `assets/images/social/twitter-banner.png`
- Create: `assets/images/social/fb-linkedin-banner.png`
- Create: `assets/images/producthunt/` — gallery images + icon
- Create: `assets/images/screenshots/` — 4 app screenshots
- Modify: Phase 13 landing page — embed video + OG image

## Implementation Steps

1. Write demo script (`demo-script.md`) with exact scene timings and narration cues.
2. Prepare demo environment:
   - Clean Mac desktop, minimal dock
   - Open PDF/textbook with target equations
   - Open Overleaf in browser tab (for paste demo)
   - Open Notion/Obsidian (for Markdown paste demo)
   - Ensure Codex CLI installed and working
   - Set cloud mode with valid API key for speed demo
3. Record screen using macOS built-in (`⌘⇧5`, full screen, 1080p).
4. Record 3-4 takes of the full workflow; pick best.
5. Edit video:
   - Add intro (logo + tagline, 5s)
   - Cut + arrange demo scenes per script
   - Add subtle zoom on key moments (hotkey press, preview window)
   - Add text overlays for context ("Equation-only mode", "Mixed content", "Cloud mode: ~3s")
   - Add outro with CTA
   - Export MP4 (H.264, ~10Mbps) + WebM (VP9, ~5Mbps)
6. Create hero GIF from video: extract 15s core workflow, resize to 800px wide, optimize < 5MB.
7. Create social media banners:
   - Base template with app icon, tagline, key screenshot
   - Adapt dimensions per platform
8. Create Product Hunt assets:
   - 5 gallery screenshots showing: (1) main capture flow, (2) preview window, (3) settings, (4) history, (5) onboarding
   - App icon on white background (240x240)
9. Create OG image: app name + tagline + mini screenshot composite.
10. Create 4 app screenshots for landing page + README.
11. Update landing page (Phase 13) with video embed + OG meta tags.
12. Update README.md with hero GIF.

## Todo List

- [ ] Write demo script with scene timings
- [ ] Prepare demo environment (clean desktop, apps open)
- [ ] Record screen captures (3-4 takes)
- [ ] Edit 60s demo video (MP4 + WebM)
- [ ] Create hero GIF for README (15s, <5MB)
- [ ] Create OG image (1200x630)
- [ ] Create Twitter banner (1200x675)
- [ ] Create Facebook/LinkedIn banner (1200x630)
- [ ] Create Product Hunt gallery (5 x 1270x760)
- [ ] Create Product Hunt icon (240x240)
- [ ] Create 4 app screenshots
- [ ] Embed video in landing page
- [ ] Add OG meta tags to landing page
- [ ] Add hero GIF to README

## Success Criteria

- [ ] Demo video clearly shows end-to-end workflow in 60 seconds
- [ ] Video demonstrates real OCR output (not mocked)
- [ ] Cloud mode ~3s response visible in video
- [ ] Hero GIF loops cleanly, < 5MB, shows core capture→preview flow
- [ ] All social banners match brand style, correct dimensions
- [ ] Product Hunt assets pass PH dimension requirements
- [ ] OG image renders correctly in Twitter/Facebook/Slack link previews

## Risk Assessment

- **Risk: OCR demo fails during recording** — Mitigation: pre-test all demo images with the agent; have backup takes; worst case, use best successful take and trim failure.
- **Risk: Video file too large for landing page** — Mitigation: compress aggressively (CRF 28-30 for H.264); use lazy-load + poster image; host on GitHub Releases if too large for repo.
- **Risk: GIF too large (>5MB)** — Mitigation: reduce to 10s, lower FPS to 10, reduce colors; consider APNG or short autoplay MP4 as alternative.

## Security Considerations

- Blur or crop any personal information visible on screen during recording.
- Do not show API keys, file paths with real usernames, or sensitive content in demo.
- Use generic demo content (public domain textbook equations).

## Next Steps

- Phase 15 (Launch) uses video + banners for social media posts and Product Hunt listing
