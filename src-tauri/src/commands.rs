use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::future::try_join_all;
use tokio::sync::Semaphore;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, EventId, Emitter, Listener, LogicalPosition, LogicalSize, Manager, State,
    WebviewWindow,
};
#[cfg(desktop)]
use tauri_plugin_autostart::ManagerExt;
use uuid::Uuid;

use crate::agents::{
    self, keychain,
    registry::{AgentInfo, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID, GEMINI_CLI_ID},
};
use crate::capture::{
    capture_active_monitor, crop_region_to_temp_png, CaptureError, CropError, MonitorSnapshot,
    SelectionRect,
};
use crate::ocr::{self, dispatcher::DispatchError, smart_format::DetectedType};
use crate::settings::{AppSettings, SettingsPatch, SettingsStore};
use crate::state::TrayStatus;
use crate::storage::{self, history as history_repo, HistoryStore};
use crate::tray;

// Keep ~last 100 records on disk by default. v1 has no Settings hook for
// this yet (Phase 8 wires the slider); change in one place to adjust.
const DEFAULT_MAX_RECORDS: usize = 100;

const OVERLAY_WINDOW_LABEL: &str = "overlay";
const CAPTURE_START_EVENT: &str = "capture-start";
const CAPTURE_REGION_EVENT: &str = "capture-region";
const CAPTURE_CANCEL_EVENT: &str = "capture-cancel";
// User has up to 60s to drag a selection before we give up. Long enough
// to switch contexts, short enough to avoid stale overlays if something
// kills the frontend mid-flow.
const SELECTION_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
pub struct HelloReply {
    pub message: String,
    pub version: &'static str,
}

