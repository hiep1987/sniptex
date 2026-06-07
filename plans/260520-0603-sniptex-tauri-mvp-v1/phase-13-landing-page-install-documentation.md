---
phase: 13
title: "Landing Page & Install Documentation"
status: mvp-scaffolded
priority: P2
effort: "2d"
dependencies: [12]
completed: "2026-06-08"
---

## Status — 2026-06-08

✅ **Astro project at `site/`** (NOT `docs/` as plan originally said — keeps docs/ for engineering markdown)
✅ **6 of 9 components shipped**: Hero, DownloadButtons, InstallGuide, VietnameseSEO, DonateBadges, Footer (+ BaseLayout)
✅ **GitHub Pages deploy workflow** (`.github/workflows/deploy-pages.yml`): push-to-main on `site/**`, uses `actions/upload-pages-artifact@v3` + `actions/deploy-pages@v4`
✅ **Build verified locally**: 20 KB index.html + 20 KB CSS + 480 B favicon (under <500 KB target)
✅ **Code review** by `code-reviewer` agent: 2 Critical + 3 High + 4 Medium findings — C1/C2/H2 fixed pre-commit
   (see `reports/reviewer-260608-phase-13-landing.md`)

⏸ **HowItWorks** (3-step walkthrough) — defer to follow-up round
⏸ **FeatureGrid** (6-cell feature highlights) — defer
⏸ **DemoVideo** — Phase 14 ships the video first
⏸ **OG image, screenshots, demo-poster** — `ui-ux-designer` agent round
⏸ **MDX install content collections** — current install-guide.astro inlines copy; revisit if drift grows
⏸ **Lighthouse audit ≥95** — needs live deploy + Chrome
⏸ **Cross-browser smoke test** — needs live deploy
⏸ **Repo Settings → Pages source = "GitHub Actions"** — user-driven one-time toggle (documented in `docs/releasing.md`?)
⏸ **Aspirational install commands** (`brew install --cask sniptex`, PowerShell `irm | iex`) — marked "SOON" badge until v0.1.0 launch, Phase 15 owns the actual tap + script

**Artifacts produced this round:**
- `site/` (new Astro project): package.json, astro.config.mjs, tailwind.config.mjs, tsconfig.json, .gitignore
- `site/src/layouts/base-layout.astro`
- `site/src/components/{hero,download-buttons,install-guide,vietnamese-seo,donate-badges,footer}.astro`
- `site/src/pages/index.astro`
- `site/src/styles/global.css`
- `site/public/favicon.svg`
- `.github/workflows/deploy-pages.yml`

**Deferred review findings (not fixed this round, tracked):**
- H1: `og-image.png` referenced but missing → social shares 404. Fix when ui-ux-designer round generates real assets.
- H3: install-guide.astro slightly diverges from `docs/install-guide.md` (Smart App Control trade-off line missing checksum verify + Reputation-based protection fallback). Sync when adding MDX content collections.
- M1: clipboard Copy button visible on touch via `sm:opacity-0` (fix applied to BOTH brew + PowerShell blocks); MVP good.
- M2-M4: canonical URL trailing-slash, hreflang for VN, Pages source toggle — track for v0.1.0 polish.

# Phase 13: Landing Page & Install Documentation

## Overview

Build a static Astro landing page on **GitHub Pages** (no custom domain in v1 per `replan.md` §13) showcasing SnipTeX with hero demo video, "How it works" 3-step walkthrough, download buttons, install guides, Vietnamese SEO section, and Open Collective + GitHub Sponsors donate links. Hero + Feature copy uses the **BYOA or BYOK** framing locked in Validation Session 3.

<!-- Updated: Validation Session 3 (2026-05-21) - Path C hybrid: hero copy + FeatureGrid mention cloud-API option; VN SEO mentions both CLI and API-key paths -->


## Key Insights

- GitHub Pages source: `/docs` directory of the main repo OR a dedicated `gh-pages` branch. v1: `/docs` directory for simplicity (already exists per `replan.md` §2 repo structure).
- Astro static export keeps page weight under 100KB initial paint. No SPA needed.
- Vietnamese SEO section is the wedge for VN users searching "công cụ chụp công thức toán" / "OCR LaTeX miễn phí" / "chuyển ảnh sang LaTeX".
- Download buttons should detect OS and prioritize correct artifact.
- Demo video embedded from YouTube (uploaded Phase 14).

## Requirements

