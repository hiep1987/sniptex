use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::agents::local_ocr_cache::{LocalHealthCache, LocalHealthStatus};
use crate::agents::registry::{
    spec_by_id, AgentInfo, LOCAL_FAST_ID, LOCAL_PADDLEOCR_ID, LOCAL_PIX2TEX_ID,
};
use crate::settings::{validate_local_ocr_url, AppSettings};

const HEALTH_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
struct HealthResponse {
    ok: bool,
    version: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
}

pub async fn detect_ready_agents(settings: &AppSettings) -> Vec<AgentInfo> {
    if !settings.local_ocr_enabled || validate_local_ocr_url(&settings.local_ocr_url).is_err() {
        return Vec::new();
    }

    let health = health_with_cache(&settings.local_ocr_url).await;
    if !health.healthy {
        return Vec::new();
    }

    agents_for_capabilities(settings, &health)
}

pub async fn health_with_cache(url: &str) -> LocalHealthStatus {
    let now = now_ms();
    if let Some(status) = cache().lock().unwrap().get(url, now) {
        return status;
    }

    let status = probe_health(url).await;
    cache().lock().unwrap().update(url, status.clone(), now_ms());
    status
}

async fn probe_health(url: &str) -> LocalHealthStatus {
    if validate_local_ocr_url(url).is_err() {
        return LocalHealthStatus::unhealthy();
    }

    let endpoint = format!("{}/health", url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(HEALTH_TIMEOUT)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(_) => return LocalHealthStatus::unhealthy(),
    };

    let response = match client.get(endpoint).send().await {
        Ok(response) => response,
        Err(_) => return LocalHealthStatus::unhealthy(),
    };
    if !response.status().is_success() {
        return LocalHealthStatus::unhealthy();
    }

    parse_health_response(response).await
}

async fn parse_health_response(response: reqwest::Response) -> LocalHealthStatus {
    match response.json::<HealthResponse>().await {
        Ok(body) if body.ok => LocalHealthStatus {
            healthy: true,
            version: body.version,
            capabilities: normalize_capabilities(body.capabilities),
        },
        _ => LocalHealthStatus::unhealthy(),
    }
}

pub fn agents_for_capabilities(
    settings: &AppSettings,
    health: &LocalHealthStatus,
) -> Vec<AgentInfo> {
    let caps: HashSet<&str> = health.capabilities.iter().map(String::as_str).collect();
    let mut ids = Vec::new();
    if settings.local_ocr_formula_enabled && caps.contains("pix2tex") {
        ids.push(LOCAL_PIX2TEX_ID);
    }
    if settings.local_ocr_text_enabled && caps.contains("paddleocr") {
        ids.push(LOCAL_PADDLEOCR_ID);
    }
    if caps.contains("classifier")
        && settings.local_ocr_formula_enabled
        && settings.local_ocr_text_enabled
        && caps.contains("pix2tex")
        && caps.contains("paddleocr")
    {
        ids.push(LOCAL_FAST_ID);
    }
    ids.into_iter()
        .filter_map(|id| {
            spec_by_id(id).map(|spec| AgentInfo {
                spec: spec.clone(),
                binary_path: PathBuf::from(&settings.local_ocr_url),
                version: health.version.clone(),
            })
        })
        .collect()
}

fn normalize_capabilities(capabilities: Vec<String>) -> Vec<String> {
    capabilities
        .into_iter()
        .map(|cap| cap.trim().to_ascii_lowercase())
        .filter(|cap| !cap.is_empty())
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn cache() -> &'static Mutex<LocalHealthCache> {
    static CACHE: OnceLock<Mutex<LocalHealthCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(LocalHealthCache::default()))
}
