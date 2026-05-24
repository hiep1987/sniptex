//! Async OCR dispatcher.
//!
//! Branches on `AgentKind`:
//!   * `CliBin` — spawns the binary with `kill_on_drop(true)` and a 90s budget.
//!     Codex captures clean output via `--output-last-message`; other CLIs use stdout.
//!   * `CloudApi` — calls the in-process HTTPS adapter.
//!
//! Output is always run through `post_process`; empty / `[UNREADABLE]`
//! is mapped to `EmptyOutput` so callers can trigger fallback.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use crate::agents::cloud_gemini_api::{self, CloudGeminiError};
use crate::agents::keychain;
use crate::agents::registry::{
    build_command_args, AgentInfo, AgentKind, CLOUD_GEMINI_ID, CODEX_ID, DEFAULT_FALLBACK_CHAIN,
    GEMINI_CLI_ID,
};
use crate::ocr::postprocess::post_process;
use crate::ocr::prompt::MASTER_PROMPT;

const DISPATCH_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("agent not available: {0}")]
    AgentNotAvailable(String),
    #[error("timeout after {0}s")]
    Timeout(u64),
    #[error("non-zero exit ({code}): {stderr}")]
    NonZeroExit { code: i32, stderr: String },
    #[error("empty output")]
    EmptyOutput,
    #[error("rate limited")]
    RateLimited,
    #[error("api auth failed (HTTP {0})")]
    AuthFailed(u16),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("missing api key for {0}")]
    MissingApiKey(&'static str),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for DispatchError {
    fn from(e: std::io::Error) -> Self {
        DispatchError::Io(e.to_string())
    }
}

impl From<CloudGeminiError> for DispatchError {
    fn from(e: CloudGeminiError) -> Self {
        match e {
            CloudGeminiError::RateLimited => DispatchError::RateLimited,
            CloudGeminiError::BadRequest(m) => DispatchError::BadRequest(m),
            CloudGeminiError::AuthFailed(c) => DispatchError::AuthFailed(c),
            CloudGeminiError::ServerError(c, m) => DispatchError::NonZeroExit {
                code: c as i32,
                stderr: m,
            },
            CloudGeminiError::Network(m) => DispatchError::Network(m),
            CloudGeminiError::EmptyResponse => DispatchError::EmptyOutput,
            CloudGeminiError::Parse(m) => DispatchError::BadRequest(m),
        }
    }
}

pub async fn run_ocr(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
    match agent.spec.kind {
        AgentKind::CliBin => run_cli_agent(agent, image_path).await,
        AgentKind::CloudApi => run_cloud_agent(agent, image_path).await,
    }
}

