//! Crop a saved full-monitor PNG to the user's drag-selected rectangle.
//!
//! The overlay window sends back CSS-space coordinates (relative to the
//! overlay's local origin, which is the monitor's logical top-left).
//! We multiply by the monitor's DPI scale factor before cropping the
//! physical PNG, then clamp to the image bounds so off-screen drags
//! don't fail.

use std::path::{Path, PathBuf};

use image::{ImageReader, RgbaImage};
use thiserror::Error;
use uuid::Uuid;

use super::staging_path;

#[derive(Debug, Error)]
pub enum CropError {
    #[error("selection has zero area")]
    ZeroArea,
    #[error("selection lies entirely outside the captured monitor")]
    OutOfBounds,
    #[error("image decode failed: {0}")]
    Decode(String),
    #[error("image encode failed: {0}")]
    Encode(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CropError {
    fn from(e: std::io::Error) -> Self {
        CropError::Io(e.to_string())
    }
}

/// CSS-space selection rectangle (origin at overlay window's top-left).
/// Coordinates are signed because a drag can begin off-screen on some
/// platforms; the clamp pass normalises before cropping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Convert the user's CSS-space selection into a physical-pixel rect
/// clamped to the captured image. Returns `None` for zero-area or
/// fully-out-of-bounds selections.
pub fn clamp_and_validate_rect(
    sel: SelectionRect,
    image_width: u32,
    image_height: u32,
    scale_factor: f32,
) -> Result<(u32, u32, u32, u32), CropError> {
    if sel.w <= 0 || sel.h <= 0 {
        return Err(CropError::ZeroArea);
    }

    let scale = if scale_factor > 0.0 { scale_factor as f64 } else { 1.0 };

    // Convert CSS-space (logical) to physical pixels.
    let px_x = (sel.x as f64 * scale).round();
    let px_y = (sel.y as f64 * scale).round();
    let px_w = (sel.w as f64 * scale).round();
    let px_h = (sel.h as f64 * scale).round();

    // Convert to inclusive-exclusive bounds and clamp to image dimensions.
    let mut x0 = px_x;
    let mut y0 = px_y;
    let mut x1 = px_x + px_w;
    let mut y1 = px_y + px_h;

    let iw = image_width as f64;
    let ih = image_height as f64;

    x0 = x0.clamp(0.0, iw);
    y0 = y0.clamp(0.0, ih);
    x1 = x1.clamp(0.0, iw);
    y1 = y1.clamp(0.0, ih);

    let cw = (x1 - x0).max(0.0) as u32;
    let ch = (y1 - y0).max(0.0) as u32;

    if cw == 0 || ch == 0 {
        return Err(CropError::OutOfBounds);
    }
    Ok((x0 as u32, y0 as u32, cw, ch))
}

/// Crop the supplied full-monitor PNG to the selection rect, save the
/// result to a fresh temp PNG, and return its path. The full-monitor
/// source PNG is left intact — callers are responsible for deleting it.
pub fn crop_region_to_temp_png(
    full_png_path: &Path,
    sel: SelectionRect,
    scale_factor: f32,
) -> Result<PathBuf, CropError> {
    let mut img: RgbaImage = ImageReader::open(full_png_path)
        .map_err(|e| CropError::Decode(e.to_string()))?
        .decode()
        .map_err(|e| CropError::Decode(e.to_string()))?
        .into_rgba8();

    let (image_width, image_height) = (img.width(), img.height());
    let (x, y, w, h) = clamp_and_validate_rect(sel, image_width, image_height, scale_factor)?;

    // image::imageops::crop is in-place but takes a view; sub_image gives
    // us a `SubImage` we materialise into a fresh `RgbaImage` for saving.
    let cropped = image::imageops::crop(&mut img, x, y, w, h).to_image();

    let out_path = staging_path(&format!("sniptex-{}.png", Uuid::new_v4()));
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    cropped
        .save(&out_path)
        .map_err(|e| CropError::Encode(e.to_string()))?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_area() {
        let r = SelectionRect { x: 10, y: 10, w: 0, h: 50 };
        let err = clamp_and_validate_rect(r, 1000, 1000, 1.0).unwrap_err();
        assert!(matches!(err, CropError::ZeroArea));

        let r = SelectionRect { x: 10, y: 10, w: 50, h: 0 };
        assert!(matches!(
            clamp_and_validate_rect(r, 1000, 1000, 1.0).unwrap_err(),
            CropError::ZeroArea
        ));
    }

    #[test]
    fn scales_with_dpi() {
        let r = SelectionRect { x: 100, y: 50, w: 200, h: 100 };
        let (x, y, w, h) = clamp_and_validate_rect(r, 4000, 4000, 2.0).unwrap();
        assert_eq!((x, y, w, h), (200, 100, 400, 200));
    }

    #[test]
    fn clamps_to_image_bounds() {
        // Selection extends past the right/bottom edges → clamp.
        let r = SelectionRect { x: 900, y: 900, w: 300, h: 300 };
        let (x, y, w, h) = clamp_and_validate_rect(r, 1000, 1000, 1.0).unwrap();
        assert_eq!((x, y, w, h), (900, 900, 100, 100));
    }

    #[test]
    fn clamps_negative_origin() {
        // Drag started "above and left of" the overlay → clamp to (0,0).
        let r = SelectionRect { x: -50, y: -30, w: 200, h: 100 };
        let (x, y, w, h) = clamp_and_validate_rect(r, 1000, 1000, 1.0).unwrap();
        assert_eq!((x, y, w, h), (0, 0, 150, 70));
    }

    #[test]
    fn rejects_fully_out_of_bounds() {
        let r = SelectionRect { x: 2000, y: 2000, w: 100, h: 100 };
        let err = clamp_and_validate_rect(r, 1000, 1000, 1.0).unwrap_err();
        assert!(matches!(err, CropError::OutOfBounds));
    }
}
