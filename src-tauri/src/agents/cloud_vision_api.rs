//! Google Cloud Vision API adapter — direct HTTP, BYOK.
//!
//! Uses `images:annotate` with `DOCUMENT_TEXT_DETECTION` feature. Unlike
//! Gemini/Mistral this endpoint returns PLAIN TEXT only — no LaTeX, no
//! Markdown structure. Math equations come out as plain Unicode/ASCII
//! and tables collapse to lines of text. Best for dense document text,
//! multi-language OCR, not math-heavy snips.
//!
//! Free tier: 1,000 requests / month. Paid tier billed per 1k images.
//! Requires the Cloud Vision API enabled in the Google Cloud project
//! tied to the API key.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum CloudVisionError {
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
struct AnnotateRequest<'a> {
    requests: [AnnotateRequestItem<'a>; 1],
}

#[derive(Serialize)]
struct AnnotateRequestItem<'a> {
    image: Image,
    features: [Feature<'a>; 1],
    #[serde(skip_serializing_if = "Option::is_none", rename = "imageContext")]
    image_context: Option<ImageContext<'a>>,
}

#[derive(Serialize)]
struct Image {
    content: String,
}

#[derive(Serialize)]
struct Feature<'a> {
    #[serde(rename = "type")]
    feature_type: &'a str,
}

#[derive(Serialize)]
struct ImageContext<'a> {
    #[serde(rename = "languageHints")]
    language_hints: &'a [&'a str],
}

#[derive(Deserialize)]
struct AnnotateResponse {
    responses: Option<Vec<AnnotateResponseItem>>,
}

#[derive(Deserialize)]
struct AnnotateResponseItem {
    #[serde(rename = "fullTextAnnotation")]
    full_text_annotation: Option<FullTextAnnotation>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct FullTextAnnotation {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    code: Option<i32>,
    message: Option<String>,
}

fn endpoint(api_key: &str) -> String {
    format!("https://vision.googleapis.com/v1/images:annotate?key={api_key}")
}

/// Send `image_bytes` to Cloud Vision API with `DOCUMENT_TEXT_DETECTION`
/// and return `fullTextAnnotation.text`. `prompt` is intentionally
/// ignored — Cloud Vision is a fixed-feature endpoint, not chat.
pub async fn call(
    image_bytes: &[u8],
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudVisionError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| CloudVisionError::Network(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let body = AnnotateRequest {
        requests: [AnnotateRequestItem {
            image: Image { content: encoded },
            features: [Feature {
                feature_type: "DOCUMENT_TEXT_DETECTION",
            }],
            image_context: Some(ImageContext {
                language_hints: &["vi", "en"],
            }),
        }],
    };

    let resp = client
        .post(endpoint(api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudVisionError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CloudVisionError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            429 => CloudVisionError::RateLimited,
            400 => CloudVisionError::BadRequest(redact_key(&text)),
            401 | 403 => CloudVisionError::AuthFailed(status.as_u16()),
            code => CloudVisionError::ServerError(code, redact_key(&text)),
        });
    }

    parse_response(&text)
}

pub fn parse_response(text: &str) -> Result<String, CloudVisionError> {
    let parsed: AnnotateResponse = serde_json::from_str(text)
        .map_err(|e| CloudVisionError::Parse(e.to_string()))?;
    let mut items = parsed.responses.ok_or(CloudVisionError::EmptyResponse)?;
    let first = items.drain(..).next().ok_or(CloudVisionError::EmptyResponse)?;
    if let Some(err) = first.error {
        if err.code.unwrap_or(0) != 0 {
            return Err(CloudVisionError::BadRequest(
                err.message.unwrap_or_else(|| "vision API error".into()),
            ));
        }
    }
    first
        .full_text_annotation
        .and_then(|a| a.text)
        .filter(|t| !t.is_empty())
        .ok_or(CloudVisionError::EmptyResponse)
}

pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudVisionError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudVisionError::Network(format!("read image: {e}")))?;
    call(&bytes, prompt, api_key).await
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
    fn parse_extracts_full_text_annotation() {
        let raw = r#"{"responses":[{"fullTextAnnotation":{"text":"hello world"}}]}"#;
        let out = parse_response(raw).unwrap();
        assert_eq!(out, "hello world");
    }

    #[test]
    fn parse_empty_responses_returns_error() {
        let raw = r#"{"responses":[]}"#;
        assert!(matches!(
            parse_response(raw),
            Err(CloudVisionError::EmptyResponse)
        ));
    }

    #[test]
    fn parse_api_error_propagates() {
        let raw = r#"{"responses":[{"error":{"code":7,"message":"PERMISSION_DENIED"}}]}"#;
        let err = parse_response(raw).unwrap_err();
        assert!(matches!(err, CloudVisionError::BadRequest(m) if m.contains("PERMISSION_DENIED")));
    }
}