#[tauri::command]
pub fn hello(name: Option<String>) -> HelloReply {
    let who = name.unwrap_or_else(|| "world".to_string());
    HelloReply {
        message: format!("Hello, {who}! SnipTeX backend is alive."),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Show a Tauri window by label and bring it to focus. Used by tray
/// menu items and onboarding/settings cross-navigation.
#[tauri::command]
pub fn show_window(app: AppHandle, label: String) -> Result<(), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("window not found: {label}"))?;
    window.show().map_err(|e| format!("show {label}: {e}"))?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

/// Hide a Tauri window by label without destroying it. Preview window
/// uses this to disappear after auto-hide timer.
#[tauri::command]
pub fn hide_window(app: AppHandle, label: String) -> Result<(), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("window not found: {label}"))?;
    window.hide().map_err(|e| format!("hide {label}: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn detect_agents() -> Result<Vec<AgentInfo>, String> {
    tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_agent(agent_id: String, image_path: String) -> Result<TestAgentReport, String> {
    let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| e.to_string())?;
    let agent = installed
        .into_iter()
        .find(|a| a.spec.id == agent_id)
        .ok_or_else(|| format!("agent not installed: {agent_id}"))?;

    let text = ocr::run_ocr(&agent, &image_path)
        .await
        .map_err(stringify_dispatch_error)?;
    let detected = ocr::detect_type(&text);
    Ok(TestAgentReport {
        ok: true,
        detected,
        char_count: text.chars().count(),
        preview: preview(&text, 200),
    })
}

#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<(), String> {
    match provider.as_str() {
        "gemini" => keychain::set_gemini_api_key(&key).map_err(|e| e.to_string()),
        "mistral" => keychain::set_mistral_api_key(&key).map_err(|e| e.to_string()),
        "goclaw" => keychain::set_cloud_goclaw_api_key(&key).map_err(|e| e.to_string()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn has_api_key(provider: String) -> Result<bool, String> {
    match provider.as_str() {
        "gemini" => Ok(keychain::has_gemini_api_key()),
        "mistral" => Ok(keychain::has_mistral_api_key()),
        "goclaw" => Ok(keychain::has_cloud_goclaw_api_key()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    match provider.as_str() {
        "gemini" => keychain::delete(keychain::GEMINI_ACCOUNT).map_err(|e| e.to_string()),
        "mistral" => keychain::delete(keychain::MISTRAL_ACCOUNT).map_err(|e| e.to_string()),
        "goclaw" => {
            keychain::delete(keychain::CLOUD_GOCLAW_ACCOUNT).map_err(|e| e.to_string())
        }
        other => Err(format!("unsupported provider: {other}")),
    }
}

/// End-to-end snip: cursor-monitor screenshot → overlay drag-select →
/// crop → OCR (specific agent or fallback chain).
///
/// Returns `SnipResult` with `status` = `"ok" | "cancelled"`. On `ok`
/// the `text`, `detected`, and `agent` fields are populated; the cropped
/// PNG is moved into the persistent history dir (so "Rerun with…" can
/// re-OCR the same image with a different agent) and a `HistoryRecord`
/// row is inserted. On `cancelled` only the full-monitor PNG is cleaned
/// up.
#[tauri::command]
pub async fn run_snip(
    app: AppHandle,
    agent_id: Option<String>,
) -> Result<SnipResult, String> {
    // Rust-side re-entrancy guard: a second hotkey press while a snip is
    // in flight would race two overlay show()s on the single shared window.
    // The frontend has a ref-guard too, but devtools could call run_snip
    // directly — defense in depth.
    let _busy = SnipBusyGuard::try_acquire()
        .ok_or_else(|| "snip already in progress".to_string())?;

    // Hide every visible SnipTeX surface BEFORE the screenshot — otherwise
    // the main / settings / history window itself ends up in the captured
    // backdrop and the user can't drag-select what's behind it.
    //
    // On macOS we use `AppHandle::hide()` (whole-app NSApplication.hide:)
    // so the next app's window actually comes forward and macOS repaints
    // the screen properly before xcap reads the framebuffer. Per-window
    // hide doesn't deactivate the app and the compositor leaves the
    // newly-uncovered region stale.
    //
    // The 300 ms delay covers two things: the macOS deactivate animation
    // (~200 ms) and the next-app's window-server repaint. Empirically
    // 150 ms is too short on Apple Silicon under heavier load.
    #[cfg(target_os = "macos")]
    let _ = app.hide();
    let _vis_guard = CaptureVisibilityGuard::hide_user_windows(&app);
    tokio::time::sleep(Duration::from_millis(300)).await;

    tray::set_status(&app, TrayStatus::Capturing);
    let result = run_snip_inner(&app, agent_id).await;

    match &result {
        Ok(snip) if snip.status == "ok" => {
            tray::set_status(&app, TrayStatus::Idle);
            let _ = app.emit("snip-complete", snip.clone());
        }
        Ok(_cancelled) => {
            tray::set_status(&app, TrayStatus::Idle);
        }
        Err(err) => {
            let _ = app.emit("snip-error", err.clone());
            tray::flash_error(app.clone());
        }
    }

    result
}

async fn run_snip_inner(
    app: &AppHandle,
    agent_id: Option<String>,
) -> Result<SnipResult, String> {
    // `cursor_position` returns physical pixels scaled by the PRIMARY
    // monitor's DPI factor (tao at macos/util/mod.rs:107). xcap's
    // `Monitor::from_point` (and `CGGetDisplaysWithPoint` under it) uses
    // the global LOGICAL points coordinate space. Convert back so the
    // right monitor is picked on Retina + multi-monitor.
    let cursor = app
        .cursor_position()
        .map_err(|e| format!("cursor_position failed: {e}"))?;
    let primary_scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let cursor_x = (cursor.x / primary_scale).round() as i32;
    let cursor_y = (cursor.y / primary_scale).round() as i32;

    let snapshot = tokio::task::spawn_blocking(move || capture_active_monitor(cursor_x, cursor_y))
        .await
        .map_err(|e| format!("capture join failed: {e}"))?
        .map_err(stringify_capture_error)?;

    // RAII: the captured full-monitor PNG MUST be removed on every exit
    // path (success, cancel, crop fail, OCR fail, panic). Holding the
    // guard until end-of-function makes that mechanical.
    let _full_png_guard = TempFileGuard::new(snapshot.full_png_path.clone());

    let selection = show_overlay_and_await_selection(app, &snapshot).await?;

    let Some(sel) = selection else {
        return Ok(SnipResult {
            status: "cancelled".into(),
            text: None,
            detected: None,
            agent: None,
            image_path: None,
            record_id: None,
        });
    };

    // User has drawn a region — overlay has hidden via OverlayHideGuard
    // and we're now in the OCR phase. Flip the tray so the user can tell
    // the slow path (network/CLI) is in progress.
    tray::set_status(app, TrayStatus::Processing);

    let full_path = snapshot.full_png_path.clone();
    let scale = snapshot.scale_factor;
    let cropped_path = tokio::task::spawn_blocking(move || {
        crop_region_to_temp_png(&full_path, sel, scale)
    })
    .await
    .map_err(|e| format!("crop join failed: {e}"))?
    .map_err(stringify_crop_error)?;

    // Cropped PNG is held under a guard until we successfully move it
    // into the persistent history dir below; if OCR errors out the guard
    // ensures the temp file is still removed.
    let cropped_guard = TempFileGuard::new(cropped_path.clone());
    let cropped_str = cropped_path.to_string_lossy().to_string();

    let priority = app
        .try_state::<SettingsStore>()
        .map(|s| s.get().agent_priority)
        .unwrap_or_default();

    let ocr_started = Instant::now();
    let (text, agent) = run_ocr_for_path(agent_id, &cropped_str, &priority).await?;
    let latency_ms = ocr_started.elapsed().as_millis() as i64;
    let detected = ocr::detect_type(&text);

    // Move cropped PNG into history.images/{uuid}.png + generate thumb +
    // insert the row. Errors here are logged-and-swallowed so the user
    // still gets their OCR result; History just won't show this snip.
    let store = app.state::<HistoryStore>();
    let persisted = persist_to_history(&store, &cropped_path, &text, &agent, &detected, latency_ms);
    // Persistence took ownership of the temp file — disarm the guard so
    // it doesn't try to delete the now-renamed file.
    let (record_id, persisted_image_path) = match persisted {
        Ok((id, img)) => {
            cropped_guard.disarm();
            (Some(id), Some(img))
        }
        Err(err) => {
            log::error!("history persist failed: {err}");
            (None, Some(cropped_str.clone()))
        }
    };

    Ok(SnipResult {
        status: "ok".into(),
        text: Some(text),
        detected: Some(detected),
        agent: Some(agent),
        image_path: persisted_image_path,
        record_id,
    })
}

/// Move the temp cropped PNG into `{app_data}/images/{uuid}.png`, render
/// the 200×200 WebP thumb, insert a history row, and trim oldest records
/// past `DEFAULT_MAX_RECORDS`. Returns `(record_id, persistent image path)`.
fn persist_to_history(
    store: &State<'_, HistoryStore>,
    cropped_path: &Path,
    text: &str,
    agent_id: &str,
    detected: &DetectedType,
    latency_ms: i64,
) -> Result<(i64, String), String> {
    let uuid_str = Uuid::new_v4().to_string();
    let images_dir = store.images_dir();
    let thumbs_dir = store.thumbs_dir();

    let image_dst = images_dir.join(format!("{uuid_str}.png"));
    let thumb_dst = thumbs_dir.join(format!("{uuid_str}.webp"));

    // Rename when on the same filesystem (the temp PNG lives in TMPDIR,
    // history lives under app-data — often different volumes on Windows),
    // fall back to copy + remove otherwise.
    if std::fs::rename(cropped_path, &image_dst).is_err() {
        std::fs::copy(cropped_path, &image_dst)
            .map_err(|e| format!("copy cropped png: {e}"))?;
        let _ = std::fs::remove_file(cropped_path);
    }

    // From here on, the persisted image at `image_dst` is committed —
    // if any subsequent step (thumbnail / insert) fails, we must remove
    // it (and the partial thumb) ourselves; the temp-file guard above
    // already saw the rename steal its path.
    if let Err(e) = storage::thumbnail::make_thumbnail(&image_dst, &thumb_dst) {
        let _ = std::fs::remove_file(&image_dst);
        return Err(format!("thumbnail: {e}"));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let detected_str = detected_to_string(detected);
    let new_record = history_repo::NewRecord {
        uuid: uuid_str,
        created_at: now,
        agent_id: agent_id.to_string(),
        output_text: text.to_string(),
        detected_type: detected_str,
        image_path: image_dst.to_string_lossy().to_string(),
        thumb_path: thumb_dst.to_string_lossy().to_string(),
        latency_ms,
    };

    let new_id_res: Result<i64, String> = (|| {
        let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
        let id = history_repo::insert(&conn, &new_record).map_err(|e| e.to_string())?;
        let evicted = history_repo::enforce_max_records(&conn, DEFAULT_MAX_RECORDS)
            .map_err(|e| e.to_string())?;
        // Drop the connection lock before touching disk so eviction
        // cleanup can't deadlock against a concurrent reader.
        drop(conn);
        for (img, thumb) in evicted {
            storage::remove_file_if_exists(&img);
            storage::remove_file_if_exists(&thumb);
        }
        Ok(id)
    })();

    match new_id_res {
        Ok(id) => Ok((id, new_record.image_path)),
        Err(e) => {
            // Roll back the moved image + generated thumb so they don't
            // orphan under app_data when the DB write fails.
            let _ = std::fs::remove_file(&image_dst);
            let _ = std::fs::remove_file(&thumb_dst);
            Err(e)
        }
    }
}

fn detected_to_string(d: &DetectedType) -> String {
    match d {
        DetectedType::EquationOnly => "EQUATION_ONLY".into(),
        DetectedType::TableOnly => "TABLE_ONLY".into(),
        DetectedType::Mixed => "MIXED".into(),
    }
}

fn detected_from_string(s: &str) -> DetectedType {
    match s {
        "EQUATION_ONLY" => DetectedType::EquationOnly,
        "TABLE_ONLY" => DetectedType::TableOnly,
        _ => DetectedType::Mixed,
    }
}

async fn show_overlay_and_await_selection(
    app: &AppHandle,
    snapshot: &MonitorSnapshot,
) -> Result<Option<SelectionRect>, String> {
    let overlay = app
        .get_webview_window(OVERLAY_WINDOW_LABEL)
        .ok_or_else(|| "overlay window not configured in tauri.conf.json".to_string())?;

    // The whole app was hidden in `run_snip` so the screenshot wouldn't
    // include our own windows. Re-show the app now so the overlay can
    // actually become key (NSApp.hide() leaves child windows in an
    // orderable-but-not-key state).
    #[cfg(target_os = "macos")]
    {
        let _ = app.show();
    }

    // Position + size the overlay over the captured monitor. xcap's
    // `Monitor::x/y/width/height` return values in the macOS global
    // points coordinate space, which matches Tauri's LogicalPosition.
    // Windows xcap is platform-different — verify in Phase 10 port.
    overlay
        .set_position(LogicalPosition::new(
            snapshot.monitor_x as f64,
            snapshot.monitor_y as f64,
        ))
        .map_err(|e| format!("overlay set_position: {e}"))?;
    overlay
        .set_size(LogicalSize::new(
            snapshot.logical_width as f64,
            snapshot.logical_height as f64,
        ))
        .map_err(|e| format!("overlay set_size: {e}"))?;

    // Channel: first event (region OR cancel) wins; the other listener
    // becomes a no-op via the Mutex<Option<_>>.
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<SelectionRect>>();
    let sender = Arc::new(Mutex::new(Some(tx)));

    let region_sender = Arc::clone(&sender);
    let region_handler = app.listen(CAPTURE_REGION_EVENT, move |event| {
        let payload = serde_json::from_str::<SelectionRectPayload>(event.payload()).ok();
        let rect = payload.map(SelectionRect::from);
        if let Ok(mut guard) = region_sender.lock() {
            if let Some(s) = guard.take() {
                let _ = s.send(rect);
            }
        }
    });

    let cancel_sender = Arc::clone(&sender);
    let cancel_handler = app.listen(CAPTURE_CANCEL_EVENT, move |_event| {
        if let Ok(mut guard) = cancel_sender.lock() {
            if let Some(s) = guard.take() {
                let _ = s.send(None);
            }
        }
    });

    // RAII guards: ensure unlisten + overlay.hide() run on every exit
    // path (including early `?` from emit/show below, panic, or timeout).
    let _listen_guard = ListenerGuard {
        app: app.clone(),
        handlers: vec![region_handler, cancel_handler],
    };
    let _overlay_guard = OverlayHideGuard {
        window: overlay.clone(),
    };

    let payload = CaptureStartPayload {
        backdrop_path: snapshot.full_png_path.to_string_lossy().into_owned(),
        logical_width: snapshot.logical_width,
        logical_height: snapshot.logical_height,
        pixel_width: snapshot.pixel_width,
        pixel_height: snapshot.pixel_height,
        scale_factor: snapshot.scale_factor,
    };
    overlay
        .emit(CAPTURE_START_EVENT, &payload)
        .map_err(|e| format!("emit capture-start: {e}"))?;
    overlay
        .show()
        .map_err(|e| format!("overlay show: {e}"))?;
    let _ = overlay.set_focus();

    match tokio::time::timeout(SELECTION_TIMEOUT, rx).await {
        Ok(Ok(rect)) => Ok(rect),
        Ok(Err(_canceled)) => Err("selection channel closed".to_string()),
        Err(_timeout) => Err("selection timed out".to_string()),
    }
}

async fn run_ocr_for_path(
    agent_id: Option<String>,
    image_path: &str,
    priority: &[String],
) -> Result<(String, String), String> {
    let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| format!("agent detect join: {e}"))?;
    if installed.is_empty() {
        return Err("no OCR agents installed (codex / gemini CLI or Gemini API key)".into());
    }

    if let Some(id) = agent_id {
        let agent = installed
            .iter()
            .find(|a| a.spec.id == id)
            .ok_or_else(|| format!("agent not installed: {id}"))?;
        let agent_id_str = agent.spec.id.to_string();
        let text = ocr::run_ocr(agent, image_path)
            .await
            .map_err(stringify_dispatch_error)?;
        Ok((text, agent_id_str))
    } else {
        let (text, agent) = ocr::run_with_fallback(&installed, image_path, priority)
            .await
            .map_err(stringify_dispatch_error)?;
        Ok((text, agent.spec.id.to_string()))
    }
}

#[derive(Serialize)]
pub struct CaptureStartPayload {
    pub backdrop_path: String,
    pub logical_width: u32,
    pub logical_height: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale_factor: f32,
}

#[derive(Deserialize)]
struct SelectionRectPayload {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl From<SelectionRectPayload> for SelectionRect {
    fn from(p: SelectionRectPayload) -> Self {
        SelectionRect { x: p.x, y: p.y, w: p.w, h: p.h }
    }
}

#[derive(Serialize)]
pub struct TestAgentReport {
    pub ok: bool,
    pub detected: DetectedType,
    pub char_count: usize,
    pub preview: String,
}

#[derive(Serialize, Clone)]
pub struct SnipResult {
    pub status: String,
    pub text: Option<String>,
    pub detected: Option<DetectedType>,
    pub agent: Option<String>,
    pub image_path: Option<String>,
    pub record_id: Option<i64>,
}

fn stringify_dispatch_error(e: DispatchError) -> String {
    e.to_string()
}

fn stringify_capture_error(e: CaptureError) -> String {
    e.to_string()
}

fn stringify_crop_error(e: CropError) -> String {
    e.to_string()
}

fn preview(s: &str, max_chars: usize) -> String {
    let collapsed: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if collapsed.chars().count() <= max_chars {
        collapsed
    } else {
        let head: String = collapsed.chars().take(max_chars).collect();
        format!("{head}...")
    }
}

/// Single-flight gate around `run_snip`. The shared overlay window can
/// only serve one capture at a time; a second invocation while the first
/// is awaiting the user's drag would race two emits/show()s on it.
static SNIP_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct SnipBusyGuard;

impl SnipBusyGuard {
    fn try_acquire() -> Option<Self> {
        SNIP_IN_FLIGHT
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| SnipBusyGuard)
    }
}

impl Drop for SnipBusyGuard {
    fn drop(&mut self) {
        SNIP_IN_FLIGHT.store(false, Ordering::Release);
    }
}

/// RAII guard: deletes the wrapped temp file on drop — including
/// `?` early-return, future cancellation, and panic. Call `disarm()`
/// after a successful rename/move so the guard becomes a no-op.
struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// RAII guard: unlistens every registered event handler on drop, so a
/// mid-function `?` or panic can't leak listeners that accumulate
/// across retries.
struct ListenerGuard {
    app: AppHandle,
    handlers: Vec<EventId>,
}

impl Drop for ListenerGuard {
    fn drop(&mut self) {
        for id in self.handlers.drain(..) {
            self.app.unlisten(id);
        }
    }
}

/// RAII guard: hides the overlay window on drop. Pairs with the
/// `ListenerGuard` so an error after `show()` still leaves the user
/// with a clean screen instead of a frozen translucent overlay.
struct OverlayHideGuard {
    window: WebviewWindow,
}

impl Drop for OverlayHideGuard {
    fn drop(&mut self) {
        let _ = self.window.hide();
    }
}

/// Hides any visible user-facing SnipTeX windows for the duration of a
/// capture and restores them on drop. Without this, clicking "Snip now"
/// from the main window captures the main window itself into the
/// backdrop — the document the user actually wants to snip is occluded.
/// The `overlay` and `preview` windows are excluded: overlay is owned
/// by the capture flow itself, and preview is auto-managed and would
/// only ever be visible from the previous snip (we want it gone too,
/// but its own hide-show cycle already covers that).
struct CaptureVisibilityGuard {
    app: AppHandle,
    restored_labels: Vec<&'static str>,
}

impl CaptureVisibilityGuard {
    fn hide_user_windows(app: &AppHandle) -> Self {
        const CANDIDATES: &[&str] = &["main", "settings", "history", "onboarding", "preview"];
        let mut hidden = Vec::new();
        for label in CANDIDATES {
            if let Some(w) = app.get_webview_window(label) {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.hide();
                    hidden.push(*label);
                }
            }
        }
        Self {
            app: app.clone(),
            restored_labels: hidden,
        }
    }
}

impl Drop for CaptureVisibilityGuard {
    fn drop(&mut self) {
        for label in &self.restored_labels {
            if let Some(w) = self.app.get_webview_window(label) {
                // `preview` re-shows itself when a fresh snip-complete
                // arrives — re-showing it here would briefly flash the
                // STALE previous-snip content. Skip it on restore.
                if *label == "preview" {
                    continue;
                }
                let _ = w.show();
            }
        }
    }
}

// Compile-time guard: cloud agent id stays exported from registry.
const _: &str = CLOUD_GEMINI_ID;
const _: &str = CLOUD_MISTRAL_ID;

// -----------------------------------------------------------------------
// PDF OCR command
// -----------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct PdfProgress {
    pub page: usize,
    pub total: usize,
}

#[tauri::command]
pub async fn run_pdf_ocr(
    app: AppHandle,
    pdf_path: String,
    agent_id: Option<String>,
) -> Result<SnipResult, String> {
    let path = Path::new(&pdf_path);
    if !path.exists() {
        return Err(format!("file not found: {pdf_path}"));
    }
    if !is_pdf_path(&pdf_path) {
        return Err("expected a .pdf file".into());
    }

    let priority = app
        .try_state::<SettingsStore>()
        .map(|s| s.get().agent_priority)
        .unwrap_or_default();

    let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| format!("agent detect join: {e}"))?;
    if installed.is_empty() {
        return Err("no OCR agents installed".into());
    }

    let agent = pick_agent(&installed, agent_id.as_deref(), &priority)?;
    let agent_id_str = agent.spec.id.to_string();

    let ocr_started = Instant::now();

    let text = dispatch_pdf_ocr(&app, agent, &pdf_path).await?;

    let latency_ms = ocr_started.elapsed().as_millis() as i64;
    let cleaned = ocr::post_process(&text);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err("OCR returned empty output".into());
    }

    let detected = ocr::detect_type(&cleaned);

    let store = app.state::<HistoryStore>();
    let record_id = persist_pdf_to_history(
        &store,
        &pdf_path,
        &cleaned,
        &agent_id_str,
        &detected,
        latency_ms,
    );

    let result = SnipResult {
        status: "ok".into(),
        text: Some(cleaned),
        detected: Some(detected),
        agent: Some(agent_id_str),
        image_path: Some(pdf_path),
        record_id: record_id.ok(),
    };

    let _ = app.emit("snip-complete", result.clone());
    Ok(result)
}

fn pick_agent<'a>(
    installed: &'a [AgentInfo],
    agent_id: Option<&str>,
    priority: &[String],
) -> Result<&'a AgentInfo, String> {
    if let Some(id) = agent_id {
        installed
            .iter()
            .find(|a| a.spec.id == id)
            .ok_or_else(|| format!("agent not installed: {id}"))
    } else {
        // Pick highest-priority installed agent
        for id in priority {
            if let Some(a) = installed.iter().find(|a| a.spec.id == id) {
                return Ok(a);
            }
        }
        installed.first().ok_or_else(|| "no agents available".into())
    }
}

