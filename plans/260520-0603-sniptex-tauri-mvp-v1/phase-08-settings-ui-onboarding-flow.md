---
phase: 8
title: "Settings UI & Onboarding Flow"
status: pending
priority: P2
effort: "2d"
dependencies: [7]
---

# Phase 8: Settings UI & Onboarding Flow

## Overview

Build out the full Settings Window (5 tabs) with persistent storage via `tauri-plugin-store`, and the 6-step Onboarding Window shown on first run. Settings include hotkey rebinding, agent priority drag-drop, default output format, history size, preview auto-hide duration, sound on success, launch at login, and theme. The Agents flow now covers **three OCR paths** (Codex CLI default, Gemini CLI experimental secondary, Gemini Vision API cloud BYOK) per Validation Session 3 / Path C.

<!-- Updated: Validation Session 3 (2026-05-21) - Path C hybrid: BYOK onboarding step added; AgentsTab gains cloud-API toggle + API-key field; Codex is the recommended default in onboarding -->


## Key Insights

- Persist settings as single JSON via `tauri-plugin-store` (`{app_data_dir}/settings.json`). Strongly typed `AppSettings` struct in Rust with `serde`.
- Hotkey rebinding: unregister old → register new → on failure, revert + show error.
- Agent priority impacts `run_with_fallback` order (Phase 3).
- Launch at login: `tauri-plugin-autostart`. Mac uses LaunchAgent plist, Windows uses Registry `Run` key.
- Onboarding triggered when settings file absent OR `onboarding_completed: false`.

## Requirements

**Functional**
- Settings persist across app restarts
- Settings tabs:
  - **General** — theme, launch-at-login, sound on success, preview auto-hide duration (slider)
  - **Agents** — list installed agents, drag-drop priority, per-agent Test button, Re-scan button
  - **Hotkeys** — capture-mode input for new hotkey, conflict detection, reset to default
  - **Formats** — default output format radio, per-format toggles (which appear in Copy as... menu)
  - **About** — version, license, GitHub link, Open Collective + GitHub Sponsors badges
- Onboarding 6 steps:
  1. Welcome — what SnipTeX does (English)
  2. Auto-detect — show found agents (Codex CLI and/or Gemini CLI) + indicate if cloud-API key exists
  3. Install guide — copy-paste install command for whichever CLI is missing (**Codex recommended as default per Session 3**; Gemini CLI shown as secondary with note "experimental, may need to fall back"). Per OpenAI docs for Codex; Gemini: `brew/winget/npm install @google/gemini-cli`.
  4. **NEW (Session 3): Cloud option** — "Want sub-5-second response? Add your Google AI Studio API key." Skippable. Provides:
     - "Get a free key (Google AI Studio link)" with deep-link
     - Password-style input + paste-from-clipboard
     - "Test key" button → calls `test_agent("cloud-gemini")` against bundled demo image
     - Privacy note: "Your key is stored in your OS keychain. Cloud mode sends image to Google's servers."
  5. Test snip — pre-bundled demo image, "Try a snip" CTA, runs against whichever agent the user has set up
  6. Hotkey tutorial — show current hotkey, offer rebind

<!-- Updated: Validation Session 2 - Codex restored to onboarding -->
<!-- Updated: Validation Session 3 - Cloud BYOK step added; Codex labelled default -->
- After last step → mark `onboarding_completed: true` and close onboarding window

**Non-functional**
- Settings changes apply instantly (no "Save" button needed where reasonable)
- Onboarding loads <300ms

## Architecture