**Functional**
- Static site builds via `astro build` to `docs/dist/` then commits to `docs/` for GH Pages
- Sections: Hero / Demo / How it works / Features / Download / Install Guide / Vietnamese SEO / Donate / Footer
- Download buttons link to GitHub Releases latest assets
- Install Guide section embeds workaround steps from Phase 11 docs
- "Open in your terminal" code blocks copy-to-clipboard on click
- Vietnamese SEO: dedicated `#vi` anchor section with VN-language copy + keywords
- Donate badges: Open Collective + GitHub Sponsors

**Non-functional**
- Lighthouse Performance ≥95
- First Contentful Paint <1s on cable connection
- Total page weight <500KB excluding embedded video poster

## Architecture

```
docs/                            (Astro project, deployed to gh-pages root)
├── astro.config.mjs             site: https://<user>.github.io/sniptex/
├── package.json
├── src/
│   ├── pages/
│   │   └── index.astro          (full landing page, single route)
│   ├── components/
│   │   ├── Hero.astro
│   │   ├── DemoVideo.astro      (YouTube iframe lazy-loaded)
│   │   ├── HowItWorks.astro     (3 steps with screenshots)
│   │   ├── FeatureGrid.astro
│   │   ├── DownloadButtons.astro (OS-aware via JS sniffing)
│   │   ├── InstallGuide.astro
│   │   ├── VietnameseSEO.astro
│   │   ├── DonateBadges.astro
│   │   └── Footer.astro
│   ├── layouts/
│   │   └── BaseLayout.astro     (HTML head, OG tags, theme)
│   ├── content/
│   │   ├── install-mac.mdx
│   │   ├── install-windows.mdx
│   │   └── troubleshooting.mdx
│   └── styles/
│       └── global.css
├── public/
│   ├── og-image.png             (1200x630)
│   ├── favicon.ico
│   ├── demo-poster.jpg          (1280x720, YouTube poster)
│   └── screenshots/             (1-5 product shots)
└── dist/                        (build output, served by GH Pages)
```

## Related Code Files

- Create entire Astro project under `docs/`
- Modify: `.github/workflows/deploy-pages.yml` — Astro build + deploy to `gh-pages` branch
- Modify: GitHub repo settings → Pages → source = `gh-pages` branch
- Modify: `tauri.conf.json` plugins.updater endpoint → `https://<user>.github.io/sniptex/latest.json` (alternative endpoint), keep GitHub Releases URL as primary

## Implementation Steps

1. Scaffold: `pnpm create astro@latest docs --template minimal --typescript strict --no-install`, then `pnpm install` inside.
2. Configure `astro.config.mjs`: `site: 'https://<user>.github.io/sniptex/'`, `base: '/sniptex/'`, integrations: `@astrojs/tailwind`, `@astrojs/mdx`.
3. Build `BaseLayout.astro`: HTML head with `<title>SnipTeX — Free OCR snip tool for LaTeX & Markdown</title>`, OG tags, theme bootstrap inline script, fonts.
4. Build `Hero.astro`: H1 "SnipTeX — free OCR snip tool for LaTeX & Markdown", subtitle "**Bring your own agent OR your own API key — your choice.** Codex CLI, Gemini CLI, or Gemini API key for sub-5-second cloud response." Primary CTA (Download for Mac), secondary CTA (Download for Windows). Tertiary "How it works" anchor.
5. Build `DemoVideo.astro`: lazy-loaded YouTube iframe with poster image fallback; replaces poster with iframe on click.
6. Build `HowItWorks.astro`: 3 numbered steps with screenshots from `public/screenshots/`.
   - 1. Press hotkey, drag region
   - 2. Wait for OCR (1-3s)
   - 3. Paste anywhere — LaTeX, Markdown, MathML
7. Build `FeatureGrid.astro`: 6-cell grid:
   - Privacy first (BYOA local mode — image stays between you and your CLI agent)
   - **3 OCR paths** (Codex CLI default · Gemini CLI experimental · Gemini API for sub-5s cloud — your choice)
   - Cross-platform (Mac + Windows; Linux in v1.x)
   - <20MB binary, <100MB RAM
   - Open source MIT
   - Smart formatter (LaTeX equations, Markdown tables, mixed pages)

   Roadmap note: "more agents (Claude Code, OpenCode) in v1.x".