/// Picks the right OCR path for a PDF based on agent type:
///   - cloud-mistral: native /v1/ocr endpoint reliably handles multi-page.
///   - cloud-gemini: PDF document modality truncates at ~2200 tokens —
///     fall back to per-page render which OCRs each page as a PNG.
///   - CLI agents: per-page render (they only accept images).
async fn dispatch_pdf_ocr(
    app: &AppHandle,
    agent: &AgentInfo,
    pdf_path: &str,
) -> Result<String, String> {
    use crate::agents::registry::AgentKind;
    match agent.spec.kind {
        AgentKind::CloudApi if agent.spec.id == CLOUD_MISTRAL_ID => {
            run_cloud_pdf_ocr(agent, pdf_path).await
        }
        _ => run_per_page_pdf_ocr(app, agent, pdf_path).await,
    }
}

fn is_pdf_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

async fn run_cloud_pdf_ocr(agent: &AgentInfo, pdf_path: &str) -> Result<String, String> {
    use crate::agents::cloud_gemini_api;
    use crate::agents::cloud_mistral_api;

    match agent.spec.id {
        CLOUD_GEMINI_ID => {
            let key = keychain::get_gemini_api_key()
                .map_err(|_| "missing Gemini API key".to_string())?;
            cloud_gemini_api::call_with_pdf_path(pdf_path, ocr::MASTER_PROMPT, &key)
                .await
                .map_err(|e| e.to_string())
        }
        CLOUD_MISTRAL_ID => {
            let key = keychain::get_mistral_api_key()
                .map_err(|_| "missing Mistral API key".to_string())?;
            cloud_mistral_api::call_with_pdf_path(pdf_path, ocr::MASTER_PROMPT, &key)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("cloud agent {} does not support PDF", agent.spec.id)),
    }
}

