//! Async OCR dispatcher.
//!
//! Branches on `AgentKind`:
//!   * `CliBin` — spawns the binary with `kill_on_drop(true)` and a 90s budget.
//!     Codex captures clean output via `--output-last-message`; Gemini CLI
//!     emits JSON and we consume only `.response`.
//!   * `CloudApi` — calls the in-process HTTPS adapter.
//!
//! Output is always run through `post_process`; empty / `[UNREADABLE]`
//! is mapped to `EmptyOutput` so callers can trigger fallback.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use crate::agents::cloud_gemini_api::{self, CloudGeminiError};
use crate::agents::cloud_mistral_api::{self, CloudMistralError};
use crate::agents::keychain;
use crate::agents::registry::{
    build_command_args, AgentInfo, AgentKind, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID, CODEX_ID,
    GEMINI_CLI_ID,
};
use crate::ocr::postprocess::post_process;
use crate::ocr::prompt::{GEMINI_CLI_PROMPT, MASTER_PROMPT};

const DISPATCH_TIMEOUT: Duration = Duration::from_secs(30);
/// Per-page budget when a CLI agent processes a PDF page. CLI agents
/// (codex, gemini-cli) are 5-10× slower than cloud APIs on a full
/// page-sized image — empirically 60-90s on a 200dpi page — so the
/// snip-tight 30s would always time out before the page completed.
pub const PDF_CLI_PAGE_TIMEOUT: Duration = Duration::from_secs(120);

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

impl From<CloudMistralError> for DispatchError {
    fn from(e: CloudMistralError) -> Self {
        match e {
            CloudMistralError::RateLimited => DispatchError::RateLimited,
            CloudMistralError::BadRequest(m) => DispatchError::BadRequest(m),
            CloudMistralError::AuthFailed(c) => DispatchError::AuthFailed(c),
            CloudMistralError::ServerError(c, m) => DispatchError::NonZeroExit {
                code: c as i32,
                stderr: m,
            },
            CloudMistralError::Network(m) => DispatchError::Network(m),
            CloudMistralError::EmptyResponse => DispatchError::EmptyOutput,
            CloudMistralError::Parse(m) => DispatchError::BadRequest(m),
        }
    }
}

pub async fn run_ocr(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
    match agent.spec.kind {
        AgentKind::CliBin => run_cli_agent(agent, image_path, DISPATCH_TIMEOUT).await,
        AgentKind::CloudApi => run_cloud_agent(agent, image_path).await,
    }
}

/// PDF page variant of `run_ocr`. Uses `PDF_CLI_PAGE_TIMEOUT` for CLI
/// agents to allow the longer per-image OCR latency they need on
/// full PDF pages. Cloud agents are unaffected (their HTTP timeout is
/// applied at the API adapter level).
pub async fn run_ocr_pdf_page(
    agent: &AgentInfo,
    image_path: &str,
) -> Result<String, DispatchError> {
    match agent.spec.kind {
        AgentKind::CliBin => run_cli_agent(agent, image_path, PDF_CLI_PAGE_TIMEOUT).await,
        AgentKind::CloudApi => run_cloud_agent(agent, image_path).await,
    }
}

