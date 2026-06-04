mod consistency;
pub mod dispatcher;

// PDF rasterization currently has a CoreGraphics-only implementation.
// Non-macOS targets pick up a stub with the same public surface so the
// rest of the OCR pipeline compiles; PDF OCR on those platforms surfaces
// a clear "not supported yet" error to the caller.
#[cfg(target_os = "macos")]
pub mod pdf_render;
#[cfg(not(target_os = "macos"))]
#[path = "pdf_render_stub.rs"]
pub mod pdf_render;

pub mod postprocess;
pub mod prompt;
pub mod smart_format;
pub mod tabular;
mod tabular_complex_grid;

pub use consistency::validate_rerun_consistency;
pub use dispatcher::{
    run_ocr, run_ocr_pdf_page, run_pdf_cli, run_with_fallback, DispatchError, PDF_CLI_PAGE_TIMEOUT,
};
pub use postprocess::post_process;
pub use prompt::MASTER_PROMPT;
pub use smart_format::{detect_type, DetectedType};
pub use tabular::markdown_tables_to_latex_tabular;
