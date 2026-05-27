//! Gemini Vision API adapter — direct HTTP, BYOK.
//!
//! Free tier: 15 RPM / 1500 RPD on `gemini-2.0-flash-exp`. Image is sent
//! as base64-inline data (no upload step). 30s timeout matches the CLI
//! dispatcher so the user sees consistent failure latency.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CLOUD_GEMINI_MODEL: &str = "gemini-2.0-flash-exp";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum CloudGeminiError {
    #[error("rate limited (HTTP 429)")]
    RateLimited,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("auth failed (HTTP {0})")]
    AuthFailed(u16),
    #[error("server error (HTTP {0}): {1}")]
    ServerError(u16, String),
    #[error("network error: {0}")]
    Network(String),
    #[error("empty response")]
    EmptyResponse,
    #[error("response parse error: {0}")]
    Parse(String),
}

#[derive(Serialize)]
struct GenerateContentRequest<'a> {
    contents: [RequestContent<'a>; 1],
}

#[derive(Serialize)]
struct RequestContent<'a> {
    parts: Vec<RequestPart<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum RequestPart<'a> {
    Text { text: &'a str },
    Inline {
        #[serde(rename = "inline_data")]
        inline_data: InlineData,
    },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

fn endpoint(api_key: &str) -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{CLOUD_GEMINI_MODEL}:generateContent?key={api_key}"
    )
}

fn mime_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

/// Send `image_bytes` + `prompt` to the Gemini Vision API and return the
/// raw text from the first candidate part. The dispatcher pipes the
/// result through `post_process` so we don't double-clean here.
pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudGeminiError> {
    call_with_timeout(image_bytes, mime_type, prompt, api_key, REQUEST_TIMEOUT).await
}

async fn call_with_timeout(
    image_bytes: &[u8],
    mime_type: &str,
    prompt: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<String, CloudGeminiError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| CloudGeminiError::Network(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let body = GenerateContentRequest {
        contents: [RequestContent {
            parts: vec![
                RequestPart::Text { text: prompt },
                RequestPart::Inline {
                    inline_data: InlineData {
                        mime_type: mime_type.to_string(),
                        data: encoded,
                    },
                },
            ],
        }],
    };

    let resp = client
        .post(endpoint(api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudGeminiError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CloudGeminiError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            429 => CloudGeminiError::RateLimited,
            400 => CloudGeminiError::BadRequest(redact_key(&text)),
            401 | 403 => CloudGeminiError::AuthFailed(status.as_u16()),
            code => CloudGeminiError::ServerError(code, redact_key(&text)),
        });
    }

    let parsed: GenerateContentResponse = serde_json::from_str(&text)
        .map_err(|e| CloudGeminiError::Parse(e.to_string()))?;

    parsed
        .candidates
        .and_then(|mut c| c.drain(..).next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|mut p| p.drain(..).next())
        .and_then(|p| p.text)
        .filter(|t| !t.is_empty())
        .ok_or(CloudGeminiError::EmptyResponse)
}

/// Convenience wrapper for the dispatcher: read the image off disk and
/// call the API. Image bytes never leave this function on the failure
/// path, so a network error cannot leak pixel data into logs.
pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudGeminiError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudGeminiError::Network(format!("read image: {e}")))?;
    call(&bytes, mime_for(image_path), prompt, api_key).await
}

/// Convenience wrapper for PDF files: read the PDF off disk and call the
/// API with `application/pdf` mime. Gemini processes all pages in one shot.
/// Timeout scales with page count (30s per page).
pub async fn call_with_pdf_path(
    pdf_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudGeminiError> {
    let pages = crate::ocr::pdf_render::page_count(pdf_path)
        .unwrap_or(1)
        .max(1);
    let timeout = Duration::from_secs(pages as u64 * 30);
    let bytes = tokio::fs::read(pdf_path)
        .await
        .map_err(|e| CloudGeminiError::Network(format!("read pdf: {e}")))?;
    call_with_timeout(&bytes, "application/pdf", prompt, api_key, timeout).await
}

/// Best-effort redaction of an `AIza...` Google API key from error
/// strings so we can attach upstream messages without leaking secrets.
fn redact_key(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"AIza[0-9A-Za-z_\-]{35}").unwrap());
    re.replace_all(s, "AIza<redacted>").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_strips_api_key_pattern() {
        let raw = "error 400: AIzaSyD1234567890abcdefghijklmnopqrstuvwxy is invalid";
        let cleaned = redact_key(raw);
        assert!(!cleaned.contains("AIzaSyD1234567890"));
        assert!(cleaned.contains("AIza<redacted>"));
    }

    #[test]
    fn mime_resolution_handles_common_extensions() {
        assert_eq!(mime_for("snap.png"), "image/png");
        assert_eq!(mime_for("snap.PNG"), "image/png");
        assert_eq!(mime_for("snap.jpg"), "image/jpeg");
        assert_eq!(mime_for("snap.jpeg"), "image/jpeg");
        assert_eq!(mime_for("snap.webp"), "image/webp");
        assert_eq!(mime_for("snap.unknown"), "image/png");
        assert_eq!(mime_for("document.pdf"), "application/pdf");
        assert_eq!(mime_for("document.PDF"), "application/pdf");
    }
}
