//! Screen capture and region-selector overlay orchestration.
//!
//! Phase 4 surface:
//!   * `screenshot::capture_active_monitor` — pick the monitor under the
//!     cursor, capture its full image, save to a temp PNG.
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
pub use screenshot::{capture_active_monitor, CaptureError, MonitorSnapshot};

/// Shared temp-file staging path: `{system temp}/sniptex/{file_name}`.
/// Used by both the full-monitor screenshot and the cropped output so
/// the assetProtocol scope `$TEMP/sniptex/**` covers every PNG we produce.
pub(crate) fn staging_path(file_name: &str) -> PathBuf {
    std::env::temp_dir().join("sniptex").join(file_name)
}
