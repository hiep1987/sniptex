//! Goclaw cloud OCR agent adapter — HTTPS multipart upload + WebSocket chat.send.
//!
//! Two-step protocol per OCR call:
//!   1. POST `/api/v1/media/upload` (multipart/form-data) → returns absolute path
//!      that Goclaw will serve to the agent at runtime.
//!   2. WSS `/ws` → `connect` then `chat.send` referencing the uploaded path. The
//!      Goclaw agent (Phase 2: `tex-ocr`, provider `openai-codex-1`, model
//!      `gpt-5.4`) runs the Phase 1 `tex-ocr` skill server-side and returns the
//!      OCR'd LaTeX/Markdown.
//!
//! The `prompt` argument is intentionally ignored — Goclaw injects the OCR rule
//! via the skill's frontmatter `description` (see Phase 1 execution notes).
//!
//! One fresh WS connection per call. No connection pooling in v1: matches the
//! production `chatbot_goclaw.py` reference. The dispatcher's fallback chain
//! handles transient failures; this adapter does not retry internally.

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

pub const GOCLAW_API_BASE: &str = "https://goclaw.tikz2svg.com/api";
pub const GOCLAW_WS_URL: &str = "wss://goclaw.tikz2svg.com/ws";
pub const GOCLAW_AGENT_ID: &str = "tex-ocr";

const UPLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const CHAT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, thiserror::Error)]
pub enum CloudGoclawError {
    #[error("rate limited")]
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

#[derive(Deserialize)]
struct UploadResponse {
    path: String,
}

/// Upload `bytes` to `/v1/media/upload` and return the server-side path that
/// `chat.send` will reference. The path Goclaw returns is an absolute container
/// path (e.g. `/app/data/temp/<uuid>.<ext>`) — pass it through unchanged.
pub async fn upload_media(
    bytes: &[u8],
    mime_type: &str,
    filename: &str,
    api_key: &str,
) -> Result<String, CloudGoclawError> {
    let client = reqwest::Client::builder()
        .timeout(UPLOAD_TIMEOUT)
        .build()
        .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;

    let part = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name(filename.to_string())
        .mime_str(mime_type)
        .map_err(|e| CloudGoclawError::BadRequest(redact_key(&e.to_string())))?;
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{GOCLAW_API_BASE}/v1/media/upload"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            429 => CloudGoclawError::RateLimited,
            401 | 403 => CloudGoclawError::AuthFailed(status.as_u16()),
            400 | 413 => CloudGoclawError::BadRequest(redact_key(&text)),
            code if status.is_server_error() => {
                CloudGoclawError::ServerError(code, redact_key(&text))
            }
            code => CloudGoclawError::BadRequest(format!("HTTP {code}: {}", redact_key(&text))),
        });
    }

    let parsed: UploadResponse =
        serde_json::from_str(&text).map_err(|e| CloudGoclawError::Parse(e.to_string()))?;
    if parsed.path.is_empty() {
        return Err(CloudGoclawError::EmptyResponse);
    }
    Ok(parsed.path)
}

