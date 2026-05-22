use serde::Serialize;

use crate::agents::{
    self, keychain,
    registry::{AgentInfo, AgentSpec, CLOUD_GEMINI_ID},
};
use crate::ocr::{self, dispatcher::DispatchError, smart_format::DetectedType};

#[derive(Serialize)]
pub struct HelloReply {
    pub message: String,
    pub version: &'static str,
}

#[tauri::command]
pub fn hello(name: Option<String>) -> HelloReply {
    let who = name.unwrap_or_else(|| "world".to_string());
    HelloReply {
        message: format!("Hello, {who}! SnipTeX backend is alive."),
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub async fn detect_agents() -> Result<Vec<AgentInfo>, String> {
    tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_agent(agent_id: String, image_path: String) -> Result<TestAgentReport, String> {
    let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| e.to_string())?;
    let agent = installed
        .into_iter()
        .find(|a| a.spec.id == agent_id)
        .ok_or_else(|| format!("agent not installed: {agent_id}"))?;

    let text = ocr::run_ocr(&agent, &image_path)
        .await
        .map_err(stringify_dispatch_error)?;
    let detected = ocr::detect_type(&text);
    Ok(TestAgentReport {
        ok: true,
        detected,
        char_count: text.chars().count(),
        preview: preview(&text, 200),
    })
}

#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<(), String> {
    match provider.as_str() {
        "gemini" => keychain::set_gemini_api_key(&key).map_err(|e| e.to_string()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn has_api_key(provider: String) -> Result<bool, String> {
    match provider.as_str() {
        "gemini" => Ok(keychain::has_gemini_api_key()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    match provider.as_str() {
        "gemini" => keychain::delete(keychain::GEMINI_ACCOUNT).map_err(|e| e.to_string()),
        other => Err(format!("unsupported provider: {other}")),
    }
}

/// Stub for Phase 4: image source / capture is not implemented yet, so
/// this currently just resolves the chosen agent and reports back. The
/// real wiring (capture → temp PNG → run_with_fallback) lands in
/// Phase 4 once the screen-capture surface exists.
#[tauri::command]
pub async fn run_snip(agent_id: Option<String>) -> Result<RunSnipReport, String> {
    let installed = tokio::task::spawn_blocking(agents::detect_installed_agents)
        .await
        .map_err(|e| e.to_string())?;
    let chosen: Option<AgentSpec> = match agent_id {
        Some(id) => installed
            .iter()
            .find(|a| a.spec.id == id)
            .map(|a| a.spec.clone()),
        None => installed.first().map(|a| a.spec.clone()),
    };
    Ok(RunSnipReport {
        status: "pending_capture".to_string(),
        agent: chosen.map(|s| s.id.to_string()),
        message: "screen capture lands in Phase 4".to_string(),
    })
}

#[derive(Serialize)]
pub struct TestAgentReport {
    pub ok: bool,
    pub detected: DetectedType,
    pub char_count: usize,
    pub preview: String,
}

#[derive(Serialize)]
pub struct RunSnipReport {
    pub status: String,
    pub agent: Option<String>,
    pub message: String,
}

fn stringify_dispatch_error(e: DispatchError) -> String {
    // DispatchError already redacts API keys via cloud_gemini_api::redact_key
    // before it constructs BadRequest/ServerError, so Display is safe here.
    e.to_string()
}

fn preview(s: &str, max_chars: usize) -> String {
    let collapsed: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if collapsed.chars().count() <= max_chars {
        collapsed
    } else {
        let head: String = collapsed.chars().take(max_chars).collect();
        format!("{head}...")
    }
}

// Compile-time guard: cloud agent id stays exported from registry.
const _: &str = CLOUD_GEMINI_ID;
