//! SQLite-backed snip history. Owns the connection pool, on-disk layout,
//! and CRUD façade used by Tauri commands.
//!
//! Layout under `{app_data_dir}`:
//!   sniptex.sqlite   — connection, WAL mode
//!   images/{uuid}.png — original cropped snip (kept for "rerun")
//!   thumbs/{uuid}.webp — 200×200 thumbnail for the history list

pub mod history;
pub mod migrations;
pub mod thumbnail;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use thiserror::Error;

pub use history::{NewRecord, Record};

const DB_FILENAME: &str = "sniptex.sqlite";
const IMAGES_SUBDIR: &str = "images";
const THUMBS_SUBDIR: &str = "thumbs";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("create dir {0}: {1}")]
    CreateDir(PathBuf, String),
    #[error("open db: {0}")]
    OpenDb(String),
    #[error("migrate: {0}")]
    Migrate(String),
    #[error("pragma: {0}")]
    Pragma(String),
}

/// Wraps the live `rusqlite::Connection` behind a `Mutex` so it can be
/// registered as Tauri-managed state and reused across commands.
pub struct HistoryStore {
    pub conn: Mutex<Connection>,
    pub app_data_dir: PathBuf,
}

impl HistoryStore {
    pub fn images_dir(&self) -> PathBuf {
        self.app_data_dir.join(IMAGES_SUBDIR)
    }
    pub fn thumbs_dir(&self) -> PathBuf {
        self.app_data_dir.join(THUMBS_SUBDIR)
    }
}

/// Initialize on-disk layout + open + migrate. Returns a `HistoryStore`
/// ready to be passed to `app.manage(...)`.
pub fn init(app_data_dir: &Path) -> Result<HistoryStore, StorageError> {
    ensure_dir(app_data_dir)?;
    ensure_dir(&app_data_dir.join(IMAGES_SUBDIR))?;
    ensure_dir(&app_data_dir.join(THUMBS_SUBDIR))?;

    let db_path = app_data_dir.join(DB_FILENAME);
    let mut conn =
        Connection::open(&db_path).map_err(|e| StorageError::OpenDb(e.to_string()))?;

    // WAL gives us crash-resilient concurrent reads (UI search) while a
    // write (snip insert) is in flight. NORMAL synchronous matches SQLite's
    // recommended WAL pairing — fsync per checkpoint, not per commit.
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| StorageError::Pragma(e.to_string()))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| StorageError::Pragma(e.to_string()))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| StorageError::Pragma(e.to_string()))?;

    migrations::run(&mut conn).map_err(|e| StorageError::Migrate(e.to_string()))?;

    Ok(HistoryStore {
        conn: Mutex::new(conn),
        app_data_dir: app_data_dir.to_path_buf(),
    })
}

fn ensure_dir(p: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(p)
        .map_err(|e| StorageError::CreateDir(p.to_path_buf(), e.to_string()))
}

/// Best-effort cleanup helper used by `delete` / eviction. Errors are
/// swallowed because the DB row is already gone — orphaned files are a
/// disk-space concern, not a correctness one, and the on-startup orphan
/// scan picks them up.
pub fn remove_file_if_exists(p: &str) {
    let _ = std::fs::remove_file(p);
}
