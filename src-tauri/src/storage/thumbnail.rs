//! 200×200 WebP thumbnail generator.
//!
//! Resizes the source PNG with `Lanczos3` filter while preserving aspect
//! ratio (the longest edge ends up at 200 px). The result is encoded as
//! WebP at quality 80 — typically <8 KB per snip, vs. ~80 KB for the
//! source PNG. We persist thumbnails to disk (not the DB) so the
//! `snip_records` table stays small enough for fast FTS scans.

use std::path::Path;

use image::imageops::FilterType;
use image::ImageReader;
use thiserror::Error;

const THUMB_MAX_EDGE: u32 = 200;
const THUMB_QUALITY: f32 = 80.0;

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("read png: {0}")]
    ReadPng(String),
    #[error("decode png: {0}")]
    DecodePng(String),
    #[error("encode webp: {0}")]
    EncodeWebp(String),
    #[error("write webp: {0}")]
    WriteWebp(String),
}

pub fn make_thumbnail(src_png: &Path, dst_webp: &Path) -> Result<(), ThumbnailError> {
    let reader = ImageReader::open(src_png)
        .map_err(|e| ThumbnailError::ReadPng(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| ThumbnailError::ReadPng(e.to_string()))?;

    let img = reader
        .decode()
        .map_err(|e| ThumbnailError::DecodePng(e.to_string()))?;

    // `resize` keeps aspect ratio. Lanczos3 gives the cleanest downscale
    // for screenshots with small text + thin LaTeX strokes.
    let resized = img.resize(THUMB_MAX_EDGE, THUMB_MAX_EDGE, FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    let (w, h) = rgba.dimensions();

    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);
    let webp_bytes = encoder.encode(THUMB_QUALITY);

    std::fs::write(dst_webp, &*webp_bytes).map_err(|e| ThumbnailError::WriteWebp(e.to_string()))?;

    Ok(())
}
