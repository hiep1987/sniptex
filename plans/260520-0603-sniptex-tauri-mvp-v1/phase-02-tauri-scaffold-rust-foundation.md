---
phase: 2
title: "Tauri Scaffold & Rust Foundation"
status: complete
priority: P1
effort: "2d"
dependencies: [1]
---

# Phase 2: Tauri Scaffold & Rust Foundation

## Overview

Bootstrap the Tauri 2 project structure (React/TS frontend + Rust backend), install required Tauri plugins, configure the build/dev pipeline, and verify cross-platform "hello world" via a global hotkey listener that prints to console.

## Key Insights

- Tauri 2 (not v1) — different plugin API, capability/permission model. Pin major version in `Cargo.toml` and `package.json`.
- Plugin set required for v1: `global-shortcut`, `clipboard-manager`, `sql`, `updater`, `store`, `dialog`, `notification`, `autostart`.
- Frontend stack chosen in `replan.md` §2: React 18 + TS + Vite + Tailwind 4 + shadcn/ui + Zustand. **Updated 2026-05-22:** React 19.2 adopted (template default; user confirmed during Phase 2 cook). React 18 spec was carried over from older brief; React 19 is stable since 2025-Q1 and compatible with Tailwind 4 + shadcn/ui + sonner.
- Repo layout target documented in `replan.md` §2 ("Repo structure").

## Requirements

**Functional**
- `pnpm tauri dev` opens dev window and hot-reloads on file change.
- `pnpm tauri build` produces platform-native bundle (`.app` on Mac, `.exe`/`.msi` on Windows).
- Pressing `CMD+Shift+M` (Mac) or `Ctrl+Shift+M` (Windows) triggers a Rust-side log line and React-side toast.

**Non-functional**
- Bundle size <20MB target (already enforceable via Tauri config).
- Cold start <800ms.

## Architecture

```
sniptex/
├── package.json              (pnpm, scripts: dev/build/tauri)
├── vite.config.ts            (React + Tauri integration)
├── tsconfig.json
├── tailwind.config.ts        (Tailwind 4)
├── src/                      (React frontend)
│   ├── main.tsx
│   ├── App.tsx               (hotkey demo)
│   └── lib/invoke.ts         (typed Tauri command wrappers)
└── src-tauri/                (Rust backend)
    ├── Cargo.toml
    ├── tauri.conf.json       (capabilities, plugins, windows)
    ├── build.rs
    └── src/
        ├── main.rs
        ├── lib.rs
        └── commands.rs       (placeholder #[tauri::command] hello())
```

## Related Code Files

- Create entire scaffold via `pnpm create tauri-app sniptex --template react-ts --manager pnpm`
- Modify: `src-tauri/Cargo.toml` — add plugin deps
- Modify: `src-tauri/tauri.conf.json` — declare plugins + capabilities
- Modify: `src-tauri/capabilities/default.json` — grant plugin permissions
- Create: `src-tauri/src/lib.rs` — register plugins + commands
- Create: `src/lib/invoke.ts` — typed wrappers around `@tauri-apps/api/core` invoke

## Implementation Steps

1. Install toolchain: Rust stable (`rustup default stable`), Node 20+, pnpm, Tauri CLI 2.x (`cargo install tauri-cli --version "^2"`).
2. Scaffold: `pnpm create tauri-app sniptex --template react-ts --manager pnpm` in `~/Projects/`. Move into `sniptex/` repo (this directory).
3. Initialize git, add MIT `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` skeletons, `.gitignore` (include `fixtures/sgk/`).
4. Install Tauri plugins in `src-tauri/Cargo.toml`:
   - `tauri-plugin-global-shortcut`
   - `tauri-plugin-clipboard-manager`
   - `tauri-plugin-sql` (with `feature = ["sqlite"]`)
   - `tauri-plugin-updater`
   - `tauri-plugin-store`
   - `tauri-plugin-dialog`
   - `tauri-plugin-notification`
   - `tauri-plugin-autostart`