/// Per-agent cap on concurrent in-flight page OCR calls. Conservative for
/// agents that share a single process or a paid token bucket; generous for
/// cloud agents with native concurrency headroom.
fn pdf_page_concurrency(agent_id: &str) -> usize {
    use crate::agents::registry::{
        CLOUD_GEMINI_ID, CLOUD_GOCLAW_ID, CLOUD_MISTRAL_ID, CODEX_ID, GEMINI_CLI_ID,
    };
    match agent_id {
        // Local CLI subprocess — keep low to avoid CPU thrash on laptops.
        CODEX_ID | GEMINI_CLI_ID => 2,
        // gpt-5.4 over Goclaw — single ChatGPT-Plus account, don't hammer.
        CLOUD_GOCLAW_ID => 2,
        // Cloud vision APIs with proper rate limits.
        CLOUD_GEMINI_ID | CLOUD_MISTRAL_ID => 5,
        // Unknown agents: default to a safe middle.
        _ => 3,
    }
}

/// Per-page PDF OCR loop. Used by:
///   - CLI agents (codex, gemini-cli) — they only accept images.
///   - cloud-gemini — its PDF document modality truncates multi-page output.
///   - cloud-goclaw — single-image chat.send per page.
///
/// Pages are dispatched in parallel with a per-agent concurrency cap
/// (`pdf_page_concurrency`) so multi-page PDFs no longer scale linearly
/// with N. Results are re-sorted by page index before concatenation so
/// the output order is stable regardless of completion order.
async fn run_per_page_pdf_ocr(
    app: &AppHandle,
    agent: &AgentInfo,
    pdf_path: &str,
) -> Result<String, String> {
    use crate::agents::registry::AgentKind;

    let overall_started = Instant::now();
    let agent_id = agent.spec.id;
    log::info!(
        "[pdf-ocr] start agent={agent_id} pdf={}",
        Path::new(pdf_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(pdf_path)
    );

    let tmp_dir = std::env::temp_dir()
        .join("sniptex")
        .join(format!("pdf-pages-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("create temp dir: {e}"))?;
    let _cleanup = TempDirGuard(tmp_dir.clone());

    let render_started = Instant::now();
    let page_pngs = ocr::pdf_render::render_pages_to_pngs(pdf_path, &tmp_dir, None)
        .map_err(|e| format!("PDF render: {e}"))?;
    let render_ms = render_started.elapsed().as_millis();

    let total = page_pngs.len();
    if total == 0 {
        return Err("PDF has no pages".into());
    }
    log::info!("[pdf-ocr] rendered {total} page(s) in {render_ms}ms");

    // CLI per-page: 120s (codex/gemini-cli are slow).
    // cloud-goclaw also gets 120s — it runs gpt-5.4 server-side, same model
    // family / latency profile as the local codex CLI. The other cloud agents
    // (gemini, mistral) are sub-30s per page and keep the tighter budget.
    use crate::agents::registry::CLOUD_GOCLAW_ID;
    let per_page = match (agent.spec.kind, agent.spec.id) {
        (AgentKind::CliBin, _) => ocr::PDF_CLI_PAGE_TIMEOUT,
        (AgentKind::CloudApi, CLOUD_GOCLAW_ID) => ocr::PDF_CLI_PAGE_TIMEOUT,
        (AgentKind::CloudApi, _) => Duration::from_secs(30),
    };
    // Overall budget = per_page × total. With parallelism, real wall-clock will
    // typically be much less, but keep the upper bound matching the worst-case
    // sequential equivalent so we don't accidentally tighten it.
    let overall_budget = per_page.saturating_mul(total as u32);

    let max_concurrent = pdf_page_concurrency(agent_id).max(1);
    log::info!(
        "[pdf-ocr] dispatching {total} page(s) with concurrency={max_concurrent}"
    );

    let sem = Arc::new(Semaphore::new(max_concurrent));
    let done_counter = Arc::new(AtomicUsize::new(0));
    let app_for_progress = app.clone();
    let agent_clone = agent.clone();

    let work = async move {
        let tasks = page_pngs.into_iter().enumerate().map(|(i, png)| {
            let sem = sem.clone();
            let agent = agent_clone.clone();
            let app = app_for_progress.clone();
            let done = done_counter.clone();
            async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| format!("page {} concurrency permit: {e}", i + 1))?;
                let path_str = png.to_string_lossy().to_string();
                let page_started = Instant::now();
                let text = ocr::run_ocr_pdf_page(&agent, &path_str)
                    .await
                    .map_err(|e| format!("page {} OCR failed: {e}", i + 1))?;
                let page_ms = page_started.elapsed().as_millis();
                let completed = done.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "pdf-progress",
                    PdfProgress {
                        page: completed,
                        total,
                    },
                );
                log::info!(
                    "[pdf-ocr] page {}/{} done in {page_ms}ms ({} chars)",
                    i + 1,
                    total,
                    text.len()
                );
                Ok::<(usize, String), String>((i, text))
            }
        });
        try_join_all(tasks).await
    };

    let mut indexed = match tokio::time::timeout(overall_budget, work).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            log::warn!(
                "[pdf-ocr] failed after {}ms: {e}",
                overall_started.elapsed().as_millis()
            );
            return Err(e);
        }
        Err(_) => {
            log::warn!(
                "[pdf-ocr] timed out after {}ms (budget {}s)",
                overall_started.elapsed().as_millis(),
                overall_budget.as_secs()
            );
            return Err(format!(
                "PDF OCR exceeded budget of {}s ({} pages × {}s)",
                overall_budget.as_secs(),
                total,
                per_page.as_secs(),
            ));
        }
    };

    // Pages may complete out of order under parallelism — sort by original
    // page index before concatenating so the final document preserves source
    // order.
    indexed.sort_by_key(|(i, _)| *i);
    let parts: Vec<String> = indexed.into_iter().map(|(_, t)| t).collect();

    let total_ms = overall_started.elapsed().as_millis();
    log::info!(
        "[pdf-ocr] done agent={agent_id} total={total_ms}ms (render={render_ms}ms, ocr={}ms, concurrency={max_concurrent})",
        total_ms.saturating_sub(render_ms)
    );

    Ok(parts.join("\n\n"))
}

