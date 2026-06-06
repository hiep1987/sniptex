---
title: "SnipTeX — Cross-Platform OCR Snip Tool MVP v1"
description: "Free, open-source, cross-platform (Mac + Windows) desktop snip tool. Region capture → CLI agent (Gemini CLI or OpenAI Codex) OCR → LaTeX or Markdown → clipboard. Built with Tauri 2 + React/TS + Rust. BYOA (Bring Your Own Agent). MIT licensed."
status: pending
priority: P1
branch: "main"
tags: ["tauri", "rust", "react", "ocr", "latex", "cross-platform", "byoa", "open-source"]
blockedBy: []
blocks: []
created: "2026-05-19T23:05:31.528Z"
createdBy: "ck:plan"
source: skill
---

# SnipTeX — Cross-Platform OCR Snip Tool MVP v1

## Overview

Free alternative to Mathpix Snip ($5-20/mo) for Mac + Windows. Hotkey-triggered region capture, sends image through one of three OCR paths: **(1) Codex CLI** (v1 default BYOA, privacy-first), **(2) Gemini CLI** (experimental secondary, gated), or **(3) Gemini Vision API direct** (new `--cloud` mode for sub-5s response, BYOK). Returns clean LaTeX or Markdown, auto-copies to clipboard. Lightweight (<20MB, <100MB RAM), zero backend cost. Source spec: `plans/replan.md`. **v1 ships Codex CLI + Gemini CLI + Gemini Vision API cloud fallback** (Path C hybrid decided Validation Session 3, 2026-05-21 — see Validation Log).

**Target audience:** Vietnamese teachers/students working with SGK Toán + technical docs; global LaTeX users wanting a free Mathpix alternative.

**Critical gate:** Phase 1 (prompt validation) gates everything. If accuracy on Vietnamese SGK + complex equations falls below acceptable threshold → pivot to direct Gemini Vision API instead of CLI agents.

## Phases

| Phase | Name | Status |
|-------|------|--------|
| 1 | [Prompt Validation & Go/No-Go](./phase-01-prompt-validation-go-no-go.md) | ✅ Validation Complete (Session 3, Path C) |
| 2 | [Tauri Scaffold & Rust Foundation](./phase-02-tauri-scaffold-rust-foundation.md) | ✅ Complete (hotkey verified 2026-05-22) |
| 3 | [Agent System & OCR Pipeline](./phase-03-agent-system-ocr-pipeline.md) | ✅ Complete (2026-05-22) |
| 4 | [Screen Capture & Region Selector](./phase-04-screen-capture-region-selector.md) | ✅ Complete (2026-05-22, smoke test verified) |
| 5 | [Hotkey & Tray Integration](./phase-05-hotkey-tray-integration.md) | ✅ Complete (2026-05-22, smoke test verified) |
| 6 | [React UI Shell & MathJax Preview](./phase-06-react-ui-shell-mathjax-preview.md) | ✅ Complete (2026-05-22, build verified; live smoke test pending) |
| 7 | [SQLite History with FTS5 Search](./phase-07-sqlite-history-with-fts5-search.md) | ✅ Complete (2026-05-23, 7 integration tests; live smoke test pending) |
| 8 | [Settings UI & Onboarding Flow](./phase-08-settings-ui-onboarding-flow.md) | ✅ Complete (2026-05-31, all 5 settings tabs + 5 onboarding steps + 4-agent BYOK shipped) |
| 9 | [Theme & Format Toggle & UX Polish](./phase-09-theme-format-toggle-ux-polish.md) | ✅ Implementation Complete (2026-06-01; formats, tabular validation, theme, sound, animations, Toast, rapid-snip queue) |
| 10 | [Windows Cross-Platform Port](./phase-10-windows-cross-platform-port.md) | ✅ Complete (2026-06-06; Mac-side code-prep + Windows 11 ARM64 validation both green, all 13 Batch B checks signed off) |
| 11 | [Distribution: Code Signing & Installers](./phase-11-distribution-code-signing-installers.md) | Pending |
| 12 | [CI/CD Release Workflow & Auto-Updater](./phase-12-ci-cd-release-workflow-auto-updater.md) | Pending |
| 13 | [Landing Page & Install Documentation](./phase-13-landing-page-install-documentation.md) | Pending |
| 14 | [Demo Video & Marketing Assets](./phase-14-launch-demo-video.md) | Pending |
| 15 | [Package Managers & Soft Launch](./phase-15-packages-marketing.md) | Pending |

