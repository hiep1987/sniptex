---
title: "Phase 05 - Documentation and Rollout Decision"
status: in-progress
priority: P2
effort: 1h
created: 2026-06-03
---

# Phase 05 - Documentation and Rollout Decision

## Context Links

- [Plan overview](./plan.md)
- Main docs directory: `docs/`
- Parent roadmap: `plans/260520-0603-sniptex-tauri-mvp-v1/plan.md`

## Overview

Document setup, benchmark results, and whether hybrid should be default, opt-in, or deferred.

## Key Insights

- Cost optimization is a product decision, not only code.
- GPT cleanup adds extra token cost; default rollout needs benchmark data.
- Existing Novita DeepSeek-OCR 2 remains the safer baseline.

## Requirements

- Functional:
  - Add setup docs for Novita hybrid.
  - Record benchmark table.
  - State rollout decision.
- Non-functional:
  - No API keys in docs.
  - Clear warning for on-demand GPU cost.

## Architecture

Documentation should explain:

```text
cloud-novita        = DeepSeek-OCR 2 direct OCR
cloud-novita-hybrid = DeepSeek-OCR 2 parse + GPT OSS 120B cleanup
```

## Related Code Files

- Create or modify: `docs/cloud-novita-hybrid.md`.
- Modify: `docs/system-architecture.md` only if present.
- Modify: `docs/project-changelog.md` only if present.
- Modify: `plans/260520-0603-sniptex-tauri-mvp-v1/plan.md` if milestone status changes.

## Implementation Steps

1. Document required env/settings:
   - Novita API key.
   - Novita API key.
2. Add benchmark table:
   - sample
   - existing agent output score
   - hybrid output score
   - DeepSeek latency
   - GPT latency
   - estimated GPT token cost
   - estimated total token cost if available.
3. Decide rollout:
   - Default fallback only if cost and quality win.
   - Otherwise manual/rerun-only.
4. Update changelog/roadmap if repo docs exist.

## Todo List

- [x] Add docs page.
- [ ] Add benchmark result table.
- [x] Record rollout decision.
- [ ] Update roadmap/changelog if present.

## Success Criteria

- A developer can configure and test hybrid without reading code.
- Cost warning is explicit.
- Rollout decision is backed by data.

## Risk Assessment

- Risk: Docs imply hybrid is default-ready before live benchmark.
  - Mitigation: explicitly state opt-in/manual status.

## Security Considerations

- Never paste real API keys.
- Avoid screenshots containing user private content in benchmark docs.

## Next Steps

- If benchmark passes, create follow-up plan to tune default priority and optional endpoint UI.
