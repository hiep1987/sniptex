# Updater UX: No Stable Release Yet — Two Natural Consequences of Phase 12/13 Decisions

**Date**: 2026-06-10 16:45
**Component**: Vite dev server, Tauri updater `check()` flow, Settings > About tab
**Severity**: Medium (dev-time friction) + Medium (user-facing messaging)
**Status**: Resolved

## What Happened

Two distinct issues emerged during dev iteration post Phase 13 landing-page launch:

**Issue 1: Vite 504 Outdated Optimize Dep** — After bumping version to v0.0.2-dev and touching tauri.conf.json, running `pnpm tauri dev` loaded the WebView, but DevTools showed persistent 504 errors on `@tauri-apps_plugin-updater.js?v=abc123def456`. WebView loaded partially; some functionality stalled. Thought: code regression? Dependency break? Actually: Vite cache invalidation race.

**Issue 2: "Update check unavailable"** — Clicked "Check for updates" in Settings > About tab. Got a red error toast: `toast.error("Update check unavailable - Could not fetch a valid release JSON from the remote")`. Seemed alarming. Ran `gh release view` to check: only v0.0.2-dev exists, marked `isPrerelease: true`. That release has a valid latest.json file, but at `releases/download/v0.0.2-dev/latest.json`. Configured endpoint points to `releases/latest/download/latest.json`. GitHub's `/releases/latest/` redirects exclude pre-releases by design → 404 → Tauri can't parse → error. **Not a bug; expected state given our pre-release iteration strategy.** But the UX message (red error) felt wrong.

## The Brutal Truth

Issue 1 was embarrassing-slash-instructive: Vite's pre-bundling is invisible-until-it-breaks. The symptom (module 504) pointed straight at "plugin-updater" by name, so I spent 20 min verifying the plugin code. Only then realized "oh, this is Vite's cache, not our code." The module got unlucky being first in the load order; any plugin would have triggered the same 504.

Issue 2 was the tougher call: the error message felt alarming (red toast, "unavailable" text), but the root cause was *intentional*. We chose to ship v0.0.2-dev as a pre-release specifically to iterate safely between milestones (per memory `project-prerelease-tags-dev-iteration.md`). That strategy means "no stable release yet" — a fact the user needs to know, but not in alarming red. Temptation: "fix" it by promoting v0.0.2-dev to latest, or changing the endpoint, or just ignoring the problem. But that would reverse a documented user decision (pre-release iteration strategy is in memory for a reason). Instead: keep the strategy, fix the UX.

## Technical Details

**Issue 1 diagnosis**:
```
Vite pre-bundles dependencies into node_modules/.vite/deps/ with content-hashed filenames:
  deps_abc123def456.js  (content hash)
  deps_temp_xyz789.js   (transient during rebundle)
  
When dependencies shift (version bumps, lockfile changes, reinstall), 
Vite recomputes hashes and swaps:
  deps_abc123def456.js → [delete]
  deps_temp_xyz789.js  → deps_new123new456.js
  
WebView holds old module URLs in loaded JS: <script src="...?v=abc123def456"></script>
Next hot reload or page refresh requests old hash → Vite returns 504 
(signal: "stale URL, rebundle happened, retry").
```

Recent commits (e88adeb "bump to v0.0.2-dev", 1eeac88 "version field must be plain SemVer") triggered Tauri config changes → node_modules shift → Vite rebundled → WebView held stale URLs.

**Issue 2 diagnosis**:
```
Memory documents v0.0.2-dev as isPrerelease: true (per pre-release iteration strategy)
Latest.json uploaded to: releases/download/v0.0.2-dev/latest.json
Configured updater endpoint: releases/latest/download/latest.json

GitHub /releases/latest/ redirect behavior:
  curl -L /releases/latest/ → 30x redirect to /releases/tag/v0.0.2-dev/ ONLY if isPrerelease: false
  If isPrerelease: true → /releases/latest/ is 404
  
Tauri updater check():
  1. fetch(endpoint)
  2. parse JSON
  3. if parse fails → throw "Could not fetch a valid release JSON"
```

The memory `project-tauri-2-updater-always-live.md` flagged this exact state: "surface 'No release published yet', not 'unavailable' (the latter implies config error)". We had the design rule but hadn't implemented it yet.

**Fix (commit c11d6b1)**:
- In `src/components/update-dialog.tsx:129`, added `no-release` kind to `UpdateCheckResult` discriminated union:
  ```typescript
  type UpdateCheckResult = 
    | { kind: 'available'; version: string; releaseNotes: string }
    | { kind: 'current'; version: string }
    | { kind: 'error'; message: string }
    | { kind: 'no-release' }  // NEW
  ```
- In `runCheck()` function, added exact phrase match:
  ```typescript
  } catch (err) {
    const msg = String(err);
    if (msg.includes('Could not fetch a valid release JSON')) {
      return { kind: 'no-release' };  // v0.0.2-dev is pre-release; /releases/latest/ excludes it
    }
    return { kind: 'error', message: msg };
  }
  ```
