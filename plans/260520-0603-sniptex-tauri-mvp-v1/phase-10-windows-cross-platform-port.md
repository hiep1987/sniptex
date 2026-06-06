---
phase: 10
title: "Windows Cross-Platform Port"
status: complete (2026-06-06; Mac-side code-prep + Windows host validation both green)
priority: P1
effort: "2d"
dependencies: [9]
---

# Phase 10: Windows Cross-Platform Port

## Overview

Build and test SnipTeX on Windows. Verify all features work cross-platform: hotkey (Ctrl+Shift+M), screen capture via `xcap`, agent detection (PATH + AppData + winget paths), tray icon (system tray), clipboard, SQLite history, settings persistence, theme, autostart (Registry Run key), and onboarding install commands (winget/npm). Fix any Windows-specific path, permission, or rendering issues.

## Status (2026-06-04)

Phase 10 is being split into two batches:

- **Batch A — Mac-side code-prep (DONE 2026-06-04):** every change that can be authored without a Windows host. Cargo dep gating, platform-conditional source guards, tray icon format switch, agent search-path restructure. Verified by `cargo check` + 27-test pure-logic suite pass on macOS.
- **Batch B — Windows-machine validation (PENDING):** all build/runtime work that requires a Windows 10/11 host or VM. MSI bundle build, multi-DPI selector verification, registry autostart key, winget package-ID verification, RAM/size profiling.

## Deferred Scope

**PDF OCR on Windows is partially deferred.** The PDF rasterizer in `ocr::pdf_render` is a CoreGraphics-only implementation; on Windows / Linux it is replaced by a stub that returns `PdfRenderError::Open("PDF OCR is not yet supported on this platform")`. What this means in practice:

- **Disabled on Windows:** CLI-agent PDF flow (Codex / Gemini CLI need page-by-page PNGs), the PDF first-page thumbnail in History.
- **Still works on Windows (untested but expected):** Cloud PDF OCR via Gemini API and Mistral API. Those adapters upload raw PDF bytes server-side; the only local use of `page_count` is for client-side timeout scaling, and the call sites already fall back to `unwrap_or(1)` when `page_count` returns Err. So cloud PDF effectively gets a default 30s timeout rather than `pages × 30s` on Windows.

A future "Phase 10.5 / Windows PDF" phase should evaluate `pdfium-render` or `lopdf + resvg` and ship a Windows-native rasterizer.

## Key Insights

- Tauri 2 handles most cross-platform concerns, but agent binary detection paths differ (no `/opt/homebrew`, add `AppData\Roaming\npm`, `AppData\Local\Programs`).
- `xcap` crate works on Windows natively — no special permissions needed (unlike Mac Screen Recording).
- Windows hotkey uses `Ctrl+Shift+M` (not `CommandOrControl`); Tauri's `CommandOrControl` maps correctly.
- Tray icon format: `.ico` (16x16) instead of template PNG.
- SQLite path: `{APPDATA}\com.sniptex.app\` on Windows vs `~/Library/Application Support/` on Mac.
- Onboarding install commands: `winget install Google.GeminiCLI` and `winget install OpenAI.Codex` (verify availability).
- Launch at login: `tauri-plugin-autostart` uses Windows Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

## Requirements

**Functional**
- All Phase 1-9 features pass on Windows 10/11
- Agent detection finds binaries in Windows-specific paths
- Hotkey Ctrl+Shift+M triggers capture
- Region selector overlay renders correctly on multi-DPI displays
- Tray icon shows in system tray with correct .ico format
- Clipboard copy works for all output formats
- SQLite history reads/writes under AppData
- Settings persist under AppData
- Theme (system/light/dark) follows Windows appearance settings
- Autostart toggle creates/removes Registry Run key
- Onboarding shows Windows-specific install commands

**Non-functional**
- Build produces `.msi` installer via `tauri build --target msi`
- App size < 20MB (installer)
- RAM usage < 100MB idle
- No Windows Defender false positives (if possible without code signing)

## Architecture

### Platform-Conditional Code

```rust
// Agent detection extra dirs
#[cfg(target_os = "windows")]
fn extra_search_dirs(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("AppData\\Roaming\\npm"),
        home.join("AppData\\Local\\Programs"),
        home.join(".bun\\bin"),
        home.join("scoop\\shims"),
    ]
}

