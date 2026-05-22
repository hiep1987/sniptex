//! Gemini CLI adapter — experimental, gated.
//!
//! Phase 1 surfaced a systemic failure: when the image has no surrounding
//! text Gemini's tool loop tries `read_file ~/.claude/.ck.json` and the
//! whole call collapses. Spawning with `--approval-mode plan` makes the
//! tool loop read-only, which sidesteps the failure mode while keeping
//! vision intact. Verified in Phase 3 cli_test.

use crate::agents::registry::{build_command_args, GEMINI_CLI_ID};

pub fn build_args(image_path: &str, prompt: &str) -> Vec<String> {
    build_command_args(GEMINI_CLI_ID, image_path, prompt, None)
}

pub fn binary_name() -> &'static str {
    "gemini"
}
