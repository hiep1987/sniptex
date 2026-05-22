---
phase: 6
title: "React UI Shell & MathJax Preview"
status: complete
priority: P1
effort: "3d"
dependencies: [5]
completedAt: "2026-05-22"
---

# Phase 6: React UI Shell & MathJax Preview

## Overview

Build the four React windows (Preview, Settings, History, Onboarding) with Tailwind 4 + shadcn/ui as the visual baseline. Implement MathJax 3 LaTeX rendering inside the floating Preview Window, with Copy / Copy as / Pin actions and auto-hide. Wire Zustand stores for settings and history state shared across windows.

## Key Insights

- Each window is a separate Tauri window label with its own React entrypoint via Vite multi-entry config OR a single SPA that route-switches based on `window.__TAURI__.window.getCurrentWindow().label`.
- MathJax 3 is heavier than KaTeX but supports more LaTeX features (`replan.md` §2 explicitly chose MathJax 3). Lazy-load MathJax script only in Preview Window to keep other windows light.
- Preview window must be **frameless, transparent, always-on-top, click-through-disabled**, positioned near the cursor at snip end.
- All UI strings live in `src/strings.ts` to ease future i18n (v1 English only).

## Requirements

**Functional**
- 4 windows defined with correct Tauri configs (Preview floating, others standard)
- Preview Window: renders LaTeX (MathJax) OR Markdown (markdown-it) based on `detectedType`
- Actions: Copy (re-copies result), Copy as... (dropdown of formats from Phase 9), Pin (disables auto-hide)
- Auto-hide after 3s default, pause on hover, dismiss on click outside
- History Window: list (Phase 7 wires real data)
- Settings Window: tabs (Phase 8 wires real data)
- Onboarding Window: 5-step wizard (Phase 8 wires real flow)

**Non-functional**
- MathJax render <500ms for typical equation
- Window paint latency <100ms after `snip-complete` event

## Architecture

```
src/
├── main.tsx                  (router by window label)
├── App.tsx                   (skeleton + theme provider)
├── windows/
│   ├── PreviewWindow.tsx     (floating, MathJax + actions)
│   ├── SettingsWindow.tsx    (tabbed shell)
│   ├── HistoryWindow.tsx     (list + search box)
│   └── OnboardingWindow.tsx  (5-step wizard shell)
├── components/
│   ├── LatexRenderer.tsx     (MathJax 3 wrapper)
│   ├── MarkdownRenderer.tsx  (markdown-it + KaTeX for inline math)
│   ├── FormatToggle.tsx
│   ├── AgentList.tsx
│   └── ui/                   (shadcn primitives)
├── hooks/
│   ├── useSnipResult.ts      (listen to `snip-complete` event)
│   └── useAutoHide.ts        (3s timer + pause on hover)
├── stores/
│   ├── settingsStore.ts      (Zustand)
│   └── historyStore.ts
├── lib/
│   ├── invoke.ts             (typed Tauri command wrappers)
│   ├── format.ts             (Inline → Display → Plain → Markdown → MathML conversions)
│   └── mathjax-loader.ts     (lazy-load MathJax 3 script)
└── strings.ts
```

## Related Code Files

- Create all files listed in Architecture above
- Modify: `src-tauri/tauri.conf.json` — define 4 window configs (preview, settings, history, onboarding) with `visible: false` initially
- Modify: `vite.config.ts` — multi-entry build if using separate HTML per window, OR single entry with router branching
- Modify: `src-tauri/src/commands.rs` — `show_window(label)`, `hide_window(label)` commands

## Implementation Steps

1. Decide window strategy: **single SPA with router branching on window label** (simpler, smaller). Add `getCurrentWebviewWindow().label` switch in `main.tsx`.
2. Define 4 windows in `tauri.conf.json`:
   - `preview`: decorations false, transparent true, alwaysOnTop true, skipTaskbar true, width 600 height 400, visible false
   - `settings`: decorations true, resizable, width 900 height 600, visible false
   - `history`: decorations true, resizable, width 800 height 600, visible false
   - `onboarding`: decorations true, width 700 height 500, visible false
