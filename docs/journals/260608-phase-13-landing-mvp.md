# Phase 13: Landing Page MVP — Live at GitHub Pages, 19.8 KB, 500ms CDN Serve

**Date**: 2026-06-08
**Component**: Landing Page, Astro 4.16 + Tailwind 3.4, GitHub Pages Deploy
**Status**: Shipped; live at https://hiep1987.github.io/sniptex/

## What Happened

Scaffolded and shipped Phase 13 landing page MVP. Astro project at `site/` (not `docs/` — existing docs contain engineering markdown; sibling `site/` keeps role boundaries clear). Deployed to GitHub Pages via `actions/deploy-pages@v4`. Live URL serving 19.8 KB index.html + 20 KB CSS + 480 B favicon (~40 KB total, under 500 KB target). Cold CDN serve ~500ms; subsequent ~10ms.

Six commits shipped: forgotten verify-CI journal, Astro scaffold + GH Pages workflow, phase doc closure, sitemap lockfile drift fix, and honest install-command cleanup.

## The Brutal Truth

Two honesty failures caught mid-session by code review + user feedback:

**C1**: `@astrojs/sitemap` 3.7.3 was in package.json but removed from `astro.config.mjs` during fixes. CI ran with `--frozen-lockfile`, hit the old lockfile reference, and failed. Dirty handoff. The integration itself crashes on Astro 4.16.19 with `base: "/sniptex/"` config anyway (`Cannot read properties of undefined (reading 'reduce')` at sitemap's `dist/index.js:85`). Dropped entirely; phase doesn't need it.

**C2**: Landing page advertised `brew install --cask sniptex` and PowerShell installer, neither of which exist yet. User in Vietnam filed honest report: "lệnh này Warning: Cask 'sniptex' is unavailable… và releases page chưa có file cài đặt". Correct on both counts. Cask formula exists in repo but isn't published (Phase 15). Releases page is empty because v0.0.1-test draft was deleted during cleanup. I initially softened with a "SOON" badge. User said remove them entirely. Applied in commit 63ca2bc. Embarrassing, but shipping an honest landing page beats misleading users.

## Technical Details

**Scope decisions (user-confirmed via 4 AskUserQuestion):**
- **Components shipped (6 of 9)**: Hero + CTA + VietnameseSEO + DonateBadges + Footer + FeatureHighlight. Deferred HowItWorks, FeatureGrid, DemoVideo (latter waits for Phase 14 video asset).
- **Visual assets deferred**: Placeholder favicon (gradient SVG with ∫ symbol) only. OG image referenced in meta but not created → social shares 404. Acceptable for MVP; ui-ux-designer picks up real assets later.
- **Build config**: Tailwind 3.4.19, MDX 3.1.9, TypeScript strict. `.github/workflows/deploy-pages.yml` using `upload-pages-artifact@v3` + `deploy-pages@v4`. 17 files under `site/`.

**Code Review Findings (2 Critical + 3 High + 4 Medium + 5 Low):**
- C1: missing `site/.gitignore` → would have committed node_modules. Fixed pre-commit.
- C2: sitemap lockfile drift (fixed above).
- H1: og-image.png missing → deferred to ui-ux-designer.
- H2: aspirational install commands shown → removed (commit 63ca2bc).
- H3: install-guide.astro diverges from docs/install-guide.md on Smart App Control trade-off and checksum verification. Acceptable for MVP; sync when adding MDX content collections.
- M1–M4, L1–L5: tracked in reviewer report; low-priority style/tone.

**Two findings worth escalating to future phases:**

1. **GitHub Pages `build_type=workflow` must be set explicitly.** Without it, Pages defaults to "Deploy from a branch" and `actions/deploy-pages@v4` silently does nothing. Fix via `gh api -X POST repos/<user>/<repo>/pages -f build_type=workflow`. Added to `docs/releasing.md` for next maintainer.

2. **@astrojs/sitemap 3.7.3 + Astro 4.16.19 + `base: "/<path>/"` = crash.** Integration freezes at `Cannot read properties of undefined (reading 'reduce')` at line 85 in sitemap's `dist/index.js`. Likely config conflict. Skip the integration or pin to older version if needed in future.

## What We Tried

1. **First deploy with sitemap integration**: CI failed. Lockfile still referenced removed package.
2. **Initial C2 fix (aspirational commands)**: Added "SOON" badge. User feedback: "Ship honest copy instead." Removed commands entirely.
3. **Initial GitHub Pages setup**: Forgot to toggle `build_type=workflow` via API; deploy action ran but produced no usable artifact. Fixed via `gh api` call.

## Root Cause Analysis

Three separate honesty lapses:

1. **Sitemap package.json → lockfile gap**: Common monorepo sync bug. Should have regenerated lockfile immediately after removing from config, not left it in package.json.
2. **Install commands without reality check**: Plan promised `brew install --cask sniptex`, but the Cask tap doesn't exist. Shipped marketing copy without verifying the backend exists. User caught it immediately — that's the job of a marketing page, so good outcome, but bad execution.
3. **GitHub Pages config assumption**: Thought uploading artifact + running deploy action was sufficient. Didn't verify that the Pages config actually switched from branch → workflow source. API toggle is undocumented in the action README.

## Lessons Learned

1. **Regenerate lockfiles immediately after `package.json` edits.** CI catch is too late; do it pre-commit.
2. **For "coming soon" features, do a quick reality check.** If brew cask / releases / pre-built installers don't exist yet, say "Download source + build" or "Coming with v0.1.0" — don't promise commands that fail silently on user machines.
3. **Read GitHub Actions docs + dig into API.** The `deploy-pages` action glosses over the Pages config prerequisite. Always verify downstream configuration (especially Settings toggles buried in web UI) before assuming an action "just works."

## Next Steps

1. **Phase 14**: Produce DemoVideo asset; export OG image + social graphics.
2. **Phase 14**: Activate HowItWorks, FeatureGrid, DemoVideo components on landing.
3. **Phase 15**: Publish real Cask formula + PowerShell installer script; update landing install commands to real workflow.
4. **Lighthouse audit ≥95** once live URL stable.
5. **Cross-browser smoke** (Chrome, Safari, Firefox).
6. **Custom domain** if applicable.
7. **Update docs/releasing.md** with the GitHub Pages API toggle lesson.

## Commits

- `1c17cb2` chore(docs): commit forgotten verify-CI session journal
- `3371e3e` feat(site): scaffold Astro landing page + GH Pages deploy workflow
- `045d5df` docs(plan): close phase 13 with MVP-scaffolded landing status
- `791347d` fix(site): regenerate pnpm-lock.yaml after dropping @astrojs/sitemap
- `63ca2bc` fix(site): drop aspirational brew + powershell install commands
- (plus 1 additional commit for review fixes)

## Unresolved Questions

1. **Custom domain**: Should landing live at `sniptex.dev` or stay at GitHub Pages URL for now?
2. **VN routing**: Implement `/vi/` alternate routes (Phase 15) or use single-page anchor links (#vi)?
3. **Real OG image**: Block launch on social sharing graphics, or ship and update later?
4. **Lighthouse ≥95**: Target for Phase 14, or deferred to Phase 15 launch prep?
