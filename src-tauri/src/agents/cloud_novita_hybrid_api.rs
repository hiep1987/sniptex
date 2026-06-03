//! Novita hybrid OCR adapter: DeepSeek-OCR 2 markdown, then GPT OSS 120B cleanup.

use serde::Serialize;
use std::time::Duration;

use crate::agents::cloud_novita_api::{self, CloudNovitaError, CLOUD_NOVITA_MODEL};
use crate::agents::novita_hybrid_contract::GPT_OSS_ENDPOINT as GPT_ENDPOINT;
pub use crate::agents::novita_hybrid_contract::{
    normalize_intermediate_markdown, parse_gpt_oss_response, redact_key, redact_url_secrets,
    CloudNovitaHybridError, GPT_OSS_ENDPOINT, GPT_OSS_MODEL, MAX_GPT_TOKENS,
};

const GPT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
struct ChatRequest {
    model: &'static str,
    messages: [ChatMessage; 1],
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    Text { text: String },
}

pub fn has_required_config() -> bool {
    true
}

pub fn mime_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaHybridError> {
    let client = reqwest::Client::new();
    let markdown = cloud_novita_api::call(image_bytes, mime_type, prompt, api_key)
        .await
        .map_err(map_deepseek_error)?;
    let normalized = normalize_intermediate_markdown(&markdown);
    if normalized.is_empty() {
        return Err(CloudNovitaHybridError::EmptyResponse);
    }
    if !needs_gpt_cleanup(&normalized) {
        return Ok(normalized);
    }
    call_gpt_oss_cleanup(&client, &normalized, prompt, api_key).await
}

pub fn needs_gpt_cleanup(markdown: &str) -> bool {
    let lower = markdown.to_lowercase();
    lower.contains("\\textbackslash")
        || lower.contains("textbackslash")
        || lower.contains("\\begin") && lower.contains("\\text{end")
        || lower.contains("\\text{end{tabular}")
        || lower.contains("\\textbackslash{begin}")
        || lower.contains("\\textbackslash{hline}")
        || lower.contains("\\ &")
        || lower.contains("\\begin{tabular}") && suspicious_tabular_shape(markdown)
}

fn suspicious_tabular_shape(markdown: &str) -> bool {
    let separator_count = markdown.matches(" & \\").count() + markdown.matches("\\ &").count();
    separator_count >= 3
}

fn map_deepseek_error(err: CloudNovitaError) -> CloudNovitaHybridError {
    match err {
        CloudNovitaError::RateLimited => CloudNovitaHybridError::RateLimited,
        CloudNovitaError::BadRequest(m) => CloudNovitaHybridError::BadRequest(m),
        CloudNovitaError::AuthFailed(c) => CloudNovitaHybridError::AuthFailed(c),
        CloudNovitaError::ServerError(c, m) => CloudNovitaHybridError::ServerError(c, m),
        CloudNovitaError::Network(m) => CloudNovitaHybridError::Network(m),
        CloudNovitaError::EmptyResponse => CloudNovitaHybridError::EmptyResponse,
        CloudNovitaError::Parse(m) => CloudNovitaHybridError::Parse(m),
    }
}

async fn call_gpt_oss_cleanup(
    client: &reqwest::Client,
    markdown: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaHybridError> {
    let text = format!(
        "Fix OCR artifacts in this math/table OCR. Return only final LaTeX/Markdown. Preserve Vietnamese labels and answer choices. Do not invent content. Rebuild escaped LaTeX tables like \\textbackslash{{begin}}, \\textbackslash{{hline}}, or stray `\\ &`. Return [UNREADABLE] if insufficient.\n\nSource from {CLOUD_NOVITA_MODEL}:\n{markdown}"
    );
    let body = ChatRequest {
        model: GPT_OSS_MODEL,
        messages: [ChatMessage {
            role: "user",
            content: vec![ContentPart::Text { text }],
        }],
        max_tokens: MAX_GPT_TOKENS,
        temperature: 0.0,
    };
    let cleaned = post_json(client, GPT_ENDPOINT, api_key, &body, GPT_TIMEOUT)
        .await
        .and_then(|text| parse_gpt_oss_response(&text))?;
    if needs_gpt_cleanup(&cleaned) {
        return Err(CloudNovitaHybridError::BadRequest(
            "GPT cleanup left OCR artifacts".to_string(),
        ));
    }
    Ok(cleaned)
}

async fn post_json(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    body: &impl Serialize,
    timeout: Duration,
) -> Result<String, CloudNovitaHybridError> {
    let resp = client
        .post(endpoint)
        .bearer_auth(api_key)
        .timeout(timeout)
        .json(body)
        .send()
        .await
        .map_err(|e| redact_network_error(e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(redact_network_error)?;
    if status.is_success() {
        return Ok(text);
    }
    Err(match status.as_u16() {
        429 => CloudNovitaHybridError::RateLimited,
        401 | 403 => CloudNovitaHybridError::AuthFailed(status.as_u16()),
        code if status.is_server_error() => {
            CloudNovitaHybridError::ServerError(code, redact_key(&text))
        }
        code => CloudNovitaHybridError::BadRequest(format!("HTTP {code}: {}", redact_key(&text))),
    })
}

fn redact_network_error(err: reqwest::Error) -> CloudNovitaHybridError {
    let redacted = redact_url_secrets(&redact_key(&err.to_string()));
    CloudNovitaHybridError::Network(redacted)
}

pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaHybridError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudNovitaHybridError::Network(format!("read image: {e}")))?;
    call(&bytes, mime_for(image_path), prompt, api_key).await
}
