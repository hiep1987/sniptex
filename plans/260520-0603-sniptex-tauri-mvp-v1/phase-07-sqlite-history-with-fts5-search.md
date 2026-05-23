---
phase: 7
title: "SQLite History with FTS5 Search"
status: in-progress
priority: P2
effort: "1d"
dependencies: [6]
---

# Phase 7: SQLite History with FTS5 Search

## Overview

Persist every successful snip to a local SQLite database with thumbnail, raw output, detected type, agent used, and timestamp. Expose full-text search via FTS5 virtual table. Wire HistoryWindow UI to display records, search, rerun with a different agent, copy, and delete.

## Key Insights

- `tauri-plugin-sql` supports SQLite; can also use `rusqlite` directly for FTS5 setup ergonomics. Pick **rusqlite directly** in Rust + expose history commands; the plugin's main use is from frontend, but we own all DB code in Rust.
- FTS5 ships with bundled SQLite in `rusqlite` if `feature = ["bundled"]` enabled.
- Thumbnails: store small WebP (≤32KB each) inline OR file path. v1: file path under `{app_data_dir}/thumbs/{record_id}.webp` to keep DB small.
- "Last 100" is the default; user can change to 50/500/unlimited in Settings (Phase 8). Eviction policy on insert.

## Requirements

**Functional**
- On successful snip → insert record (id, ts, agent_id, output_text, detected_type, thumb_path, image_path)
- FTS5 search across `output_text`
- History Window lists records (newest first), supports search, right-click menu (Rerun with..., Delete, Export, Copy)
- "Rerun with..." submenu shows installed agents; spawns OCR on the saved original image with chosen agent

**Non-functional**
- Search returns within 100ms for 10k records
- Thumbnail generation <100ms per snip

## Architecture

```
{app_data_dir}/
├── sniptex.sqlite              SQLite database
├── images/                     Original snip PNGs (kept for rerun)
│   └── {uuid}.png
└── thumbs/                     200x200 WebP thumbnails
    └── {uuid}.webp
```

Schema:

```sql
CREATE TABLE snip_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  uuid TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,        -- unix epoch seconds
  agent_id TEXT NOT NULL,
  output_text TEXT NOT NULL,
  detected_type TEXT NOT NULL,        -- 'EquationOnly' | 'TableOnly' | 'Mixed'
  image_path TEXT NOT NULL,
  thumb_path TEXT NOT NULL,
  latency_ms INTEGER NOT NULL
);

CREATE VIRTUAL TABLE snip_records_fts USING fts5(
  output_text, content='snip_records', content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER snip_records_ai AFTER INSERT ON snip_records BEGIN
  INSERT INTO snip_records_fts(rowid, output_text) VALUES (new.id, new.output_text);
END;
CREATE TRIGGER snip_records_ad AFTER DELETE ON snip_records BEGIN
  INSERT INTO snip_records_fts(snip_records_fts, rowid, output_text)
    VALUES('delete', old.id, old.output_text);
END;
```

## Related Code Files

- Create: `src-tauri/src/storage/{mod,history,thumbnail,migrations}.rs`
- Modify: `src-tauri/Cargo.toml` — `rusqlite = { version = "0.32", features = ["bundled"] }`, `webp = "0.3"`
- Modify: `src-tauri/src/commands.rs` — add history commands
- Modify: `src/windows/HistoryWindow.tsx` — wire real data
- Modify: `src/stores/historyStore.ts` — fetch + state management
- Create: `src/components/HistoryRow.tsx`, `src/components/RerunMenu.tsx`

## Implementation Steps

1. Add `rusqlite` + `webp` to Cargo. Create `storage/migrations.rs` with V1 schema (table + FTS5 virtual + triggers).
2. Implement `storage/mod.rs::init_db(app_data_dir)`:
   - Open connection at `{app_data_dir}/sniptex.sqlite`
   - Run pending migrations
   - Return connection wrapped in `Mutex<Connection>` managed by Tauri state
3. Implement `storage/thumbnail.rs::make_thumbnail(src_png, dst_webp)`:
   - Load PNG via `image` crate, resize to 200x200 keeping aspect ratio, encode WebP at quality 80
4. Implement `storage/history.rs`:
   - `insert(record: NewRecord) -> Result<i64>` — INSERT row, return new id
   - `recent(limit: usize) -> Vec<Record>`
   - `search(query: &str, limit: usize) -> Vec<Record>` — JOIN against FTS5 with MATCH
   - `delete(id: i64) -> Result<()>` — also delete thumb + image files
   - `enforce_max_records(max: usize)` — DELETE oldest beyond limit
