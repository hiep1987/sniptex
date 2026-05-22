use std::sync::Mutex;

use serde::Serialize;
use tauri::tray::TrayIcon;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayStatus {
    Idle,
    Capturing,
    Processing,
    Error,
}

impl TrayStatus {
    pub fn tooltip(self) -> &'static str {
        match self {
            TrayStatus::Idle => "SnipTeX — idle (Cmd/Ctrl+Shift+M to snip)",
            TrayStatus::Capturing => "SnipTeX — drag to select region",
            TrayStatus::Processing => "SnipTeX — running OCR…",
            TrayStatus::Error => "SnipTeX — last snip failed",
        }
    }
}

/// Process-wide state managed by Tauri. Holds the tray handle (so any
/// command can swap its icon) and the current visible status.
///
/// `tray_handle` is initialized once in `tray::init_tray`. After that,
/// `tray::set_status` can read it from any thread.
pub struct AppState {
    pub current_status: Mutex<TrayStatus>,
    pub tray_handle: Mutex<Option<TrayIcon>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_status: Mutex::new(TrayStatus::Idle),
            tray_handle: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