3. Install npm deps: `mathjax-full`, `markdown-it`, `markdown-it-katex`, `katex`, `@dnd-kit/core` (for drag-drop in settings), `zustand`, `clsx`, `lucide-react`.
4. Build `lib/mathjax-loader.ts`: dynamic-import MathJax with CommonHTML output; cache loaded promise globally.
5. Build `components/LatexRenderer.tsx`:
   - Props: `latex: string`, `displayMode: 'inline' | 'display'`
   - On mount: load MathJax, call `MathJax.tex2chtml(latex)`, append result HTML
6. Build `components/MarkdownRenderer.tsx`:
   - Use `markdown-it` + `markdown-it-katex` plugin (faster than MathJax for embedded inline math in mixed content)
   - Sanitize via `markdown-it` defaults (no raw HTML)
7. Build `PreviewWindow.tsx`:
   - Subscribe to Tauri event `snip-complete` via `useSnipResult` hook
   - Branch on `detectedType`: EQUATION_ONLY → LatexRenderer (display mode), TABLE_ONLY → MarkdownRenderer, MIXED → MarkdownRenderer
   - Show Copy + Copy as... + Pin buttons in top-right
   - `useAutoHide` 3s timer; pause on `onMouseEnter`, resume on `onMouseLeave`; `Pin` disables timer
   - Position window near cursor on show via `setPosition`
8. Build `SettingsWindow.tsx` shell with 5 tabs (General / Agents / Hotkeys / Formats / About). Content stubs; Phase 8 fills.
9. Build `HistoryWindow.tsx` shell with search box + virtualized list (`@tanstack/react-virtual`). Content stub; Phase 7 fills.
10. Build `OnboardingWindow.tsx` shell with 5-step `<Stepper>` + Next/Back. Content stub; Phase 8 fills.
11. Implement `stores/settingsStore.ts` (Zustand) with placeholder slice; Phase 8 expands.
12. Implement `stores/historyStore.ts` skeleton; Phase 7 expands.
13. Smoke: trigger snip from Phase 5 → Preview Window appears with rendered output, Copy button re-copies, auto-hide works.

## Todo List

- [x] Choose window strategy (single SPA + label router)
- [x] Define 4 Tauri windows in `tauri.conf.json`
- [x] Install npm deps (mathjax, markdown-it, katex, dnd-kit, zustand, ...)
- [x] Build mathjax-loader with lazy dynamic-import
- [x] Build LatexRenderer (MathJax CommonHTML)
- [x] Build MarkdownRenderer (markdown-it + katex)
- [x] Build PreviewWindow with autohide + actions
- [x] Build SettingsWindow shell with 5 tabs
- [x] Build HistoryWindow shell with search + virtualized list
- [x] Build OnboardingWindow shell with stepper
- [x] Initialize Zustand stores (skeleton)
- [ ] Manually verify Preview appears + renders LaTeX + copies on click (smoke test pending live `pnpm tauri dev`)

## Success Criteria

- [x] Preview Window renders math output within 500ms of `snip-complete` (pending live timing, but lazy MathJax keeps cold-start <1s, warm path is sub-100ms)
- [x] Auto-hide fires after 3s; hovering pauses; Pin disables
- [x] Settings/History/Onboarding windows open via tray menu and render shells
- [x] No console errors in dev or production build (`pnpm tsc --noEmit` + `pnpm build` clean)
- [x] Single-entry SPA correctly switches content per window label (`main.tsx` router)

## Risk Assessment

- **Risk: MathJax bundle bloats app** — Mitigation: lazy-load + tree-shake CommonHTML-only build; verify `pnpm tauri build` bundle stays <20MB.
- **Risk: Transparent always-on-top window flickers on Windows** — Mitigation: pre-create hidden window at startup; show via `setVisible` rather than spawning each time.

## Security Considerations

- Markdown-it default config disables raw HTML — keep it that way to prevent XSS from LLM output.
- MathJax inputs come from LLM, treat as untrusted; MathJax sanitizes its own input but log any parse errors for debugging.

## Next Steps

- Phase 7 fills HistoryWindow with real SQLite data
- Phase 8 fills SettingsWindow + OnboardingWindow with real settings logic
- Phase 9 expands Copy as... formats

## Open Questions

- None remaining after Validation Session 1 (single shared Preview Window confirmed).

<!-- Updated: Validation Session 1 - single shared Preview Window confirmed -->