## Key Dependencies

- **External (user pre-installed OR API key):** OpenAI Codex (`codex` — default), Gemini CLI (`gemini` — experimental secondary), OR a Google AI Studio API key (cloud fallback, sub-5s). Onboarding guides install via `winget`/`brew`/`npm` AND API-key acquisition flow.
- **Toolchain:** Rust stable, Node 20+, pnpm, Tauri CLI 2.x.
- **CI:** GitHub Actions runners — `macos-latest` (ARM + Intel cross-build) and `windows-latest`.
- **Distribution channels:** GitHub Releases (primary), Homebrew Cask (Mac), Winget (Windows).

## Timeline

5 weeks total. Phase 1-3 = Week 1, Phase 4-6 = Week 2, Phase 7-9 = Week 3, Phase 10-12 = Week 4, Phase 13-15 = Week 5. See `plans/replan.md` §9 for detailed day-by-day breakdown.

## Stop-Loss / Pivot Points

- **End of Phase 1 (Day 2):** If LLM accuracy on Vietnamese SGK <80% OR latency p95 >8s OR format consistency <90% → halt Tauri work, evaluate Gemini Vision API direct call OR LightOnOCR-2-1B local fallback.
- **End of Phase 10 (Day 23):** If Windows port reveals fundamental Tauri/xcap issues → ship Mac-only v1.0, defer Windows to v1.1.

## Validation Log

### Session 1 — 2026-05-20
**Trigger:** Post-plan-creation `/ck:plan validate` interview.
**Questions asked:** 8

#### Questions & Answers

1. **[Scope]** "replan.md §6 notes Codex image-input syntax 'cần verify tuần 1' — it's unverified. What's the v1 scope decision if Codex image support is broken or absent?"
   - Options: Drop Codex from v1 (Recommended) | Keep Codex, validate Phase 1 | Defer Codex to v1.x explicitly
   - **Answer:** Drop Codex from v1
   - **Rationale:** Cuts Phase 3 surface area, simplifies onboarding to a single install command, reduces Phase 1 risk; Codex returns in v1.x when image-input syntax stabilizes.

2. **[Assumptions]** "Phase 1 go/no-go gate thresholds I proposed: accuracy ≥80% on SGK, latency p95 ≤6s, format consistency ≥90%. Are these the right bar?"
   - Options: Keep proposed thresholds (Recommended) | Stricter 90/5s/95 | Looser 70/8s/80 | Defer to end of Phase 1
   - **Answer:** Keep proposed thresholds
   - **Rationale:** Balanced bar for a free tool; strict enough for credibility, lenient enough to tolerate LLM run-to-run variance.

3. **[Architecture]** "OCR provider strategy: CLI-agent-only (BYOA) vs hybrid that also calls Gemini Vision API directly when CLI fails or isn't installed?"
   - Options: CLI-only BYOA in v1 (Recommended) | Hybrid CLI + Gemini Vision API | CLI-only but build pivot bridge
   - **Answer:** CLI-only BYOA in v1
   - **Rationale:** Preserves "no API keys in app" privacy framing; simpler Phase 3 dispatcher; hybrid stays a v1.x option.

4. **[Architecture]** "Phase 7 history stores original snip images for 'rerun with different agent'. What's the retention policy?"
   - Options: Keep forever (Recommended) | Delete after 30 days | Opt-in only | Tied to history_size
   - **Answer:** Keep forever, user-driven cleanup
   - **Rationale:** Simplest UX; matches user expectation from Photos/Notes; user controls cleanup via History window + Settings.