fn persist_pdf_to_history(
    store: &State<'_, HistoryStore>,
    pdf_path: &str,
    text: &str,
    agent_id: &str,
    detected: &DetectedType,
    latency_ms: i64,
) -> Result<i64, String> {
    let uuid_str = Uuid::new_v4().to_string();
    let images_dir = store.images_dir();
    let thumbs_dir = store.thumbs_dir();

    // Copy PDF into history images dir so reruns work
    let image_dst = images_dir.join(format!("{uuid_str}.pdf"));
    std::fs::copy(pdf_path, &image_dst)
        .map_err(|e| format!("copy pdf to history: {e}"))?;

    // Render first page as thumbnail
    let thumb_dst = thumbs_dir.join(format!("{uuid_str}.webp"));
    let thumb_tmp = std::env::temp_dir().join("sniptex").join(format!("{uuid_str}-thumb"));
    let _ = std::fs::create_dir_all(&thumb_tmp);
    let thumb_result = ocr::pdf_render::render_pages_to_pngs(
        pdf_path,
        &thumb_tmp,
        Some(72.0),
    );
    match thumb_result {
        Ok(pages) if !pages.is_empty() => {
            if let Err(e) = storage::thumbnail::make_thumbnail(&pages[0], &thumb_dst) {
                log::warn!("pdf thumbnail failed: {e}");
                // Non-fatal: continue without thumbnail
            }
        }
        _ => {
            log::warn!("could not render pdf first page for thumbnail");
        }
    }
    let _ = std::fs::remove_dir_all(&thumb_tmp);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let detected_str = detected_to_string(detected);
    let new_record = history_repo::NewRecord {
        uuid: uuid_str,
        created_at: now,
        agent_id: agent_id.to_string(),
        output_text: text.to_string(),
        detected_type: detected_str,
        image_path: image_dst.to_string_lossy().to_string(),
        thumb_path: thumb_dst.to_string_lossy().to_string(),
        latency_ms,
    };

    let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
    let id = history_repo::insert(&conn, &new_record).map_err(|e| e.to_string())?;
    let evicted = history_repo::enforce_max_records(&conn, DEFAULT_MAX_RECORDS)
        .map_err(|e| e.to_string())?;
    drop(conn);
    for (img, thumb) in evicted {
        storage::remove_file_if_exists(&img);
        storage::remove_file_if_exists(&thumb);
    }
    Ok(id)
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// -----------------------------------------------------------------------
// Phase 7: history commands
// -----------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct HistoryRecordDto {
    pub id: i64,
    pub uuid: String,
    pub created_at: i64,
    pub agent: String,
    pub text: String,
    pub detected: DetectedType,
    pub image_path: String,
    pub thumb_path: String,
    pub latency_ms: i64,
}

