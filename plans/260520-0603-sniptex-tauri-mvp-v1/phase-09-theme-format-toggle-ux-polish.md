---
phase: 9
title: "Theme & Format Toggle & UX Polish"
status: in-progress
priority: P2
effort: "3d"
dependencies: [8]
---

> **Partial ship 2026-05-31:** LaTeX `tabular` slice landed first (closes user-reported "Copy as TeX = Markdown" bug on table snips). Complex-grid reconstruction for flattened `cloud-mistral` OCR tables also landed. Theme switch / sounds / animations / Toast / queueing still pending.
> **UX latency fix 2026-05-31:** Hotkey-to-selector latency fixed by switching from "capture full monitor PNG before overlay" to "show live transparent overlay first, capture selected region after mouse-up".

# Phase 9: Theme & Format Toggle & UX Polish

## Overview

Implement system/light/dark theme switching, the format toggle (Smart/Inline/Display/Plain/Markdown/MathML/Unicode), LaTeX `\begin{tabular}` table-output mode, and final UX polish: sound effects, animations, error states, and edge-case handling. This phase makes the app feel production-ready.

<!-- Session 3: LaTeX tabular toggle added here per dual-output decision — Markdown stays master-prompt default; this phase adds the LaTeX table mode + its own validation pass on the 9 table-only fixtures -->

## Key Insights

- Theme: CSS variables + Tailwind dark mode class strategy; Tauri window background must also update (via `set_background_color` or transparent + CSS).
- Format toggle: output format selection in Preview Window's "Copy as..." menu AND Settings → Formats tab (Phase 8 wires the schema, this phase implements the conversion logic).
- LaTeX tabular mode: post-process Markdown tables into `\begin{tabular}{...}...\end{tabular}`. Requires a dedicated validation pass on the 9 TABLE_ONLY fixtures from Phase 1.
- `cloud-mistral` uses Mistral OCR API (`mistral-ocr-latest` / dashboard `mistral-ocr-2512`), not prompt-controlled chat completion. Complex-grid quality therefore lives in deterministic Markdown-to-TeX reconstruction, not prompt tuning.
- Complex LaTeX table reconstruction is active for known table groups. Current taxonomy is documented in `docs/latex-table-reconstruction.md`: simple grid, two-level column header, title row spanning all columns, and raw complex LaTeX pass-through.
- Capture overlay latency: selector must appear immediately on `Cmd+Shift+M`; do not block overlay display on screenshot capture or full-monitor PNG encoding.
- Sound: short success chime on clipboard copy; respect `sound_on_success` setting from Phase 8.
- Animations: preview window fade-in/out, tray icon state transitions.

## Requirements

**Functional**
- Theme switching (system/light/dark) applies instantly to all open windows
- Format toggle in Preview Window toolbar: dropdown showing enabled formats from Settings
- "Copy as..." menu items convert output on the fly:
  - Smart (default) — auto-detected format
  - Inline LaTeX — wrap in `$...$`
  - Display LaTeX — wrap in `$$...$$`
  - Plain LaTeX — no delimiters
  - Markdown — full doc with `$...$` inline math
  - LaTeX Tabular — convert Markdown tables to `\begin{tabular}` (new, Session 3)
- Sound effect on successful snip + clipboard copy (configurable)
- Smooth animations: preview fade-in (150ms), fade-out (300ms), tray icon pulse during processing
- Error states: toast notification for agent timeout, rate limit, empty output, no agent found
- Edge cases: rapid consecutive snips queued (not dropped), clipboard write failure graceful

**Non-functional**
- Theme switch < 50ms (no flash of wrong theme)
- Format conversion < 10ms for any output size
- Sound latency < 100ms after clipboard write

## Architecture

### Format Conversion Pipeline

```
Raw OCR output (from Phase 3 dispatcher)
    ↓
smart_format.rs → DetectedType (EquationOnly | TableOnly | Mixed)
    ↓
format_converter.rs → convert(raw, detected_type, target_format) → String
    ↓
clipboard copy + preview render
```

