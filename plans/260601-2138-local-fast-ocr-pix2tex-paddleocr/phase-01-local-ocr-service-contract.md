---
phase: 1
title: "Local OCR Service Contract"
status: complete
priority: P1
effort: 0.5d
dependencies: []
---

# Phase 1: Local OCR Service Contract

## Context Links

- `src-tauri/src/agents/registry.rs`
- `src-tauri/src/agents/mod.rs`
- `src-tauri/src/agents/local_ocr_api.rs`
- `src-tauri/src/ocr/dispatcher.rs`
- `src/lib/invoke.ts`
- `src/windows/settings/agents-tab.tsx`

## Overview

Define a stable localhost HTTP contract for optional OCR packs. SnipTeX must not care whether the daemon uses Python, Docker, or a future native sidecar.

## Key Insights

- The speed win comes from keeping models warm in a daemon.
- CLI-per-snip would recreate the same latency problem as Codex/Gemini CLI.
- Local services need health/status, not API keys.
- Daemon lifecycle is **user-managed in v1** — user runs `scripts/run-local-ocr-server.sh` manually (or sets up launchd/Task Scheduler on their own). Tauri sidecar auto-spawn is deferred to vNext because it requires bundling a Python runtime, which contradicts the "no Python dependency for users who skip local OCR" goal.
- Dispatcher must cache last-known daemon health so a dead daemon does NOT cost 800 ms classify timeout per snip. Cache TTL: 30 s for unhealthy, 5 s for healthy (re-check often when working, back off when broken).

## Requirements

Functional:
- Add configurable `local_ocr_url`, default `http://127.0.0.1:8765`.
- Detect local daemon readiness without blocking app startup.
- Surface available capabilities: `pix2tex`, `paddleocr`, `classifier`.
- Return structured OCR responses with confidence and detected type hints.

Non-functional:
- Health check timeout <= 500ms.
- OCR request timeout separate from CLI timeout, target 10s.
- No internet needed after local packs are installed.

## Architecture

```http
GET /health
-> { "ok": true, "version": "...", "capabilities": ["pix2tex", "paddleocr", "classifier"] }

POST /classify multipart image=@capture.png
-> { "kind": "equation" | "text" | "mixed" | "table" | "unknown", "confidence": 0.0-1.0 }

POST /ocr/pix2tex multipart image=@capture.png
-> { "text": "\\frac{a}{b}", "detected": "EQUATION_ONLY", "confidence": 0.0-1.0 }

POST /ocr/paddleocr multipart image=@capture.png
-> 200 { "text": "...", "detected": "MIXED", "confidence": 0.0-1.0 }
-> 422 { "error": "unsupported_table" }   // v1 is paragraph-only; daemon rejects table-shaped input here so the sniptex adapter falls through to cloud-mistral. TABLE_ONLY is never returned by this endpoint in v1.
```

## Related Code Files

- Modify: `src-tauri/src/settings.rs` — add local OCR URL and enable flags.
- Modify: `src-tauri/src/agents/registry.rs` — add `AgentKind::LocalHttp` and local agent specs.
- Create: `src-tauri/src/agents/local_ocr_api.rs` — health + OCR adapter.
- Modify: `src-tauri/src/agents/mod.rs` — detect local capabilities via health endpoint.
- Modify: `src/lib/invoke.ts` — expose local readiness fields.

## Implementation Steps

1. Add settings fields:
   - `local_ocr_enabled: bool`
   - `local_ocr_url: String` — validated to start with `http://127.0.0.1` or `http://localhost`; any other prefix is rejected at settings-update time
   - `local_ocr_formula_enabled: bool`
   - `local_ocr_text_enabled: bool`
2. Add `AgentKind::LocalHttp`.
3. Add specs:
   - `local-pix2tex`
   - `local-paddleocr`
   - `auto-local-fast`
4. Implement health check with short timeout.
5. Only mark local agents installed if daemon advertises capability.
6. Add a `LocalHealthCache` struct (per-URL: `last_check_ts`, `last_status: Healthy | Unhealthy`, `last_capabilities: Vec<String>`) consulted by dispatcher/detect_agents:
   - `Healthy` TTL: 5 s — re-probe to catch crashes quickly.
   - `Unhealthy` TTL: 30 s — back off so a dead daemon doesn't repeatedly steal 500 ms × every snip.
   - On any successful OCR call against a local agent, refresh the cache.
7. Add unit tests for parsing health/capability responses + cache TTL behaviour (clock-injectable for test).

## Todo List

- [x] Settings fields added with URL prefix validation.
- [x] Local agent specs added.
- [x] Health response parser implemented.
- [x] `LocalHealthCache` implemented with TTL and clock injection.
- [x] Detection surfaces local agents only when ready.

## Success Criteria

- [x] `detect_agents` includes local agents only when daemon is reachable.
- [x] App still works when daemon is missing.
- [x] No local model dependency included in Tauri app build.
- [x] Settings rejects `local_ocr_url` values that are not `http://127.0.0.1*` or `http://localhost*`.
- [x] Health cache test passes: after one failed probe, 29 further snip dispatches do NOT issue a real HTTP call.

## Risk Assessment

- Risk: local health check slows settings scan. Mitigation: 500ms timeout, best-effort failure, cached result.
- Risk: localhost port conflict. Mitigation: configurable URL within the loopback whitelist.
- Risk: stale health cache hides a recovered daemon for up to 30 s. Acceptable trade-off — user can hit "Test local OCR" in Settings to force a re-probe (Phase 5 wires the button).

## Security Considerations

- Only allow `http://127.0.0.1` or `http://localhost`. **Do NOT** allow arbitrary user-confirmed URLs in v1 — the existing cloud agents already cover the "talk to a remote service" use case, and a wider allow-list would surface privacy/exfil concerns this phase deliberately sidesteps.
- Never send API keys to local daemon.

## Next Steps

- Phase 2 implements pix2tex adapter.