impl From<history_repo::Record> for HistoryRecordDto {
    fn from(r: history_repo::Record) -> Self {
        Self {
            id: r.id,
            uuid: r.uuid,
            created_at: r.created_at,
            agent: r.agent_id,
            text: r.output_text,
            detected: detected_from_string(&r.detected_type),
            image_path: r.image_path,
            thumb_path: r.thumb_path,
            latency_ms: r.latency_ms,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Latex,
    Markdown,
    Plain,
}

#[tauri::command]
pub fn get_history(
    store: State<'_, HistoryStore>,
    limit: usize,
) -> Result<Vec<HistoryRecordDto>, String> {
    let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
    let rows = history_repo::recent(&conn, limit).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(HistoryRecordDto::from).collect())
}

#[tauri::command]
pub fn search_history(
    store: State<'_, HistoryStore>,
    query: String,
    limit: usize,
) -> Result<Vec<HistoryRecordDto>, String> {
    let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
    let rows = history_repo::search(&conn, &query, limit).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(HistoryRecordDto::from).collect())
}

#[tauri::command]
pub fn delete_record(store: State<'_, HistoryStore>, id: i64) -> Result<(), String> {
    let paths = {
        let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
        history_repo::delete(&conn, id).map_err(|e| e.to_string())?
    };
    if let Some((img, thumb)) = paths {
        storage::remove_file_if_exists(&img);
        storage::remove_file_if_exists(&thumb);
    }
    Ok(())
}