/// OCR a PDF via a CLI agent: render each page to a temp PNG, run OCR on
/// each sequentially, and concatenate results. Temp dir cleaned up on all
/// exit paths via RAII. Overall budget scales to `pages * DISPATCH_TIMEOUT`
/// — per-page enforcement still lives in `run_cli_agent`; the outer wrap
/// guards against runaway iteration if a per-page error path stalled.
pub async fn run_pdf_cli(
    agent: &AgentInfo,
    pdf_path: &str,
) -> Result<String, DispatchError> {
    let tmp = TempDir::new(staging_path(&format!(
        "pdf-pages-{}",
        Uuid::new_v4()
    )))?;

    let page_pngs = crate::ocr::pdf_render::render_pages_to_pngs(
        pdf_path,
        tmp.path(),
        None,
    )
    .map_err(|e| DispatchError::Io(e.to_string()))?;

    if page_pngs.is_empty() {
        return Err(DispatchError::EmptyOutput);
    }

    let total = page_pngs.len();
    let overall_budget = PDF_CLI_PAGE_TIMEOUT.saturating_mul(total as u32);

    let work = async {
        let mut parts: Vec<String> = Vec::with_capacity(total);
        for (i, png) in page_pngs.iter().enumerate() {
            let path_str = png.to_string_lossy();
            match run_cli_agent(agent, &path_str, PDF_CLI_PAGE_TIMEOUT).await {
                Ok(text) => parts.push(text),
                Err(e) => {
                    log::warn!("[pdf-cli] page {} failed: {e}", i + 1);
                    return Err(e);
                }
            }
        }
        Ok(parts)
    };

    let parts = match timeout(overall_budget, work).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err(DispatchError::Timeout(overall_budget.as_secs())),
    };

    let combined = parts.join("\n\n");
    if combined.is_empty() {
        return Err(DispatchError::EmptyOutput);
    }
    Ok(combined)
}

async fn run_cli_agent(
    agent: &AgentInfo,
    image_path: &str,
    cmd_timeout: Duration,
) -> Result<String, DispatchError> {
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
    let gemini_cwd_guard = match agent.spec.id {
        GEMINI_CLI_ID => Some(TempDir::new(staging_path(&format!(
            "gemini-cwd-{}",
            Uuid::new_v4()
        )))?),
        _ => None,
    };
    let prompt = match agent.spec.id {
        GEMINI_CLI_ID => GEMINI_CLI_PROMPT,
        _ => MASTER_PROMPT,
    };

    let args = build_command_args(
        agent.spec.id,
        image_path,
        prompt,
        last_msg_str.as_deref(),
    );
    let mut cmd = Command::new(&agent.binary_path);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(cwd) = &gemini_cwd_guard {
        cmd.current_dir(cwd.path());
    }

    let output = match timeout(cmd_timeout, cmd.output()).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(DispatchError::Io(e.to_string())),
        Err(_) => return Err(DispatchError::Timeout(cmd_timeout.as_secs())),
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
    } else if agent.spec.id == GEMINI_CLI_ID {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        log::debug!("[gemini-cli] raw stdout before guard: {}", stdout.trim());
        if looks_like_gemini_tool_error(&stdout) {
            return Err(DispatchError::BadRequest(
                "gemini-cli response contains tool execution error".into(),
            ));
        }
        stdout
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let cleaned = post_process(&raw);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(DispatchError::EmptyOutput);
    }
    Ok(cleaned)
}

#[derive(Debug, Deserialize)]
struct GeminiCliJsonResponse {
    response: Option<String>,
    error: Option<serde_json::Value>,
    stats: Option<GeminiCliStats>,
}

#[derive(Debug, Deserialize)]
struct GeminiCliStats {
    tools: Option<GeminiCliToolStats>,
}

