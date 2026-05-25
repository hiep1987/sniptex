---
phase: 1
title: "Implement event emission and listener"
status: implementation-complete-gui-smoke-pending
priority: P2
effort: "1h"
dependencies: []
---

# Phase 1: Implement event emission and listener

## Overview

Wire HistoryRow click → Tauri event → PreviewWindow render. No Rust changes needed — Tauri's `emit`/`listen` API bridges JS contexts across webview windows.

## Key Insights

- Each Tauri window runs an isolated JS context; stores can't communicate directly
- `@tauri-apps/api/event` `emit()` broadcasts to ALL webview windows via Rust IPC
- PreviewWindow already handles `snip-complete` via `useSnipResult` hook — extend that hook to also catch `history-preview-open`
- The payload shape must match `SnipResult` so the existing render/copy/autohide flow works unchanged
- HistoryRow's click target is the text/content area (not the action buttons which already have handlers)

## Architecture

```
src/components/history-row.tsx
  └── onClick on content <button> area
      └── emit("history-preview-open", { status: "ok", text, detected, agent, image_path, record_id })

src/hooks/use-snip-result.ts
  └── listen("snip-complete", ...)    ← existing
  └── listen("history-preview-open", ...)  ← NEW, same handler

src/windows/preview-window.tsx
  └── useSnipResult() — no changes needed, hook returns same SnipEvent shape
  └── showPreviewNearCursor() → show + focus + render + auto-copy + auto-hide
```

## Related Code Files

- Modify: `src/components/history-row.tsx` — add click handler on content area, emit event
- Modify: `src/hooks/use-snip-result.ts` — listen for `history-preview-open` alongside `snip-complete`
- Read: `src/windows/preview-window.tsx` — confirm no changes needed (consumes `useSnipResult` as-is)
- Read: `src/lib/invoke.ts` — `SnipResult` type definition for payload shape

## Implementation Steps

1. **`history-row.tsx`**: Add `onClick` handler to the content `<button>` (the `min-w-0 flex-1` area containing text + metadata). The handler calls `emit("history-preview-open", payload)` where payload maps `HistoryItem` → `SnipResult`:
   ```ts
   {
     status: "ok",
     text: item.text,
     detected: item.detected,
     agent: item.agent,
     image_path: item.image_path,
     record_id: item.id,
   }
   ```
   Add `cursor-pointer` to the clickable area.

2. **`use-snip-result.ts`**: Add a second `listen("history-preview-open", ...)` call inside the existing `useEffect`. Use the same `seq` counter and `setEvent` callback so the event is indistinguishable from `snip-complete` downstream. Both listeners share cleanup in the return function.

3. **`preview-window.tsx`**: Verify no changes needed. The `showPreviewNearCursor` → auto-copy → auto-hide chain triggers off `event.seq` changing, which already happens when `useSnipResult` emits a new event from either source.

4. **Show preview window from Rust**: The PreviewWindow shows itself in the `useEffect` keyed on `event.seq` via `showPreviewNearCursor()`. Since the event now also fires from History click, the preview will auto-show. However, `showPreviewNearCursor` positions near cursor — this is correct UX for history click too (user clicked, preview appears near mouse).

## Success Criteria

- [x] Clicking a History row content area emits a preview event with that row's OCR text
- [x] Preview reuses existing render path for Markdown/LaTeX based on `detected` type
- [x] Auto-copy fires on preview open via existing PreviewWindow effect
- [x] Auto-hide timer starts as normal via existing `event.seq` handling
- [x] Pin/dismiss/copy-as buttons reuse existing PreviewWindow behavior
- [x] Action buttons (copy, rerun, delete) still work independently as sibling controls
- [x] Multiple clicks on different rows update the preview via shared sequence counter

## Completion Notes

- Implemented `history-preview-open` event emission from the History row content button.
- `useSnipResult` now listens to both `snip-complete` and `history-preview-open`.
- Code review flagged invalid button descendants; fixed by using phrasing elements and an explicit `aria-label`.
- `pnpm exec tsc --noEmit`, `npm run build`, and `git diff --check` passed.
- GUI runtime smoke remains recommended because there are no frontend integration tests for cross-webview events.

## Risk Assessment

- **Event propagation**: clicking action buttons (copy/rerun/delete) must NOT also trigger the row click. Use `e.stopPropagation()` on action button container, or scope the click handler to the content area only (already isolated in its own `<button>`).
- **Window visibility**: PreviewWindow may be hidden. `showPreviewNearCursor()` calls `win.show()` + `win.setFocus()`, which handles this.