/// Re-OCR the persisted image with a chosen agent and update the same
/// row in-place. Returns the refreshed record so the UI can swap text +
/// agent badge without a refetch.
#[tauri::command]
pub async fn rerun_snip(
    app: AppHandle,
    record_id: i64,
    agent_id: String,
) -> Result<HistoryRecordDto, String> {
    let store = app.state::<HistoryStore>();
    let record = {
        let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
        history_repo::find_by_id(&conn, record_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("record not found: {record_id}"))?
    };

    let image_path = record.image_path.clone();
    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("image missing: {image_path}"));
    }
    log::info!("[rerun] record={record_id} agent={agent_id} image={image_path}");
    let priority = app
        .try_state::<SettingsStore>()
        .map(|s| s.get().agent_priority)
        .unwrap_or_default();
    let started = Instant::now();

    // PDF records need the per-page pipeline. CLI agents can't read PDF
    // bytes; cloud agents need agent-specific routing (mistral has a
    // native endpoint, gemini needs per-page rendering).
    let (text, used_agent) = if is_pdf_path(&image_path) {
        let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
            .await
            .map_err(|e| format!("agent detect join: {e}"))?;
        let agent = installed
            .iter()
            .find(|a| a.spec.id == agent_id)
            .ok_or_else(|| format!("agent not installed: {agent_id}"))?;
        let used = agent.spec.id.to_string();
        let text = dispatch_pdf_ocr(&app, agent, &image_path).await?;
        (text, used)
    } else {
        run_ocr_for_path(Some(agent_id), &image_path, &priority).await?
    };

    let latency_ms = started.elapsed().as_millis() as i64;
    if used_agent == GEMINI_CLI_ID {
        ocr::validate_rerun_consistency(&record.output_text, &text)
            .map_err(|reason| format!("gemini-cli output rejected: {reason}"))?;
    }
    let detected = ocr::detect_type(&text);
    let detected_str = detected_to_string(&detected);

    {
        let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
        let updated = history_repo::update_output(
            &conn,
            record_id,
            &text,
            &used_agent,
            &detected_str,
            latency_ms,
        )
        .map_err(|e| e.to_string())?;
        if updated == 0 {
            // A concurrent `delete_record` removed the row between our
            // initial fetch and now. Surface that to the caller instead
            // of pretending the rerun succeeded.
            return Err(format!("record {record_id} no longer exists"));
        }
    }

    Ok(HistoryRecordDto {
        id: record.id,
        uuid: record.uuid,
        created_at: record.created_at,
        agent: used_agent,
        text,
        detected,
        image_path: record.image_path,
        thumb_path: record.thumb_path,
        latency_ms,
    })
}