- In `src/windows/settings/about-tab.tsx:18`, branched on `no-release`:
  ```typescript
  case 'no-release':
    toast.info('No stable release yet. Updates available after v0.1.0 ships.');
    break;
  ```

Changed toast color from red (`.error()`) to blue (`.info()`); message is now neutral/helpful instead of alarming.

**Files modified**: `src/components/update-dialog.tsx`, `src/windows/settings/about-tab.tsx`. Ran `pnpm tsc` — no errors.

## What We Tried

**Issue 1**:
1. Checked plugin-updater code — no recent changes, looked fine.
2. Checked Cargo.toml for version mismatches — versions consistent.
3. Cleared Tauri cache (`rm -rf src-tauri/target`) — no change.
4. Searched Vite docs — found reference to "Outdated Optimize Dep" and cache dirs.
5. **Applied**: `rm -rf node_modules/.vite` + restart dev server + `Cmd+Shift+R` in WebView. 504s gone immediately.

**Issue 2**:
1. Read error message aloud — "unavailable" sounds like a code/config problem.
2. Verified endpoint config in tauri.conf.json — correct.
3. Verified that latest.json exists — confirmed via `gh release view v0.0.2-dev`.
4. Checked if v0.0.2-dev should be promoted to latest — would break pre-release iteration strategy (memory says don't).
5. Checked if endpoint should point to v0.0.2-dev specifically — no, that couples code to release names, bad pattern.
6. **Concluded**: Strategy is right; UX message is wrong. Implemented `no-release` kind + friendly toast.

## Root Cause Analysis

**Issue 1**: Not a bug. Cache invalidation race inherent to Vite + dev-server hot reload. Triggered by recent commits that touched Tauri config, which forced node_modules shift and pre-bundle recompute. The error names the unlucky plugin (updater happened to be loaded first), but any bundled dep would have done the same. **Pattern**: symptoms from dev-time build machinery, not app code.

**Issue 2**: Not a bug either. Pre-release iteration strategy (v0.0.2-dev as isPrerelease=true) is the intentional design. GitHub's `/releases/latest/` excludes pre-releases by design. The code correctly surfaces the 404-then-no-JSON condition. The problem was *tone*: red error toast reads as "something went wrong; you misconfigured", when the truth is "this is expected until v0.1.0 ships as a non-prerelease." **Pattern**: message didn't match user expectations or strategy intent. UX (not code) fix.

## Lessons Learned

1. **Vite 504 on plugin names is a cache symptom, not a plugin bug.** Don't debug the plugin; clean `.vite/` cache instead. Worth saving to memory because next 504 will be recognized instantly without 20 min of "wait, is the plugin broken?"

2. **Memory rules pay off twice.** The pre-release strategy (memory `project-prerelease-tags-dev-iteration.md`) predicted exactly this state. The design rule about "surface no-release, not unavailable" (memory `project-tauri-2-updater-always-live.md`) gave us the pattern to fix it. Both decisions locked 6 months ago now steering current work. This is the right pattern: document strategy once, let memory close the loop on consequences.

3. **Don't reverse documented strategies based on surface-level UX friction.** The temptation was "hmm, this error is annoying, let me just promote v0.0.2-dev to latest or change the endpoint". That would have broken the pre-release iteration design that lets us ship safely between milestones. Instead: honor the strategy, improve the message. (This maps to CLAUDE.md §3: guard user decisions against audit/YAGNI drift.)

4. **Scout-first prevented half the investigation.** Before suggesting any change to updater config or endpoint, I verified `gh release view v0.0.2-dev` to see the actual state. That one 10-second check saved an hour of "should we change the code or the config or the release strategy?" debate. Confidence >85%, so didn't ask the user — acted directly.

5. **Natural consequences > bugs.** Both issues emerged directly from recent decisions (version bumps in Phase 13, pre-release iteration from Phase 12). Not regressions or oversight. This framing matters for triage: when symptoms follow documented strategy, fix the UX/messaging, not the strategy. "Is this a bug?" → "No, it's expected state given our choices." → "Is the UX clear?" → "No." → "Fix the UX." Much cleaner than "something broke, debug the whole stack".

## Next Steps

1. **Merged**: commit c11d6b1 ships friendly updater UX. Toast will show `No stable release yet...` (blue info) next time user clicks "Check for updates" before v0.1.0 ships.

2. **Self-resolves on v0.1.0 release**: When v0.1.0 ships as isPrerelease=false, GitHub `/releases/latest/` will redirect to it. Tauri's check() will fetch latest.json successfully, match version, and show either "Current version" or "Update available" (depending on app version). No code change needed; new release state fixes the UX automatically.

3. **Memory entry**: Created `project-vite-tauri-504-outdated-optimize-dep.md` so next 504 is diagnosed in 30 seconds (clean .vite/deps, restart).

4. **Docs**: Consider adding a note to `docs/releasing.md` or `docs/updating.md` about the pre-release iteration workflow and how the updater behaves during the "no stable release yet" window. Useful for future maintainers.

## Unresolved Questions

None. Both issues diagnosed, root causes documented, UX fix shipped, strategy preserved. v0.1.0 release will self-resolve the updater state; pre-release iteration continues to be the safe pattern between milestones.