#[cfg(target_os = "macos")]
fn extra_search_dirs(home: &Path) -> Vec<PathBuf> {
    vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]
}
```

### Icon Assets

```
src-tauri/icons/
├── icon.icns          # Mac app icon
├── icon.ico           # Windows app icon
├── tray-icon.png      # Mac tray (template, 32x32)
└── tray-icon.ico      # Windows tray (16x16)
```

## Related Code Files

Batch A (done from Mac, 2026-06-04):

- Modified: `src-tauri/Cargo.toml` — gated `core-graphics` + `core-foundation` to `[target.'cfg(target_os = "macos")'.dependencies]`; added tauri `image-ico` feature.
- Modified: `src-tauri/src/ocr/mod.rs` — cfg-branched `pub mod pdf_render;` so non-macOS picks up the stub.
- Created: `src-tauri/src/ocr/pdf_render_stub.rs` — non-macOS stub with the same public API (`PdfRenderError`, `render_pages_to_pngs`, `page_count`).
- Modified: `src-tauri/src/tray.rs` — Windows tray loads `.ico` via cfg-branched `include_bytes!`; macOS / Linux keep `.png` template format.
- Modified: `src-tauri/src/agents/mod.rs` — Windows-specific search dirs (`AppData/Roaming/npm`, `AppData/Local/Programs`, `scoop/shims`); macOS-specific (`/opt/homebrew/bin`, `/usr/local/bin`); shared (`.local/bin`, `.cargo/bin`, `.bun/bin`, `.npm-global/bin`, mise installs).
- Already correct: `src-tauri/src/hotkey.rs` (`CommandOrControl` parsing already platform-aware), `src-tauri/tauri.conf.json` (bundle.targets already include `msi` + `nsis`; `icon.ico` already in icon list), `src/windows/onboarding/install-step.tsx` (already branches `mac` vs `win` commands).

Batch B (requires Windows host):

- Verify: `src-tauri/src/capture/region_selector.rs` — multi-DPI rendering on 100/125/150/200% scaling.
- Verify: `src-tauri/tauri.conf.json` — MSI / NSIS bundle config (signing config lives in Phase 11).
- Verify: `src/windows/onboarding/install-step.tsx` — decide whether to add `winget install Google.GeminiCLI` / `winget install OpenAI.Codex` once package IDs are confirmed; current npm commands ship as-is.

## Implementation Steps

1. Set up Windows dev environment: install Rust + Node + pnpm + Tauri CLI prerequisites (WebView2, Visual Studio Build Tools).
2. Run `cargo build` on Windows — fix any compilation errors from Mac-only code paths (`#[cfg]` guards).
3. Test agent detection: install Gemini CLI + Codex on Windows, verify `detect_installed_agents()` finds them in `AppData\Roaming\npm` and PATH.
4. Test hotkey: verify Ctrl+Shift+M triggers capture; test rebind in Settings.
5. Test region selector overlay: verify rendering on 100%, 125%, 150%, 200% DPI scaling. Fix coordinate math if needed.
6. Test tray icon: create `.ico` asset, verify shows in system tray, context menu works.
7. Test clipboard: all 7 output formats copy correctly.
8. Test SQLite: verify DB created under `{APPDATA}\com.sniptex.app\`, history persists across restarts.
9. Test settings: verify `settings.json` path, theme follows Windows appearance, autostart creates Registry key.
10. Test onboarding: verify Windows install commands shown (winget for Gemini/Codex, npm fallback).
11. Build MSI installer: `npx tauri build --target msi`, verify install/uninstall on clean Windows VM.
12. Profile: measure app size, idle RAM, startup time. Optimize if over budget.

## Todo List

Batch A — Mac-side code-prep:

- [x] Gate `core-graphics` + `core-foundation` to macOS in `Cargo.toml`
- [x] Cfg-gate `pdf_render` module; provide non-macOS stub with matching API
- [x] Switch tray icon to `.ico` on Windows; add tauri `image-ico` feature
- [x] Restructure `agents::candidate_dirs()` with platform-conditional paths
- [x] Audit `capture/` and `storage/` for Mac-only symbols (none found)
- [x] `cargo check` + 27 pure-logic tests still pass on macOS

Batch B — Windows-machine validation (all verified 2026-06-05 / 2026-06-06 on Windows 11 ARM64 in Parallels at 250% DPI, account `hiep1987`):

- [x] Windows compilation passes end-to-end (`pnpm tauri build --bundles msi`)
- [x] MSI installer builds and installs cleanly on Windows 11 ARM64 (x64 cross-build deferred to Phase 12 CI)
- [x] Hotkey Ctrl+Shift+M registers and triggers capture
- [x] Region selector renders correctly on multi-DPI (250% verified)
- [x] Agent detection works against Windows paths (`AppData\Roaming\npm`, `AppData\Local\Programs`, `scoop\shims`)
- [x] Tray icon `.ico` switches across all four states (idle / capturing / processing / error)
- [x] Clipboard works for all seven output formats (Smart / Inline / Display / Plain / Markdown / MathML / Unicode)
- [x] SQLite history file under `{APPDATA}\com.sniptex.app\sniptex.sqlite`
- [x] Settings persistence under `{APPDATA}\com.sniptex.app\settings.json` (with cold-start race fix; see Bugfix log)
- [x] Theme follows Windows Personalization → Colors mode setting when SnipTeX theme = System
- [x] Autostart toggle writes / clears `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\SnipTeX`; UI surfacing in Settings → Apps → Startup deferred to Phase 11 code signing
- [x] Onboarding step shows Windows install commands (`npm install -g @openai/codex`, `npm install -g @google/gemini-cli`)
- [x] App size 19.29 MB MSI / 20.44 MB extracted exe (≤ 20 MB MSI budget); RAM idle 67.5 MB (≤ 100 MB budget)

## Bugfix log

- **2026-06-05 — `xcap` coord-space mismatch on Windows (snip offset bug).**
  At 250% Windows DPI the captured region was the wrong area of the screen.
  Root cause: `xcap::Monitor::width/height` and `Monitor::capture_region`
  use logical points on macOS but physical pixels on Windows, while the
  capture pipeline assumed Mac semantics everywhere. Result: the overlay
  was sized to 2.5× the screen and xcap was handed CSS-space coords as if
  they were physical. Fixed in `src-tauri/src/capture/screenshot.rs` with
  two cfg-branched normalizations — see commit `cb3b040`. Mac path
  unchanged by construction (the `not(target_os = "windows")` branches
  reproduce the prior expressions byte-for-byte; verified via 35 existing
  tests still passing on macOS).

- **2026-06-06 — settings vanish on cold start, reappear after one change.**
  On Windows the Win32 main webview started running React before the
  Tauri `setup()` hook had executed `app.manage(settings_store)`, so the
  very first `get_settings` IPC failed with "state not managed", the
  zustand store stayed on its in-memory defaults, and the UI rendered
  the defaults until any later command triggered a refetch. Fixed in
  three commits: `78ffe77` reorders `setup()` so settings load + manage
  happens before `storage::init`, `18eabd9` widens the frontend retry
  ladder to [0, 100, 300, 700, 1500, 3000] ms so the slowest first-run
  cold start on a Parallels VM still catches up, and `bfdf501` adds an
  inline pre-mount script in `index.html` that applies the cached theme
  class to `<html>` from `localStorage` before React mounts. `f075c88`
  gates `useTheme` on `loaded` so the pre-applied class survives the
  fetch window. Mac is unaffected — it succeeds on attempt 0.

- **2026-06-06 — duplicate tray icon after close + reopen.**
  Closing the main window hides it to the tray (intentional close-to-tray
  intercept in `on_window_event`) but reopening sniptex.exe from the
  Start menu spawned a second process that registered its own tray icon.
  Fixed by adding `tauri-plugin-single-instance` as the first plugin in
  the desktop chain and forwarding the second launch's focus request to
  the existing main window — see commits `6f7ddfa` and `1e8e6e0` (the
  latter moves the registration ahead of all other plugins).

- **2026-06-06 — autostart functional but invisible in Apps → Startup.**
  `tauri-plugin-autostart` correctly writes `HKCU\…\Run\SnipTeX` =
  `C:\Program Files\SnipTeX\sniptex.exe`, so Windows will launch SnipTeX
  at login as designed. The Settings → Apps → Startup list filters
  entries based on file signature / SmartScreen heuristics; an unsigned
  MSI build like this one frequently does not surface there even though
  the registry side is correct. Deferred to Phase 11 (code signing).

## Toolchain dependency learned

`scripts/windows-bootstrap.ps1` documents the full setup needed on a
fresh Windows host. Non-obvious requirement: the `ring` crate (transitive
dep via rustls / reqwest / tokio-tungstenite) runs perlasm-generated
assembly through `clang` during its build script, so LLVM/Clang must be
installed separately from VS Build Tools and explicitly added to PATH
(the silent LLVM installer skips the add-to-PATH prompt). Bootstrap
script handles both.

## Success Criteria

- [ ] Full end-to-end flow works on Windows 11: hotkey → capture → OCR → preview → clipboard
- [ ] Agent detection finds Gemini CLI installed via npm/winget
- [ ] MSI installer: install, run, uninstall — no leftover files/registry keys
- [ ] Multi-DPI display: region selector coordinates correct at 150% scaling
- [ ] Autostart toggle creates/removes Registry Run key (verified via `regedit`)

## Risk Assessment

- **Risk: Multi-DPI coordinate issues in region selector** — Mitigation: use Tauri's logical vs physical pixel APIs; test on 125% and 150% scaling explicitly.
- **Risk: Windows Defender flags unsigned app** — Mitigation: document in README; this is expected without code signing. Phase 11 addresses signing.
- **Risk: Codex/Gemini CLI not available via winget** — Mitigation: fall back to npm install instructions; verify winget package availability before hardcoding.
- **Risk: WebView2 not pre-installed on older Windows 10** — Mitigation: Tauri MSI bundles WebView2 bootstrapper by default.

## Security Considerations

- No elevated permissions required on Windows (unlike Mac Screen Recording).
- Registry Run key: only writes to `HKCU` (user scope), not `HKLM` (system scope).
- Agent binary validation: verify executable exists and has `.exe` extension before spawning.

## Next Steps

- Phase 11 (Distribution) builds on the MSI output from this phase
- Phase 12 (CI/CD) adds Windows to the GitHub Actions build matrix

## Open Questions

- Is Codex CLI available via `winget`? If not, npm is the only install path for Windows. (Decision deferred to Batch B — keep npm commands until package IDs verified on a Windows host.)
- Should we test on Windows 10 in addition to 11? (Tauri 2 supports Windows 10 1803+)
- Windows PDF OCR: pick `pdfium-render` vs `lopdf + resvg` vs deferring until usage demands it. Tracked in "Deferred Scope" section.
