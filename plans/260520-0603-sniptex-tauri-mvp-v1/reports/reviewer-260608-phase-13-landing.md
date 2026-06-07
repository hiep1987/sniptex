# Reviewer Report — Phase 13 Landing Page MVP

Date: 2026-06-08
Reviewer: code-reviewer
Scope: `site/**` + `.github/workflows/deploy-pages.yml` (6 components + base layout + workflow)
Verification: `astro check` clean (0 err / 0 warn / 0 hints, 11 files); user confirmed `pnpm build` 20 KB HTML, preview 200 OK.

---

## Findings

### CRITICAL

#### C1. `site/node_modules` is NOT gitignored — risks 100+ MB commit
- Root `/Users/hieplequoc/Projects/sniptex/.gitignore:5` uses `/node_modules` (anchored to repo root). Pattern does NOT match `site/node_modules`.
- `site/` directory has no local `.gitignore`. `git ls-files --others --exclude-standard site/` lists `site/node_modules/.bin/*`, `site/node_modules/.pnpm/*` as untracked-but-eligible.
- A naive `git add site/` (or future `git add -A` from any agent) will stage the entire pnpm store. Same for `site/.astro/types.d.ts` (Astro-generated, also unignored).
- **Fix:** create `site/.gitignore` with:
  ```
  node_modules/
  .astro/
  dist/
  .DS_Store
  *.log
  ```
  (Root already covers `dist/` and `.DS_Store`, but a local file makes intent explicit and is the Astro starter default.)
- **Severity rationale:** silent footgun. CI passes because lockfile is fine, but first commit on a clean clone could balloon repo history.

#### C2. `package.json` still declares `@astrojs/sitemap` though it was dropped at runtime
- `site/package.json:14` lists `"@astrojs/sitemap": "^3.2.1"` as a dep.
- `site/astro.config.mjs:12` integrations array does NOT import or register sitemap (only tailwind + mdx). User noted sitemap was removed because of `Cannot read properties of undefined (reading 'reduce')`.
- Result: ~unused dep installed on every CI run, plus reads like sitemap is wired when it isn't. Either re-add the integration once the crash is fixed (separate issue — likely zero-page or trailingSlash interaction) or remove from `dependencies`. Pick one to avoid lying about state.
- **Fix:** `pnpm remove @astrojs/sitemap` and regenerate `pnpm-lock.yaml`. File an issue tracking the crash for follow-up. (Sitemap matters for SEO target "OCR LaTeX miễn phí" — but YAGNI says ship without, fix later.)
- **Severity rationale:** misleading dep state + small surface for supply-chain bloat; not a runtime risk, but blocks honest review.

---

### HIGH

#### H1. OG image referenced but does not exist → 404 on every share
- `site/src/layouts/base-layout.astro:14` defaults `ogImage = "og-image.png"`. `site/public/` only contains `favicon.svg`.
- Every Twitter/Facebook/LinkedIn/Discord link preview will fetch `https://hiep1987.github.io/sniptex/og-image.png` → 404 → fallback to a generic "no preview" card. Hurts the launch we are building this page for.
- **Fix (MVP-acceptable):** drop the OG image meta tags when the file is absent (guard in layout), OR ship a 1200×630 PNG placeholder (gradient + "SnipTeX — Free OCR snip tool for LaTeX & Markdown" text). The second option costs 30s with imagemagick:
  ```bash
  magick -size 1200x630 -define gradient:vector=0,0,1200,630 \
    gradient:'#2563eb-#7c3aed' -gravity center -fill white \
    -font Helvetica-Bold -pointsize 72 -annotate 0 'SnipTeX' \
    site/public/og-image.png
  ```
  Flag for ui-ux-designer to replace later. Note this in phase-13 completion section.
- **Severity rationale:** SEO/marketing regression on a marketing page; not a code bug but defeats Phase 13's purpose.

#### H2. Aspirational install commands shown without disclaimer → user-facing 404 / `brew error`
- `site/src/components/download-buttons.astro:43` shows `brew install --cask sniptex`. Tap does not exist (Phase 15 task). Users running this command get `Error: Cask 'sniptex' is unavailable`.
- `site/src/components/download-buttons.astro:76` shows `irm https://hiep1987.github.io/sniptex/install.ps1 | iex`. `install.ps1` is not in `site/public/`. PowerShell error 404 → no install.
- `docs/install-guide.md:22-26` already handles this with a clear ">" callout pointing to the local tap workaround. Landing page does NOT.
- **Fix:** either gate these blocks behind `import.meta.env.PUBLIC_HOMEBREW_TAP_PUBLISHED` / `PUBLIC_PS_INSTALLER_PUBLISHED` env flags (off for MVP), OR add an inline "(coming with v0.2 — use the DMG/MSI above for now)" badge under each `code-block`. Same friction-free copy/paste UX, no broken commands.
- **Severity rationale:** worse-than-no-information UX; v0.1 visitors will assume the project is broken and bounce.

