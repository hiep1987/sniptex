---
title: "Local Fast OCR With pix2tex and PaddleOCR"
description: "Add optional local OCR fast paths for equation-only and Vietnamese text snips while keeping cloud OCR as quality fallback."
status: in-progress
priority: P2
effort: 5d
branch: main
tags: [feature, backend, frontend, ocr, performance]
created: 2026-06-01
blockedBy: []
blocks: []
---

# Local Fast OCR With pix2tex and PaddleOCR

## Overview

Add optional local OCR agents that beat the CLI agents (Codex / Gemini CLI, p95 ~14 s baseline measured Session 3) for common SnipTeX work: equation-only images and Vietnamese text. Local is a fast path for users who set it up; cloud agents (Mistral OCR, Gemini API, Goclaw) remain the default for trivial cases and the only path for mixed/complex tables. Honest comparison: local will tie or marginally beat cloud BYOK on small snips, but win decisively against CLI agents because warm models skip the per-snip process spawn.

## Goals

- Fast local equation OCR via `local-pix2tex`.
- Fast Vietnamese **paragraph text** OCR via `local-paddleocr` (tables fall back to cloud — see Non-Goals).
- `auto-local-fast` router chooses local tool, then falls back to cloud.
- No bundled model weights in the app or repo.
- No Python dependency required for users who do not enable local OCR.
- Ship a reference daemon implementation (FastAPI + heuristic classifier) so power users can set up local OCR without inventing the server.
- Daemon serializes per-model requests (asyncio lock) — concurrent snips queue instead of racing PyTorch state.

## Non-Goals

- Do not replace `cloud-mistral` OCR.
- Do not support ANY tables locally in v1 — simple or complex. PaddleOCR's line-with-bbox output does not include row/column grouping; assembling Markdown from bboxes is non-trivial reconstruction work (cluster by y, then x). All table snips fall back to cloud-mistral. Plain text + equations only locally in v1; table support arrives in v1.x once a row/column clustering pass is benchmarked against the 9 TABLE_ONLY fixtures.
- Do not spawn local Python CLIs per snip; local must run as daemon/server.
- Do not add heavyweight VLMs such as Qwen/Surya in this plan.
- Do not allow non-localhost daemon URLs in v1 (only `http://127.0.0.1` and `http://localhost`); LAN / remote-proxy modes are covered by the existing cloud agents.
- Do not surface local OCR in onboarding. It's a power-user feature visible only in Settings → Agents (matches memory `feedback-copy-as-menu-simplification` rule about not advertising features without clear universal value).

## Phases

| # | Phase | Status | Effort | Link |
|---|-------|--------|--------|------|
| 1 | Service Contract | Complete | 0.5d | [phase-01](./phase-01-local-ocr-service-contract.md) |
| 2 | pix2tex Agent | Complete | 1d | [phase-02](./phase-02-local-pix2tex-agent.md) |
| 3 | PaddleOCR Agent | Complete | 1d | [phase-03](./phase-03-local-paddleocr-agent.md) |
| 4 | Auto Router | Complete | 1d | [phase-04](./phase-04-auto-local-fast-router.md) |
| 5 | Installer UI + Verification | Pending | 0.5d | [phase-05](./phase-05-installer-ui-and-verification.md) |
| 6 | Local Daemon Reference Impl | Pending | 1d | [phase-06](./phase-06-local-daemon-reference-impl.md) |

## Architecture

```text
SnipTeX Tauri
  -> detect local daemon health
  -> local-pix2tex | local-paddleocr | auto-local-fast
  -> if empty/low-confidence/unsupported: cloud-mistral OCR
```

Local daemon:

```text
GET  /health
POST /classify
POST /ocr/pix2tex
POST /ocr/paddleocr
```

## Key Decisions

- Add `AgentKind::LocalHttp` instead of pretending localhost services are cloud APIs.
- Keep individual agents selectable for testing/rerun.
- Add `auto-local-fast` as a convenience router in Phase 4; keep it hidden from readiness detection until the router exists.
- Store local daemon URL in settings, default `http://127.0.0.1:8765`.
- Scripts install optional model packs outside git-tracked source.

## Success Criteria

- Equation-only snips route to local pix2tex when daemon is ready.
- Vietnamese text snips route to local PaddleOCR when daemon is ready.
- Local unavailable never blocks normal cloud workflow.
- **Benchmark gate (hard requirement before declaring complete):** measured on M1 Pro with daemon hot, recorded in `docs/local-fast-ocr.md`:
  - equation-only p95 < 2 s (Codex CLI baseline ~14 s, Session 3 fixture set)
  - Vietnamese text p95 < 2 s
  - classifier mis-route rate < 5 % across the 9 TABLE_ONLY + 10 EQUATION_ONLY + 10 MIXED fixtures from Phase 1 of MVP plan
- When daemon health is unknown or unhealthy in the last 30 s, dispatcher skips local agents in fallback chain (no 800 ms classify wait per snip — see Phase 1 health cache).
- Settings clearly show local OCR install/readiness state.
