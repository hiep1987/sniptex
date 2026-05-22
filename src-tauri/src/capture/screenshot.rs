//! Capture the full image of the monitor that currently contains the
//! mouse cursor and save it to a temp PNG.
//!
//! The full-monitor PNG is the "frozen backdrop" the overlay renders
//! behind the selection rectangle so the overlay never appears inside
//! its own screenshot.

use std::path::PathBuf;

use thiserror::Error;
use uuid::Uuid;
use xcap::Monitor;

use super::staging_path;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("no monitor under cursor at ({0}, {1})")]
    NoMonitorAtCursor(i32, i32),
    #[error("screen capture failed: {0}")]
    Capture(String),
    #[error("missing screen recording permission (macOS System Settings → Privacy → Screen Recording)")]
    PermissionDenied,
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CaptureError {
    fn from(e: std::io::Error) -> Self {
        CaptureError::Io(e.to_string())
    }
}

/// Result of a successful monitor capture.
#[derive(Debug, Clone)]
pub struct MonitorSnapshot {
    /// Path to the full-monitor PNG written to the temp staging dir.
    pub full_png_path: PathBuf,
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
pub fn capture_active_monitor(
    cursor_x: i32,
    cursor_y: i32,
) -> Result<MonitorSnapshot, CaptureError> {
    let monitor = Monitor::from_point(cursor_x, cursor_y).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            CaptureError::NoMonitorAtCursor(cursor_x, cursor_y)
        } else {
            CaptureError::Capture(msg)
        }
    })?;

    let monitor_x = monitor.x().map_err(|e| CaptureError::Capture(e.to_string()))?;
    let monitor_y = monitor.y().map_err(|e| CaptureError::Capture(e.to_string()))?;
    let logical_width = monitor
        .width()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let logical_height = monitor
        .height()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;
    let scale_factor = monitor
        .scale_factor()
        .map_err(|e| CaptureError::Capture(e.to_string()))?;

    let img = monitor.capture_image().map_err(|e| {
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
    })?;

    let pixel_width = img.width();
    let pixel_height = img.height();

    let full_png_path = staging_path(&format!("sniptex-full-{}.png", Uuid::new_v4()));
    if let Some(parent) = full_png_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(&full_png_path)
        .map_err(|e| CaptureError::Capture(format!("png write failed: {e}")))?;

    Ok(MonitorSnapshot {
        full_png_path,
        monitor_x,
        monitor_y,
        logical_width,
        logical_height,
        pixel_width,
        pixel_height,
        scale_factor,
    })
}

#[cfg(test)]
mod tests {
    use super::super::staging_path;

    #[test]
    fn staging_path_lives_under_temp_sniptex() {
        let p = staging_path("test-name.png");
        assert!(p.starts_with(std::env::temp_dir()));
        assert_eq!(p.parent().and_then(|p| p.file_name()), Some(std::ffi::OsStr::new("sniptex")));
        assert_eq!(p.file_name(), Some(std::ffi::OsStr::new("test-name.png")));
    }
}