#### H3. Install-guide divergence vs `docs/install-guide.md` already starting
- `site/src/components/install-guide.astro` is a shortened mirror of `docs/install-guide.md`. Several specifics differ:
  - Landing line 64: "Smart App Control is one-way. Once off, can't be re-enabled without a Windows reinstall." Docs:108 says same content but adds "weigh the trade-off".
  - Landing: omits the Reputation-based protection fallback (docs:101-104) entirely. Users hitting "More info doesn't appear" have no path.
  - Landing: omits the "Verifying the DMG checksum" + `shasum -a 256` section (docs:74-82). Hero copy says "SHA-256 checksums are listed in the release notes for manual verification" but provides no command.
  - Landing: omits Linux note + troubleshooting matrix.
- **Fix:** acceptable for MVP, but landing should make the "full guide" link more prominent than the small footer line on `install-guide.astro:82-87`. Promote that link to a button-styled CTA at the top of each card. Also add a single source-of-truth header comment in `install-guide.astro` that says "Mirror of docs/install-guide.md sections 1.4 + 2.1. Update both when content changes." to prevent silent drift.
- **Severity rationale:** drift accumulates fast; one round of edits to docs/install-guide.md and the landing falls behind.

---

### MEDIUM

#### M1. Clipboard "Copy" button invisible on touch devices (a11y + mobile UX)
- `download-buttons.astro:47` + `:80` use `opacity-0 group-hover:opacity-100`. iPad/iPhone/Android users have no `:hover` state; button stays at `opacity-0` permanently → completely unreachable.
- Also fails screen-reader logical order: button is positioned `absolute top-2 right-2`, visually before the code on mobile, but DOM-after, so VoiceOver reads "brew install --cask sniptex / Copy command" out of order.
- **Fix:** drop `opacity-0 group-hover:opacity-100`. Make the button always visible (`opacity-100` or `opacity-70 hover:opacity-100`). Keep the absolute positioning. Cost: one always-visible chip per code block; gain: it actually works on mobile + AT.
- **Severity rationale:** real a11y problem (WCAG 2.5.5 target size + invisible interactive element); affects ~40% of traffic on landing pages.

#### M2. `Astro.url.pathname` canonical may be `/sniptex/` or `/sniptex` depending on host
- `base-layout.astro:19`: `const canonical = new URL(Astro.url.pathname, Astro.site).toString()`.
- With `trailingSlash: "ignore"` (astro.config.mjs:11), Astro.url.pathname for index can be either `/sniptex/` or `/sniptex` depending on whether the request hits `/sniptex/` or `/sniptex`. Both are valid; canonical becomes whichever the SSG saw at build time.
- For prerendered `index.html`, Astro stamps pathname based on `output: "static"` default → `/sniptex/`. Should be stable, but worth verifying after first deploy by curling the live URL.
- **Fix (optional, defensive):** explicitly set `trailingSlash: "always"` for clarity on a project-Pages site, or normalize in canonical compute:
  ```ts
  const pathname = Astro.url.pathname.endsWith("/") ? Astro.url.pathname : Astro.url.pathname + "/";
  ```
- **Severity rationale:** SEO duplicates if both paths get indexed; not a bug today but a subtle one to verify post-deploy.

#### M3. Vietnamese SEO sub-tree won't be ranked as Vietnamese content
- `vietnamese-seo.astro:5` puts `lang="vi"` on the `<section>` only; document `<html lang={lang}>` (`base-layout.astro:24`) stays `"en"` because `index.astro` doesn't pass `lang="vi"`.
- Google's language detection respects the document-level `lang` attribute primarily; section-level `lang` is honored for accessibility (screen-reader pronunciation) but does NOT make Google treat the page as Vietnamese in search verticals.
- Result: the page targets "OCR LaTeX miễn phí" but registers as English; competing with English-language pages instead of Vietnamese ones.
- **Fix (MVP-acceptable):** add hreflang alternates to head:
  ```html
  <link rel="alternate" hreflang="en" href="https://hiep1987.github.io/sniptex/" />
  <link rel="alternate" hreflang="vi" href="https://hiep1987.github.io/sniptex/#vi" />
  ```
  Long-term (Phase 14+): split into `/vi/` route with full `<html lang="vi">` page. Won't fully fix until then, but hreflang at least signals intent.
