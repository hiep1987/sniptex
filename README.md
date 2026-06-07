# SnipTeX

> Free, open-source OCR snip tool for LaTeX and Markdown. Mac + Windows.
> Bring your own agent or your own key.

SnipTeX captures a screen region with a hotkey, sends the image through
your chosen OCR path — Codex CLI, Gemini CLI, or Gemini Vision API direct
— and drops clean LaTeX or Markdown on your clipboard. No backend,
zero subscription. MIT licensed.

A free alternative to Mathpix Snip ($5–20/month) for technical writers,
students, and educators working with equations, tables, and Vietnamese
SGK textbook content.

## Platform support

| Platform | Architecture | Format | v0.x status |
|----------|--------------|--------|-------------|
| macOS 12+ | Apple Silicon | `.dmg` (ad-hoc signed) + Homebrew Cask | shipping |
| Windows 11 | x64 | `.msi` (unsigned, SmartScreen workaround) | shipping via CI |
| macOS 12+ | Intel (x86_64) | — | deferred (no free Intel GH runner) |
| Windows 11 | ARM64 | — | deferred (post-v1.0) |

v0.x ships Apple Silicon Mac + Windows x64 only. Intel Mac requires a
paid GitHub Larger Runner since `macos-13` retired in December 2025;
we'll revisit once donations cover the runner cost or cross-compile
becomes reliable.

## Install

### macOS (Homebrew)

```bash
brew install --cask sniptex
```

> Until the official `homebrew/homebrew-cask` PR lands (Phase 15), install
> from this repo: `brew install --cask ./Casks/sniptex.rb`.

### macOS (DMG)

Download from [Releases](https://github.com/hiep1987/sniptex/releases),
drag to **Applications**, then **right-click → Open** the first time so
Gatekeeper lets it run.

### Windows (MSI) — v0.2.0+

Download from [Releases](https://github.com/hiep1987/sniptex/releases),
run the installer. SmartScreen shows "Windows protected your PC" — click
**More info → Run anyway**. On Win 11 with Smart App Control enabled,
see the install guide.

Full Gatekeeper / SmartScreen walkthrough: [`docs/install-guide.md`](docs/install-guide.md).

## How it works

1. **Hotkey** (`Cmd+Shift+M` on Mac, `Ctrl+Shift+M` on Windows) fires the
   region selector overlay. Rebind in Settings.
2. **Drag** to select the area you want to OCR.
3. **OCR pipeline** sends the cropped image to your configured agent:
   - **Codex CLI** (default, BYOA, privacy-first, ~14s p95)
   - **Gemini CLI** (experimental secondary)
   - **Gemini Vision API direct** (BYOK cloud mode, sub-5s p95)
4. **Result** lands on your clipboard as LaTeX or Markdown. Preview window
   pops with MathJax-rendered output and a copy-as toggle.
5. **History** (SQLite + FTS5) keeps every snip locally — rerun with a
   different agent, search by content, never lose work.

## Bring your own agent (BYOA) or key (BYOK)

SnipTeX never bundles a cloud account. You provide one of:

- **OpenAI Codex CLI** — install once, sign in once, use everywhere.
- **Google Gemini CLI** — same model, experimental wrapper.
- **Google AI Studio API key** — pasted into Settings (stored in OS
  keychain), used by the `--cloud` adapter for sub-5s responses.

Onboarding walks you through pick-one + install in under 2 minutes.

## Distribution & signing

- macOS bundle is **ad-hoc signed** (`codesign --sign -`) — fixes the
  "app is damaged" dialog but still triggers first-launch Gatekeeper.
  Full notarization deferred until donations cover the Apple Developer
  Program ($99/yr).
- Windows MSI is **unsigned** — SmartScreen workaround documented in
  [`docs/install-guide.md`](docs/install-guide.md). EV cert (~$200/yr)
  deferred to the same donation milestone.
- All releases publish SHA256 checksums in the release notes. Verify with
  `shasum -a 256` (Mac) or `Get-FileHash -Algorithm SHA256` (Windows).

## Build from source

```bash
# prerequisites: Rust stable, Node 20+, pnpm, Tauri CLI 2.x
pnpm install
pnpm tauri dev       # development build
pnpm tauri build     # production build
scripts/sign-mac.sh  # ad-hoc sign the macOS bundle
```

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`.

## License

[MIT](LICENSE) © 2026 SnipTeX contributors.

## Community

Discord (coming with Phase 15 launch). Issues + discussion live on
[GitHub Issues](https://github.com/hiep1987/sniptex/issues).
