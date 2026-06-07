# SnipTeX Install Guide

Pick the install method that matches your platform. Both Mac and Windows
ship **unsigned by an identity** — Apple Developer Program ($99/yr) and a
Windows EV cert (~$200/yr) are deferred until donations cover the cost.
You'll see a one-time "are you sure?" dialog from your OS the first time
you open SnipTeX. The steps below walk through it.

---

## macOS

### Option A — Homebrew Cask (recommended)

Cask installs bypass Gatekeeper, so you don't see the quarantine dialog.

```bash
brew install --cask sniptex
```

> The Cask isn't on the official `homebrew/homebrew-cask` repo yet (it gets
> submitted at launch — see Phase 15). Until then, install from this repo:
>
> ```bash
> brew install --cask ./Casks/sniptex.rb
> ```

Launch from Spotlight (`⌘ Space`, type "SnipTeX") or `/Applications`.

### Option B — DMG download

> v0.x ships **Apple Silicon only** (`aarch64.dmg`). Intel Mac builds
> are deferred indefinitely — GitHub's free Mac runner is ARM and the
> only Intel options (`macos-13` retired Dec 2025, `macos-13-large` /
> `macos-15-large` paid) aren't viable right now.

1. Download `SnipTeX_<version>_aarch64.dmg` from the
   [Releases page](https://github.com/hiep1987/sniptex/releases).
2. Double-click the DMG to mount it.
3. Drag **SnipTeX.app** onto the **Applications** shortcut.
4. First-launch path depends on your macOS version:
   - **macOS 12–14 (Monterey, Ventura, Sonoma):**
     **right-click SnipTeX.app → Open**, then click **Open** in the
     dialog. You must use right-click (or Control-click) the first time —
     a normal double-click shows "cannot be opened because the developer
     cannot be verified" with no Open button.
   - **macOS 15+ (Sequoia):** Apple removed the right-click bypass.
     Double-click once and dismiss the "cannot be opened" dialog. Then
     open **System Settings → Privacy & Security**, scroll to the
     "SnipTeX was blocked" notice, click **Open Anyway**, and authenticate
     with Touch ID / password.
5. Subsequent launches work normally (double-click, Spotlight, Dock).

#### Stuck on "SnipTeX is damaged and can't be opened"?

Quarantine sometimes flags the bundle. Clear it:

```bash
xattr -cr /Applications/SnipTeX.app
```

Then try **right-click → Open** again.

#### Verifying the ad-hoc signature (optional)

```bash
codesign --verify --verbose=2 /Applications/SnipTeX.app
```

Expect `valid on disk` and `satisfies its Designated Requirement`. The
signing identity will be `-` (ad-hoc).

#### Verifying the DMG checksum

The SHA256 of every DMG is published in the release notes.

```bash
shasum -a 256 SnipTeX_<version>_aarch64.dmg
```

Compare against the value in the GitHub release. Homebrew Cask installs
verify this automatically.

---

## Windows

SnipTeX ships an unsigned MSI for x64. SmartScreen flags new unsigned
installers until they earn download reputation (~3,000 downloads).

### Install

> Windows MSIs land with v0.2.0 from the CI release pipeline. The steps
> below describe the install flow once the artifact is published.

1. Download `SnipTeX_<version>_x64-setup.msi` from the
   [Releases page](https://github.com/hiep1987/sniptex/releases).
2. Double-click the MSI.
3. SmartScreen shows **"Windows protected your PC"** — click
   **More info**, then **Run anyway**.
   - If "More info" doesn't appear, open Settings → Privacy & Security →
     Windows Security → App & browser control → Reputation-based
     protection, and temporarily disable **Check apps and files**. Re-enable
     after install.
4. **If Smart App Control blocks the MSI silently (Win 11 22H2+ clean
   installs):** the installer aborts with no SmartScreen dialog. Open
   Settings → Privacy & Security → Windows Security → App & browser
   control → Smart App Control settings, switch to **Off**, reboot, then
   re-run the MSI. (Smart App Control is one-way — turning it off cannot
   be reversed without a Windows reinstall, so weigh the trade-off.)
5. Follow the installer prompts. Default install path is
   `C:\Program Files\SnipTeX\`.
6. Launch from Start Menu → "SnipTeX".

### Verifying the MSI checksum

```powershell
Get-FileHash -Algorithm SHA256 .\SnipTeX_<version>_x64-setup.msi
```

Compare against the SHA256 published in the release notes.

### Uninstall

Settings → Apps → Installed apps → SnipTeX → Uninstall.

---

## Linux

Not officially supported in v1. The codebase is Tauri-based and should
build on Linux, but it's not in the v1 test matrix. Track Linux support in
the project roadmap.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Mac: "App is damaged" dialog | `xattr -cr /Applications/SnipTeX.app` |
| Mac: "Cannot verify developer" without an Open button | Right-click (not double-click) → Open |
| Mac: app crashes on launch | Check Console.app → search "SnipTeX" |
| Windows: SmartScreen has no "More info" link | Disable Reputation-based protection temporarily |
| Windows: MSI install fails silently | Re-run as Administrator |
| Hotkey doesn't fire | Grant Accessibility (Mac) or Background-app (Windows) permission |

For anything else, open an issue at
<https://github.com/hiep1987/sniptex/issues>.
