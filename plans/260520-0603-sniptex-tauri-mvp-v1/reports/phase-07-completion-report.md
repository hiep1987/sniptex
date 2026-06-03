# Phase 7 — SQLite History with FTS5 Search — Completion Report

**Date:** 2026-06-03
**Status:** Completed
**Closed by:** Live GUI smoke test confirmed all 5 success criteria

## What shipped

- **Schema + migrations** — `src-tauri/src/storage/migrations.rs` V1: `snip_records` table + `snip_records_fts` FTS5 virtual table + insert/delete sync triggers. WAL journal mode, foreign keys on.
- **Connection management** — `storage/mod.rs::init` opens `{app_data_dir}/sniptex.sqlite`, runs pending migrations, returns `Mutex<Connection>` managed via Tauri state.
- **Thumbnail generator** — `storage/thumbnail.rs`: PNG → Lanczos3 resize to 200×200 max → WebP encode (quality 80, ≤32 KB per thumb).
- **History repository** — `storage/history.rs` CRUD + FTS5 search with BM25 ranking, oldest-first eviction (`enforce_max_records` returns evicted paths so caller can clean up disk files).
- **5 Tauri commands** — `get_history`, `search_history`, `delete_record`, `rerun_snip`, `export_record` (all registered in `lib.rs`).
- **Post-OCR persist pipeline** — `persist_to_history` runs after every successful snip: copies temp PNG to `images/{uuid}.png`, generates thumb, inserts DB row, enforces max. Rolls back the persisted PNG + thumb if the INSERT fails (no orphan files).
- **Frontend** — `src/stores/history-store.ts` with debounced search (200 ms); `src/components/history-row.tsx` and `src/components/rerun-menu.tsx`; `src/windows/history-window.tsx` virtualized list via `@tanstack/react-virtual`.
- **UX divergence from spec** — implementation uses hover-revealed icon buttons (Copy / Rerun / Delete) instead of a right-click context menu. Phase doc updated to reflect the actual affordance.
- **Asset protocol** — `tauri.conf.json` scope extended to `$APPDATA/images/**` and `$APPDATA/thumbs/**` so the webview can render thumbnails via `convertFileSrc`.

## Verification

| Check | Result |
|------|--------|
| `cargo test --lib` | clean |
| `cargo test --test storage_history` | 7/7 pass (insert / recent / search / delete / update / eviction / FTS escape) |
| `cargo run --bin history_smoke` | pass |
| `pnpm tsc --noEmit` | clean |
| `pnpm build` | clean |
| Live GUI: insert 100+ snips, persist across restart | pass |
| Live GUI: FTS5 search filter feels instant (<100 ms) | pass |
| Live GUI: Delete icon removes row + thumb + PNG from disk | pass |
| Live GUI: Rerun icon updates row text in-place without manual refresh | pass |
| Live GUI: Eviction at 100 snips drops oldest from UI and disk | pass |

## Key decisions

- **`rusqlite` directly, not `tauri-plugin-sql`.** All DB code is Rust-owned so we can ship the FTS5 virtual table + triggers + WAL setup as one atomic init. The plugin's main value is frontend-direct SQL, which we deliberately don't want — every query goes through a typed Tauri command for shape stability.
- **Thumb format: 200×200 WebP at quality 80.** Inline blobs in SQLite would balloon the DB; separate `thumbs/{uuid}.webp` files keep DB <1 MB at 100 snips and let the webview load them through `convertFileSrc` with proper cache headers.
- **Eviction is oldest-first by `created_at`, not LRU.** Predictable for users ("last 100 snips"); LRU would discard snips users haven't looked at recently which is wrong for an OCR-history use case.
- **Hover icons over right-click menu.** Discoverable on first hover; right-click context menu would hide all actions and require user discovery. Icons fade in on `group-hover` so the resting row stays clean.
- **Rerun consistency validator (added 2026-05-24 in plan `260524-1304-gemini-cli-clean-rerun-output-fix/`).** Before overwriting a row's text on rerun, validate that the new OCR output is consistent with the original (label match + content overlap). Prevents Gemini CLI's "plausible but unrelated" hallucinations from silently corrupting saved snips.

## Touchpoints (regression surface)

- Snip pipeline (Phase 3-5): `run_snip` now persists post-OCR; capture/dispatch paths unchanged.
- Settings (Phase 8): `history_size` setting drives `enforce_max_records` cap; default 100, slider exposes 50 / 100 / 500 / unlimited.
- Asset protocol: scope expansion in `tauri.conf.json` is the only way thumbs load — regression candidate if scope edits in future phases drop these.

## Phases this unblocked

- **Phase 8 (Settings)** — history size slider + "Clear history" button wired to repo functions.
- **Phase 9 (Format toggle)** — Copy-as menu options reuse `export_record` for history-row copy actions.

## Outstanding (deferred to follow-up plans)

- Search ranking polish (snippet preview with `<mark>` highlights around matched FTS5 tokens) — deferred; current BM25 + truncated preview is enough for v1.
- Bulk delete / select-multiple — deferred; per-row delete + "Clear history" cover the v1 cases.

## Unresolved questions

None.