/// Open a WS, send `connect` then `chat.send` referencing the uploaded path,
/// wait for the matching `res` frame, return its `payload.content`.
pub async fn chat_with_media(
    uploaded_path: &str,
    basename: &str,
    api_key: &str,
) -> Result<String, CloudGoclawError> {
    let session_uuid = Uuid::new_v4();
    let user_id = format!("sniptex-{session_uuid}");
    let session_key = format!("tex-ocr:{session_uuid}");

    let result = timeout(CHAT_TIMEOUT, async {
        let (ws_stream, _) = connect_async(GOCLAW_WS_URL)
            .await
            .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;
        let (mut writer, mut reader) = ws_stream.split();

        let connect_req = serde_json::json!({
            "type": "req",
            "id": "1",
            "method": "connect",
            "params": { "token": api_key, "user_id": user_id }
        });
        writer
            .send(Message::Text(connect_req.to_string()))
            .await
            .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;

        let chat_req = serde_json::json!({
            "type": "req",
            "id": "2",
            "method": "chat.send",
            "params": {
                "agentId": GOCLAW_AGENT_ID,
                "sessionKey": session_key,
                "message": "",
                "media": [{ "path": uploaded_path, "filename": basename }]
            }
        });
        writer
            .send(Message::Text(chat_req.to_string()))
            .await
            .map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;

        // Read frames until the `res` matching id=2 arrives. Ignore evt frames,
        // pings/pongs, and the `connect` ack (id=1).
        while let Some(frame) = reader.next().await {
            let msg = frame.map_err(|e| CloudGoclawError::Network(redact_key(&e.to_string())))?;
            let text = match msg {
                Message::Text(t) => t,
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                    continue
                }
                Message::Close(_) => {
                    return Err(CloudGoclawError::Network(
                        "ws closed before chat response".into(),
                    ))
                }
            };
            if let Some(res) = parse_frame_for_chat_id(&text) {
                let _ = writer.close().await;
                return res;
            }
        }

        Err(CloudGoclawError::Network(
            "ws stream ended without chat response".into(),
        ))
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(CloudGoclawError::Network("chat timeout (120s)".into())),
    }
}

/// Inspect one frame. Returns `Some(...)` only when this is the `res` for
/// `id=2` (our chat.send). All other frames (`evt`, `res` for id=1, malformed)
/// are skipped by returning `None`, leaving the read loop to continue.
fn parse_frame_for_chat_id(text: &str) -> Option<Result<String, CloudGoclawError>> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("type").and_then(|t| t.as_str()) != Some("res") {
        return None;
    }
    if v.get("id").and_then(|i| i.as_str()) != Some("2") {
        return None;
    }
    Some(extract_chat_result(&v))
}

fn extract_chat_result(v: &serde_json::Value) -> Result<String, CloudGoclawError> {
    let ok = v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
    if !ok {
        let code = v
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let message = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        return Err(match code {
            "UNAUTHORIZED" => CloudGoclawError::AuthFailed(401),
            "RATE_LIMITED" => CloudGoclawError::RateLimited,
            "NOT_FOUND" => CloudGoclawError::BadRequest(format!("agent not found: {message}")),
            "INVALID_REQUEST" => CloudGoclawError::BadRequest(if message.is_empty() {
                "invalid request".into()
            } else {
                message
            }),
            "" => CloudGoclawError::Parse("error frame missing error.code".into()),
            other => CloudGoclawError::ServerError(500, format!("{other}: {message}")),
        });
    }
    let content = v
        .get("payload")
        .and_then(|p| p.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");
    if content.is_empty() {
        Err(CloudGoclawError::EmptyResponse)
    } else {
        Ok(content.to_string())
    }
}

/// Public helper for unit tests and other tools that need to interpret a
/// single chat response frame string.
pub fn parse_chat_response(text: &str) -> Result<String, CloudGoclawError> {
    parse_frame_for_chat_id(text).unwrap_or_else(|| {
        Err(CloudGoclawError::Parse(
            "frame is not a res matching the chat.send id".into(),
        ))
    })
}

/// Convenience: upload bytes + chat. The `_prompt` argument is ignored since
/// Goclaw injects the OCR rule via the agent's skill frontmatter.
pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudGoclawError> {
    let basename = filename_for_mime(mime_type);
    let uploaded = upload_media(image_bytes, mime_type, &basename, api_key).await?;
    chat_with_media(&uploaded, &basename, api_key).await
}

pub async fn call_with_image_path(
    image_path: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudGoclawError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudGoclawError::Network(format!("read image: {e}")))?;
    let mime = mime_for(image_path);
    let basename = basename_or_default(image_path, "image.png");
    let uploaded = upload_media(&bytes, mime, &basename, api_key).await?;
    chat_with_media(&uploaded, &basename, api_key).await
}

