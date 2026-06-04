//! Non-macOS stub for the PDF rendering module.
//!
//! The real implementation lives in `pdf_render.rs` and depends on
//! CoreGraphics (macOS-only). On Windows / Linux this stub takes its place
//! so the OCR dispatcher and cloud agents still compile; every entrypoint
//! returns a clear "not supported on this platform yet" error so callers
//! can surface that to the user instead of silently failing.

use std::path::{Path, PathBuf};

const UNSUPPORTED_MSG: &str = "PDF OCR is not yet supported on this platform";

#[derive(Debug, thiserror::Error)]
pub enum PdfRenderError {
    #[error("failed to open PDF: {0}")]
    Open(String),
    #[error("page {0} out of range")]
    PageOutOfRange(usize),
    #[error("render failed for page {0}")]
    RenderFailed(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image encode error: {0}")]
    ImageEncode(String),
}

pub fn render_pages_to_pngs(
    _pdf_path: &str,
    _out_dir: &Path,
    _dpi: Option<f64>,
) -> Result<Vec<PathBuf>, PdfRenderError> {
    Err(PdfRenderError::Open(UNSUPPORTED_MSG.into()))
}

pub fn page_count(_pdf_path: &str) -> Result<usize, PdfRenderError> {
    Err(PdfRenderError::Open(UNSUPPORTED_MSG.into()))
}
