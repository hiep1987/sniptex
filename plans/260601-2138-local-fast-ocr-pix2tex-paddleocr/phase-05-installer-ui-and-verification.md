---
phase: 5
title: "Installer UI and Verification"
status: pending
priority: P2
effort: 0.5d
dependencies: [1, 2, 3, 4, 6]
---

# Phase 5: Installer UI and Verification

## Context Links

- `src/windows/settings/agents-tab.tsx`
- `src/stores/settings-store.ts`
- `docs/latex-table-reconstruction.md`

## Overview

Make local OCR understandable for open-source users. Local packs are optional, visible, testable, and easy to disable.

## Key Insights

- Do not bundle model weights.
- Do not require Python for default app use.
- One-click install can come later; v1 can provide scripts and health checks.
- macOS and Windows are first-class. SnipTeX MVP Phase 10 ships Windows port; local OCR Mac-only would create feature asymmetry. Provide platform-specific scripts side-by-side from v1.
- Daemon lifecycle is user-managed (see Phase 1 Key Insights). Phase 5 ships the scripts AND a "Test local OCR" button that forces a health re-probe so users can confirm their setup works.
- Local OCR is a **power-user feature** surfaced ONLY in Settings → Agents. Onboarding stays at 5 steps (Welcome / Install CLI / CloudKey / Hotkey / Ready) — adding a 6th step for an optional advanced feature would dilute first-run UX. New users find local OCR by exploring Settings.

## Requirements

Functional:
- Settings shows Local OCR section.
- Show status: Not installed, Ready, Error, Capability missing.
- Buttons:
  - Test local OCR — forces a re-probe, bypassing health cache TTL.
  - Open install guide
  - Disable local fast path
- Provide install scripts/docs for **macOS AND Windows** as parallel deliverables. Linux is best-effort (same Python scripts as macOS, no `.deb`/`.rpm`).

Non-functional:
- User can recover from broken local environment without editing config.
- App fallback remains obvious.

## Architecture

```text
Settings -> Agents
  Local OCR
    URL input
    Health status
    capabilities chips
    install guide links
```

## Related Code Files

- Modify: `src/windows/settings/agents-tab.tsx`.
- Modify: `docs/system-architecture.md` — add `AgentKind::LocalHttp` to agent taxonomy diagram.
- Modify: `docs/codebase-summary.md` — note the new `agents/local_ocr_api.rs` and `scripts/local-ocr-server/` daemon subtree.
- Create: `docs/local-fast-ocr.md` — install guide + benchmark table + troubleshooting.
- Create (macOS / Linux): `scripts/install-local-pix2tex.sh`.
- Create (macOS / Linux): `scripts/install-local-paddleocr.sh`.
- Create (macOS / Linux): `scripts/run-local-ocr-server.sh`.
- Create (Windows): `scripts/install-local-pix2tex.ps1`.
- Create (Windows): `scripts/install-local-paddleocr.ps1`.
- Create (Windows): `scripts/run-local-ocr-server.ps1`.

## Implementation Steps

1. Add Local OCR card in Settings → Agents.
2. Add health check button invoking backend health command (with `force_refresh: true` to bypass cache TTL).
3. Document install sizes and expectations in `docs/local-fast-ocr.md`:
   - pix2tex: formula-only (~600 MB venv with torch CPU)
   - PaddleOCR: Vietnamese paragraph text (~1.2 GB venv with paddle CPU; tables fall back to cloud)
   - cloud fallback remains recommended
4. Add macOS / Linux shell scripts using virtualenv under app data or user-selected directory.
5. Add Windows PowerShell scripts (`.ps1`) mirroring the shell scripts — same venv pattern, `python -m venv`, `pip install` from the Phase 6 daemon's `requirements.txt`.
6. Add verification checklist + benchmark run:
   - equation fixture (latency recorded)
   - Vietnamese paragraph fixture (latency recorded; tables are out of scope for local)
   - 9 TABLE_ONLY fixtures: classifier mis-route rate (target <5%) — all should route to cloud fallback
   - 10 EQUATION_ONLY fixtures: classifier mis-route rate (target <5%) — all should route to pix2tex
   - 10 MIXED fixtures: classifier mis-route rate (target <5%) — all should route to cloud fallback
   - local unavailable fallback
   - cloud fallback for any table (simple or complex)
   - results pasted into `docs/local-fast-ocr.md` benchmark table
7. Update docs: `docs/system-architecture.md` agent taxonomy, `docs/codebase-summary.md` module list.

## Todo List

- [ ] Settings Local OCR card added.
- [ ] "Test local OCR" button forces health re-probe.
- [ ] docs/local-fast-ocr.md added with install + benchmark + troubleshooting sections.
- [ ] macOS / Linux install + run scripts added.
- [ ] Windows .ps1 install + run scripts added.
- [ ] docs/system-architecture.md updated with `AgentKind::LocalHttp`.
- [ ] docs/codebase-summary.md updated with new modules.
- [ ] Benchmark table recorded with measured p95 latencies + classifier mis-route rate.

## Success Criteria

- New user can ignore local OCR and app still works.
- Power user on macOS OR Windows can install local OCR and see Ready status.
- Local fast path can be disabled with one toggle.
- Benchmark gate from `plan.md` Success Criteria passes (equation p95 < 2 s, text p95 < 2 s, mis-route < 5 %).

## Risk Assessment

- Risk: Python installs are fragile. Mitigation: explicit scripts, logs, and no default dependency.
- Risk: GitHub release size grows. Mitigation: no bundled models.

## Security Considerations

- Installation scripts must not download or execute unpinned arbitrary shell from the internet.
- Never store model files in git.

## Next Steps

- After v1, consider Pix2Text/Surya as an advanced mixed/table local pack only if benchmark beats cloud latency enough to justify install weight.