8. Build `DownloadButtons.astro`:
   - Detects `navigator.userAgent` for Mac vs Windows
   - Primary: download link for detected OS pulled from latest GitHub Release via small build-time fetch (Astro `getStaticPaths`)
   - Secondary: "Install via brew/winget" code blocks with copy buttons
9. Build `InstallGuide.astro`: includes `<MDXContent>` from install-mac.mdx and install-windows.mdx; expandable accordions per OS; troubleshooting accordion.
10. Build `VietnameseSEO.astro`: VN-language section targeting search keywords:
    - H2 "Công cụ OCR LaTeX miễn phí cho giáo viên và sinh viên Việt Nam"
    - Use cases for SGK Toán
    - Install guide in VN — **hai lựa chọn**: (1) cài Codex/Gemini CLI nếu muốn riêng tư, (2) dán Google AI Studio API key vào app nếu muốn nhanh (~5s response).
    - Bullet list of features in VN
    - Backlink anchor `#vi` for in-app referrals
11. Build `DonateBadges.astro`: Open Collective + GitHub Sponsors widgets/links + Apple Developer fundraising progress bar (`$0 / $99`).
12. Build `Footer.astro`: GitHub repo link, issue tracker, **Discord community link**, license, version (pulled from package.json at build).
13. Create OG image (1200x630) and screenshots via `ai-multimodal` skill or designer.
14. Set up GH Pages deploy workflow: build Astro on push to main, push `dist/` to `gh-pages` branch using `peaceiris/actions-gh-pages@v4`.
15. Configure GH repo: Pages source = `gh-pages` branch, root. Verify HTTPS auto-enabled.
16. Lighthouse audit, fix any issue scoring <90.
17. Manual cross-browser test (Chrome, Safari, Firefox, Edge).

## Todo List

- [x] Scaffold Astro project under site/ (moved from docs/ — docs/ stays for engineering markdown)
- [x] Configure astro.config with GH Pages base + Tailwind + MDX
- [x] Build BaseLayout with SEO meta + OG tags (placeholder og-image.png)
- [x] Build Hero with H1 + tagline + CTAs
- [ ] Build DemoVideo with YouTube lazy-load + poster — deferred until Phase 14 video lands
- [ ] Build HowItWorks 3-step section with screenshots — deferred
- [ ] Build FeatureGrid 6-cell — deferred
- [x] Build DownloadButtons with OS detection + brew/winget code blocks (aspirational commands marked "SOON")
- [x] Build InstallGuide with inline copy per OS (MDX content collections deferred)
- [x] Build VietnameseSEO section with keyword-targeted copy
- [x] Build DonateBadges (OC + Sponsors + Apple fund progress)
- [x] Build Footer
- [ ] Create OG image, screenshots, demo-poster — defer to ui-ux-designer round
- [x] Set up GH Pages deploy workflow
- [ ] Lighthouse audit ≥95 Performance — needs live deploy
- [ ] Cross-browser smoke test — needs live deploy

## Success Criteria

- [ ] Site live at `https://<user>.github.io/sniptex/`
- [ ] Download buttons resolve to latest GitHub Release artifacts per OS
- [ ] Install guides include working copy-paste commands
- [ ] Vietnamese SEO section indexable (verified via Google Search Console after launch)
- [ ] Lighthouse Performance ≥95, A11y ≥90, SEO ≥95

## Risk Assessment

- **Risk: GH Pages serves cached old version after deploy** — Mitigation: 5-min CDN TTL; document expected delay; force-refresh in announcement.
- **Risk: YouTube embed blocks privacy-conscious users** — Mitigation: lazy-load only on click; poster image visible without scripts.
- **Risk: VN keywords compete with established edu sites** — Mitigation: long-tail keywords + backlink from in-app onboarding.

## Security Considerations

- Static site, no backend; minimal attack surface.
- Avoid embedding tracking scripts (privacy commitment in `replan.md` §1).
- OG image and screenshots checked for sensitive content (no real user data in demos).
- **Privacy disclosure (Session 3):** the page MUST include a "How your data flows" section explaining the two privacy modes — BYOA (image goes only to your local CLI's chosen LLM provider) vs BYOK cloud (image is sent to Google's Gemini API over TLS, key stored in your OS keychain). Link to the per-provider privacy/data-use docs.

## Next Steps

- Phase 14 produces the demo video to embed
- Phase 15 announces site URL across distribution channels

## Open Questions

- Add multilingual landing (`/en/`, `/vi/`) or single page with VN section? Default v1: single page with `#vi` anchor — simpler, less translation drift.
