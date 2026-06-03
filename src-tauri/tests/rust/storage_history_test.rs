use std::fs;

use sniptex_lib::storage::{self, history};

fn tmp_dir(suffix: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "sniptex-test-{}-{}-{}",
        suffix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn new_record(uuid: &str, ts: i64, text: &str) -> history::NewRecord {
    history::NewRecord {
        uuid: uuid.to_string(),
        created_at: ts,
        agent_id: "codex".into(),
        output_text: text.into(),
        detected_type: "EQUATION_ONLY".into(),
        image_path: format!("/tmp/{uuid}.png"),
        thumb_path: format!("/tmp/{uuid}.webp"),
        latency_ms: 1234,
    }
}

#[test]
fn init_runs_migration_creates_tables() {
    let dir = tmp_dir("init");
    let store = storage::init(&dir).expect("init");
    let conn = store.conn.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('snip_records','snip_records_fts')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 1);
}

#[test]
fn insert_then_recent_roundtrips_rows_in_descending_order() {
    let dir = tmp_dir("recent");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    history::insert(&conn, &new_record("a", 100, "alpha integral")).unwrap();
    history::insert(&conn, &new_record("b", 200, "beta integral")).unwrap();
    history::insert(&conn, &new_record("c", 300, "gamma table")).unwrap();
    let rows = history::recent(&conn, 10).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].uuid, "c");
    assert_eq!(rows[1].uuid, "b");
    assert_eq!(rows[2].uuid, "a");
}

#[test]
fn search_matches_via_fts5_and_ignores_unmatched_terms() {
    let dir = tmp_dir("search");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    history::insert(&conn, &new_record("a", 100, "alpha integral")).unwrap();
    history::insert(&conn, &new_record("b", 200, "beta sum")).unwrap();
    history::insert(&conn, &new_record("c", 300, "gamma table data")).unwrap();

    let hits = history::search(&conn, "integral", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].uuid, "a");

    let hits = history::search(&conn, "TABLE", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].uuid, "c");

    let hits = history::search(&conn, "doesnotexist", 10).unwrap();
    assert_eq!(hits.len(), 0);
}

#[test]
fn search_with_special_chars_does_not_crash_fts_parser() {
    let dir = tmp_dir("search-special");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    history::insert(&conn, &new_record("a", 100, "frac x y")).unwrap();
    // FTS5 MATCH operators that would otherwise blow up: NEAR, ^, *, AND
    let raw_queries = ["NEAR(a b)", "frac AND y", "a^b", "*literal", "\"quotes\""];
    for q in raw_queries {
        history::search(&conn, q, 10).expect("query should not error");
    }
}

#[test]
fn delete_removes_row_and_returns_file_paths() {
    let dir = tmp_dir("delete");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    let id = history::insert(&conn, &new_record("a", 100, "alpha")).unwrap();
    let removed = history::delete(&conn, id).unwrap();
    assert!(removed.is_some());
    let (img, thumb) = removed.unwrap();
    assert_eq!(img, "/tmp/a.png");
    assert_eq!(thumb, "/tmp/a.webp");
    assert_eq!(history::recent(&conn, 10).unwrap().len(), 0);
}

#[test]
fn update_output_replaces_text_and_keeps_fts_in_sync() {
    let dir = tmp_dir("update");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    let id = history::insert(&conn, &new_record("a", 100, "old phrase")).unwrap();

    let updated =
        history::update_output(&conn, id, "fresh phrase", "gemini-cli", "MIXED", 999).unwrap();
    assert_eq!(updated, 1);

    let missing = history::update_output(&conn, 99999, "x", "y", "MIXED", 1).unwrap();
    assert_eq!(missing, 0);

    let rec = history::find_by_id(&conn, id).unwrap().unwrap();
    assert_eq!(rec.output_text, "fresh phrase");
    assert_eq!(rec.agent_id, "gemini-cli");
    assert_eq!(rec.detected_type, "MIXED");
    assert_eq!(rec.latency_ms, 999);

    // FTS should match the new term but not the old one.
    assert_eq!(history::search(&conn, "fresh", 10).unwrap().len(), 1);
    assert_eq!(history::search(&conn, "old", 10).unwrap().len(), 0);
}

#[test]
fn enforce_max_records_trims_oldest_first() {
    let dir = tmp_dir("evict");
    let store = storage::init(&dir).unwrap();
    let conn = store.conn.lock().unwrap();
    for i in 0..5 {
        history::insert(&conn, &new_record(&format!("u{i}"), i, &format!("t{i}"))).unwrap();
    }
    let victims = history::enforce_max_records(&conn, 3).unwrap();
    assert_eq!(victims.len(), 2);
    let remaining = history::recent(&conn, 10).unwrap();
    assert_eq!(remaining.len(), 3);
    // Oldest two (u0, u1) should be gone; u2/u3/u4 remain.
    let uuids: Vec<_> = remaining.iter().map(|r| r.uuid.clone()).collect();
    assert!(uuids.contains(&"u4".into()));
    assert!(uuids.contains(&"u3".into()));
    assert!(uuids.contains(&"u2".into()));
}
