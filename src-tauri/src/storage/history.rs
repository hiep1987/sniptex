//! CRUD + FTS5 search over `snip_records`.
//!
//! All public functions are called with a `&Connection` borrowed from
//! the Tauri-managed `Mutex<Connection>`; callers handle locking.

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Row};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct NewRecord {
    pub uuid: String,
    pub created_at: i64,
    pub agent_id: String,
    pub output_text: String,
    pub detected_type: String,
    pub image_path: String,
    pub thumb_path: String,
    pub latency_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub id: i64,
    pub uuid: String,
    pub created_at: i64,
    pub agent_id: String,
    pub output_text: String,
    pub detected_type: String,
    pub image_path: String,
    pub thumb_path: String,
    pub latency_ms: i64,
}

pub fn insert(conn: &Connection, rec: &NewRecord) -> SqlResult<i64> {
    conn.execute(
        "INSERT INTO snip_records
            (uuid, created_at, agent_id, output_text,
             detected_type, image_path, thumb_path, latency_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            rec.uuid,
            rec.created_at,
            rec.agent_id,
            rec.output_text,
            rec.detected_type,
            rec.image_path,
            rec.thumb_path,
            rec.latency_ms,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn recent(conn: &Connection, limit: usize) -> SqlResult<Vec<Record>> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, created_at, agent_id, output_text,
                detected_type, image_path, thumb_path, latency_ms
         FROM snip_records
         ORDER BY created_at DESC, id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], row_to_record)?;
    rows.collect()
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> SqlResult<Vec<Record>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return recent(conn, limit);
    }
    let fts_query = sanitize_fts_query(trimmed);

    // JOIN the FTS table to the content table; ORDER BY rank gives us
    // BM25 relevance (newer rows tied by id desc).
    let mut stmt = conn.prepare(
        "SELECT r.id, r.uuid, r.created_at, r.agent_id, r.output_text,
                r.detected_type, r.image_path, r.thumb_path, r.latency_ms
         FROM snip_records_fts f
         JOIN snip_records r ON r.id = f.rowid
         WHERE snip_records_fts MATCH ?1
         ORDER BY rank, r.created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![fts_query, limit as i64], row_to_record)?;
    rows.collect()
}

pub fn find_by_id(conn: &Connection, id: i64) -> SqlResult<Option<Record>> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, created_at, agent_id, output_text,
                detected_type, image_path, thumb_path, latency_ms
         FROM snip_records WHERE id = ?1",
    )?;
    stmt.query_row(params![id], row_to_record).optional()
}

pub fn update_output(
    conn: &Connection,
    id: i64,
    new_text: &str,
    new_agent: &str,
    new_detected: &str,
    new_latency_ms: i64,
) -> SqlResult<usize> {
    conn.execute(
        "UPDATE snip_records
         SET output_text = ?1,
             agent_id = ?2,
             detected_type = ?3,
             latency_ms = ?4
         WHERE id = ?5",
        params![new_text, new_agent, new_detected, new_latency_ms, id],
    )
}

pub fn delete(conn: &Connection, id: i64) -> SqlResult<Option<(String, String)>> {
    let mut stmt = conn.prepare("SELECT image_path, thumb_path FROM snip_records WHERE id = ?1")?;
    let paths: Option<(String, String)> = stmt
        .query_row(params![id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?;
    conn.execute("DELETE FROM snip_records WHERE id = ?1", params![id])?;
    Ok(paths)
}

/// Trim oldest rows past `max`. Returns the (image_path, thumb_path) of
/// every evicted record so the caller can clean up disk files.
pub fn enforce_max_records(conn: &Connection, max: usize) -> SqlResult<Vec<(String, String)>> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM snip_records", [], |r| r.get(0))?;
    let max_i = max as i64;
    if total <= max_i {
        return Ok(Vec::new());
    }
    let to_remove = total - max_i;

    let mut select = conn.prepare(
        "SELECT id, image_path, thumb_path FROM snip_records
         ORDER BY created_at ASC, id ASC LIMIT ?1",
    )?;
    let victims: Vec<(i64, String, String)> = select
        .query_map(params![to_remove], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    let mut delete_stmt = conn.prepare("DELETE FROM snip_records WHERE id = ?1")?;
    let mut paths = Vec::with_capacity(victims.len());
    for (id, img, thumb) in victims {
        delete_stmt.execute(params![id])?;
        paths.push((img, thumb));
    }
    Ok(paths)
}

fn row_to_record(row: &Row<'_>) -> SqlResult<Record> {
    Ok(Record {
        id: row.get(0)?,
        uuid: row.get(1)?,
        created_at: row.get(2)?,
        agent_id: row.get(3)?,
        output_text: row.get(4)?,
        detected_type: row.get(5)?,
        image_path: row.get(6)?,
        thumb_path: row.get(7)?,
        latency_ms: row.get(8)?,
    })
}

/// Quote every whitespace-separated term as an FTS5 "phrase" so user
/// input cannot inject MATCH operators (`NEAR`, `*`, `^`, `OR`, etc.).
/// Embedded `"` characters are doubled per the FTS5 grammar.
fn sanitize_fts_query(raw: &str) -> String {
    raw.split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let escaped = t.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_quotes_terms_and_escapes_inner_quotes() {
        assert_eq!(sanitize_fts_query("foo bar"), "\"foo\" \"bar\"");
        assert_eq!(sanitize_fts_query("  foo   bar  "), "\"foo\" \"bar\"");
        assert_eq!(sanitize_fts_query("a\"b"), "\"a\"\"b\"");
        assert_eq!(sanitize_fts_query("NEAR(a b)"), "\"NEAR(a\" \"b)\"");
    }
}
