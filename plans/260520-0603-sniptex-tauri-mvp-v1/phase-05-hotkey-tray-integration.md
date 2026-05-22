---
phase: 5
title: "Hotkey & Tray Integration"
status: complete
priority: P1
effort: "2d"
dependencies: [4]
---

# Phase 5: Hotkey & Tray Integration

## Overview

Replace the Phase 2 demo hotkey listener with the production hotkey-to-snip pipeline, add native tray icon (Mac menu bar / Windows system tray) with menu, and expose tray icon state machine (idle / capturing / processing / error). Hotkey is user-configurable in Phase 8.

## Key Insights

- Mac menu bar icon needs **template image** (black-on-transparent, OS auto-tints for light/dark menu bar). 22x22 @1x and 44x44 @2x.
- **Use solid filled silhouettes, not thin strokes.** First attempt used 2.2px strokes on a 32x32 canvas — Mac template rendering produced an invisible icon because most pixels were anti-aliased grey, not solid black. Fix: redesign with `fill="#000"` and `evenodd` lens holes, no `stroke="..."` thin lines.
- Windows tray needs ICO format, 16x16 + 32x32 frames (we also include 24/48 for high-DPI).
- Tauri requires `tray-icon` and `image-png` features on the `tauri` crate to enable `TrayIconBuilder` + `Image::from_bytes`.
- Use Tauri 2 built-in tray API (`TrayIconBuilder`); avoid third-party crate.
- Hotkey conflict handling: `tauri-plugin-global-shortcut::register` returns error if shortcut already in use system-wide — show dialog asking user to change.

## Requirements

**Functional**
- Hotkey `CMD+Shift+M` (Mac) / `Ctrl+Shift+M` (Windows) triggers `run_snip` end-to-end
- Tray icon visible on app launch
- Tray menu: Snip Now / Show History / Settings / About / Quit
- Tray icon state changes: 🔵 idle, 🟡 capturing, 🟢 processing, 🔴 error (auto-revert to idle after 2s)
- Single-instance enforcement (clicking tray "Snip Now" while a snip is in flight = noop with brief feedback)

**Non-functional**
- Tray menu opens instantly (no lag, no JS evaluation)

## Architecture

```
src-tauri/src/
├── hotkey.rs           (register global shortcut, debounce, call run_snip)
├── tray.rs             (TrayIconBuilder, menu items, state machine)
└── state.rs            (AppState: in_flight: Mutex<bool>, current_status enum)
```

State machine:

```
        ┌──────────┐
        │   Idle   │ ◄────────────────────────┐
        └─────┬────┘                          │
              │ hotkey or "Snip Now"          │
              ▼                               │
        ┌──────────┐    capture done    ┌─────┴──────┐
        │Capturing │ ─────────────────► │ Processing │
        └──────────┘                    └─────┬──────┘
              │                               │ OCR done / error
              │ Esc / cancel                  │
              └───────────────────────────────┘
                                              │
                                              ▼ on error
                                         ┌─────────┐
                                         │  Error  │ → auto-reset after 2s
                                         └─────────┘
```

## Related Code Files

