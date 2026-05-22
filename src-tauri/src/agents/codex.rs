//! OpenAI Codex CLI adapter.
//!
//! Verified Session-3 invocation:
//!   codex exec --skip-git-repo-check --image <FILE>
//!              --output-last-message <FILE> -- "<prompt>"
//!
//! `--skip-git-repo-check` is required because the snip-staging dir is
//! not a git repo. `--output-last-message` writes a clean assistant-only
//! reply (no session header/footer), which we prefer over stdout.

use crate::agents::registry::{build_command_args, CODEX_ID};

pub fn build_args(image_path: &str, prompt: &str, last_message_file: &str) -> Vec<String> {
    build_command_args(CODEX_ID, image_path, prompt, Some(last_message_file))
}

pub fn binary_name() -> &'static str {
    "codex"
}