- **Severity rationale:** the entire vietnamese-seo component exists to capture VN search traffic; without correct lang signaling that ROI is sharply reduced.

#### M4. `deploy-pages.yml` does not document the manual Pages source switch
- Workflow uses `actions/deploy-pages@v4`, which **requires** Pages source to be set to "GitHub Actions" (Settings → Pages → Build and deployment → Source = GitHub Actions). If left as default "Deploy from a branch", the deploy step fails with `Error: Get Pages site failed`.
- No documentation in `phase-13-*.md` or `docs/releasing.md` reminding the maintainer to flip this once.
- **Fix:** add one line to the phase-13 completion section / journal: "Before first push: Settings → Pages → Source = GitHub Actions." Five seconds to add; saves a confused debugging session on first deploy.
- **Severity rationale:** one-time bootstrap step, but failure is silent-looking (workflow just errors) and not obvious from the error message.

---

### LOW

#### L1. `download-buttons.astro:107` OS detection assumes Mac card = index 0
- `cards.forEach((card, idx) => { const isMacCard = idx === 0; ... })`. Implicit coupling: any future re-order (e.g., Win-first for VN audience) silently breaks ring highlight.
- **Fix:** use `data-os="mac"` / `data-os="windows"` attributes on the cards, query by attribute. Costs 2 extra HTML attrs; eliminates the bug class.

#### L2. `vietnamese-seo.astro:9` emoji 🇻🇳 in source code
- Not a bug, but the rest of the codebase avoids emojis in source (per the agent profile). Inline emoji works in HTML but: (a) doesn't render on every OS (Win 10 pre-Anniversary, some Linux distros without color emoji font), (b) potentially flagged by automation. Cost-of-change: replace with `<span aria-hidden="true">🇻🇳</span>` and let the OS render or fall back.
- Tiny, but flagged for consistency.

#### L3. `donate-badges.astro:9` divide-by-zero edge already handled but coupled
- `fundPct = Math.min(100, Math.round((0 / 99) * 100))` = 0. Fine today. If `fundTarget` ever becomes 0 (typo, env-driven future), `0/0 = NaN → Math.round(NaN) = NaN → Math.min(100, NaN) = NaN`, then `style={`width: NaN%`}` renders a broken progress bar.
- **Fix:** `const fundPct = fundTarget > 0 ? Math.min(100, Math.round((fundRaised / fundTarget) * 100)) : 0;` — one-liner, defends against future config bugs.

#### L4. Inline theme bootstrap script + no CSP
- `base-layout.astro:52-65` uses `<script is:inline>`. GitHub Pages serves no `Content-Security-Policy` header by default → inline script runs fine in production. Astro also won't add one. If/when a CSP is introduced later (via a Cloudflare proxy or meta tag), this script breaks and theme defaults to light.
- Not blocking. Note for future. Adding `<meta http-equiv="Content-Security-Policy" content="...">` with a nonce'd script is the upgrade path.

#### L5. `release` link inconsistency
- `hero.astro:2` + `vietnamese-seo.astro:2` + `download-buttons.astro:7` each independently declare `const releasesLatest = "https://github.com/hiep1987/sniptex/releases/latest"`. DRY violation; if the repo ever moves (e.g., to an org), three files need editing.
- **Fix:** extract to `src/lib/constants.ts` or pass via Astro's `site` config. Low priority; trivial to fix later.

---

### INFO / POSITIVE

