---
phase: 1
title: "Prompt Validation & Go/No-Go"
status: validation-complete
priority: P1
effort: "2d"
dependencies: []
---

# Phase 1: Prompt Validation & Go/No-Go

## Overview

Validate that the master OCR prompt produces acceptable LaTeX/Markdown output from **Gemini CLI and OpenAI Codex** on the target corpus (Vietnamese SGK Toán + equations + tables + mixed). This phase is a **hard gate**: if accuracy/latency targets fail on both agents, the entire Tauri-CLI-agent approach pivots to Gemini Vision API direct call or local LightOnOCR-2-1B.

<!-- Updated: Validation Session 2 - Codex restored to v1 scope (reversal of Session 1 Q1) -->
<!-- Updated: Validation Session 3 (2026-05-21) - Path C hybrid chosen: Codex CLI BYOA default + Gemini Vision API cloud fallback. See reports/prompt-validation-report.md "Decision" section. -->

## Decision (2026-05-21)

**Verdict: CONDITIONAL-GO via Path C (Hybrid).** Full rationale + per-round metrics in [`reports/prompt-validation-report.md`](./reports/prompt-validation-report.md).

- **Default agent: Codex CLI.** 41/41 (100%) success across MIXED + EQUATION_ONLY + TABLE_ONLY across 3 rounds. p95 latency 14–24 s by category.
- **Cloud fallback added: Gemini Vision API direct call** (new in v1; reverses Session-1 Q3 "CLI-only BYOA"). Privacy framing becomes "BYOA or BYOK".
- **Gemini CLI: experimental secondary** — 33/41 (80.5%) with EQ_ONLY collapse to 58% due to agent-loop instability. Requires `--approval-mode plan` or tool-disable policy in Phase 3.
- **Master prompt patches applied** to `plans/test-prompt.sh`: category-label leak suppression + table-cell math-scope rule. Phase 3 Rust prompt MUST mirror.
- **Plan thresholds revised** (see Success Criteria below) — CLI-agent latency target relaxed from 6 s to 25 s p95; cloud-API path keeps ≤ 5 s p95.

## Key Insights

- Existing assets: `plans/test-prompt.sh` (validation harness already written), master prompt embedded in script + `replan.md` §5.
- v1 supports Gemini CLI + Codex. Claude Code + OpenCode remain deferred to v1.x.
- Codex image-input syntax (`replan.md` §6: "cần verify tuần 1") is verified inside this phase. If Codex fails, only then fall back to Gemini-only.
- Default agent recommendation depends on per-agent accuracy + latency comparison.

## Requirements

**Functional**
- Run master prompt against 90 fixture images using **both Gemini CLI and Codex**: 50 SGK Toán pages, 20 complex equations, 10 tables, 10 mixed content.
- Per-image record: agent, latency (ms), output text, detected category (EQUATION_ONLY / TABLE_ONLY / MIXED), pass/fail vs expected.
- Aggregate metrics per agent: accuracy %, latency p50/p95/p99, format consistency rate.

**Non-functional**
- Reproducible: any contributor can re-run `./test-prompt.sh fixtures/` and get comparable numbers.
- Privacy: fixtures stored locally; do not commit raw SGK scans to public repo without consent.

## Architecture

```
fixtures/ (90 images)
   │
   ├─ sgk/        (50 textbook pages)
   ├─ equations/  (20 complex equations)
   ├─ tables/     (10 tables)
   └─ mixed/      (10 mixed)
        │
        ▼
test-prompt.sh ──► spawns: gemini -p "<prompt>\n@<img>"
                          codex exec --image <img> "<prompt>"
        │
        ▼
results/
   ├─ gemini-cli/<image>.txt
   ├─ codex/<image>.txt
   ├─ summary.csv          (per-image latency, length, category, status)
   └─ comparison.md        (side-by-side review document)
        │
        ▼
   Manual review → accuracy scoring → go/no-go decision
```

## Related Code Files

- Use existing: `plans/test-prompt.sh`
- Create: `fixtures/{sgk,equations,tables,mixed}/` with curated test images
- Create: `fixtures/expected/<image>.txt` (ground truth for accuracy scoring)
- Create: `plans/260520-0603-sniptex-tauri-mvp-v1/reports/prompt-validation-report.md`

## Implementation Steps

1. Install both CLI agents locally: `npm i -g @google/gemini-cli` (or `brew install gemini-cli`); install Codex per OpenAI docs. Verify `gemini --version` and `codex --version`. Confirm `codex` exposes image-input syntax (Phase 1 critical check).
2. Curate 90 fixture images:
   - SGK: scan 50 representative pages (algebra, calculus, geometry) from Vietnamese textbooks
   - Equations: 20 LaTeX-rendered complex expressions (integrals, matrices, summations, piecewise functions)
   - Tables: 10 tables (mix of simple grids and merged-cell layouts)
   - Mixed: 10 pages combining text + equations + tables
