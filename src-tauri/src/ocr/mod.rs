pub mod dispatcher;
mod consistency;
pub mod pdf_render;
pub mod postprocess;
pub mod prompt;
pub mod smart_format;

pub use consistency::validate_rerun_consistency;
pub use dispatcher::{
    run_ocr, run_ocr_pdf_page, run_pdf_cli, run_with_fallback, DispatchError,
    PDF_CLI_PAGE_TIMEOUT,
};
pub use postprocess::post_process;
pub use prompt::MASTER_PROMPT;
pub use smart_format::{detect_type, DetectedType};