5. **[Tradeoffs]** "Phase 2 has Tailwind 4 — it may still be alpha when development starts. Commit now or hedge?"
   - Options: Commit to Tailwind 4 (Recommended) | Ship v1 on Tailwind 3, upgrade later | Defer decision to Phase 2 start
   - **Answer:** Commit to Tailwind 4
   - **Rationale:** Avoids a forced v3→v4 migration in v1.x; pin to a tested alpha build if not yet stable at Phase 2 start.

6. **[Architecture]** "Phase 6 Preview Window: single shared instance (hide/show) vs spawn-per-snip?"
   - Options: Single shared instance (Recommended) | Spawn-per-snip
   - **Answer:** Single shared instance
   - **Rationale:** Lower latency, predictable memory, no race conditions on rapid snips; standard pattern.

7. **[Risks]** "Apple Developer Program ($99/yr) — defer until donations cover, or fund out-of-pocket sooner?"
   - Options: Stay deferred (Recommended) | Fund out-of-pocket at v1 launch | Wait for 1k downloads
   - **Answer:** Stay deferred (donations cover)
   - **Rationale:** Homebrew Cask bypasses Gatekeeper for most users; documented workaround for direct DMG users; uses donation goal as community signal.

8. **[Scope]** "Community channel — Discord, Telegram, both, or neither in v1?"
   - Options: Discord only (Recommended) | Telegram only | Both | GitHub Discussions only
   - **Answer:** Discord only
   - **Rationale:** Better threading + roles; LaTeX/edtech communities already there; single channel keeps moderation effort bounded.

#### Confirmed Decisions
- **v1 supported agent = Gemini CLI only** — affects Phases 1, 3, 8, 13, 15
- **Phase 1 thresholds locked** — 80% accuracy / 6s p95 / 90% format consistency
- **CLI-only BYOA** — no direct LLM API integration in v1; Phase 3 dispatcher stays CLI-focused
- **History images retained forever**, user-driven cleanup — Phase 7
- **Tailwind 4 committed** — Phase 2/6 remove v3 hedge
- **Single shared Preview Window** — Phase 6 confirmed
- **Apple Developer Program deferred** — Phase 11 unchanged; donation goal $99
- **Discord-only community** — Phase 15 drops Telegram option

#### Impact on Phases
- Phase 1: Remove Codex from fixture matrix; test Gemini only
- Phase 3: Remove Codex from `AGENTS` registry, delete `agents/codex.rs` from create-list
- Phase 8: Onboarding install-guide step shows Gemini CLI only
- Phase 13: Landing page hero copy = "Bring your own Gemini CLI" (singular)
- Phase 15: Marketing posts mention Gemini CLI; community link = Discord only
- Phase 2: Commit to Tailwind 4; drop "fall back to Tailwind 3" hedge in Risk Assessment
- Phase 6: Confirm single-shared Preview Window in Open Questions
- Phase 7: Keep-forever retention; remove cap question from Open Questions

### Whole-Plan Consistency Sweep — 2026-05-20

Sweep applied across `plan.md` and all 15 `phase-*.md` files after propagation.

| Concern | Result |
|---------|--------|
| Codex references in implementation steps | Removed from Phase 1, 3, 8, 13, 15 |
| Codex still mentioned in replan.md (source brief) | Acknowledged — replan.md is historical, plan.md supersedes |
| Tailwind 3 fallback language | Removed from Phase 2, 6 |
| Hybrid OCR / Gemini Vision API references | None present in plan files; pivot remains in Stop-Loss only (Phase 1 fail-path) |
| Codex-aware drag-drop UI in Settings | Phase 8 reverts to single-agent display + "more agents in v1.x" copy |
| Apple Developer language consistent | Phase 11, 15, plan.md all say "deferred until donations cover" — consistent |
| Telegram references | Removed from Phase 15 |
| Discord linked from landing footer | Phase 13 updated to single-channel link |
| Conflict in Preview Window architecture | None — Phase 6 already defaulted to single-shared; Open Question removed |

**Status:** Zero unresolved contradictions. Plan eligible for `/ck:cook`.

### Session 2 — 2026-05-20 (Amendment)
**Trigger:** User reverted Session 1 Question 1 decision after review.
**Questions asked:** 1 (revisit only)

#### Questions & Answers

