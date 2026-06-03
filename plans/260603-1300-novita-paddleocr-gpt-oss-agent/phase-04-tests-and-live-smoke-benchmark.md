---
title: "Phase 04 - Tests and Live Smoke Benchmark"
status: in-progress
priority: P1
effort: 2h
created: 2026-06-03
---

# Phase 04 - Tests and Live Smoke Benchmark

## Context Links

- Existing tests: `src-tauri/tests/rust/cloud_goclaw_api_test.rs`
- Existing smoke: `src-tauri/src/bin/novita_smoke.rs`

## Overview

Add deterministic unit tests and a manual live smoke command that reports quality, latency, and estimated cost.

## Key Insights

- Unit tests should use JSON fixtures, not live Novita.
- Live tests need a real Novita API key, so they must be manual.
- Cost benchmark is required before making hybrid a default fallback.

## Requirements

- Functional:
  - Parser tests for GPT OSS response.
  - Parser tests for GPT OSS response.
  - Error mapping tests.
  - Smoke binary supports image path.
- Non-functional:
  - No secrets in test output.
  - Live smoke prints cost estimate only.

## Architecture

```text
cargo test --test cloud_novita_hybrid_api

NOVITA_API_KEY=...
cargo run --bin novita_hybrid_smoke -- image.png
```

## Related Code Files

- Create: `src-tauri/tests/rust/cloud_novita_hybrid_api_test.rs`.
- Create: `src-tauri/src/bin/novita_hybrid_smoke.rs`.
- Modify: `src-tauri/Cargo.toml`.

## Implementation Steps

1. Add test target in `Cargo.toml`.
2. Test parser happy path.
3. Test empty output paths.
4. Test redaction.
5. Test DispatchError mapping.
6. Add smoke binary:
   - Reads env vars.
   - Runs hybrid adapter.
   - Prints char counts, estimated GPT tokens, latency.
   - Prints output preview only if user passes `--show-output`.
7. Run:
   - `pnpm build`
   - `cargo test --test cloud_novita_hybrid_api`
   - relevant full Rust test subset.

## Todo List

- [x] Add no-network unit tests.
- [x] Add live smoke binary.
- [x] Run frontend compile.
- [x] Run targeted Rust tests.
- [ ] Run live smoke with real endpoint.

## Success Criteria

- All no-network tests pass.
- Live smoke produces usable LaTeX/Markdown on at least 5 representative screenshots.
- p95 latency and per-call cost are documented.
- Hybrid quality beats existing `cloud-novita` on math/table samples or rollout stops.

## Risk Assessment

- Risk: DeepSeek quality good but GPT cleanup worsens output.
  - Mitigation: compare DeepSeek-only vs hybrid output in smoke.
- Risk: Live smoke leaks screenshot content in terminal history.
  - Mitigation: preview off by default.

## Security Considerations

- Env vars must not be committed.
- Redact keys in all failures.
- Avoid writing smoke outputs to tracked files.

## Next Steps

- Phase 05 documents benchmark and rollout decision.