`AppSettings` schema (Rust + TS mirrors):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub hotkey: String,                    // "CommandOrControl+Shift+M"
    pub agent_priority: Vec<String>,       // default ["codex", "cloud-gemini", "gemini-cli"] (Session 3)
    pub default_format: OutputFormat,      // Smart | Inline | Display | Plain | Markdown | MathML | UnicodePretty
    pub copy_as_formats: Vec<OutputFormat>,
    pub history_size: HistorySize,         // Fifty | OneHundred | FiveHundred | Unlimited
    pub preview_duration_ms: u32,          // default 3000
    pub sound_on_success: bool,
    pub launch_at_login: bool,
    pub theme: ThemeMode,                  // System | Light | Dark
    pub onboarding_completed: bool,
    // Session-3 additions for cloud BYOK mode
    pub cloud_mode_enabled: bool,          // user explicitly opted into cloud, default false
    // NOTE: actual API key NEVER stored here — lives in OS keychain via `agents/keychain.rs`
}
```

## Related Code Files

- Create: `src-tauri/src/settings.rs` — `AppSettings` + load/save + defaults
- Modify: `src-tauri/src/commands.rs` — `get_settings`, `update_settings` commands
- Modify: `src-tauri/src/hotkey.rs` — accept new hotkey via `rebind_hotkey` command
- Modify: `src/stores/settingsStore.ts` — full slice
- Create: `src/windows/SettingsWindow/{GeneralTab,AgentsTab,HotkeysTab,FormatsTab,AboutTab}.tsx`
- Create: `src/windows/OnboardingWindow/{WelcomeStep,DetectStep,InstallGuideStep,CloudKeyStep,TestSnipStep,HotkeyStep}.tsx` (CloudKeyStep is new — Session 3)
- Create: `src/components/HotkeyInput.tsx` — capture key combo
- Create: `src/components/ApiKeyInput.tsx` — password-style input + paste + reveal toggle + test button
- Modify: `src-tauri/Cargo.toml` — add `tauri-plugin-autostart`

## Implementation Steps

1. Define `AppSettings` in Rust with serde + sensible defaults. Implement `load(store)` / `save(store)`.
2. Wire `tauri-plugin-store` and `tauri-plugin-autostart` in setup.
3. Expose commands:
   - `get_settings() -> AppSettings`
   - `update_settings(patch: PartialSettings)` (patch via serde merge)
   - `rebind_hotkey(new_shortcut: String) -> Result<()>` (unregister old, register new, on success update store)
   - `set_launch_at_login(enabled: bool)`
4. Build `settingsStore.ts` (Zustand) with fetch on mount + `updateSetting(key, value)` action that calls `update_settings`.
5. Build SettingsWindow tab components per spec:
   - **AgentsTab** (Session 3 expanded): list installed agents from `detect_agents` (v1 = Codex CLI, Gemini CLI, Cloud Gemini API). 3-row drag-drop via `@dnd-kit/sortable` for fallback priority. Each row shows agent kind badge (`CLI` or `Cloud`), status (installed / key set / not configured), and Test button → `test_agent(id)`. Re-scan button reruns detection. **Cloud-Gemini row** includes:
     - "Set API key" / "Update API key" button → opens modal with `ApiKeyInput`
     - "Remove key" button (with confirm) → `delete_api_key("gemini")`
     - Toggle "Use cloud mode automatically when CLI is slow" (controls `cloud_mode_enabled`)
     - Link to "Get a free key at Google AI Studio"
   - Show "More agents (Claude Code, OpenCode) in v1.x" hint.
   - **HotkeysTab**: custom `HotkeyInput` component that listens to keypress, displays combo, validates, calls `rebind_hotkey`. Show "Default: CMD+Shift+M" link.
   - **GeneralTab**: theme select, autostart toggle, sound toggle, preview duration slider.
   - **FormatsTab**: default format radio, copy-as checkbox group.
   - **AboutTab**: pull version from `app.getVersion()`, GitHub + Sponsors links via `open` shell.
6. Build OnboardingWindow steps; route through `Stepper` from shell.
   - Step 2 uses `detect_agents` results to pick branch (found vs missing); always recommend Codex first.
   - Step 3 shows platform-specific install command (Mac brew vs Windows winget vs npm fallback); offers both Codex (default) and Gemini CLI (experimental secondary) install commands.
   - Step 4 (Session 3 NEW) — `CloudKeyStep`: explainer card + `ApiKeyInput` + "Test key" button + "Skip — I'll do this later" link. Persists `cloud_mode_enabled=true` on save.
   - Step 5 triggers `run_snip` with a bundled `demo-image.png` if user has no agent yet (use `--dry-run` mode that returns canned text). Step picks the agent in this order: cloud-gemini (if key set) → codex (if installed) → gemini-cli (if installed) → dry-run.
7. First-run logic in `lib.rs` setup: if `settings.onboarding_completed == false`, show OnboardingWindow on launch instead of tray-only mode.
8. Smoke tests:
   - Toggle theme → all open windows update
   - Rebind hotkey → trigger via new hotkey works; old hotkey no longer
   - Drag agent priority → next snip uses new order
   - Toggle launch-at-login → Mac LaunchAgent plist created; reverse removes
   - Fresh install (delete settings.json) → onboarding fires

## Todo List

- [ ] Define `AppSettings` schema (Rust + TS mirror)
- [ ] Wire `tauri-plugin-store` + `tauri-plugin-autostart`
- [ ] Implement get/update/rebind/autostart commands
- [ ] Build settingsStore.ts with reactive updates
- [ ] Build 5 settings tabs (General/Agents/Hotkeys/Formats/About) — AgentsTab covers all 3 agent paths
- [ ] Build HotkeyInput component with capture + conflict detection
- [ ] Build ApiKeyInput component (password-style, paste, reveal toggle, test button)
- [ ] Build 6 onboarding steps with platform-aware install guide + CloudKeyStep
- [ ] First-run detection routes to onboarding
- [ ] Verify hotkey rebind end-to-end
- [ ] Verify launch-at-login on Mac
- [ ] Verify onboarding flow on fresh install

## Success Criteria

- [ ] Settings persist across app restart (verified by killing process)
- [ ] Hotkey rebind succeeds; old shortcut no longer triggers
- [ ] Agent priority drag-drop affects fallback order on next snip
- [ ] Launch-at-login toggle creates/removes LaunchAgent plist (Mac)
- [ ] Onboarding shows on fresh install, never on subsequent launches

## Risk Assessment

- **Risk: User picks hotkey that clashes with OS** — Mitigation: validate before register; show conflict dialog with "Try different shortcut" option.
- **Risk: Mac LaunchAgent plist requires user approval the first time** — Mitigation: on toggle ON, show explainer dialog before triggering autostart plugin.

## Security Considerations

- Settings file is plain JSON in app data dir — no secrets stored, low risk.
- Validate `agent_priority` entries against registered agents to prevent loading unexpected binaries (defense in depth).
- **API key handling (Session 3)**: never stored in settings.json or any plain file. Always via `agents/keychain.rs` (macOS Keychain / Windows Credential Manager via `keyring` crate). `ApiKeyInput` component MUST mask input by default, MUST NOT persist in React state across remounts beyond the modal's lifetime, MUST send to Tauri command directly without intermediate logging.
- Clipboard paste of API key should auto-clear clipboard after 30 seconds (offer toggle).

## Next Steps

- Phase 9 (Theme + Format toggle + UX polish) builds on theme + format settings
- Phase 10 (Windows port) verifies settings work cross-platform

## Open Questions

- Should onboarding be re-runnable from Settings → About? Default: yes, "Replay onboarding" link.
