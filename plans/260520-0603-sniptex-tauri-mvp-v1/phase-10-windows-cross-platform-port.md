---
phase: 10
title: "Windows Cross-Platform Port"
status: partial — mac-side code-prep complete, windows-machine validation pending
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

Batch B — Windows-machine validation (PENDING):

- [ ] Windows compilation passes end-to-end (`cargo build --target x86_64-pc-windows-msvc`)
- [ ] Agent detection finds Codex / Gemini installed via npm + winget on Windows paths
- [ ] Hotkey Ctrl+Shift+M registers and triggers capture
- [ ] Region selector renders correctly on 100/125/150/200% DPI scaling
- [ ] Tray icon (`.ico`) shows correctly in system tray; status switches work
- [ ] Clipboard works for all output formats
- [ ] SQLite history reads/writes under `{APPDATA}\com.sniptex.app\`
- [ ] Settings persistence under `{APPDATA}\com.sniptex.app\`
- [ ] Theme follows Windows appearance setting
- [ ] Autostart toggle creates/removes `HKCU\…\Run` registry key
- [ ] Onboarding shows correct Windows install commands; verify winget package IDs
- [ ] MSI installer builds and installs cleanly on Windows 10/11
- [ ] App size < 20MB, RAM < 100MB idle

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