pub async fn call_with_pdf_path(
    pdf_path: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudGoclawError> {
    let bytes = tokio::fs::read(pdf_path)
        .await
        .map_err(|e| CloudGoclawError::Network(format!("read pdf: {e}")))?;
    let basename = basename_or_default(pdf_path, "document.pdf");
    let uploaded = upload_media(&bytes, "application/pdf", &basename, api_key).await?;
    chat_with_media(&uploaded, &basename, api_key).await
}

fn basename_or_default(path: &str, default: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| default.to_string())
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

fn filename_for_mime(mime: &str) -> String {
    match mime {
        "application/pdf" => "document.pdf",
        "image/jpeg" => "image.jpg",
        "image/webp" => "image.webp",
        _ => "image.png",
    }
    .to_string()
}

/// Strip `goclaw_<key>` patterns from any string before surfacing it through
/// logs or `DispatchError`. Matches the redaction conventions used by
/// `cloud_gemini_api::redact_key` and `cloud_mistral_api::redact_key`.
pub fn redact_key(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"goclaw_[0-9A-Za-z_\-]{20,}").unwrap());
    re.replace_all(s, "goclaw_<redacted>").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_strips_goclaw_key_pattern() {
        let raw = "auth failed: goclaw_4c7540a5810249ce3f3ec9ba88b7fd98 invalid";
        let cleaned = redact_key(raw);
        assert!(!cleaned.contains("4c7540a5810249ce3f3ec9ba88b7fd98"));
        assert!(cleaned.contains("goclaw_<redacted>"));
    }

    #[test]
    fn redact_leaves_non_key_strings_alone() {
        let raw = "connection refused at goclaw.tikz2svg.com:443";
        assert_eq!(redact_key(raw), raw);
    }

    #[test]
    fn parse_chat_response_extracts_content() {
        let frame = r#"{"type":"res","id":"2","ok":true,"payload":{"content":"\\int_0^1 x dx = \\frac{1}{2}"}}"#;
        let res = parse_chat_response(frame).expect("ok");
        assert_eq!(res, "\\int_0^1 x dx = \\frac{1}{2}");
    }

    #[test]
    fn parse_chat_response_propagates_rate_limited() {
        let frame = r#"{"type":"res","id":"2","ok":false,"error":{"code":"RATE_LIMITED","message":"slow down"}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        assert!(matches!(err, CloudGoclawError::RateLimited));
    }

    #[test]
    fn parse_chat_response_propagates_unauthorized() {
        let frame = r#"{"type":"res","id":"2","ok":false,"error":{"code":"UNAUTHORIZED","message":"bad key"}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        assert!(matches!(err, CloudGoclawError::AuthFailed(401)));
    }

    #[test]
    fn parse_chat_response_propagates_not_found() {
        let frame = r#"{"type":"res","id":"2","ok":false,"error":{"code":"NOT_FOUND","message":"agent tex-ocr"}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        match err {
            CloudGoclawError::BadRequest(m) => assert!(m.contains("agent tex-ocr")),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn parse_chat_response_empty_content_returns_empty_response() {
        let frame = r#"{"type":"res","id":"2","ok":true,"payload":{"content":""}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        assert!(matches!(err, CloudGoclawError::EmptyResponse));
    }

    #[test]
    fn parse_chat_response_skips_evt_frames() {
        let frame = r#"{"type":"evt","payload":{"kind":"typing"}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        assert!(matches!(err, CloudGoclawError::Parse(_)));
    }

    #[test]
    fn parse_chat_response_skips_other_id() {
        let frame = r#"{"type":"res","id":"1","ok":true,"payload":{}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        assert!(matches!(err, CloudGoclawError::Parse(_)));
    }

    #[test]
    fn parse_chat_response_handles_unknown_error_code() {
        let frame = r#"{"type":"res","id":"2","ok":false,"error":{"code":"WEIRD","message":"x"}}"#;
        let err = parse_chat_response(frame).unwrap_err();
        match err {
            CloudGoclawError::ServerError(500, m) => assert!(m.contains("WEIRD")),
            other => panic!("expected ServerError, got {other:?}"),
        }
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
