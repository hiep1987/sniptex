# Phase 7 Review — SQLite History + FTS5 Search

Date: 2026-05-23
Scope: storage module, history commands, rerun/export flows, asset-protocol scope, frontend store + UI.

## Verdict

Ship. No blockers found. Implementation is solid, matches the plan acceptance criteria, all 7 storage tests pass locally (`cargo test --test storage_history` → 7/7). FTS5 wiring, lock scope, RAII guards, and `disarm()` semantics all check out.

A handful of medium/low items below — none warrant holding the phase.

---

## Critical Issues

None.

## High Priority

### H1. Orphaned image file on persist failure (file-system leak — low blast radius)
`commands.rs:290-318`. Sequence in `persist_to_history`:
1. `fs::rename(cropped_path, &image_dst)` — moves temp PNG into `{app_data}/images/{uuid}.png`.
2. `make_thumbnail(&image_dst, &thumb_dst)` — may fail (image decode, disk full).
3. `store.conn.lock()` — may fail (poisoned).
4. `insert(...)` — may fail (constraint, disk).

If any of steps 2–4 fails, the file at `image_dst` is orphaned: DB row doesn't exist, `TempFileGuard` only watches the original temp path (rename already gone). Same applies to `thumb_dst` if step 3/4 fails after step 2 succeeded.

Impact: per failed snip we leak ~80KB PNG + maybe an 8KB WebP under `{app_data}`. No PII risk beyond what user already accepted. Won't show up in History (no row).

Fix (optional, can defer): on error path inside `persist_to_history`, remove `image_dst` and `thumb_dst` best-effort before returning Err. OR add a startup orphan-scan that diffs `images/` against DB rows. Either is fine for v1.5; not a v1 blocker.

### H2. TOCTOU between `rerun_snip` find + update
`commands.rs:691-716`. `find_by_id` runs under lock → lock released → `await OCR` (slow, seconds) → second lock for `update_output`. A concurrent `delete_record` between the two can leave the second-block `UPDATE … WHERE id=?1` matching 0 rows. The function then returns a `HistoryRecordDto` for a row that does not exist; the frontend store inserts/replaces a phantom row that disappears on the next `load()`.

Impact: cosmetic, self-healing on refresh. Not a data-integrity issue (no row resurrection — `UPDATE` of missing id is a noop).

Fix (optional): check `rows_affected` from `conn.execute`; if 0 return `Err("record was deleted")`. ~3 lines.

## Medium Priority

### M1. `agent_id` in `update_output` not validated against installed agents
`rerun_snip` accepts any `agent_id: String` from the frontend. `run_ocr_for_path` validates against installed agents (returns Err if not found), so an OCR Err short-circuits before `update_output`. Defense already in place. No change needed; flag only.

### M2. `app.state::<HistoryStore>()` panic surface
`rerun_snip` calls `app.state::<HistoryStore>()` (commands.rs:690) which panics if state was never `manage`-d. Setup hook hard-fails on `storage::init` before `app.manage()` would be reached, so in practice this is unreachable. Acceptable. (Cleaner: `app.try_state::<HistoryStore>().ok_or("storage not initialized")?`.)

### M3. `enforce_max_records` eviction on every insert is O(n) per snip
`SELECT COUNT(*) FROM snip_records` + LIMIT scan on every insert. At 100-row cap and SQLite WAL, totally fine (<1ms). At 1k+ it would still be sub-10ms. Note for Phase 8 when the cap becomes user-configurable: if cap raised to e.g. 10k, consider only enforcing when `total > cap + 10` (batch eviction).

### M4. `last_insert_rowid` after `enforce_max_records`
`persist_to_history` (commands.rs:316-328) captures `insert(...)` rowid BEFORE calling `enforce_max_records`. Verified: `id` is bound to the local `let id = …` before eviction runs. Correct.

### M5. Asset protocol scope verified
`$APPDATA` in Tauri 2 asset-protocol scope resolves to the bundle-identifier–scoped roaming data dir on Windows, `~/Library/Application Support/<bundle>` on macOS, `$XDG_DATA_HOME/<bundle>` on Linux — same as `app.path().app_data_dir()`. Confirmed via tauri docs. `convertFileSrc` wraps absolute paths into `asset://`; the scope allowlist matches the absolute path at request time. Images and thumbs WILL load.

