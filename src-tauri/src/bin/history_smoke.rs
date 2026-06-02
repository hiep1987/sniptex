//! Standalone smoke test for the Phase 7 history pipeline.
//!
//! Boots the SQLite store under a temp dir, seeds 5 fake snips with
//! varying detected_type + agent_id, runs the full read path (recent /
//! search / find_by_id), then exercises delete + eviction. Exits 0 on
//! success, non-zero with a diagnostic on failure.
//!
//! Skip the live UI parts (capture overlay, MathJax) so this can run
//! headless on CI.

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use sniptex_lib::storage::{self, history};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn step(name: &str, ok: bool, detail: impl ToString) -> bool {
    let prefix = if ok { "  ok  " } else { " FAIL " };
    println!("{prefix} {name}: {}", detail.to_string());
    ok
}

fn main() -> ExitCode {
    let tmp = std::env::temp_dir().join(format!("sniptex-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let store = match storage::init(&tmp) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("init failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("➜ storage initialised under {}", tmp.display());

    let conn = store.conn.lock().unwrap();
    let mut all_ok = true;

    // Seed 5 records with distinct text + agent + type.
    let fixtures = [
        ("eq-1", "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}", "EQUATION_ONLY", "codex"),
        ("eq-2", "\\int_0^1 x^2 dx", "EQUATION_ONLY", "gemini-cli"),
        ("tab-1", "| a | b |\n|---|---|\n| 1 | 2 |", "TABLE_ONLY", "codex"),
        ("mix-1", "Equation E=mc^2 explains relativity", "MIXED", "cloud-gemini"),
        ("mix-2", "Vận tốc là độ biến thiên vị trí", "MIXED", "codex"),
    ];
    let mut ids = Vec::new();
    for (i, (uuid, text, dtype, agent)) in fixtures.iter().enumerate() {
        let rec = history::NewRecord {
            uuid: (*uuid).to_string(),
            created_at: now() + i as i64,
            agent_id: (*agent).to_string(),
            via_agent_id: None,
            output_text: (*text).to_string(),
            detected_type: (*dtype).to_string(),
            image_path: format!("/dev/null/{uuid}.png"),
            thumb_path: format!("/dev/null/{uuid}.webp"),
            latency_ms: 1234 + (i as i64) * 50,
        };
        let id = history::insert(&conn, &rec).expect("insert");
        ids.push(id);
    }
    all_ok &= step("seed 5 records", ids.len() == 5, format!("ids = {:?}", ids));

    // recent: newest first
    let recent = history::recent(&conn, 10).unwrap();
    all_ok &= step(
        "recent() returns newest-first",
        recent.len() == 5 && recent[0].uuid == "mix-2",
        format!("first uuid = {}, count = {}", recent[0].uuid, recent.len()),
    );

    // FTS: Vietnamese diacritic-insensitive match (unicode61 remove_diacritics 2)
    let viet = history::search(&conn, "van toc", 10).unwrap();
    all_ok &= step(
        "FTS Vietnamese diacritic-insensitive",
        viet.iter().any(|r| r.uuid == "mix-2"),
        format!("hits = {:?}", viet.iter().map(|r| r.uuid.clone()).collect::<Vec<_>>()),
    );

    // FTS: latex command
    let latex = history::search(&conn, "frac", 10).unwrap();
    all_ok &= step(
        "FTS finds \\frac",
        latex.iter().any(|r| r.uuid == "eq-1"),
        format!("hits = {:?}", latex.iter().map(|r| r.uuid.clone()).collect::<Vec<_>>()),
    );

    // FTS: special chars don't blow up the parser
    let _ = history::search(&conn, "NEAR(a b) ^*\"q\"", 10).expect("special chars survive");
    all_ok &= step("FTS special chars don't crash", true, "ok");

    // Update: rerun-style in-place mutation; FTS resync
    let target = ids[0];
    let updated = history::update_output(
        &conn,
        target,
        "y = mx + b",
        "gemini-cli",
        None,
        "EQUATION_ONLY",
        9999,
    )
    .unwrap();
    let after = history::find_by_id(&conn, target).unwrap().unwrap();
    all_ok &= step(
        "update_output rewrites row + FTS",
        updated == 1
            && after.output_text == "y = mx + b"
            && after.agent_id == "gemini-cli"
            && history::search(&conn, "mx", 10).unwrap().len() == 1
            && history::search(&conn, "frac", 10).unwrap().is_empty(),
        format!("rows_updated = {updated}, agent = {}", after.agent_id),
    );

    // Delete: returns paths + drops row
    let deleted = history::delete(&conn, target).unwrap();
    let remaining = history::recent(&conn, 10).unwrap();
    all_ok &= step(
        "delete drops row + returns paths",
        deleted.is_some() && remaining.iter().all(|r| r.id != target),
        format!("paths = {:?}, remaining = {}", deleted, remaining.len()),
    );

    // Eviction: shrink the kept window to 2
    let victims = history::enforce_max_records(&conn, 2).unwrap();
    let after_evict = history::recent(&conn, 10).unwrap();
    all_ok &= step(
        "enforce_max_records trims oldest",
        victims.len() == 2 && after_evict.len() == 2,
        format!("evicted = {}, remaining = {}", victims.len(), after_evict.len()),
    );

    drop(conn);
    // Final state recap so the user sees the surviving rows.
    let final_conn = store.conn.lock().unwrap();
    let final_rows = history::recent(&final_conn, 10).unwrap();
    println!("\n➜ final history snapshot:");
    for r in &final_rows {
        println!(
            "    [{id:>4}] {ts:>10} {agent:<12} {dtype:<14} → {text}",
            id = r.id,
            ts = r.created_at,
            agent = r.agent_id,
            dtype = r.detected_type,
            text = r.output_text.lines().next().unwrap_or("").chars().take(60).collect::<String>(),
        );
    }
    drop(final_conn);

    let _ = std::fs::remove_dir_all(&tmp);
    if all_ok {
        println!("\n✓ Phase 7 history smoke test PASSED");
        ExitCode::SUCCESS
    } else {
        println!("\n✗ Phase 7 history smoke test FAILED");
        ExitCode::FAILURE
    }
}
