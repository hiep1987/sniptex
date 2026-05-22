---
phase: 4
title: "Screen Capture & Region Selector"
status: complete
priority: P1
effort: "2d"
dependencies: [3]
completed: "2026-05-22"
---

# Phase 4: Screen Capture & Region Selector

## Overview

Build the snip UX: a fullscreen transparent always-on-top Tauri overlay window with crosshair cursor that lets the user drag-select a rectangle; on release, capture the underlying region with the `xcap` Rust crate, save to a UUID-named temp PNG, and pass to the OCR pipeline from Phase 3.

## Key Insights

- `xcap` is the agreed cross-platform capture crate (`replan.md` §2). Captures full screen by index, then we crop to region.
- Overlay window must NOT capture itself — strategy: take screenshot FIRST, then show overlay rendered on top of the captured image. User selects from frozen snapshot.
- Multi-monitor: identify which screen the cursor is on at hotkey press; capture that screen only.
- Mac requires Screen Recording permission (System Settings → Privacy → Screen Recording). First-run flow handles this in Phase 8 onboarding; for now log clear error if permission missing.

## Requirements

**Functional**
- After hotkey → fullscreen overlay appears within 150ms on the active monitor
- User drags rectangle with crosshair cursor
- `Esc` cancels capture, hides overlay
- `Enter` or mouse-up commits selection
- Captured image saved to `{temp}/sniptex-{uuid}.png`
- `run_snip` Tauri command returns the temp path to frontend

**Non-functional**
- Overlay does not flicker or show before screenshot is captured (avoid self-capture)
- Total capture-to-OCR-start latency <200ms

## Architecture

```
User presses hotkey
   │
   ▼
Rust: hotkey handler
   │  1. xcap screenshot all monitors NOW (before showing UI)
   │  2. Identify monitor under cursor
   │  3. Save full-monitor PNG → temp
   │  4. Tell frontend: "show overlay with this PNG as backdrop"
   ▼
Frontend (overlay window):
   │  - Tauri window: fullscreen, always_on_top, transparent, decorations:false, skip_taskbar
   │  - Background <img> = captured full-monitor PNG
   │  - <canvas> overlay for selection rectangle
   │  - Crosshair cursor (CSS), dim overlay
   │  - On mousedown → mouseup: emit `capture_region` with rect coords
   ▼
Rust: capture handler
   │  - Crop full PNG to rect → save as final PNG
   │  - Hide overlay
   │  - Return path to caller (run_snip)
   ▼
Phase 3 OCR pipeline runs on cropped PNG
```

## Related Code Files

- Create: `src-tauri/src/capture/{mod,screenshot,region_selector}.rs`
- Modify: `src-tauri/Cargo.toml` — add `xcap = "0.0"`, `image = "0.25"` (for crop), keep `uuid`
- Modify: `src-tauri/tauri.conf.json` — declare `overlay` window config (fullscreen, transparent, etc.)
- Create: `src/windows/CaptureOverlayWindow.tsx` — React overlay UI
- Modify: `src-tauri/src/commands.rs` — implement `run_snip` (was stub in Phase 3)
- Modify: `src/lib/invoke.ts` — add `runSnip` typed wrapper

## Implementation Steps

1. Add `xcap` + `image` Rust deps. Add `image` crate for PNG crop.
2. Implement `capture/screenshot.rs`:
   - `pub fn capture_active_monitor() -> Result<(PathBuf, Monitor), CaptureError>`
   - Use `xcap::Monitor::all()` → pick monitor under current cursor (`xcap` exposes monitor bounds; system cursor pos via platform API or via the in-app cursor tracked from last hotkey)
   - `monitor.capture_image()` → save full-monitor PNG to `{temp}/sniptex-full-{uuid}.png`