### LaTeX Tabular Converter

```rust
// Markdown table → LaTeX tabular
// | a | b |       →  \begin{tabular}{|c|c|}
// |---|---|       →  \hline
// | 1 | 2 |       →  a & b \\ \hline
//                    1 & 2 \\ \hline
//                    \end{tabular}
```

### Complex Table Reconstruction

Handled as deterministic OCR Markdown → TeX post-processing, before the generic Markdown table converter:

- **Simple grid** — normal Markdown table to `tabular`.
- **Two-level column header** — reconstructs `\multirow`, `\multicolumn{N}{c|}{...}`, and `\cline`, validated on the `Nhóm / Loại I / Loại II` fixture across `cloud-mistral`, `cloud-gemini`, `cloud-goclaw`, and `gemini-cli` flattened outputs.
- **Title row spanning all columns** — reconstructs `\multicolumn{N}{|c|}{...}`, validated on the `Country List` fixture.
- **Raw complex LaTeX from agent** — preserves existing `\multirow`, `\multicolumn`, and `\cline` blocks instead of flattening them to Markdown.

Reference: `docs/latex-table-reconstruction.md`.

### Theme System

```
Settings.theme (System | Light | Dark)
    ↓
ThemeProvider (React context)
    ↓
document.documentElement.classList.toggle('dark')
    +
Tauri: window.set_background_color() for native frame
```

### Capture Overlay Fast Path

```
hotkey
  ↓
hide SnipTeX windows + read monitor geometry only
  ↓
show transparent overlay immediately
  ↓
user selects region
  ↓
hide overlay, wait one compositor frame, capture selected region only
  ↓
OCR
```

## Related Code Files

- Create: `src-tauri/src/ocr/format_converter.rs` — all format conversion functions
- Create: `src-tauri/src/ocr/tabular.rs` — Markdown table → LaTeX tabular converter
- Create: `src-tauri/src/ocr/tabular_complex_grid.rs` — reconstruct flattened merged-header tables from OCR Markdown
- Modify: `src-tauri/src/ocr/mod.rs` — re-export new modules
- Modify: `src-tauri/src/commands.rs` — `convert_format(raw, detected_type, target)` command
- Create: `src/components/ThemeProvider.tsx` — theme context + class toggling
- Modify: `src/components/PreviewToolbar.tsx` — format dropdown + copy-as actions
- Create: `src/components/Toast.tsx` — error/success toast notifications
- Create: `src/hooks/useTheme.ts` — theme hook reading from settings store
- Modify: `src/styles/globals.css` — CSS variables for light/dark themes
- Modify: `src-tauri/src/capture/screenshot.rs` — monitor geometry + selected-region capture fast path
- Modify: `src/windows/capture-overlay-window.tsx` — live transparent selector that no longer requires a pre-rendered backdrop image
- Create: `src-tauri/resources/sounds/success.wav` — short chime (~0.3s)
- Modify: `src/stores/settingsStore.ts` — wire theme + sound preferences

## Implementation Steps

1. Implement `format_converter.rs` with `convert(raw: &str, detected: DetectedType, target: OutputFormat) -> String` covering all 7 format variants.
2. Implement `tabular.rs` — parse Markdown table syntax, emit LaTeX `tabular` environment. Handle: alignment detection from `---`/`:---`/`:---:`, cell content with inline math, multi-row tables.
3. Expose `convert_format` Tauri command; wire into Preview Window's Copy-as menu.
4. Run validation pass: convert the 9 TABLE_ONLY fixtures through `tabular.rs`, compare against hand-verified expected output.
5. Build `ThemeProvider` — reads `settings.theme`, listens to system preference changes via `window.matchMedia('(prefers-color-scheme: dark)')`, applies `dark` class.
6. Define CSS variable sets for light/dark in `globals.css`; update all component styles to use variables.
7. Add sound playback: load `success.wav` via Web Audio API, play after clipboard write if `sound_on_success` is true.
8. Add preview window animations: CSS `opacity` + `transform` transitions on mount/unmount.
9. Build `Toast` component for error states; wire to dispatcher error events (timeout, rate limit, empty output).
10. Handle rapid-snip edge case: queue snip requests in Rust, process sequentially, show "Processing..." state for queued items.
11. Smoke test: theme toggle, all format conversions, sound toggle, error toast on agent timeout.

