//! SQLite schema migrations. Versioning is tracked via `PRAGMA user_version`
//! so we don't need an external metadata table.
//!
//! V1 adds:
//!   * `snip_records` — one row per successful OCR snip.
//!   * `snip_records_fts` — external-content FTS5 virtual table over `output_text`.
//!   * Triggers to keep FTS in sync (insert + delete + update).

use rusqlite::{Connection, Result as SqlResult};

const CURRENT_VERSION: i32 = 1;

pub fn run(conn: &mut Connection) -> SqlResult<()> {
    let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version >= CURRENT_VERSION {
        return Ok(());
    }

    let tx = conn.transaction()?;
    if version < 1 {
        apply_v1(&tx)?;
    }
    tx.pragma_update(None, "user_version", CURRENT_VERSION)?;
    tx.commit()?;
    Ok(())
}

fn apply_v1(tx: &rusqlite::Transaction<'_>) -> SqlResult<()> {
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS snip_records (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid          TEXT    NOT NULL UNIQUE,
            created_at    INTEGER NOT NULL,
            agent_id      TEXT    NOT NULL,
            output_text   TEXT    NOT NULL,
            detected_type TEXT    NOT NULL,
            image_path    TEXT    NOT NULL,
            thumb_path    TEXT    NOT NULL,
            latency_ms    INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_snip_records_created_at
            ON snip_records (created_at DESC);

        CREATE VIRTUAL TABLE IF NOT EXISTS snip_records_fts USING fts5(
            output_text,
            content='snip_records',
            content_rowid='id',
            tokenize='unicode61 remove_diacritics 2'
        );

        CREATE TRIGGER IF NOT EXISTS snip_records_ai
        AFTER INSERT ON snip_records BEGIN
            INSERT INTO snip_records_fts(rowid, output_text)
            VALUES (new.id, new.output_text);
        END;

        CREATE TRIGGER IF NOT EXISTS snip_records_ad
        AFTER DELETE ON snip_records BEGIN
            INSERT INTO snip_records_fts(snip_records_fts, rowid, output_text)
            VALUES ('delete', old.id, old.output_text);
        END;

        CREATE TRIGGER IF NOT EXISTS snip_records_au
        AFTER UPDATE OF output_text ON snip_records BEGIN
            INSERT INTO snip_records_fts(snip_records_fts, rowid, output_text)
            VALUES ('delete', old.id, old.output_text);
            INSERT INTO snip_records_fts(rowid, output_text)
            VALUES (new.id, new.output_text);
        END;
        "#,
    )?;
    Ok(())
}