#[derive(Debug, Deserialize)]
struct GeminiCliToolStats {
    #[serde(rename = "totalCalls", alias = "total_calls")]
    total_calls: Option<u64>,
    #[serde(rename = "totalFail", alias = "total_fail")]
    total_fail: Option<u64>,
    #[serde(rename = "byName", alias = "by_name")]
    by_name: Option<HashMap<String, GeminiCliNamedToolStats>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCliNamedToolStats {
    count: Option<u64>,
    fail: Option<u64>,
}

pub fn parse_gemini_cli_json_response(stdout: &str) -> Result<String, DispatchError> {
    let parsed: GeminiCliJsonResponse = serde_json::from_str(stdout.trim()).map_err(|e| {
        DispatchError::BadRequest(format!("gemini-cli returned invalid JSON: {e}"))
    })?;
    if let Some(error) = parsed.error {
        return Err(DispatchError::BadRequest(format!(
            "gemini-cli error: {}",
            format_json_error(&error)
        )));
    }
    if !gemini_tool_usage_is_allowed(&parsed) {
        return Err(DispatchError::BadRequest(
            "gemini-cli used tools during OCR-only call".into(),
        ));
    }
    let response = parsed
        .response
        .ok_or_else(|| DispatchError::BadRequest("gemini-cli JSON missing response".into()))?;
    if response.trim().is_empty() {
        return Err(DispatchError::EmptyOutput);
    }
    if looks_like_gemini_tool_error(&response) {
        return Err(DispatchError::BadRequest(
            "gemini-cli response contains tool execution error".into(),
        ));
    }
    Ok(response)
}

fn gemini_tool_usage_is_allowed(response: &GeminiCliJsonResponse) -> bool {
    let Some(tools) = response.stats.as_ref().and_then(|s| s.tools.as_ref()) else {
        return true;
    };
    let total_calls = tools.total_calls.unwrap_or(0);
    if total_calls == 0 {
        return true;
    }
    if tools.total_fail.unwrap_or(0) > 0 {
        return false;
    }
    let read_file = tools
        .by_name
        .as_ref()
        .and_then(|by_name| by_name.get("read_file"));
    let read_file_count = read_file.and_then(|stats| stats.count).unwrap_or(0);
    let read_file_fail = read_file.and_then(|stats| stats.fail).unwrap_or(0);

    total_calls == read_file_count && read_file_fail == 0
}

fn format_json_error(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

pub fn looks_like_gemini_tool_error(response: &str) -> bool {
    let lower = response.to_lowercase();
    response.contains("Error executing tool")
        || lower.contains("path not in workspace")
        || (lower.contains("default_api_")
            && (lower.contains("error")
                || lower.contains("failed")
                || lower.contains("failure")))
        || lower.contains("error executing tool read_file")
        || lower.contains("error executing tool write_file")
        || lower.contains("attempted path")
}

async fn run_cloud_agent(agent: &AgentInfo, image_path: &str) -> Result<String, DispatchError> {
    let raw = match agent.spec.id {
        CLOUD_GEMINI_ID => {
            let key = keychain::get_gemini_api_key()
                .map_err(|_| DispatchError::MissingApiKey("gemini"))?;
            cloud_gemini_api::call_with_image_path(image_path, MASTER_PROMPT, &key).await?
        }
        CLOUD_MISTRAL_ID => {
            let key = keychain::get_mistral_api_key()
                .map_err(|_| DispatchError::MissingApiKey("mistral"))?;
            cloud_mistral_api::call_with_image_path(image_path, MASTER_PROMPT, &key).await?
        }
        _ => return Err(DispatchError::AgentNotAvailable(agent.spec.id.to_string())),
    };
    let cleaned = post_process(&raw);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(DispatchError::EmptyOutput);
    }
    Ok(cleaned)
}

/// Try `agents` in the given `priority` order, return the first `Ok`.
/// Last error is propagated so callers can surface "all agents failed".
pub async fn run_with_fallback<'a>(
    agents: &'a [AgentInfo],
    image_path: &str,
    priority: &[String],
) -> Result<(String, &'a AgentInfo), DispatchError> {
    let ordered = order_by_priority(agents, priority);
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

fn order_by_priority<'a>(agents: &'a [AgentInfo], priority: &[String]) -> Vec<&'a AgentInfo> {
    let mut ordered: Vec<&AgentInfo> = Vec::new();
    for id in priority {
        if let Some(a) = agents.iter().find(|a| a.spec.id == id) {
            ordered.push(a);
        }
    }
    // Append any installed agents not listed in priority.
    for a in agents {
        if !ordered.iter().any(|o| o.spec.id == a.spec.id) {
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

/// RAII temp-directory guard used to give Gemini CLI a neutral working
/// directory without exposing the project workspace as context.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(path: PathBuf) -> Result<Self, DispatchError> {
        std::fs::create_dir_all(&path)
            .map_err(|e| DispatchError::Io(format!("create temp dir {}: {e}", path.display())))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
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
