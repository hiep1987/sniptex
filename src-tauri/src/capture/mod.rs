//! Screen capture and region-selector overlay orchestration.
//!
//! Phase 4 surface:
//!   * `screenshot::active_monitor_geometry` — pick the monitor under the
//!     cursor without capturing screen pixels, so the selector can show fast.
//!   * `screenshot::capture_monitor_region_to_temp_png` — capture only the
//!     selected region after the overlay is hidden.
//!   * `region_selector::crop_region_to_temp_png` — crop a saved PNG by
//!     CSS-space rect, scaled by the monitor's DPI factor.
//!
//! The orchestration (event ping-pong with the overlay window) lives in
//! `commands::run_snip` so the Tauri AppHandle stays out of this module.

use std::path::PathBuf;

pub mod region_selector;
pub mod screenshot;

pub use region_selector::{
    clamp_and_validate_rect, crop_region_to_temp_png, CropError, SelectionRect,
};
pub use screenshot::{
    active_monitor_geometry, capture_monitor_region_to_temp_png, CaptureError, MonitorGeometry,
};

/// Shared temp-file staging path: `{system temp}/sniptex/{file_name}`.
/// Keeps capture PNGs under the assetProtocol scope `$TEMP/sniptex/**`.
pub(crate) fn staging_path(file_name: &str) -> PathBuf {
    std::env::temp_dir().join("sniptex").join(file_name)
}
