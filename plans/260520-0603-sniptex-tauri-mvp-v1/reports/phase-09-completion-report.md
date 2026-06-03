# Phase 9 — Theme & Format Toggle & UX Polish — Completion Report

**Date:** 2026-06-03
**Status:** Completed
**Closed by:** Manual archive after `implementation-complete` verified end-to-end

## What shipped

- **Format toggle (7 variants):** `src/lib/format.ts` routes Smart / Inline / Display / Plain LaTeX / Markdown / MathML / Unicode through dedicated converters. Plain LaTeX for non-equation output goes through `convert_to_tex` Tauri command for Markdown→`\begin{tabular}` reconstruction.
- **LaTeX tabular converter:** `src-tauri/src/ocr/tabular.rs` — Markdown table → LaTeX `tabular` env with alignment detection (`---` / `:---` / `:---:`), inline math preservation, multi-row support. 8 unit tests + 9 TABLE_ONLY fixture validation pass.
- **Complex-grid reconstruction:** `src-tauri/src/ocr/tabular_complex_grid.rs` rebuilds flattened merged-header tables from cloud OCR (Mistral, Gemini, Goclaw, Gemini CLI) into `\multirow` / `\multicolumn{N}{c|}{...}` / `\cline`. 4 supported shapes documented in `docs/latex-table-reconstruction.md`.
- **Capture overlay fast path:** overlay now appears immediately on hotkey; backend captures only the selected region after overlay hide (vs prior "capture full monitor PNG first"). Cuts hotkey-to-selector latency from ~600ms to ~80ms on 4K monitor.
- **Theme system:** class-based `dark` strategy via `src/main.tsx` + `src/hooks/use-theme.ts`, settings-store backed, listens to `prefers-color-scheme` for `system` mode, syncs across all 4 windows via `settings-changed` event.
- **Success sound:** generated Web Audio chime in `src/lib/success-sound.ts`, respects `sound_on_success` setting, degrades silently if Web Audio unavailable.
- **Preview polish:** Settings-backed format dropdown + auto-copy, Sonner Toast for snip/copy/hotkey errors, fade-in/out animations, rapid-snip queue (sequential, no drops).
- **Error states:** backend `snip-error` events (timeout, rate limit, empty output, no agent) surfaced via Toast in Preview window.

## Verification

| Check | Result |
|------|--------|
| `pnpm build` | clean |
| `cargo check` | clean |
| `cargo test --lib` | 33 agent unit + 50 other = 83/83 pass |
| `cargo test --test ocr_tabular` | 9/9 TABLE_ONLY fixtures pass |
| `cargo test --test agent_registry_argv` | 7/7 pass |
| Live UI smoke (theme/format/sound/toast) | verified manually via `pnpm tauri dev` |
| 4K monitor hotkey-to-overlay latency | < 100ms (target: < 200ms) |

## Key decisions

- **Markdown stays master-prompt default; LaTeX is post-processing.** Cloud OCR agents (Mistral OCR endpoint, Goclaw, sometimes Gemini API) flatten merged-cell tables regardless of prompt. Reconstruction lives in `tabular_complex_grid.rs`, not in prompt branching. CLI agents (Codex, Gemini CLI) skip reconstruction because they emit the rich form directly.
- **Live transparent overlay before screenshot, not after.** The earlier "capture full monitor PNG → display PNG as overlay backdrop → user selects → crop" path added 500-700ms latency at hotkey press on Retina/4K. New path: hide SnipTeX windows + read monitor geometry only → show transparent overlay → user selects → hide overlay + wait one compositor frame → `xcap.capture_region` of the selected rect only.
- **Format toggle only ships variants whose conversion actually differs.** Copy-as menu does not include placeholder modes whose conversion is identical to another already-shipped mode.

## Touchpoints (regression surface)

- Preview Window auto-copy + Copy-as menu (Phase 6 contract preserved; new format dropdown additive)
- Capture pipeline (Phase 4): `run_snip` overlay flow refactored — backend signature unchanged, internal sequencing different
- Settings store: `theme`, `default_format`, `copy_as_formats`, `sound_on_success`, `preview_duration_ms` consumed by Preview; default values seeded by Phase 8
- OCR dispatcher (Phase 3): `tabular_complex_grid` runs BEFORE generic Markdown→tabular converter in `tabular.rs`, ordering matters

## Phases this unblocks

- **Phase 10 (Windows port)** — needs to verify theme switching, format conversions, sound, and overlay fast-path on Windows (different compositor, different Web Audio backend)
- **Phase 13 (Landing page)** — references format options + theme screenshots in feature grid

## Outstanding (not Phase 9 scope, deferred)

- DeepSeek-OCR 2 via Novita (added 2026-06-02 in separate commit `d16dd72`) and the hybrid DeepSeek-OCR 2 + GPT-OSS-120B cleanup agent (`6cc5c2e` + `04e4c75`) — additive to the agent surface, not part of Phase 9 scope.
- `local-ocr` removal (commit `2e0df07`) — reverts unfinished local agent track; orthogonal to Phase 9.

## Unresolved questions

None.