async fn run_cli_agent(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
    // RAII guard removes the temp file on every exit path — including
    // panic, future cancellation, and timeout — so we don't leak files
    // into the system temp dir.
    let last_msg_guard = match agent.spec.id {
        CODEX_ID => Some(TempFile::new(staging_path(&format!(
            "codex-{}.last.txt",
            Uuid::new_v4()
        )))),
        _ => None,
    };
    let last_msg_str = last_msg_guard
        .as_ref()
        .map(|g| g.path().to_string_lossy().to_string());

    let args = build_command_args(
        agent.spec.id,
        image_path,
        MASTER_PROMPT,
        last_msg_str.as_deref(),
    );

    let mut cmd = Command::new(&agent.binary_path);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if agent.spec.id == GEMINI_CLI_ID {
        if let Some(home) = dirs::home_dir() {
            cmd.current_dir(home);
        }
    }

    let output = match timeout(DISPATCH_TIMEOUT, cmd.output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(DispatchError::Io(e.to_string())),
        Err(_) => return Err(DispatchError::Timeout(DISPATCH_TIMEOUT.as_secs())),
    };

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        if looks_like_rate_limit(&stderr) {
            return Err(DispatchError::RateLimited);
        }
        return Err(DispatchError::NonZeroExit {
            code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    let raw = if let Some(guard) = &last_msg_guard {
        match tokio::fs::read_to_string(guard.path()).await {
            Ok(s) if !s.trim().is_empty() => s,
            _ => String::from_utf8_lossy(&output.stdout).to_string(),
        }
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let cleaned = post_process(&raw);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(DispatchError::EmptyOutput);
    }
    Ok(cleaned)
}

async fn run_cloud_agent(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
    if agent.spec.id != CLOUD_GEMINI_ID {
        return Err(DispatchError::AgentNotAvailable(agent.spec.id.to_string()));
    }
    let key = keychain::get_gemini_api_key()
        .map_err(|_| DispatchError::MissingApiKey("gemini"))?;

    let raw =
        cloud_gemini_api::call_with_image_path(image_path, MASTER_PROMPT, &key).await?;
    let cleaned = post_process(&raw);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(DispatchError::EmptyOutput);
    }
    Ok(cleaned)
}

/// Try `agents` in order, return the first `Ok`. Last error is propagated
/// so callers can surface "all agents failed: <reason>".
pub async fn run_with_fallback<'a>(
    agents: &'a [AgentInfo],
    image_path: &str,
) -> Result<(String, &'a AgentInfo), DispatchError> {
    let ordered = order_by_default_chain(agents);
    if ordered.is_empty() {
        return Err(DispatchError::AgentNotAvailable("<none installed>".into()));
    }
    let mut last_err: Option<DispatchError> = None;
    for agent in ordered {
        match run_ocr(agent, image_path).await {
            Ok(text) => return Ok((text, agent)),
            Err(e) => {
                eprintln!("[sniptex] agent {} failed: {}", agent.spec.id, e);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or(DispatchError::AgentNotAvailable("<none responded>".into())))
}

fn order_by_default_chain(agents: &[AgentInfo]) -> Vec<&AgentInfo> {
    let mut ordered: Vec<&AgentInfo> = Vec::new();
    for id in DEFAULT_FALLBACK_CHAIN {
        if let Some(a) = agents.iter().find(|a| &a.spec.id == id) {
            ordered.push(a);
        }
    }
    // Append any agents not in the default chain (e.g. future additions).
    for a in agents {
        if !ordered.iter().any(|x| x.spec.id == a.spec.id) {
            ordered.push(a);
        }
    }
    ordered
}

fn looks_like_rate_limit(stderr: &str) -> bool {
    // Tight test: require either the literal phrase OR a word-boundary 429
    // co-occurring with a rate-limit token. Prevents a stray "429" port
    // number / request id in stderr from flipping the fallback path.
    let lower = stderr.to_lowercase();
    if lower.contains("rate limit") || lower.contains("quota") || lower.contains("too many requests")
    {
        return true;
    }
    static RE_429: OnceLock<Regex> = OnceLock::new();
    let re = RE_429.get_or_init(|| Regex::new(r"\b429\b").unwrap());
    re.is_match(&lower)
        && (lower.contains("rate")
            || lower.contains("quota")
            || lower.contains("limit")
            || lower.contains("error"))
}

fn staging_path(file_name: &str) -> PathBuf {
    std::env::temp_dir().join("sniptex").join(file_name)
}

/// RAII temp-file guard. `Drop` runs even on panic / future cancellation,
/// preventing temp-dir leaks when the dispatcher exits abnormally.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { path }
    }
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_detects_literal_phrase() {
        assert!(looks_like_rate_limit("Error: Rate limit exceeded"));
        assert!(looks_like_rate_limit("API quota exhausted"));
        assert!(looks_like_rate_limit("HTTP 429 - too many requests"));
    }

    #[test]
    fn rate_limit_ignores_bare_429_substring() {
        // Was the H1 false-positive: "429" alone (port, id, build number)
        // must not trigger the rate-limit branch.
        assert!(!looks_like_rate_limit("connected to 127.0.0.1:4296"));
        assert!(!looks_like_rate_limit("build 429 of branch main"));
        assert!(!looks_like_rate_limit("crash at line 4290 in foo.rs"));
    }

    #[test]
    fn rate_limit_detects_429_with_context() {
        assert!(looks_like_rate_limit("HTTP error 429: please retry"));
        assert!(looks_like_rate_limit("rate exceeded (429)"));
    }
}