1. **[Scope — REVERSAL of Session 1 Q1]** "Keep Codex in v1 vs Gemini-only?"
   - **Answer:** Keep Codex in v1
   - **Rationale:** User preference to validate both agents in Phase 1; preserves choice in onboarding; Codex represents a meaningful free-tier option for ChatGPT Plus subscribers. Risk: Phase 1 outcome now depends on Codex image-input syntax being functional at validation time.

#### Confirmed Decisions (overrides Session 1)
- **v1 supported agents = Gemini CLI AND Codex** — restored across Phases 1, 3, 8, 13, 15
- Codex image-input syntax verification moves back into Phase 1 critical path; if Codex fails at Phase 1, only then drop to Gemini-only

#### Impact on Phases (reversal of Session 1 propagation)
- Phase 1: Restore Codex in fixture matrix + both lanes in test-prompt.sh; risk re-added
- Phase 3: Restore Codex in `AGENTS`, `agents/codex.rs` back in create-list, dispatcher fallback chain has 2 agents
- Phase 8: Onboarding install guide covers both agents; AgentsTab supports drag-drop priority with real >1-item list
- Phase 13: Landing FeatureGrid mentions both agents
- Phase 15: Marketing mentions Gemini CLI + Codex
- Plan description + dependencies list restored

### Whole-Plan Consistency Sweep — 2026-05-20 (after Session 2)

| Concern | Result |
|---------|--------|
| Codex restored to implementation steps | Phase 1, 3, 8, 13, 15 updated back |
| Phase 3 `agents/codex.rs` create-list | Restored |
| Phase 1 Codex risk in risk-assessment | Re-added |
| Phase 8 AgentsTab "single-agent stub" copy | Reverted to 2-agent drag-drop |
| Telegram + Apple-Dev + Tailwind + History + Preview decisions from Session 1 | Unchanged (only Q1 was reversed) |

**Status:** Zero unresolved contradictions after Session 2. Plan eligible for `/ck:cook` with Gemini + Codex scope.

### Session 3 — 2026-05-21 (Phase 1 close-out + Path C hybrid)
**Trigger:** `/ck:cook` execution of Phase 1 produced validation data (n=51 fixtures across 3 rounds) that surfaced new facts:
1. CLI-agent latency p95 is structurally 14–46 s — neither agent meets the 6 s plan threshold; Codex is the fastest at 14 s p95 on EQ_ONLY.
2. Gemini CLI's agent-loop has systemic failure mode on EQ_ONLY content (58% success) — tries to `read_file ~/.claude/.ck.json` when image lacks surrounding text, crashes when denied.
3. Codex CLI is reliable across all 3 categories (41/41, 100%).
4. Master prompt had a 5% category-label leak bug on both agents (output begins `MIXED\n`). Patched + verified.
5. TABLE_ONLY emits Markdown tables with inline `$...$`, NOT LaTeX `tabular`. Per "SnipTeX" branding, a LaTeX `tabular` toggle is needed.
6. Inter-agent inconsistency on what gets wrapped in `$...$` inside table cells — Gemini wrapped `$[40;45)$` intervals, Codex did not. Patched + verified.

**Questions asked:** 4 (validation routing + go/no-go + table-format + scope adjustments).

#### Questions & Answers

1. **[Spec]** "TABLE_ONLY format — keep Markdown / switch to LaTeX `tabular` / dual output / both-in-one?"
   - **Answer:** Dual output — Markdown by default in master prompt; Phase 9 owns the LaTeX `\begin{tabular}` toggle as a separate output mode + own validation pass.

2. **[Spec]** "Inter-agent inconsistency on table-cell math scope — patch master prompt now + re-run, patch without re-run, or leave it?"
   - **Answer:** Patch + re-run. Patch verified on the user-cited image (`15.01.25@2x.png`): Gemini now matches Codex character-for-character (intervals stay plain, variables/equations wrapped).

