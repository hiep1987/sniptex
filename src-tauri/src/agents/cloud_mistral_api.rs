//! Mistral OCR API adapter -- direct HTTP, BYOK.
//!
//! Uses the dedicated `/v1/ocr` endpoint with `mistral-ocr-latest` model.
//! Priced per page (not per token). Image sent as base64 data URI via
//! `image_url` document type. Response returns `pages[].markdown`.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CLOUD_MISTRAL_MODEL: &str = "mistral-ocr-latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum CloudMistralError {
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
struct OcrRequest {
    model: &'static str,
    document: OcrDocument,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OcrDocument {
    Image {
        #[serde(rename = "type")]
        doc_type: &'static str,
        image_url: String,
    },
    Document {
        #[serde(rename = "type")]
        doc_type: &'static str,
        document_url: String,
    },
}

#[derive(Deserialize)]
struct OcrResponse {
    pages: Option<Vec<OcrPage>>,
}

#[derive(Deserialize)]
struct OcrPage {
    markdown: Option<String>,
}

fn endpoint() -> &'static str {
    "https://api.mistral.ai/v1/ocr"
}

pub fn mime_for(path: &str) -> &'static str {
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

pub fn parse_response(text: &str) -> Result<String, CloudMistralError> {
    let parsed: OcrResponse =
        serde_json::from_str(text).map_err(|e| CloudMistralError::Parse(e.to_string()))?;
    let pages = parsed.pages.ok_or(CloudMistralError::EmptyResponse)?;
    let combined: String = pages
        .into_iter()
        .filter_map(|p| p.markdown)
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if combined.is_empty() {
        return Err(CloudMistralError::EmptyResponse);
    }
    Ok(combined)
}

pub fn redact_key(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static BEARER_RE: OnceLock<Regex> = OnceLock::new();
    static BARE_KEY_RE: OnceLock<Regex> = OnceLock::new();
    let bearer_re =
        BEARER_RE.get_or_init(|| Regex::new(r"(?i)bearer\s+[0-9A-Za-z._\-]+").unwrap());
    let bare_key_re = BARE_KEY_RE.get_or_init(|| {
        Regex::new(r"\b(?:sk-[0-9A-Za-z._\-]{12,}|[0-9A-Za-z]{3,}_[0-9A-Za-z._\-]{20,})\b")
            .unwrap()
    });
    let without_bearer = bearer_re.replace_all(s, "Bearer <redacted>");
    bare_key_re
        .replace_all(&without_bearer, "<redacted-mistral-key>")
        .to_string()
}

/// Send image to Mistral OCR API and return the markdown text.
pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudMistralError> {
    call_with_timeout(image_bytes, mime_type, api_key, REQUEST_TIMEOUT).await
}

async fn call_with_timeout(
    image_bytes: &[u8],
    mime_type: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<String, CloudMistralError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| CloudMistralError::Network(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let document = if mime_type == "application/pdf" {
        OcrDocument::Document {
            doc_type: "document_url",
            document_url: format!("data:{mime_type};base64,{encoded}"),
        }
    } else {
        OcrDocument::Image {
            doc_type: "image_url",
            image_url: format!("data:{mime_type};base64,{encoded}"),
        }
    };
    let body = OcrRequest {
        model: CLOUD_MISTRAL_MODEL,
        document,
    };

    let resp = client
        .post(endpoint())
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudMistralError::Network(redact_key(&e.to_string())))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CloudMistralError::Network(redact_key(&e.to_string())))?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            429 => CloudMistralError::RateLimited,
            400 => CloudMistralError::BadRequest(redact_key(&text)),
            401 | 403 => CloudMistralError::AuthFailed(status.as_u16()),
            code if status.is_server_error() => {
                CloudMistralError::ServerError(code, redact_key(&text))
            }
            code => CloudMistralError::BadRequest(format!(
                "HTTP {code}: {}",
                redact_key(&text)
            )),
        });
    }

    parse_response(&text)
}

pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudMistralError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudMistralError::Network(format!("read image: {e}")))?;
    call(&bytes, mime_for(image_path), prompt, api_key).await
}

/// Timeout scales with page count (30s per page).
pub async fn call_with_pdf_path(
    pdf_path: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudMistralError> {
    let pages = crate::ocr::pdf_render::page_count(pdf_path)
        .unwrap_or(1)
        .max(1);
    let timeout = Duration::from_secs(pages as u64 * 30);
    let bytes = tokio::fs::read(pdf_path)
        .await
        .map_err(|e| CloudMistralError::Network(format!("read pdf: {e}")))?;
    call_with_timeout(&bytes, "application/pdf", api_key, timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_bearer_token() {
        let raw = "Authorization: Bearer sk-test-secret failed";
        let cleaned = redact_key(raw);
        assert_eq!(cleaned, "Authorization: Bearer <redacted> failed");
    }

    #[test]
    fn test_redact_lowercase_bearer_and_bare_key() {
        let raw = "auth bearer abc_1234567890123456789012345 raw abc_1234567890123456789012345";
        let cleaned = redact_key(raw);
        assert_eq!(
            cleaned,
            "auth Bearer <redacted> raw <redacted-mistral-key>"
        );
    }

    #[test]
    fn test_mime_for_common_extensions() {
        assert_eq!(mime_for("snap.png"), "image/png");
        assert_eq!(mime_for("snap.JPG"), "image/jpeg");
        assert_eq!(mime_for("snap.jpeg"), "image/jpeg");
        assert_eq!(mime_for("snap.webp"), "image/webp");
        assert_eq!(mime_for("snap.gif"), "image/png");
        assert_eq!(mime_for("document.pdf"), "application/pdf");
        assert_eq!(mime_for("document.PDF"), "application/pdf");
    }

    #[test]
    fn parse_response_concatenates_all_pages() {
        let json = r#"{"pages":[{"markdown":"page 1"},{"markdown":"page 2"},{"markdown":"page 3"}]}"#;
        let result = parse_response(json).unwrap();
        assert_eq!(result, "page 1\n\npage 2\n\npage 3");
    }

    #[test]
    fn parse_response_single_page() {
        let json = r#"{"pages":[{"markdown":"only page"}]}"#;
        let result = parse_response(json).unwrap();
        assert_eq!(result, "only page");
    }

    #[test]
    fn parse_response_skips_empty_pages() {
        let json = r#"{"pages":[{"markdown":"page 1"},{"markdown":""},{"markdown":"page 3"}]}"#;
        let result = parse_response(json).unwrap();
        assert_eq!(result, "page 1\n\npage 3");
    }

    #[test]
    fn parse_response_all_empty_returns_error() {
        let json = r#"{"pages":[{"markdown":""}]}"#;
        assert!(parse_response(json).is_err());
    }
}