- ✓ Tailwind v3.4 `darkMode: ["class", '[data-theme="dark"]']` correctly aliases `dark:` variants to `[data-theme="dark"]` selectors. Walked one example: `dark:bg-ink-900` → compiled selector `[data-theme="dark"] .dark\:bg-ink-900 { background-color: #0f172a }`. Theme bootstrap sets `documentElement.setAttribute("data-theme", "dark")` → dark variants activate site-wide. Verified by build producing 20 KB CSS that contains the variants (per user-reported build output).
- ✓ `BASE_URL` math is correct. `import.meta.env.BASE_URL = "/sniptex/"`, strip trailing → `"/sniptex"`. `${base}/favicon.svg` = `/sniptex/favicon.svg`. No double-prefix because Astro does NOT auto-rewrite `<link href>` attrs (only its own asset pipeline). Confirmed by `astro check` clean.
- ✓ Hash anchors (`#vi`, `#install`, `#download`, `#donate`) are fragment-relative; unaffected by BASE_URL. Smooth-scroll works via `scroll-smooth` on html.
- ✓ `actions/deploy-pages@v4` + `actions/upload-pages-artifact@v3` + `concurrency: pages` + `cancel-in-progress: true` is the current canonical GH Pages workflow. Correct. `paths:` filter on push events IS honored by GH Actions (verified in docs); the workflow only triggers on `site/**` or its own edits.
- ✓ `cache-dependency-path: site/pnpm-lock.yaml` is correct AND the lockfile EXISTS on disk (`site/pnpm-lock.yaml` 164 KB, generated by user's local `pnpm install`). User assumption was wrong — lock is there.
- ✓ `pkg.version` import in `footer.astro` works because Astro's strict tsconfig sets `resolveJsonModule: true`. `astro check` confirms 0 errors.
- ✓ Phase 11/12 files (`src-tauri/`, `scripts/`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/install-guide.md`, `docs/releasing.md`) are NOT touched in the staged diff. `git status` shows only `site/` + `.github/workflows/deploy-pages.yml` + one journal as untracked. Clean separation.
- ✓ File sizes all well under 200 LOC. Largest is `install-guide.astro` at 89 lines. Aligns with project standard.
- ✓ Component naming kebab-case throughout (`download-buttons.astro`, not `DownloadButtons.astro`). Matches the codebase convention.

---

## Final Verdict

**Score: 7.5 / 10** — Ship-ready after C1 (gitignore) and H2 (disclaimer aspirational commands). H1 (OG image) is the "real launch quality" gap; ship without OG meta or with a placeholder, decide which. Everything else is iterative polish.

Blocking for landing: **C1, H2.**
Strongly recommended before announcing: **H1, M1, M3.**
Defer to follow-up PRs: **C2, H3, M2, M4, L1-L5.**

---

## Recommended Actions (priority order)

1. **C1** — Add `site/.gitignore` (3 lines: `node_modules/`, `.astro/`, `dist/`).
2. **H2** — Either env-gate brew/PS commands OR add "(coming with v0.2)" badge under each. Pick one in 5 min.
3. **H1** — Generate placeholder `og-image.png` with imagemagick (one-liner above) OR strip OG meta in layout when file absent.
4. **M1** — Drop `opacity-0 group-hover:opacity-100` on copy buttons; make always-visible.
5. **M3** — Add hreflang alternates in `base-layout.astro` head.
6. **C2** — Remove `@astrojs/sitemap` from package.json; regenerate lockfile.
7. **M4** — Add Pages source setup note to phase-13 completion / journal.
8. **H3** — Add divergence-prevention header comment on `install-guide.astro`.
9. **L1-L5** — fold into follow-up PR.

---

## Unresolved Questions

1. **`@astrojs/sitemap` crash root cause**: user noted `Cannot read properties of undefined (reading 'reduce')`. Is this a config issue (`trailingSlash: "ignore"` + `base: "/sniptex/"` interaction known to break v3.2.1?), or sitemap-3.2.1 vs astro-4.16.19 compat? Worth a 5-min investigation before next PR — sitemap matters for the SEO target.
2. **OG image strategy**: do we ship the placeholder PNG in C1 now, or wait for ui-ux-designer? Strong recommendation: ship placeholder, replace later. Costs nothing, fills a real gap.
3. **`hiep1987.github.io` Pages enabled?** Workflow assumes Pages is configured. If first run fails on `actions/deploy-pages@v4`, that's the cause.
4. **VN routing strategy**: M3 fix is hreflang-as-bandaid. Long-term plan: keep #vi anchor (low effort, partial SEO) or split into `/vi/` route in Phase 14 (correct, more effort)?

---

**Status:** DONE_WITH_CONCERNS
**Summary:** 6 components + base layout + workflow are functionally correct and type-clean; two pre-ship blockers (gitignore + aspirational commands) and one launch-quality gap (OG image) require fixes before announcing. Build math, deploy workflow, and component architecture all verified.
**Concerns/Blockers:** C1 (gitignore), H2 (aspirational install commands shown without disclaimer), H1 (OG image 404). Address C1+H2 before commit; H1 before announce.
