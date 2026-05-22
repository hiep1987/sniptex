# Phase 6 — React UI Shell & MathJax Preview — Completion Report

**Date:** 2026-05-22
**Status:** Complete (build verified; live smoke test pending `pnpm tauri dev` session)
**Reviewer:** code-reviewer subagent (DONE_WITH_CONCERNS → addressed)

## What shipped

- Replaced single `main` Tauri window with 4 dedicated windows: `preview` (frameless transparent always-on-top, hidden), `settings` (tabbed shell), `history` (search + virtualized list), `onboarding` (5-step stepper). `overlay` from Phase 4 untouched.
- New Rust commands `show_window` / `hide_window` (`src-tauri/src/commands.rs`).
- Tray now routes History/Settings/About to dedicated windows via direct webview-window-show; removed `show_main_window` helper.
- Capabilities config updated to grant permissions to all 5 window labels.
- Frontend:
  - Lazy MathJax 3 CHTML loader; lazy KaTeX styles via dynamic import.
  - LatexRenderer + MarkdownRenderer with markdown-it `html:false` XSS guard.
  - PreviewWindow with Copy / Copy as… (6 formats) / Pin / Dismiss; 3s auto-hide with hover-pause + pin-disable + sequence-keyed reset.
  - useSnipResult returns `{seq, result}` so identical re-snips still restart the timer.
  - useSnipTrigger owns hotkey + tray-snip-now subscriptions; subscribes each listener independently so partial failure doesn't leak handles.
  - snip-error toast surfacing in PreviewWindow.
  - Zustand stores for settings and history (skeletons; Phase 7/8 fill).
  - Format conversions: raw / inline / display / plain / markdown / mathml (mathml uses SerializedMmlVisitor as placeholder; Phase 9 wires final).
  - main.tsx label router; App.tsx trimmed to fallback only.

## Verification

| Check | Result |
|------|--------|
| `pnpm tsc --noEmit` | clean |
| `pnpm build` | clean; MathJax + KaTeX split into own chunks |
| `cargo check` | clean |
| `cargo test --lib` | 11 / 11 pass |
| Bundle impact | chtml chunk 322KB ungzipped (79KB gzip), loaded only on Preview Window |
| Live smoke (`pnpm tauri dev`) | not yet run — flagged as outstanding |

## Code review findings (all addressed)

1. **Major — Auto-hide resetKey collision** on identical snips → fixed by switching `useSnipResult` to return a monotonic `seq` consumed as the resetKey.
2. **Minor — Promise.all subscribe race** in `use-snip-trigger.ts` → switched to per-listener subscribe so partial failure doesn't leak handles.
3. **Minor — `snip-error` not surfaced in PreviewWindow** → added independent listener with toast.
4. **Nit — `cargo check` clean, no console errors expected** → confirmed.

## Touchpoints (regression surface)

- Tray menu paths (Phase 5) — routes unchanged in behavior, only target window differs.
- Snip pipeline (Phases 3-5) — `run_snip` invocation moved from App.tsx to PreviewWindow via `useSnipTrigger`; `snip-complete` event contract unchanged.
- Capabilities — `main` removed; no live source references to the removed label (grep clean).
- App.tsx — now fallback-only; production windows never route to it.

## Phases this unblocks

- Phase 7 (SQLite history) — fills `useHistoryStore` from backend + wires HistoryWindow virtualized list to real data.
- Phase 8 (Settings UI + Onboarding) — fills SettingsWindow tabs + Onboarding step bodies.
- Phase 9 (Theme + Format Toggle + UX Polish) — formalises Copy-as formats (LaTeX tabular toggle) and theme.

## Outstanding

- Live smoke test in `pnpm tauri dev`: trigger snip → confirm Preview Window appears at cursor, renders LaTeX, Copy re-copies, Pin holds, 3s auto-hide works.
- MathML format path uses an undocumented MathJax `end: 2` contract; Phase 9 should formalise.
- KaTeX `katex.min.css` adds ~29KB to first Markdown render — acceptable for v1, revisit if bundle budget tightens.

## Unresolved Questions

- Should re-snip of identical content auto-show preview again, or stay hidden? Current behaviour: auto-show (intentional; user reviewing the OCR run is the point).
- Should About menu jump to Settings>About tab immediately, or just open Settings? Current: jumps via `tray-about` event. Confirm UX.
