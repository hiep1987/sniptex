//! Static agent catalogue and per-agent CLI argv builder.
//!
//! Three agents ship in v1:
//!   1. `codex` — default OpenAI Codex CLI (BYOA).
//!   2. `gemini-cli` — experimental Gemini CLI gated with `--approval-mode plan`
//!      to prevent the tool-loop failure mode discovered in Phase 1.
//!   3. `cloud-gemini` — Gemini Vision API direct HTTP call (BYOK, free-tier
//!      15 RPM / 1500 RPD).
//!
//! Adding more (Claude Code, OpenCode) becomes a mechanical change here
//! plus a new adapter file under `agents/`.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AgentKind {
    /// External CLI binary spawned as a child process.
    CliBin,
    /// HTTPS call from inside the Tauri process.
    CloudApi,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub binary_names: &'static [&'static str],
    pub supports_vision: bool,
    pub kind: AgentKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub spec: AgentSpec,
    /// CLI agents: resolved binary path. Cloud agents: synthetic placeholder.
    pub binary_path: PathBuf,
    pub version: Option<String>,
}

pub const CODEX_ID: &str = "codex";
pub const GEMINI_CLI_ID: &str = "gemini-cli";
pub const CLOUD_GEMINI_ID: &str = "cloud-gemini";

pub const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        id: CODEX_ID,
        display_name: "OpenAI Codex",
        binary_names: &["codex"],
        supports_vision: true,
        kind: AgentKind::CliBin,
    },
    AgentSpec {
        id: GEMINI_CLI_ID,
        display_name: "Gemini CLI (experimental)",
        binary_names: &["gemini"],
        supports_vision: true,
        kind: AgentKind::CliBin,
    },
    AgentSpec {
        id: CLOUD_GEMINI_ID,
        display_name: "Gemini Vision API",
        binary_names: &[],
        supports_vision: true,
        kind: AgentKind::CloudApi,
    },
];

/// Default fallback order: Codex first (most reliable per Session-3),
/// then cloud Gemini (no local install required), then CLI Gemini.
pub const DEFAULT_FALLBACK_CHAIN: &[&str] =
    &[CODEX_ID, CLOUD_GEMINI_ID, GEMINI_CLI_ID];

pub fn spec_by_id(id: &str) -> Option<&'static AgentSpec> {
    AGENTS.iter().find(|a| a.id == id)
}

/// CLI argv builder. Cloud agents are handled directly by the dispatcher,
/// so this returns an empty vector for them — callers must branch on
/// `AgentKind` first.
pub fn build_command_args(
    agent_id: &str,
    image_path: &str,
    prompt: &str,
    last_message_file: Option<&str>,
) -> Vec<String> {
    match agent_id {
        CODEX_ID => {
            // Session-3-verified syntax:
            //   codex exec --skip-git-repo-check --image <FILE>
            //              --output-last-message <FILE> -- "<prompt>"
            let mut args: Vec<String> = vec![
                "exec".into(),
                "--skip-git-repo-check".into(),
                "--image".into(),
                image_path.into(),
            ];
            if let Some(last) = last_message_file {
                args.push("--output-last-message".into());
                args.push(last.into());
            }
            args.push("--".into());
            args.push(prompt.into());
            args
        }
        GEMINI_CLI_ID => {
            // `--approval-mode plan` makes the agent loop read-only, which
            // prevents the `read_file ~/.claude/.ck.json` failure surfaced
            // in Phase 1. Gemini CLI 0.42 made `--yolo` and `--approval-mode`
            // mutually exclusive, so plan-mode is the entire safety contract.
            vec![
                "-p".into(),
                format!("{prompt}\n@\"{image_path}\""),
                "--approval-mode".into(),
                "plan".into(),
            ]
        }
        CLOUD_GEMINI_ID => Vec::new(),
        other => panic!("Unknown agent id: {other}"),
    }
}
