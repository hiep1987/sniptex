# Contributing to SnipTeX

Thanks for considering a contribution. SnipTeX is a free, open-source OCR snip tool for LaTeX and Markdown.

## Development setup

1. Install prerequisites — see [Tauri prerequisites](https://tauri.app/start/prerequisites/).
   - Rust stable (via `rustup`)
   - Node 20+ and `pnpm`
   - Tauri CLI 2.x: `cargo install tauri-cli --version "^2"`
2. Install JS dependencies: `pnpm install`
3. Run the desktop app in dev mode: `pnpm tauri dev`
4. Build a release bundle: `pnpm tauri build`

## How to contribute

- File an issue before starting non-trivial work to align on direction.
- One logical change per pull request.
- Match the existing code style; run `pnpm build` and `cargo check` from `src-tauri/` before pushing.
- Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `refactor:`, etc.).

## License

By contributing, you agree that your work is released under the [MIT License](./LICENSE).
