---
phase: 4
title: "auto-local-fast Router"
status: complete
priority: P1
effort: 1d
dependencies: [2, 3]
---

# Phase 4: auto-local-fast Router

## Context Links

- `src-tauri/src/ocr/dispatcher.rs`
- `src-tauri/src/ocr/smart_format.rs`
- `src-tauri/src/agents/local_ocr_api.rs`
- `src-tauri/src/agents/local_ocr_router.rs`
- `src-tauri/src/agents/local_ocr_client.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/storage/history.rs`

## Overview

Add an agent that chooses `local-pix2tex` or `local-paddleocr` per image. If local is unavailable, low-confidence, or unsupported, fallback continues to existing cloud agents.

## Key Insights

- Router must be conservative. Wrong fast path is worse than cloud fallback.
- Initial v1 should classify into `equation`, `text`, `table`, `mixed`, `unknown`.
- Mixed and all table images should prefer cloud in v1.
- Router MUST consult the `LocalHealthCache` from Phase 1 before calling `/classify`. If health is `Unhealthy` (cached within last 30 s) the router returns `AgentNotAvailable` immediately — no HTTP round trip.
- The classifier itself lives in the daemon (Phase 6). The router is the *consumer* of `/classify`; it does not own the classification model.

## Requirements

Functional:
- `auto-local-fast` appears as selectable agent when classifier capability exists.
- Fallback chain can include `auto-local-fast` before `cloud-mistral`.
- Direct selection of `auto-local-fast` still falls back internally to cloud only if the normal fallback chain is active; direct rerun should return router failure clearly unless configured otherwise.
- Record history agent as the **final agent that actually produced text** (e.g. `local-pix2tex`) — DO NOT invent compound identifiers like `auto-local-fast:local-pix2tex`. `HistoryRecord.agent: String` stays a single registered agent id; the router origin is captured in a new optional `via: Option<String>` metadata field (defaulting to `None` for direct calls, `Some("auto-local-fast")` when routed).

Non-functional:
- Classification budget <= 300ms target, 800ms hard timeout.
- Router adds minimal overhead.

## Architecture

```text
auto-local-fast(image)
  -> /classify
  -> if equation confidence high: local-pix2tex
  -> if text confidence high: local-paddleocr
  -> else: return unsupported so dispatcher tries next fallback
```

Rules (thresholds calibrated in Phase 6 — see `scripts/local-ocr-server/classifier.py` constants `EQUATION_THRESHOLD`, `TEXT_THRESHOLD`; do NOT hard-code the numbers in two places):

```text
equation >= EQUATION_THRESHOLD     -> pix2tex
text     >= TEXT_THRESHOLD AND
           classifier.kind == text  -> paddleocr
table (any)                         -> fallback (cloud-mistral)
mixed / unknown                     -> fallback
```

PaddleOCR adapter itself also rejects table-shaped output as a defense in depth (see Phase 3).

## Related Code Files

- Modify: `src-tauri/src/agents/registry.rs`.
- Modify: `src-tauri/src/ocr/dispatcher.rs`.
- Modify: `src-tauri/src/commands.rs` if history agent label needs final-agent detail.
- Modify: `src/stores/settings-store.ts` default priority.

## Implementation Steps

1. Add `AUTO_LOCAL_FAST_ID`.
2. Check `LocalHealthCache` (Phase 1) — short-circuit to `AgentNotAvailable` if unhealthy.
3. Implement classifier call.
4. Dispatch internally to local pix2tex/PaddleOCR adapter functions; populate `via: Some("auto-local-fast")` on the result.
5. Return `DispatchError::AgentNotAvailable` or `BadRequest("local unsupported")` for fallback-compatible cases.
6. Update default fallback chain after user opt-in (flag is `settings.local_ocr_enabled`):
   - `auto-local-fast`
   - `cloud-mistral`
   - other configured agents
   - When the flag is OFF, default chain is unchanged from current (no surprises for users who don't install local OCR).
7. Add fixtures:
   - equation-only routes pix2tex
   - Vietnamese paragraph routes PaddleOCR
   - mixed formula+text returns fallback
   - table simple AND table complex both return fallback (no local table support in v1)
   - daemon dead → cached unhealthy → no HTTP attempt + clean fallback

## Todo List

- [x] Router id added.
- [x] Classifier response parser implemented.
- [x] Conservative routing rules added.
- [x] Fallback behavior tested.
- [x] History stores final `agent_id` plus nullable `via_agent_id`.
- [x] Local image POSTs disable proxy and redirects.

## Success Criteria

- Common text/equation images avoid Codex/Gemini CLI.
- Mixed hard images still reach `cloud-mistral`.
- No regression to complex table handling.
- History records show concrete final agent with optional `via auto-local-fast`.

## Risk Assessment

- Risk: classifier wrong. Mitigation: conservative thresholds and fallback.
- Risk: history agent label confusing. Mitigation: `agent` field = final agent (`local-pix2tex` etc.); `via` field carries the router origin. UI in Settings → History can display "via auto-local-fast" as a small badge next to the agent name when `via` is present.

## Security Considerations

- Do not let classifier response choose arbitrary endpoint names.
- Do not follow redirects for local OCR image uploads; the configured base URL must remain loopback-only.

## Next Steps

- Phase 5 handles install UX and docs.
