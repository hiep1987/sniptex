---
title: "Gemini CLI Clean Rerun Output Fix"
status: implementation-complete-gui-smoke-pending
created: 2026-05-24
priority: P1
scope: "Phase 7 history rerun reliability"
---

# Gemini CLI Clean Rerun Output Fix

## Goal

Make History rerun with `gemini-cli` produce cleaner OCR output and fail safely when Gemini's tool loop leaks errors, while keeping Codex as the default reliable agent.

## Context

- Active parent phase: [Phase 7 SQLite History with FTS5 Search](../260520-0603-sniptex-tauri-mvp-v1/phase-07-sqlite-history-with-fts5-search.md)
- Evidence: [Prompt Validation Report](../260520-0603-sniptex-tauri-mvp-v1/reports/prompt-validation-report.md)
- Existing note: [Gemini CLI Rerun Fix](../../260523-gemini-cli-rerun-fix.md)

## Phases

1. [Phase 01 - Gemini CLI JSON Output Contract](./phase-01-gemini-cli-json-output-contract.md) - complete
2. [Phase 02 - Isolated Gemini Workspace Staging](./phase-02-isolated-gemini-workspace-staging.md) - complete
3. [Phase 03 - Unsafe Output Guard and Fallback Policy](./phase-03-unsafe-output-guard-and-fallback-policy.md) - complete
4. [Phase 04 - Tests and Live Rerun Validation](./phase-04-tests-and-live-rerun-validation.md) - automated checks complete; GUI smoke pending

## Key Dependencies

- `gemini` CLI supports headless `--output-format json` and `-e none`.
- Current OCR dispatcher remains the single integration point for CLI agents.
- History rerun continues updating the same record in-place.

## Non-Goals

- Do not make Gemini CLI the default agent.
- Do not replace `cloud-gemini` direct API.
- Do not build a new OCR prompt variant unless validation proves JSON/workspace guards are insufficient.

## Definition of Done

- Gemini CLI response is parsed from JSON `.response`, not raw stdout.
- Gemini CLI runs in a clean staging workspace containing only the image and minimal Gemini config.
- Tool-loop/error outputs are rejected before updating history.
- Rust tests cover JSON parsing, unsafe-output rejection, argv shape, and workspace path creation.
- Automated and CLI fixture checks confirm Codex still works and Gemini either returns clean OCR or surfaces an error without corrupting the record.
- Manual GUI rerun smoke remains tracked in the parent Phase 7 plan.