#[tauri::command]
pub fn export_record(
    store: State<'_, HistoryStore>,
    id: i64,
    format: ExportFormat,
) -> Result<String, String> {
    let conn = store.conn.lock().map_err(|e| format!("db lock: {e}"))?;
    let rec = history_repo::find_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("record not found: {id}"))?;
    Ok(format_export(&rec.output_text, &format))
}

fn format_export(text: &str, format: &ExportFormat) -> String {
    match format {
        ExportFormat::Plain => text.to_string(),
        ExportFormat::Markdown => text.to_string(),
        // Wrap raw output in a `$$ … $$` block when it looks like a
        // bare LaTeX expression so it pastes cleanly into a .tex file.
        // Phase 9 will replace this stub with the proper Format Toggle
        // (Markdown ↔ tabular ↔ raw LaTeX).
        ExportFormat::Latex => {
            if text.contains("\\begin{") || text.contains("$$") {
                text.to_string()
            } else {
                format!("$$\n{}\n$$", text.trim())
            }
        }
    }
}

// -----------------------------------------------------------------------
// Settings commands
// -----------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> AppSettings {
    store.get()
}

#[tauri::command]
pub fn update_settings(
    store: State<'_, SettingsStore>,
    patch: SettingsPatch,
) -> Result<AppSettings, String> {
    store.update(patch)
}

#[tauri::command]
pub fn rebind_hotkey(
    app: AppHandle,
    new_shortcut: String,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        crate::hotkey::rebind(&app, &new_shortcut)?;
        let settings_store = app.state::<SettingsStore>();
        settings_store.update(SettingsPatch {
            hotkey: Some(new_shortcut),
            ..Default::default()
        })?;
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, new_shortcut);
    }
    Ok(())
}

#[tauri::command]
pub fn set_launch_at_login(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let autostart = app.autolaunch();
        if enabled {
            autostart.enable().map_err(|e| format!("enable autostart: {e}"))?;
        } else {
            autostart.disable().map_err(|e| format!("disable autostart: {e}"))?;
        }
        let settings_store = app.state::<SettingsStore>();
        settings_store.update(SettingsPatch {
            launch_at_login: Some(enabled),
            ..Default::default()
        })?;
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, enabled);
    }
    Ok(())
}