5. Install npm counterparts: `@tauri-apps/plugin-global-shortcut`, `@tauri-apps/plugin-clipboard-manager`, etc.
6. Add frontend dev deps: Tailwind 4 (`tailwindcss@4 @tailwindcss/vite`), shadcn/ui (init via `npx shadcn@latest init`), Zustand.
7. Configure `tauri.conf.json`:
   - `productName: "SnipTeX"`, `version: "0.1.0"`, `identifier: "com.sniptex.app"`
   - `bundle.targets`: Mac → `["app", "dmg"]`, Windows → `["msi", "nsis"]`
   - Register plugins under `plugins: {}`
8. Configure `capabilities/default.json` to allow plugin permissions on main window.
9. Implement `src-tauri/src/lib.rs`:
   - `tauri::Builder::default().plugin(...).invoke_handler(generate_handler![hello]).run()`
   - Register `CommandOrControl+Shift+M` shortcut → `println!("hotkey")` + emit `"hotkey-pressed"` event
10. Frontend `App.tsx` listens to `hotkey-pressed` and renders toast (use shadcn toast).
11. Verify `pnpm tauri dev` on Mac. Verify build on CI-equivalent Windows VM or wait for Phase 10.
12. Commit: `feat: scaffold Tauri 2 + React/TS + plugin baseline`

## Todo List

- [x] Install Rust stable, Node 20+, pnpm, Tauri CLI 2.x (rust 1.95.0, node 26, pnpm 10.33.2, cargo-tauri 2.11.2)
- [x] Scaffold project via `pnpm create tauri-app` template react-ts (scaffold→/tmp→merge into existing repo)
- [x] Add MIT LICENSE, contributing docs, .gitignore (src-tauri/target/, gen/schemas/, WixTools/)
- [x] Add 8 Tauri plugins (Cargo + npm) — pinned exact patch versions; updater/autostart/global-shortcut gated behind cfg(not(android/ios))
- [x] Wire up Tailwind 4 + Zustand (shadcn deps installed: clsx, tailwind-merge, class-variance-authority, lucide-react, sonner; `shadcn init` deferred to first component need)
- [x] Configure tauri.conf.json (productName SnipTeX, com.sniptex.app, bundle [app, dmg, msi, nsis])
- [x] Configure capabilities permissions (least-privilege; explicit per-plugin grants, no allowAll)
- [x] Register global shortcut Cmd/Ctrl+Shift+M with console log + frontend toast (cfg(target_os = "macos") routes to Super, else Control)
- [x] Run `pnpm tauri dev` and confirm hotkey roundtrip works on Mac (verified 2026-05-22 — single press → single toast; required Rust 150 ms debounce for macOS Sequoia + global-hotkey 0.7 double-fire AND React StrictMode-safe `listen` cleanup)
- [x] Commit baseline (b6acff3 scaffold + follow-up fix commit)

## Success Criteria

- [ ] `pnpm tauri dev` runs and opens dev window
- [ ] Pressing global hotkey produces both Rust log and React toast
- [ ] `pnpm tauri build` produces `.app` + `.dmg` artifacts on Mac
- [ ] All 8 plugins listed in dependency tree with no version conflicts
- [ ] Repo structure matches `replan.md` §2 target

## Risk Assessment

- **Risk: Tauri 2 plugin API changes mid-development** — Mitigation: pin exact patch versions of plugins; check Tauri 2 changelog before bumping.
- **Risk: Tailwind 4 still alpha at scaffold time** — Mitigation: pin to a tested alpha build; **do not** fall back to v3 (Validation Session 1 confirmed v4 commitment).
- **Realised 2026-05-22: macOS Sequoia + `global-hotkey` 0.7 double-fires `kEventHotKeyPressed`** — one physical press produces two `Pressed→Released` cycles at the Carbon API. Worked around with a 150 ms application-layer debounce in `lib.rs`. Revisit when Tauri bumps `global-hotkey` to a fixed version. Also added React StrictMode-safe `listen` cleanup in `App.tsx` to prevent dev-mode listener duplication.

## Security Considerations

- Capabilities file should grant only the permissions the app needs (least privilege). Avoid `allowAll`.
- No secrets in repo — `.env.example` only.

## Next Steps

- Proceed to Phase 3 (agent system + OCR pipeline) — uses Rust foundation built here.

## Open Questions

- None remaining after Validation Session 1 (Tailwind 4 committed).

<!-- Updated: Validation Session 1 - Tailwind 4 committed, no v3 fallback -->