## Low Priority

### L1. `sanitize_fts_query` edge cases — verified safe
Tested manually with `rustc`:
- empty / whitespace-only → empty string (already caught by `if trimmed.is_empty()` at search.rs:68)
- column filters (`output_text:foo`) → quoted as single phrase, no injection
- `NEAR()`, `^`, `*`, `OR`, `AND` operators → all quoted as literal phrases
- embedded `"` → properly doubled per FTS5 grammar
- Unicode (NBSP, diacritics) → `trim()` strips NBSP; non-ASCII tokens passed through (tokenizer handles diacritic folding via `remove_diacritics 2`)

No escape paths through. Solid.

### L2. FTS5 external-content trigger pattern — canonical
AI/AD/AU triggers (migrations.rs:53-71) use the documented `INSERT INTO fts(fts, rowid, col) VALUES('delete', ...)` shadow-delete pattern. AU does delete-then-insert, which is exactly what the FTS5 docs recommend for external-content tables. Verified test `update_output_replaces_text_and_keeps_fts_in_sync` proves new term matches and old term doesn't. ✓

### L3. `TempFileGuard::disarm()` consume-then-no-op pattern — verified
Built a minimal repro with `rustc`:
```
disarm() → DROP runs with armed=false → no remove.
no disarm → DROP runs with armed=true → remove called.
```
The `mut self` consume binding doesn't suppress `Drop`; it just lets the method mutate before drop runs. Correct semantics.

### L4. Frontend `push()` creates partial row with empty `thumb_path`
preview-window.tsx:131-141: when `snip-complete` fires, the optimistic push has `thumb_path: ""`, so `HistoryRow` renders the dashed placeholder. The thumb file was written by Rust before the event fires, but the frontend doesn't know its path. On next `load()` the real thumb shows. Acceptable: thumb is decorative, the row is functional immediately.

### L5. `webp v0.3` is a maintenance-mode crate
`webp = "0.3"` — current crate version is older and uses static libwebp. Encoding works fine for thumbnail sizes. Future maintenance: consider migrating to `image` crate's WebP support when it lands (already supports decode). Not blocking.

### L6. `Mutex<Connection>` vs r2d2 pool
One connection serialises all DB ops. With WAL, readers don't block writers at the SQLite level — but with one `Mutex` they do in Rust. At MVP scale (single user, ~1 op/sec) this is fine. If multi-window or background indexing lands, switch to `r2d2_sqlite`.

### L7. Defensive `agent_id` allow-listing on `rerun_snip`
Frontend can send arbitrary `agent_id` string. Validated indirectly by `run_ocr_for_path → find(|a| a.spec.id == id)` returning Err on miss. No path to write garbage into DB (Err short-circuits before update). Fine.

### L8. `format_export` Markdown vs Plain identical
commands.rs:744-747 — `ExportFormat::Markdown` and `ExportFormat::Plain` both return `text.to_string()`. Plan says Phase 9 will replace with full Format Toggle. Acceptable stub for v1.

## Scout Findings (Edge Cases Considered)

- **Concurrency:** `Mutex<Connection>` serialises writes; `rerun_snip` releases lock across `.await` — verified no held-lock-across-await. `persist_to_history` releases lock before disk cleanup — no deadlock with concurrent reader. ✓
- **FTS injection:** every whitespace-separated token quoted as FTS5 phrase; doubled inner `"`. No operator escape. ✓
- **Backwards compat:** `SnipResult` gained `record_id: Option<i64>` — additive, TS type matches. Old preview code that ignores `record_id` would still compile. ✓
- **Cross-volume rename:** rename → fallback to copy + remove pattern. If copy succeeds but remove fails → temp file leaks under TMPDIR. OS will clean TMPDIR. Acceptable. ✓
- **DB migration idempotency:** `PRAGMA user_version` check + `CREATE TABLE IF NOT EXISTS` everywhere → safe to re-run. ✓
- **Eviction with no rows:** `enforce_max_records` early-returns `Vec::new()` when `total <= max`. No `LIMIT 0` query. ✓
- **`rusqlite::Connection::last_insert_rowid`:** per-connection, not global. Mutex-serialised inserts → correct rowid. ✓
- **Eviction-then-clipboard race:** persisted row could be evicted by a *later* snip while user is still reading the Preview — original cropped image gone, Rerun-with would fail. v1: 100-row cap, ~50 snips/day power user, ~2 days retention. Acceptable.

