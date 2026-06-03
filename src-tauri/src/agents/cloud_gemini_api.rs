//! Gemini Vision API adapter — direct HTTP, BYOK.
//!
//! Free tier availability depends on Google account quota. Image is sent
//! as base64-inline data (no upload step). 30s timeout matches the CLI
//! dispatcher so the user sees consistent failure latency.
//!
//! Note: previously used `gemini-2.0-flash-exp`, but Google removed the
//! experimental endpoint when 2.0-flash went GA. `gemini-2.5-flash-lite`
//! is the current stable Flash-Lite model for cost-efficient text+image input.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CLOUD_GEMINI_MODEL: &str = "gemini-2.5-flash-lite";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum CloudGeminiError {
    /// HTTP 429. Google uses this for two distinct failure modes:
    ///   - transient per-minute throttling (RPM/TPM exceeded)
    ///   - permanent `limit: 0` when the project tied to a new
    ///     AI-Studio-issued key has no Gemini quota provisioned (common
    ///     with the newer `AQ.…` Express keys until billing is enabled
    ///     or quota propagates).
    /// Surface Google's own message so the user can tell the two apart
    /// instead of blindly retrying a permanent failure.
    #[error("rate limited or quota exhausted (HTTP 429): {0}")]
    RateLimited(String),
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
    Text {
        text: &'a str,
    },
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
            429 => CloudGeminiError::RateLimited(redact_key(&extract_error_message(&text))),
            400 => CloudGeminiError::BadRequest(redact_key(&text)),
            401 | 403 => CloudGeminiError::AuthFailed(status.as_u16()),
            code => CloudGeminiError::ServerError(code, redact_key(&text)),
        });
    }

    let parsed: GenerateContentResponse =
        serde_json::from_str(&text).map_err(|e| CloudGeminiError::Parse(e.to_string()))?;

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

/// Best-effort redaction of Google API key formats from error strings:
///   - legacy `AIza…` (39 chars total)
///   - newer Express-mode `AQ.Ab8…` (variable length, base64url body)
/// Belt-and-suspenders: Google's error JSON does not echo back the key,
/// but request URLs and stray logs sometimes do.
fn redact_key(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE_AIZA: OnceLock<Regex> = OnceLock::new();
    static RE_AQ: OnceLock<Regex> = OnceLock::new();
    let re_aiza = RE_AIZA.get_or_init(|| Regex::new(r"AIza[0-9A-Za-z_\-]{35}").unwrap());
    let re_aq = RE_AQ.get_or_init(|| Regex::new(r"AQ\.[0-9A-Za-z_\-]{40,}").unwrap());
    let step1 = re_aiza.replace_all(s, "AIza<redacted>");
    re_aq.replace_all(&step1, "AQ.<redacted>").to_string()
}

/// Pull the human-readable `error.message` out of Google's error JSON.
/// Falls back to the first 500 chars of the raw body if parsing fails,
/// so we still surface *something* useful instead of silently dropping
/// the upstream signal.
fn extract_error_message(body: &str) -> String {
    #[derive(Deserialize)]
    struct ErrorEnvelope {
        error: ErrorBody,
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        message: Option<String>,
    }
    if let Ok(env) = serde_json::from_str::<ErrorEnvelope>(body) {
        if let Some(msg) = env.error.message {
            return msg;
        }
    }
    body.chars().take(500).collect()
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
    fn redact_strips_express_aq_key_pattern() {
        let raw = "url ...?key=AQ.Ab8RN6KPRUC5zQUsyMhqNQTrmXvhIjSvrkDZxrIVpJ8ohbUYDg failed";
        let cleaned = redact_key(raw);
        assert!(!cleaned.contains("AQ.Ab8RN6"));
        assert!(cleaned.contains("AQ.<redacted>"));
    }

    #[test]
    fn extract_error_message_pulls_google_error_field() {
        let body = r#"{"error":{"code":429,"message":"Quota exceeded for metric X","status":"RESOURCE_EXHAUSTED"}}"#;
        let msg = extract_error_message(body);
        assert_eq!(msg, "Quota exceeded for metric X");
    }

    #[test]
    fn extract_error_message_falls_back_to_raw_body() {
        let body = "<html>500 Internal Server Error</html>";
        let msg = extract_error_message(body);
        assert_eq!(msg, body);
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
