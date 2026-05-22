use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, EventId, Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewWindow,
};

use crate::agents::{
    self, keychain,
    registry::{AgentInfo, CLOUD_GEMINI_ID},
};
use crate::capture::{
    capture_active_monitor, crop_region_to_temp_png, CaptureError, CropError, MonitorSnapshot,
    SelectionRect,
};
use crate::ocr::{self, dispatcher::DispatchError, smart_format::DetectedType};

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
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn has_api_key(provider: String) -> Result<bool, String> {
    match provider.as_str() {
        "gemini" => Ok(keychain::has_gemini_api_key()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    match provider.as_str() {
        "gemini" => keychain::delete(keychain::GEMINI_ACCOUNT).map_err(|e| e.to_string()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

/// End-to-end snip: cursor-monitor screenshot → overlay drag-select →
/// crop → OCR (specific agent or fallback chain).
///
/// Returns `SnipResult` with `status` = `"ok" | "cancelled"`. On `ok`
/// the `text`, `detected`, and `agent` fields are populated; the temp
/// PNG is deleted after OCR. On `cancelled` the full-monitor PNG is
/// also cleaned up.
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

    let selection = show_overlay_and_await_selection(&app, &snapshot).await?;

    let Some(sel) = selection else {
        return Ok(SnipResult {
            status: "cancelled".into(),
            text: None,
            detected: None,
            agent: None,
            image_path: None,
        });
    };

    let full_path = snapshot.full_png_path.clone();
    let scale = snapshot.scale_factor;
    let cropped_path = tokio::task::spawn_blocking(move || {
        crop_region_to_temp_png(&full_path, sel, scale)
    })
    .await
    .map_err(|e| format!("crop join failed: {e}"))?
    .map_err(stringify_crop_error)?;

    // Cropped PNG is also a temp file with potentially-sensitive screen
    // content — guard it the same way.
    let _cropped_guard = TempFileGuard::new(cropped_path.clone());
    let cropped_str = cropped_path.to_string_lossy().to_string();

    let (text, agent) = run_ocr_for_path(agent_id, &cropped_str).await?;
    let detected = ocr::detect_type(&text);
    Ok(SnipResult {
        status: "ok".into(),
        text: Some(text),
        detected: Some(detected),
        agent: Some(agent),
        image_path: Some(cropped_str),
    })
}

async fn show_overlay_and_await_selection(
    app: &AppHandle,
    snapshot: &MonitorSnapshot,
) -> Result<Option<SelectionRect>, String> {
    let overlay = app
        .get_webview_window(OVERLAY_WINDOW_LABEL)
        .ok_or_else(|| "overlay window not configured in tauri.conf.json".to_string())?;

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
        let (text, agent) = ocr::run_with_fallback(&installed, image_path)
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

#[derive(Serialize)]
pub struct SnipResult {
    pub status: String,
    pub text: Option<String>,
    pub detected: Option<DetectedType>,
    pub agent: Option<String>,
    pub image_path: Option<String>,
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
/// `?` early-return, future cancellation, and panic.
struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
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

// Compile-time guard: cloud agent id stays exported from registry.
const _: &str = CLOUD_GEMINI_ID;