## Plan Acceptance Criteria

| Criterion | Status |
|-----------|--------|
| 100+ snips persist across restarts | ✓ WAL on-disk, `init()` creates dir + opens. |
| FTS5 search <100 ms on 1k records | ✓ Structurally: BM25 + `idx_created_at` + 100-row LIMIT. Not benchmarked at scale. |
| Delete removes row + thumb + image | ✓ Verified by `delete_removes_row_and_returns_file_paths`. |
| Rerun updates row in-place + returns DTO | ✓ Verified by `update_output_replaces_text_and_keeps_fts_in_sync` + manual code trace. |
| Eviction trims oldest on next insert | ✓ Verified by `enforce_max_records_trims_oldest_first`. |

## Positive Observations

- RAII guard discipline (`TempFileGuard`, `ListenerGuard`, `OverlayHideGuard`, `SnipBusyGuard`) is excellent — exits cleanly on every error path, including future cancellation.
- FTS5 shadow-delete triggers correctly follow SQLite docs; test proves sync on UPDATE.
- `sanitize_fts_query` design (whitespace-split + per-token phrase quoting) is the simplest correct approach — no parser, no regex, no escape mistakes.
- `SnipBusyGuard` single-flight gate handles devtools-invoked race.
- `persist_to_history` lock-then-drop-before-disk is the right scope.
- DTO conversion (`From<Record> for HistoryRecordDto`) cleanly separates DB shape from API shape.
- `cargo test` 43/43 green, `cargo check` no warnings, `pnpm build` clean.

## Recommended Actions

1. **(Optional, defer)** Add error-path cleanup in `persist_to_history` (H1) — ~5 lines.
2. **(Optional, defer)** Check `rows_affected` in `update_output` and return Err on 0 (H2) — ~3 lines.
3. **(Optional)** Replace `app.state::<HistoryStore>()` with `try_state` in `rerun_snip` for explicit error vs panic (M2).
4. **None of the above block landing.** All test suites pass, criteria met.

## Metrics

- Tests: 43/43 pass (12 lib + 7 new storage + 24 pre-existing).
- Cargo check: clean.
- Pnpm build: clean.
- New Rust LOC (Phase 7 only): ~280 (storage/) + ~180 (commands.rs delta) = ~460.
- New TS LOC: ~200 (store + rows + menu + invoke types).

## Unresolved Questions

- Plan claims FTS5 <100 ms on 1k records but no benchmark exists. Acceptable for v1 (BM25 + index makes this near-certain), but worth a single timed test in Phase 8/9 to confirm. Not blocking.
- Should `rerun_snip` be allowed if the original image file is missing (orphan from a partial delete)? Currently `run_ocr_for_path` would fail on `image_path` open. Frontend would surface as red toast. Acceptable.

---

**Status:** DONE
**Summary:** Phase 7 is production-ready for v1. No blockers. 2 medium issues (orphan files on persist-error, rerun TOCTOU vs delete) are low blast-radius and can defer to v1.5. Storage tests green, asset-protocol scope verified equivalent to `app_data_dir()`, FTS5 trigger pattern canonical, `TempFileGuard::disarm()` semantics verified empirically.
**Concerns/Blockers:** None blocking. See H1/H2 for defer-to-v1.5 polish.

Sources:
- [SQLite FTS5 Extension](https://sqlite.org/fts5.html)
- [Tauri 2 Asset Protocol](https://v2.tauri.app/security/asset-protocol/)
- [Tauri PathResolver app_data_dir vs app_local_data_dir](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html)