3. **[Go/No-Go]** "Path A Soft-GO Codex-only / Path B Hard-NO-GO pivot / Path C Hybrid?"
   - **Answer:** Path C Hybrid — Codex CLI as BYOA default for privacy-first users, AND add Gemini Vision API direct-call as a built-in `--cloud` (or `--api`) fallback for sub-5-second response. **Reverses Session-1 Q3** ("CLI-only BYOA in v1"). Privacy framing changes from "BYOA only" to "BYOA or BYOK".

4. **[Scope]** "Propagate Path C across Phase 3 / 8 / 13 / 15 now, or defer?"
   - **Answer:** Propagate now, commit separately from the validation commit.

#### Confirmed Decisions (overrides Session 1 Q3)

- **v1 OCR strategy = Hybrid**: Codex CLI (default BYOA) + Gemini CLI (experimental secondary) + Gemini Vision API direct call (new `--cloud` fallback). Reverses Session-1 Q3 "CLI-only BYOA".
- **Privacy framing**: "BYOA or BYOK" (Bring Your Own Agent or Bring Your Own Key). Landing + onboarding + marketing all updated.
- **Latency thresholds revised**: CLI path p95 ≤ 25 s; cloud-API path p95 ≤ 5 s.
- **TABLE_ONLY format**: Markdown stays the master-prompt default. Phase 9 adds the LaTeX `tabular` toggle as a separate output mode with its own validation pass.
- **Master prompt patches** (now in `plans/test-prompt.sh`, must mirror to Rust `ocr/prompt.rs` in Phase 3):
  - `DETECTION (internal, do not emit):` wording — suppresses category-label leak.
  - Table-cell math scope rule — intervals/integers/words stay plain text; only variables/fractions/equations wrap in `$...$`.
- **Gemini CLI gating**: ship behind `--approval-mode plan` or `--policy` tool-disable flags to prevent the `read_file ~/.claude/.ck.json` failure mode. Documented as "experimental secondary" until Phase 3 confirms gating works.

#### Impact on Phases (Session 3 propagation)

- **Phase 1**: status `pending` → `validation-complete`; success criteria checked off / revised; Decision block added pointing to the report.
- **Phase 3**: `cloud_gemini_api.rs` adapter added alongside `gemini_cli.rs` and `codex.rs`. Dispatcher fallback chain has 3 agents. Master prompt const mirrors Session-3 patches. Gemini CLI adapter adds `--approval-mode plan` flag.
- **Phase 8**: Onboarding gains a "Bring Your Own Key (Gemini API)" step. Settings AgentsTab gains a "Use cloud API (faster)" toggle + secure key storage (keychain via `keyring` crate). Default agent recommendation = Codex CLI.
- **Phase 9**: LaTeX `\begin{tabular}` table-output mode added to the Format Toggle scope; own re-validation pass on the 9 table-only fixtures when implemented.
- **Phase 13**: Hero subtitle changes to "Bring your own agent OR your own API key — your choice". FeatureGrid mentions cloud mode. VietnameseSEO mentions cả hai (cả CLI lẫn API key).
- **Phase 15**: Privacy talking point updated to "BYOA or BYOK"; sub-5s response time as a marketing point for cloud mode.
- **`replan.md`** (source brief, historical): Session-1 Q3 noted as reversed by Session 3.

### Whole-Plan Consistency Sweep — 2026-05-21 (after Session 3)

| Concern | Result |
|---------|--------|
| Phase 1 success criteria reflect new thresholds | Updated — checked items + Session-3 revision noted |
| Phase 3 includes cloud Gemini API adapter | Updated — `cloud_gemini_api.rs` added to architecture + steps |
| Phase 8 onboarding + AgentsTab cover cloud-API mode | Updated — new step + toggle |
| Phase 9 includes LaTeX `tabular` toggle | Updated — added to Format Toggle scope |
| Phase 13 landing copy + FeatureGrid reflect BYOA-or-BYOK | Updated |
| Phase 15 marketing reflects hybrid v1 + cloud mode talking point | Updated |
| Plan.md status table for Phase 1 | ✅ Validation Complete |
| Session 1 / Session 2 decisions still in force where not explicitly reversed | Confirmed (only Q3 reversed) |

**Status:** Zero unresolved contradictions after Session 3. Plan eligible for `/ck:cook` from Phase 2.