- Create: `src-tauri/src/hotkey.rs`
- Create: `src-tauri/src/tray.rs`
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs` — initialize state, tray, hotkey on `Builder::setup`
- Create assets: `src-tauri/icons/tray-idle.png`, `tray-capturing.png`, `tray-processing.png`, `tray-error.png` (template style for Mac) + `tray-idle.ico` set for Windows
- Modify: `src-tauri/Cargo.toml` — ensure `tauri = { features = ["tray-icon"] }`

## Implementation Steps

1. Design 4 tray icons (template-style monochrome): idle (camera), capturing (camera with focus brackets), processing (spinner-like), error (camera + small dot). Export Mac template PNGs + Windows ICOs.
2. Define `AppState` in `state.rs`:
   ```rust
   pub struct AppState {
       pub in_flight: Mutex<bool>,
       pub current_status: Mutex<TrayStatus>,
       pub tray_handle: OnceCell<TrayIcon>,
   }
   ```
3. Implement `tray.rs::init_tray(app)`:
   - Build `TrayIcon` with idle icon
   - Add menu items: Snip Now / Show History / Open Settings / separator / About / Quit
   - On menu click → emit window events or call command handlers
4. Implement `tray.rs::set_tray_status(status)` that swaps icon + (optionally) tooltip.
5. Implement `hotkey.rs::register_default(app)`:
   - Read shortcut from settings store (default `CommandOrControl+Shift+M`)
   - `app.global_shortcut().register(shortcut, move |_| trigger_snip(app_handle))`
   - On registration failure → emit `hotkey-conflict` event for frontend to show dialog
6. Implement `trigger_snip(app)`:
   - Acquire `in_flight` lock; if true, set tray status briefly and return
   - Set status → Capturing → call `run_snip` (Phase 4)
   - On capture success: set status → Processing → call OCR dispatcher (Phase 3)
   - On all success: set status → Idle, emit `snip-complete` event with result to Preview window
   - On any error: set status → Error → wait 2s → reset to Idle, emit `snip-error` event
7. Update `lib.rs` setup:
   - Create `AppState`, manage as Tauri state
   - Call `init_tray` then `register_default` in setup hook
8. Smoke test:
   - Launch app → tray visible
   - Press hotkey → tray turns yellow → drag rect → tray turns green → text appears in clipboard → tray returns blue
   - Click tray "Quit" → app exits cleanly

## Todo List

- [x] Design + export 4 tray icons (Mac template PNG + Windows ICO) — `src-tauri/icons/tray/`
- [x] Implement `AppState` with status + tray-handle mutexes — `src-tauri/src/state.rs`
- [x] Init tray icon with idle state and menu items — `src-tauri/src/tray.rs::init_tray`
- [x] Implement `set_status` icon swapper + `flash_error` auto-reset — `src-tauri/src/tray.rs`
- [x] Register global shortcut (default CMD/Ctrl+Shift+M) — `src-tauri/src/hotkey.rs::build_plugin`
- [x] Handle hotkey registration conflict by emitting `hotkey-conflict` event — `hotkey.rs::verify_registration`; frontend toast in `src/App.tsx`
- [x] Wire `run_snip` state-machine transitions (Capturing → Processing → Idle/Error) — `commands.rs::run_snip`
- [x] Wire tray menu items (Snip Now, History, Settings, About, Quit) — `tray.rs::on_menu_event`
- [x] Smoke test: hotkey → snip → clipboard end-to-end on Mac (verified 2026-05-22)
- [x] Smoke test: tray state transitions visible during flow (verified 2026-05-22)
- [x] Smoke test: tray "Quit" menu item exits app cleanly (verified 2026-05-22)
- [x] Smoke test: tray "Snip Now" menu item triggers same snip flow (verified 2026-05-22)

## Success Criteria

- [ ] Hotkey triggers full snip→OCR→clipboard flow without errors
- [ ] Tray icon changes visibly during each phase of the flow
- [ ] Hotkey conflict shows graceful dialog with "Change shortcut" CTA
- [ ] Tray menu items work (Settings opens settings window, Quit exits)
- [ ] Re-pressing hotkey during in-flight snip is harmless

## Risk Assessment

- **Risk: Hotkey conflicts with macOS system shortcut (e.g., Spotlight, Finder)** — Mitigation: pick uncommon default; let user override in settings; show diagnostic on registration failure.
- **Risk: Tray icon doesn't tint correctly on Mac dark menu bar** — Mitigation: ensure PNGs are pure black + alpha (no embedded color); test on light + dark menu bars.
- **Risk: Windows tray icon blurry at high-DPI** — Mitigation: include 16/24/32/48 px frames in ICO.

## Security Considerations

- Global hotkey can theoretically be sniffed by other apps that register the same combo. Document that hotkey is opt-in user trigger only, not for secrets.

## Next Steps

- Phase 6 (React UI) builds the Preview Window that consumes `snip-complete` event.
- Phase 8 (Settings UI) makes hotkey configurable.

## Open Questions

- Show "Snip count today" in tray menu? Defer to v1.x (would require history query on each tray open).
