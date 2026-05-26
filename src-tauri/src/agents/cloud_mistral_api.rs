//! Mistral Vision API adapter -- direct HTTP, BYOK.
//!
//! Uses the OpenAI-compatible chat completions endpoint with base64 data
//! URLs. The dispatcher post-processes the raw model text.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CLOUD_MISTRAL_MODEL: &str = "mistral-small-latest";
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
struct ChatCompletionRequest<'a> {
    model: &'static str,
    messages: [Message<'a>; 1],
    max_tokens: u16,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'static str,
    content: Vec<ContentPart<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentPart<'a> {
    #[serde(rename = "text")]
    Text { text: &'a str },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: String },
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Option<Vec<Choice>>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

fn endpoint() -> &'static str {
    "https://api.mistral.ai/v1/chat/completions"
}

pub fn mime_for(image_path: &str) -> &'static str {
    let lower = image_path.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

pub fn parse_response(text: &str) -> Result<String, CloudMistralError> {
    let parsed: ChatCompletionResponse =
        serde_json::from_str(text).map_err(|e| CloudMistralError::Parse(e.to_string()))?;
    parsed
        .choices
        .and_then(|mut c| c.drain(..).next())
        .and_then(|c| c.message.content)
        .filter(|t| !t.is_empty())
        .ok_or(CloudMistralError::EmptyResponse)
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

/// Send `image_bytes` + `prompt` to Mistral Vision and return raw text.
pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudMistralError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| CloudMistralError::Network(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let body = ChatCompletionRequest {
        model: CLOUD_MISTRAL_MODEL,
        messages: [Message {
            role: "user",
            content: vec![
                ContentPart::Text { text: prompt },
                ContentPart::ImageUrl {
                    image_url: format!("data:{mime_type};base64,{encoded}"),
                },
            ],
        }],
        max_tokens: 4096,
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
    }
}
