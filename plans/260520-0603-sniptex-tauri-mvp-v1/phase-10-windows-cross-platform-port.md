---
phase: 10
title: "Windows Cross-Platform Port"
status: pending
priority: P1
effort: "2d"
dependencies: [9]
---

# Phase 10: Windows Cross-Platform Port

## Overview

Build and test SnipTeX on Windows. Verify all features work cross-platform: hotkey (Ctrl+Shift+M), screen capture via `xcap`, agent detection (PATH + AppData + winget paths), tray icon (system tray), clipboard, SQLite history, settings persistence, theme, autostart (Registry Run key), and onboarding install commands (winget/npm). Fix any Windows-specific path, permission, or rendering issues.

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

- Modify: `src-tauri/src/agents/mod.rs` — Windows-specific search paths for agent binaries
- Modify: `src-tauri/src/capture/region_selector.rs` — verify multi-DPI rendering on Windows
- Modify: `src-tauri/src/tray.rs` — conditional icon format (ico vs png)
- Modify: `src-tauri/src/hotkey.rs` — verify `CommandOrControl` mapping
- Modify: `src/windows/OnboardingWindow/InstallGuideStep.tsx` — Windows install commands (winget)
- Create: `src-tauri/icons/tray-icon.ico` — 16x16 ICO for Windows system tray
- Modify: `src-tauri/tauri.conf.json` — verify Windows-specific bundle config
- Modify: `src-tauri/Cargo.toml` — verify no Mac-only dependencies leak into Windows build

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

- [ ] Windows compilation passes (fix #[cfg] guards)
- [ ] Agent detection works on Windows paths
- [ ] Hotkey Ctrl+Shift+M works
- [ ] Region selector renders on multi-DPI
- [ ] Tray icon (.ico) shows in system tray
- [ ] Clipboard works for all formats
- [ ] SQLite history under AppData
- [ ] Settings persistence under AppData
- [ ] Theme follows Windows appearance
- [ ] Autostart via Registry Run key
- [ ] Onboarding shows Windows install commands
- [ ] MSI installer builds and installs cleanly
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

- Is Codex CLI available via `winget`? If not, npm is the only install path for Windows.
- Should we test on Windows 10 in addition to 11? (Tauri 2 supports Windows 10 1803+)
