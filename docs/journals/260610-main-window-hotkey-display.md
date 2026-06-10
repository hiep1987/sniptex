# Main Window Hotkey Display: Hardcoded Shortcut Ignores User Settings

**Date**: 2026-06-10 17:30
**Component**: Main window empty-state, Settings > Keybinds integration
**Severity**: Low (UX papercut, not functional breakage)
**Status**: Resolved

## What Happened

User reported: "I rebound the main-window hotkey from Cmd+Shift+M to Cmd+Shift+E in Settings, but the empty-state hint still shows Cmd+Shift+M." Opened the Main window, verified the complaint — the `<kbd>` label was hardcoded to "Cmd/Ctrl + Shift + M" regardless of `useSettingsStore().hotkey` value.

Scout found the problem immediately: `src/App.tsx:193` had the shortcut literal baked into JSX. The settings store was wired correctly upstream (`main.tsx:51-57` listens for `SETTINGS_CHANGED_EVENT` and re-fetches), so the rest of the app saw updates fine. Just this one display was frozen.

## The Brutal Truth

Classic "I wired everything except this one spot" bug. User changed a setting, all the logic worked, but one hardcoded string made them doubt whether their change saved. That erodes trust. A small UX papercut, but exactly the kind that compounds if ignored — next week it's three overlooked spots, then "I don't know if my changes ever work."

## Technical Details

**Hardcoded site**: `src/App.tsx:193` in the empty-state JSX.

**Source of truth**: `useSettingsStore().hotkey` — default `"Command+Shift+M"` in `src/stores/settings-store.ts:23`, user-changeable via Settings UI.

**Formatter pattern already exists**: `src/windows/onboarding/ready-step.tsx:6` has a `.replace()` chain that maps platform keys to symbols:
```typescript
hotkey
  .replace('Command', '⌘')
  .replace('Control', '⌃')
  .replace('Shift', '⇧')
  .replace('Alt', '⌥')
```

**Dead code discovered during scout**: `src/strings.ts:17` contains `emptyHint` with the same hardcoded text, never imported anywhere. Left untouched (out of scope per YAGNI).

## What We Tried

1. **Verified the setting works elsewhere**: Confirmed `ready-step.tsx` successfully reads and displays the hotkey from the store.
2. **Checked if cross-window sync was broken**: Confirmed `main.tsx:51-57` correctly listens to `SETTINGS_CHANGED_EVENT` and re-fetches the store.
3. **Applied the fix**: Import `useSettingsStore()`, read `hotkey`, apply the same `.replace()` chain from `ready-step.tsx`, render in the `<kbd>`.

## Root Cause Analysis

The empty-state template was written before the settings store had a hotkey field, or before the feature was fully wired. When the feature shipped, someone updated the settings store + UI + cross-window messaging, but missed this one spot in the App's render path. Common pattern: new features add state in multiple places; one always gets skipped on first pass.

## Lessons Learned

1. **Grep for hardcoded values when shipping dynamic features.** Before a feature lands, grep the whole src/ for old hardcoded strings. Catches these misses pre-commit.

2. **Don't extract too early, but watch for 3rd consumer.** The formatter is now in 2 places (App.tsx + ready-step.tsx). Still too early to extract a `formatHotkey()` helper — would be premature. But if a 3rd site appears, extract immediately.

3. **One hardcoded string can undermine user trust.** Settings work, message just lagged. User doesn't know that; they see "I changed the hotkey, but it still shows the old one" → doubt. Fast fix (5 min once found) saves the UX credibility.

## Next Steps

1. **Merged**: Commit a4c6981 ships the fix. Main window now reads and displays the user's actual hotkey.
2. **Watch for formatter reuse**: If a 3rd hotkey-display site emerges, extract `formatHotkey(combo: string): string` to `src/lib/hotkey-formatter.ts`.
3. **No docs update needed**: The feature is user-visible; no architecture or process change.

## Unresolved Questions

Should we proactively extract a `formatHotkey()` helper now (YAGNI says no, wait for 3rd consumer), or is 2 call sites enough to justify it? Per development rules, waiting for 3 is the call.

## Commit

- `a4c6981` fix(main): read hotkey from settings store instead of hardcoded default
