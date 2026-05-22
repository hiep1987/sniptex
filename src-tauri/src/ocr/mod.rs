pub mod dispatcher;
pub mod postprocess;
pub mod prompt;
pub mod smart_format;

pub use dispatcher::{run_ocr, run_with_fallback, DispatchError};
pub use postprocess::post_process;
pub use prompt::MASTER_PROMPT;
pub use smart_format::{detect_type, DetectedType};
