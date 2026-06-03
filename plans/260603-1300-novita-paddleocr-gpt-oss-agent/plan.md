---
title: "Novita OCR + GPT OSS Agent Plan"
description: "Add a cost-controlled Novita hybrid OCR agent that uses DeepSeek-OCR 2 for document parsing and GPT OSS 120B for LaTeX cleanup."
status: in-progress
priority: P2
effort: 10h
branch: main
tags: [feature, backend, frontend, api, experimental]
created: 2026-06-03
---

# Novita OCR + GPT OSS Agent Plan

## Overview

Add `cloud-novita-hybrid`: image -> DeepSeek-OCR 2 markdown -> GPT OSS 120B final LaTeX/Markdown. Optimize cost by sending only OCR markdown into GPT OSS, and do not replace the existing DeepSeek-OCR 2 Novita agent.

## Key Decisions

- Keep existing `cloud-novita` agent unchanged.
- Add new agent ID: `cloud-novita-hybrid`.
- Reuse existing Novita API key provider `"novita"`.
- Require only the existing Novita API key before marking hybrid as available.
- Do not add hybrid to default fallback until benchmark passes.
- GPT cleanup receives markdown only by default, not the original image.

## Phases

| # | Phase | Status | Effort | Link |
|---|-------|--------|--------|------|
| 1 | API Contract and Cost Controls | In Progress | 2h | [phase-01](./phase-01-api-contract-and-cost-controls.md) |
| 2 | Hybrid Backend Adapter | Complete | 3h | [phase-02](./phase-02-hybrid-backend-adapter.md) |
| 3 | Registry, Settings UI, and Config | Complete | 2h | [phase-03](./phase-03-registry-settings-ui-and-config.md) |
| 4 | Tests and Live Smoke Benchmark | In Progress | 2h | [phase-04](./phase-04-tests-and-live-smoke-benchmark.md) |
| 5 | Documentation and Rollout Decision | In Progress | 1h | [phase-05](./phase-05-documentation-and-rollout-decision.md) |

## Dependencies

- Novita API key already supported in keychain and Settings.
- Need live Novita API key for smoke tests.
- GPT OSS 120B endpoint: `https://api.novita.ai/openai/v1/chat/completions`.

## Architecture

```text
Snip image
  -> cloud_novita_hybrid_api::call_with_image_path
  -> DeepSeek-OCR 2 serverless
  -> normalized intermediate markdown
  -> GPT OSS 120B chat completion
  -> shared post_process
  -> History / Preview
```

## Cost Policy

- Do not require GPU endpoint management from the desktop app.
- Keep GPT input text-only so cleanup cost stays small.
- Cap GPT input markdown and output tokens.
- Log only cost metrics, not OCR content.
- Add manual smoke command reporting estimated GPT token cost.

## Cook Handoff

Run after review:

```bash
/ck:cook /Users/hieplequoc/Projects/sniptex/plans/260603-1300-novita-paddleocr-gpt-oss-agent/plan.md
```
