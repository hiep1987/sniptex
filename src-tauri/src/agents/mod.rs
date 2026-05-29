//! Agent registry, detection, and per-agent adapters.

pub mod cloud_gemini_api;
pub mod cloud_mistral_api;
pub mod cloud_vision_api;
pub mod codex;
pub mod gemini_cli;
pub mod keychain;
pub mod registry;

use registry::{
    AgentInfo, AgentKind, AgentSpec, AGENTS, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID, CLOUD_VISION_ID,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub use registry::AGENTS as AGENT_CATALOG;

const VERSION_PROBE_BUDGET: Duration = Duration::from_secs(2);

/// Scan the user's PATH plus a curated list of platform-specific
/// install dirs for each known CLI agent. Also surfaces `cloud-gemini`
/// as an "installed" agent if a Gemini API key is present in the
/// keychain.
pub fn detect_installed_agents() -> Vec<AgentInfo> {
    let dirs = candidate_dirs();
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };

    let mut results = Vec::new();
    for spec in AGENTS {
        match spec.kind {
            AgentKind::CliBin => {
                if let Some(info) = locate_cli_agent(spec, &dirs, exe_suffix) {
                    results.push(info);
                }
            }
            AgentKind::CloudApi => {
                if spec.id == CLOUD_GEMINI_ID && keychain::has_gemini_api_key() {
                    results.push(AgentInfo {
                        spec: spec.clone(),
                        binary_path: PathBuf::from("<cloud-api>"),
                        version: Some("v1beta".to_string()),
                    });
                }
                if spec.id == CLOUD_MISTRAL_ID && keychain::has_mistral_api_key() {
                    results.push(AgentInfo {
                        spec: spec.clone(),
                        binary_path: PathBuf::from("<cloud-api>"),
                        version: Some("v1".to_string()),
                    });
                }
                if spec.id == CLOUD_VISION_ID && keychain::has_cloud_vision_api_key() {
                    results.push(AgentInfo {
                        spec: spec.clone(),
                        binary_path: PathBuf::from("<cloud-api>"),
                        version: Some("v1".to_string()),
                    });
                }
            }
        }
    }
    results
}

fn locate_cli_agent(
    spec: &AgentSpec,
    dirs: &[PathBuf],
    exe_suffix: &str,
) -> Option<AgentInfo> {
    for bin_name in spec.binary_names {
        let full = format!("{bin_name}{exe_suffix}");
        for dir in dirs {
            let candidate = dir.join(&full);
            if is_executable(&candidate) {
                return Some(AgentInfo {
                    spec: spec.clone(),
                    binary_path: candidate.clone(),
                    version: detect_version(&candidate),
                });
            }
        }
    }
    None
}

fn candidate_dirs() -> Vec<PathBuf> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let path_env = env::var("PATH").unwrap_or_default();
    let mut dirs: Vec<PathBuf> = path_env.split(separator).map(PathBuf::from).collect();

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".local/share/mise/installs"));
        dirs.push(home.join(".bun/bin"));
        dirs.push(home.join(".cargo/bin"));
        dirs.push(home.join(".npm-global/bin"));
        dirs.push(home.join("AppData").join("Roaming").join("npm"));
    }
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }

    // Dedupe while preserving order.
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

#[cfg(unix)]
pub fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn is_executable(p: &Path) -> bool {
    p.is_file() && p.extension().map_or(false, |e| {
        let ext = e.to_string_lossy().to_ascii_lowercase();
        ext == "exe" || ext == "cmd" || ext == "bat"
    })
}

/// Run `<binary> --version` with a 2s budget. Best-effort: returns None
/// if the probe times out, errors, or produces nothing on stdout.
pub fn detect_version(binary: &Path) -> Option<String> {
    let start = Instant::now();
    let mut cmd = Command::new(binary);
    cmd.arg("--version");
    // We deliberately use blocking std::process here because detection
    // runs once at app start and we want it self-contained (no tokio
    // runtime required from callers like the Tauri setup hook).
    let output = cmd.output().ok()?;
    if start.elapsed() > VERSION_PROBE_BUDGET {
        return None;
    }
    let text = if !output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };
    let first_line = text.lines().next().unwrap_or("").trim().to_string();
    if first_line.is_empty() {
        None
    } else {
        Some(first_line)
    }
}