3. Define overlay window in `tauri.conf.json`:
   - `label: "overlay"`, `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, `fullscreen: true`, `visible: false` (shown on demand)
4. Implement `commands::run_snip`:
   - Call `capture_active_monitor` → get full-monitor PNG + monitor info
   - Set overlay window backdrop URL via Tauri asset protocol
   - Show overlay window on the same monitor (position + size = monitor bounds)
   - Await `capture_region` event from frontend with rect `{x,y,w,h}`
   - Crop full-monitor PNG to rect using `image` crate → save `{temp}/sniptex-{uuid}.png`
   - Hide overlay, delete full-monitor PNG
   - Pass cropped path into Phase 3 OCR pipeline; return `SnipResult` to caller
5. Implement `CaptureOverlayWindow.tsx`:
   - Listen for `capture-start` event with backdrop image URL + dimensions
   - Render `<img src={backdrop}>` fullscreen + `<canvas>` overlay
   - Mouse handlers for drag rectangle (mousedown = start, mousemove = update, mouseup = emit)
   - CSS crosshair cursor, dim mask outside selection
   - `Esc` → emit `capture-cancel` and hide
6. Wire frontend events: `emit('capture-region', {x, y, w, h})` to Rust; Rust matches via `listen` in `run_snip`.
7. Manual test on Mac: trigger hotkey → overlay → drag → cropped PNG exists at expected path → OCR fires → text returned.
8. Edge cases:
   - Click without drag (zero-area rect) → reject, keep overlay open
   - Drag off-screen → clamp to monitor bounds
   - Multi-monitor: ensure capture matches the monitor where cursor was at hotkey press

## Todo List

- [x] Add `xcap` + `image` crates
- [x] Implement `capture_active_monitor` (detect cursor's monitor + screenshot)
- [x] Define overlay window in `tauri.conf.json`
- [x] Implement `run_snip` orchestration in commands.rs
- [x] Build `capture-overlay-window.tsx` with drag-rect div + crosshair (kebab-case per repo convention)
- [x] Wire `capture-region` / `capture-cancel` events between Rust and frontend
- [x] Crop full-monitor PNG to rect via `image` crate
- [x] Cleanup full-monitor PNG after crop succeeds
- [x] Handle Esc cancel, zero-area drag, off-screen drag (clamp_and_validate_rect)
- [x] Smoke test end-to-end: hotkey → drag → OCR result text (verified 2026-05-22 on M-series Retina 1512×982@2x: Codex returned 381 chars for 561×138 selection; Esc cancel verified)

## Success Criteria

- [ ] Hotkey-to-overlay latency <200ms on M-series Mac
- [ ] Overlay does not appear in its own screenshot (verified visually)
- [ ] Cropped PNG matches the selected rect pixel-for-pixel
- [ ] Multi-monitor works on dual-screen Mac (capture happens on the monitor under cursor)
- [ ] Esc cancels cleanly, no leaked windows or temp files

## Risk Assessment

- **Risk: `xcap` cursor-position API differs Mac/Windows** — Mitigation: use OS-native fallback (`CGEventSource::location()` on Mac, `GetCursorPos` on Windows) via small platform shim.
- **Risk: Wayland/Linux capture later differs** — Mitigation: out of scope for v1 (Linux deferred).
- **Risk: HiDPI mismatch (rect coords vs physical pixels)** — Mitigation: read DPI scale from `xcap::Monitor::scale_factor()`, multiply rect coords accordingly before crop.

## Security Considerations

- Mac Screen Recording permission required — when not granted, `xcap` returns error. Catch and emit user-friendly dialog directing to System Settings.
- Captured PNG may contain sensitive content (passwords on screen). Delete temp files aggressively (success AND failure paths).

## Next Steps

- Phase 5 (Hotkey + Tray) wires the hotkey trigger into `run_snip`
- Phase 6 (Preview Window) consumes `SnipResult` and displays the rendered output

## Open Questions

- Should overlay show a small instructions text ("Drag to select • Esc to cancel") or stay clean? Default: stay clean v1; add as setting later.
- Multi-monitor smoke test deferred until a dual-screen test rig is available — single-monitor Retina path is verified.
- Windows-side validation of overlay coordinate space (`xcap::Monitor::x/y` vs Tauri `LogicalPosition`) is deferred to Phase 10 (Windows Cross-Platform Port).

## Code Review Adjustments (2026-05-22)

Applied after `code-reviewer` agent flagged 3 blockers + several majors:

- **HiDPI cursor coords:** `app.cursor_position()` returns physical pixels scaled by the primary monitor's DPI factor (tao `macos/util/mod.rs:107`). `xcap::Monitor::from_point` expects logical points (CG global display space). `run_snip` now divides by `primary_monitor().scale_factor()` before passing to `capture_active_monitor`.
- **assetProtocol scope:** removed the dangerously permissive `**/sniptex-*.png` glob; kept only `$TEMP/sniptex/**`.
- **RAII cleanup guards** in `commands.rs`: `TempFileGuard` (full + cropped PNG), `ListenerGuard` (unlisten on every exit), `OverlayHideGuard` (hide overlay even on error), `SnipBusyGuard` (Rust-side single-flight gate around `run_snip` — devtools can't race two snips).
- **Frontend keydown re-bind:** overlay's Enter/Esc handler now reads `dragRef.current` instead of depending on `[drag]`, so it binds once instead of on every mouse-move tick.
- Lifted duplicate `staging_path` into `capture/mod.rs`; simplified agent-lookup helper; removed unused `Clone` on `CaptureStartPayload`.

## Smoke Test Findings (2026-05-22)

Initial `pnpm tauri dev` revealed two pre-existing config gaps:

- `cli_test` second `[[bin]]` made `cargo run` ambiguous → added `default-run = "sniptex"` to `[package]`.
- Transparent overlay window required `app.macOSPrivateApi: true` in `tauri.conf.json` AND `macos-private-api` feature on the `tauri` Cargo dep — without these, the overlay rendered opaque/default and clicks didn't reach the React drag handlers.

After both fixes, end-to-end pipeline verified on M-series Retina (1512×982@2x):
- Physical→logical cursor conversion correct (e.g. cursor at (2568, 1288) → logical (1284, 644)).
- Capture writes 3024×1964 PNG to `{TEMP}/sniptex/`.
- Overlay positions/sizes correctly over monitor; backdrop shows; drag rectangle works.
- 561×138 logical selection → Codex CLI OCR returned 381 chars; success toast displayed.
- Esc cancel returns `selection: None`, overlay hides, no leaked temp PNGs.