5. Wire into `trigger_snip` flow (Phase 5):
   - After OCR success, copy temp PNG into `images/{uuid}.png`, make thumb, insert record, enforce max
6. Expose Tauri commands:
   - `get_history(limit: usize) -> Vec<Record>`
   - `search_history(query: String, limit: usize) -> Vec<Record>`
   - `delete_record(id: i64)`
   - `rerun_snip(record_id: i64, agent_id: String) -> SnipResult` — fetch image_path, call dispatcher with chosen agent
   - `export_record(id: i64, format: ExportFormat) -> String` — emit text in chosen format
7. Wire `historyStore.ts`:
   - On HistoryWindow mount → `getHistory(100)`
   - On search input change (debounced 200ms) → `searchHistory(query, 100)`
   - On delete → call backend + remove from local state
8. Build `HistoryRow.tsx`: thumbnail + truncated text preview + relative timestamp + agent badge + context menu trigger.
9. Build `RerunMenu.tsx`: list installed agents, on click call `rerunSnip`, replace row's output in store.
10. Smoke test: do 5 snips → all appear in History → search filters correctly → delete works → rerun with different agent updates row.

## Todo List

- [x] Add rusqlite (bundled) + webp crates
- [x] Write V1 migration with snip_records + FTS5 virtual + triggers
- [x] Implement init_db with managed connection (WAL + foreign_keys)
- [x] Implement thumbnail generator (200×200 WebP, Lanczos3)
- [x] Implement history CRUD + search (BM25 ranking)
- [x] Wire history insert into post-OCR pipeline (`persist_to_history`)
- [x] Implement enforce_max_records eviction (oldest-first, returns paths)
- [x] Expose 5 Tauri commands (get/search/delete/rerun/export)
- [x] Wire historyStore.ts with fetch + debounced search (200 ms)
- [x] Build HistoryRow + RerunMenu components
- [x] 7 Rust integration tests covering insert/search/delete/update/eviction/FTS escape
- [ ] Live smoke test in the running app: insert, search, delete, rerun

## Success Criteria

- [ ] 100+ snips persist across app restarts
- [ ] FTS5 search returns matching records <100ms (verified on seeded 1k records)
- [ ] Delete removes row + thumb + image files
- [ ] Rerun with different agent produces new text; UI updates without manual refresh
- [ ] Eviction policy trims to user-set max on next insert

## Risk Assessment

- **Risk: SQLite file corruption on power loss** — Mitigation: WAL mode (`PRAGMA journal_mode = WAL`); accept rare corruption as users can delete + re-init.
- **Risk: Thumb directory grows unbounded if records deleted without cleanup** — Mitigation: cleanup runs in `delete` command + on app start (orphan scan).
- **Risk: FTS5 not compiled in some rusqlite builds** — Mitigation: `bundled` feature ensures bundled SQLite includes FTS5.

## Security Considerations

- DB stored under app data dir with user-level permissions; no encryption v1 (local-only, low risk).
- Search query passed to FTS5 MATCH must be escaped (`"query"` quoting) to prevent special-char syntax errors.

## Next Steps

- Phase 8 (Settings) exposes history size + clear history button
- Phase 9 (Format toggle) adds Copy-as-format options from History row context menu

## Open Questions

- None remaining after Validation Session 1 (keep-forever retention with user-driven cleanup confirmed). Settings exposes "Clear history" + per-row "Delete".

## Implementation Notes (2026-05-23)

- DB lives at `{app_data_dir}/sniptex.sqlite`; images under `images/`, thumbs under `thumbs/`. WAL + NORMAL synchronous.
- `DetectedType` Rust enum got `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` to match the TS shape already used by Phase 6.
- Asset protocol scope extended in `tauri.conf.json` to include `$APPDATA/images/**` and `$APPDATA/thumbs/**` so the webview can load thumbnails via `convertFileSrc`.
- 100-record cap is hard-coded as `DEFAULT_MAX_RECORDS` in `commands.rs`; Phase 8 wires the Settings slider into this constant.
- Code review (reviewer-phase-07-sqlite-history.md) flagged 2 H-level polish items; both addressed before commit:
  - `persist_to_history` now rolls back the persisted PNG + thumb if the DB insert fails.
  - `update_output` returns rowcount; `rerun_snip` errors when zero rows match (TOCTOU with a concurrent `delete_record`).

<!-- Updated: Validation Session 1 - keep-forever image retention -->
<!-- Updated: 2026-05-23 - Implementation complete; live smoke test pending -->