## Todo List

- [ ] Implement format_converter.rs (7 format variants)
- [x] Implement tabular.rs (Markdown → LaTeX tabular) — `src-tauri/src/ocr/tabular.rs`; 8 unit tests incl. câu 7 price-table fixture
- [x] Reconstruct flattened complex-grid table output from `cloud-mistral` OCR API — `src-tauri/src/ocr/tabular_complex_grid.rs`; integration tests cover live `cloud-mistral`, `cloud-gemini`, `cloud-goclaw`, and `gemini-cli` shapes for the "Nhóm / Loại I / Loại II" fixture
- [x] Reconstruct title-row span tables — `Country List` fixture now emits `\multicolumn{3}{|c|}{Country List}` and `\begin{tabular}{|l|c|c|}`
- [x] Document supported LaTeX table groups — `docs/latex-table-reconstruction.md`
- [x] Fix hotkey-to-overlay latency — overlay now appears before screenshot capture; backend captures only the selected region after overlay hide
- [x] Wire `convert_to_tex` Tauri command (slice of original `convert_format`) — registered in `lib.rs`, called from `src/lib/format.ts` `case "tex"`; `export_record` LaTeX branch now reuses the same converter
- [ ] Validate tabular conversion against 9 TABLE_ONLY fixtures (manual sweep pending; 1/9 covered by câu 7 test + complex merged-header fixture covered by 4-agent integration tests)
- [ ] Build ThemeProvider + CSS variable system
- [ ] Wire theme to all windows (main, preview, settings, history)
- [ ] Add success sound playback with setting toggle
- [ ] Add preview window fade-in/out animations
- [ ] Build Toast notification component
- [ ] Wire error states to Toast (timeout, rate limit, empty, no agent)
- [ ] Handle rapid consecutive snips (queue, don't drop)
- [ ] End-to-end smoke test all formats + theme + sound

## Success Criteria

- [ ] Theme toggle applies instantly to all open windows without flash
- [ ] All 7 format conversions produce correct output (spot-check 3 fixtures each)
- [ ] LaTeX tabular output validates against 9 TABLE_ONLY fixtures
- [ ] Sound plays on success, silent when disabled
- [ ] Error toast appears for agent timeout (simulate by killing agent mid-run)
- [ ] Rapid double-snip: both results captured, no crash or dropped snip

## Risk Assessment

- **Risk: unsupported complex table shapes** — Mitigation: supported shapes are documented and fixture-driven; unknown complex-grid Markdown falls back to simple table conversion instead of guessing arbitrary spans.
- **Risk: overlay appears in captured image after live-selector refactor** — Mitigation: emit region, deactivate overlay UI, hide overlay via RAII, then wait one compositor frame before `xcap.capture_region`.
- **Risk: Sound playback fails on some systems** — Mitigation: wrap in try-catch, degrade silently; sound is non-critical UX.
- **Risk: Theme flash on app startup** — Mitigation: read theme from store in Rust before creating webview; inject as `data-theme` attribute on HTML element in `index.html` inline script.

## Security Considerations

- Format conversion is pure string manipulation — no external calls, no injection risk.
- Sound file bundled as static resource — no user-supplied audio.

## Next Steps

- Phase 10 (Windows port) tests theme + format + sound cross-platform
- Phase 13 (Landing page) references format options in feature grid
