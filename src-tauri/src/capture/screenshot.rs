//! Capture screen content from the monitor that currently contains the
//! mouse cursor.
//!
//! The fast path used by the snip overlay captures only the selected
//! region after the overlay has been hidden.

use std::path::PathBuf;

use thiserror::Error;
use uuid::Uuid;
use xcap::Monitor;

use super::region_selector::SelectionRect;
use super::staging_path;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("no monitor under cursor at ({0}, {1})")]
    NoMonitorAtCursor(i32, i32),
    #[error("screen capture failed: {0}")]
    Capture(String),
    #[error(
        "missing screen recording permission (macOS System Settings → Privacy → Screen Recording)"
    )]
    PermissionDenied,
    #[error("selection has zero area")]
    ZeroAreaSelection,
    #[error("selection lies entirely outside the captured monitor")]
    SelectionOutOfBounds,
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CaptureError {
    fn from(e: std::io::Error) -> Self {
        CaptureError::Io(e.to_string())
    }
}

/// Logical bounds and DPI metadata for a monitor.
#[derive(Debug, Clone)]
pub struct MonitorGeometry {
    /// Stable display id from xcap for the duration of the capture flow.
    pub monitor_id: u32,
    /// Monitor logical bounds in desktop CSS coordinates (top-left + size).
    pub monitor_x: i32,
    pub monitor_y: i32,
    pub logical_width: u32,
    pub logical_height: u32,
    /// Physical pixel dimensions of the captured image (post DPI scaling).
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Device pixel ratio for this monitor (1.0 on standard, 2.0 on Retina).
    pub scale_factor: f32,
}

/// Capture the monitor under (cursor_x, cursor_y). Coordinates must be
/// in desktop (logical) pixel space — matches what
/// `tauri::AppHandle::cursor_position()` returns.
pub fn active_monitor_geometry(
    cursor_x: i32,
    cursor_y: i32,
) -> Result<MonitorGeometry, CaptureError> {
    let monitor = monitor_from_point(cursor_x, cursor_y)?;
    geometry_from_monitor(&monitor)
}

/// Capture only the selected region of the previously chosen monitor,
/// save it to a temp PNG, and return its path. `sel` is in overlay CSS
/// coordinates, relative to the monitor's logical top-left.
pub fn capture_monitor_region_to_temp_png(
    geometry: &MonitorGeometry,
    sel: SelectionRect,
) -> Result<PathBuf, CaptureError> {
    let (x, y, w, h) =
        clamp_selection_to_monitor(sel, geometry.logical_width, geometry.logical_height)?;
    let monitor = monitor_by_id(geometry.monitor_id)?;
    let img = monitor
        .capture_region(x, y, w, h)
        .map_err(map_capture_error)?;
    save_temp_png(img, "sniptex")
}

fn monitor_from_point(cursor_x: i32, cursor_y: i32) -> Result<Monitor, CaptureError> {
    Monitor::from_point(cursor_x, cursor_y).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            CaptureError::NoMonitorAtCursor(cursor_x, cursor_y)
        } else {
            CaptureError::Capture(msg)
        }
    })
}

fn monitor_by_id(id: u32) -> Result<Monitor, CaptureError> {
    let monitors = Monitor::all().map_err(|e| CaptureError::Capture(e.to_string()))?;
    for monitor in monitors {
        if monitor
            .id()
            .map_err(|e| CaptureError::Capture(e.to_string()))?
            == id
        {
            return Ok(monitor);
        }
    }
    Err(CaptureError::Capture(format!(
        "monitor {id} no longer available"
    )))
}

fn geometry_from_monitor(monitor: &Monitor) -> Result<MonitorGeometry, CaptureError> {
    let monitor_id = monitor
        .id()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let monitor_x = monitor
        .x()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let monitor_y = monitor
        .y()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let logical_width = monitor
        .width()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let logical_height = monitor
        .height()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let scale_factor = monitor
        .scale_factor()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;

    let pixel_width = ((logical_width as f32) * scale_factor).round() as u32;
    let pixel_height = ((logical_height as f32) * scale_factor).round() as u32;

    Ok(MonitorGeometry {
        monitor_id,
        monitor_x,
        monitor_y,
        logical_width,
        logical_height,
        pixel_width,
        pixel_height,
        scale_factor,
    })
}

fn save_temp_png(img: image::RgbaImage, prefix: &str) -> Result<PathBuf, CaptureError> {
    let out_path = staging_path(&format!("{prefix}-{}.png", Uuid::new_v4()));
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(&out_path)
        .map_err(|e| CaptureError::Capture(format!("png write failed: {e}")))?;

    Ok(out_path)
}

fn clamp_selection_to_monitor(
    sel: SelectionRect,
    monitor_width: u32,
    monitor_height: u32,
) -> Result<(u32, u32, u32, u32), CaptureError> {
    if sel.w <= 0 || sel.h <= 0 {
        return Err(CaptureError::ZeroAreaSelection);
    }

    let mut x0 = sel.x as i64;
    let mut y0 = sel.y as i64;
    let mut x1 = x0 + sel.w as i64;
    let mut y1 = y0 + sel.h as i64;

    x0 = x0.clamp(0, monitor_width as i64);
    y0 = y0.clamp(0, monitor_height as i64);
    x1 = x1.clamp(0, monitor_width as i64);
    y1 = y1.clamp(0, monitor_height as i64);

    let w = (x1 - x0).max(0) as u32;
    let h = (y1 - y0).max(0) as u32;
    if w == 0 || h == 0 {
        return Err(CaptureError::SelectionOutOfBounds);
    }
    Ok((x0 as u32, y0 as u32, w, h))
}

fn map_capture_error(e: xcap::XCapError) -> CaptureError {
    let msg = e.to_string();
    // xcap surfaces a generic CGError on macOS when Screen Recording
    // permission isn't granted; the message contains "1003" (kCGErrorFailure)
    // or "permission". We bias toward a user-actionable error.
    if msg.to_lowercase().contains("permission")
        || msg.contains("1003")
        || msg.contains("ScreenCapture")
    {
        CaptureError::PermissionDenied
    } else {
        CaptureError::Capture(msg)
    }
}

#[cfg(test)]
#[path = "screenshot_tests.rs"]
mod screenshot_tests;