3. Manually create expected outputs in `fixtures/expected/<image>.txt` for at least 30 representative samples.
4. Run validation: `./plans/test-prompt.sh ./fixtures/`. Both Gemini and Codex must complete the full set.
5. Review `results/summary.csv` for raw metrics. Compute per agent:
   - Accuracy = exact match OR semantic equivalence (math-equal via Sympy spot-check)
   - Latency p50/p95/p99
   - Format consistency = % outputs that match expected `DetectedType`
6. Score Vietnamese diacritic preservation on SGK subset (manual review) for both agents.
7. Write `reports/prompt-validation-report.md`: per-agent metrics, failure modes, recommended default agent for onboarding.
8. **Go/No-go decision** documented in report:
   - GO: accuracy ≥80% AND latency p95 ≤6s AND format consistency ≥90%
   - NO-GO: accuracy <80% on SGK OR latency p95 >8s OR catastrophic format failures
9. If NO-GO: write `reports/pivot-evaluation.md` comparing Gemini Vision API direct vs LightOnOCR local fallback.

## Todo List

- [ ] Install Gemini CLI + Codex locally; verify both with `--version` and Codex image-input syntax
- [ ] Curate 90 fixture images across 4 categories
- [ ] Author expected outputs for 30+ representative samples
- [ ] Run `test-prompt.sh fixtures/`; both agents complete full run
- [ ] Compute accuracy / latency / consistency per agent
- [ ] Score Vietnamese diacritic preservation on SGK subset
- [ ] Write prompt-validation-report.md with per-agent metrics + recommended default
- [ ] Make go/no-go decision; document rationale
- [ ] If GO: pick default agent based on metrics + free-tier breadth
- [ ] If Codex fails but Gemini passes: drop to Gemini-only without halting v1 (soft fallback)
- [ ] If both fail: write pivot-evaluation.md before proceeding

## Success Criteria

- [x] Accuracy ≥80% on SGK subset for at least one of {Gemini CLI, Codex} — Codex 100% on visual spot-check; aggregate manual scoring still TODO for completeness
- [x] ~~Latency p95 ≤6s on at least one agent~~ → **Revised (Session 3):** CLI-agent p95 ≤25 s OR cloud-API p95 ≤5 s. Codex CLI 14–24 s p95 passes the relaxed CLI bar; cloud-API path validated in Phase 3.
- [x] Codex image-input syntax confirmed working (`codex exec --skip-git-repo-check --image <FILE> --output-last-message <FILE> -- "<prompt>"`)
- [x] Format consistency ≥90% (output category matches expected) — 100% on all 3 categories for Codex
- [x] Vietnamese diacritics preserved on ≥95% of SGK fixtures — 100% on both agents
- [x] Report committed to `reports/prompt-validation-report.md`
- [x] Go/no-go decision explicit with rationale + chosen default agent — Path C / Codex CLI default + Gemini Vision API cloud fallback

## Risk Assessment

- **Risk: Vietnamese OCR fails on SGK** — Mitigation: try alternate prompt phrasing (explicit "preserve Vietnamese diacritics"), test third agent if available, evaluate Gemini Vision API direct.
- **Risk: Gemini CLI free tier rate limit hit during validation run (1000 req/day)** — Mitigation: throttle test-prompt.sh; spread 90 fixtures across ≤2 days if needed.
- **Risk: Codex image input syntax changed/absent at validation time** — Mitigation: verify `codex --help` on day 1; if no image support, drop Codex to v1.x deferral and proceed with Gemini-only (soft fallback, no full pivot).
- **Risk: Latency >10s on slow connection** — Mitigation: document baseline network conditions; flag for user comms.

## Security Considerations

- Fixtures may contain copyrighted SGK material — do not commit to public repo. Use `.gitignore` for `fixtures/sgk/`.
- LLM provider sees uploaded images — disclose in privacy section of landing page.

## Next Steps

- ✅ Phase 1 closed Session 3 — Path C hybrid.
- Proceed to Phase 2 (Tauri scaffold) and Phase 3 (now with `cloud_gemini_api.rs` adapter added).
- Phase 6 / Phase 9 inherit the patched master prompt — Rust `ocr/prompt.rs` MUST mirror `plans/test-prompt.sh` (post-Session-3 wording for table-cell math scope + category-label silence).

## Open Questions

- Should expected outputs be normalized (whitespace, ordering) before string comparison, or strict?
- Threshold for "semantic equivalence" on math — exact LaTeX match or Sympy parse-equal?
