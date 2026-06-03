//! Novita.ai OpenAI-compatible chat completions adapter -- BYOK.
//!
//! Routes to DeepSeek-OCR 2 (`deepseek/deepseek-ocr-2`) via Novita's
//! `/openai/v1/chat/completions` endpoint. Image is sent inline as a
//! base64 data URI under the `image_url` content part (OpenAI vision
//! convention). Temperature pinned to 0 for deterministic OCR.
//!
//! The caller's prompt is intentionally ignored. DeepSeek-OCR 2 is a
//! task-specialized model that rejects arbitrary instructions: anything
//! outside its fixed task prompts returns HTTP 400 `invalid_request_error`
//! or — worse — hallucinates content (a custom "transcribe verbatim"
//! instruction produced "screenshot of a terminal showing Hello World"
//! when given a screenshot of LaTeX source). We hardcode `Free OCR.`
//! after empirical testing of all known task prompts.
//!
//! **Best for:** natural document content — math problems, prose,
//! data tables (proper markdown table output).
//! **Poor for:** screenshots of LaTeX/code source. The model is trained
//! to "render document → markdown" so it tries to interpret `\hline` and
//! `\begin{tabular}` as commands and mangles them into half-rendered
//! half-escaped output. Mistral's `/v1/ocr` endpoint handles code-as-image
//! verbatim and is the better choice for those snips.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const CLOUD_NOVITA_MODEL: &str = "deepseek/deepseek-ocr-2";
pub const CLOUD_NOVITA_ENDPOINT: &str = "https://api.novita.ai/openai/v1/chat/completions";
/// `Free OCR.` outperforms `<|grounding|>Convert the document to markdown.`
/// on real-world content: the grounding mode emits tables as flat
/// `<table>...</table>` with no cell separators (rows/cells jammed into a
/// single string), while Free OCR produces proper pipe-delimited markdown
/// tables. Both modes emit math as `\(...\)` / `\[...\]` which the
/// post-processor below converts to `$...$` / `$$...$$`.
pub const NOVITA_PROMPT: &str = "Free OCR.";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// DeepSeek-OCR 2 returns `invalid_request_error` if `max_tokens` exceeds
/// the model's per-call output cap. 4096 is the empirical safe ceiling.
const MAX_TOKENS: u32 = 4096;

#[derive(Debug, thiserror::Error)]
pub enum CloudNovitaError {
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
struct ChatRequest<'a> {
    model: &'static str,
    messages: [ChatMessage<'a>; 1],
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: Vec<ContentPart<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart<'a> {
    Text { text: &'a str },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
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

pub fn parse_response(text: &str) -> Result<String, CloudNovitaError> {
    let parsed: ChatResponse =
        serde_json::from_str(text).map_err(|e| CloudNovitaError::Parse(e.to_string()))?;
    let raw = parsed
        .choices
        .and_then(|mut c| c.drain(..).next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .filter(|t| !t.trim().is_empty())
        .ok_or(CloudNovitaError::EmptyResponse)?;
    let cleaned = clean_grounding_output(&raw);
    if cleaned.trim().is_empty() {
        return Err(CloudNovitaError::EmptyResponse);
    }
    Ok(cleaned)
}

/// Normalize DeepSeek-OCR 2 output to the rest of the SnipTeX pipeline's
/// conventions: convert `\(...\)` → `$...$` and `\[...\]` → `$$...$$`.
/// Also defensively strips `text[[x, y, w, h]]` bounding-box markers in
/// case the grounding mode leaks through (Free OCR shouldn't emit them
/// but a future Novita-side change could).
fn clean_grounding_output(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static MARKER_RE: OnceLock<Regex> = OnceLock::new();
    static INLINE_MATH_RE: OnceLock<Regex> = OnceLock::new();
    static DISPLAY_MATH_RE: OnceLock<Regex> = OnceLock::new();
    let marker_re = MARKER_RE.get_or_init(|| {
        Regex::new(r"(?m)^(?:text|image|title|figure|table|list)\[\[[^\]]+\]\]\s*\n?").unwrap()
    });
    let inline_re = INLINE_MATH_RE.get_or_init(|| Regex::new(r"\\\((.*?)\\\)").unwrap());
    let display_re = DISPLAY_MATH_RE.get_or_init(|| Regex::new(r"(?s)\\\[(.*?)\\\]").unwrap());

    let no_markers = marker_re.replace_all(s, "");
    let inline_fixed = inline_re.replace_all(&no_markers, |c: &regex::Captures| {
        format!("${}$", c[1].trim())
    });
    let display_fixed = display_re.replace_all(&inline_fixed, |c: &regex::Captures| {
        format!("$${}$$", c[1].trim())
    });
    display_fixed.trim().to_string()
}

pub fn redact_key(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static BEARER_RE: OnceLock<Regex> = OnceLock::new();
    static SK_RE: OnceLock<Regex> = OnceLock::new();
    let bearer_re = BEARER_RE.get_or_init(|| Regex::new(r"(?i)bearer\s+[0-9A-Za-z._\-]+").unwrap());
    let sk_re = SK_RE.get_or_init(|| Regex::new(r"\bsk_[0-9A-Za-z._\-]{12,}\b").unwrap());
    let without_bearer = bearer_re.replace_all(s, "Bearer <redacted>");
    sk_re
        .replace_all(&without_bearer, "<redacted-novita-key>")
        .to_string()
}

pub async fn call(
    image_bytes: &[u8],
    mime_type: &str,
    _prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaError> {
    call_with_timeout(image_bytes, mime_type, api_key, REQUEST_TIMEOUT).await
}

async fn call_with_timeout(
    image_bytes: &[u8],
    mime_type: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<String, CloudNovitaError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| CloudNovitaError::Network(e.to_string()))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let data_uri = format!("data:{mime_type};base64,{encoded}");

    let body = ChatRequest {
        model: CLOUD_NOVITA_MODEL,
        messages: [ChatMessage {
            role: "user",
            // Image comes first per Novita's vision content convention,
            // then the fixed DeepSeek-OCR 2 task string. The model rejects
            // any other text with `invalid_request_error`.
            content: vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrl { url: data_uri },
                },
                ContentPart::Text {
                    text: NOVITA_PROMPT,
                },
            ],
        }],
        max_tokens: MAX_TOKENS,
        temperature: 0.0,
    };

    let resp = client
        .post(CLOUD_NOVITA_ENDPOINT)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudNovitaError::Network(redact_key(&e.to_string())))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CloudNovitaError::Network(redact_key(&e.to_string())))?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            429 => CloudNovitaError::RateLimited,
            400 => CloudNovitaError::BadRequest(redact_key(&text)),
            401 | 403 => CloudNovitaError::AuthFailed(status.as_u16()),
            code if status.is_server_error() => {
                CloudNovitaError::ServerError(code, redact_key(&text))
            }
            code => CloudNovitaError::BadRequest(format!("HTTP {code}: {}", redact_key(&text))),
        });
    }

    parse_response(&text)
}

pub async fn call_with_image_path(
    image_path: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, CloudNovitaError> {
    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| CloudNovitaError::Network(format!("read image: {e}")))?;
    call(&bytes, mime_for(image_path), prompt, api_key).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_for_common_extensions() {
        assert_eq!(mime_for("snap.png"), "image/png");
        assert_eq!(mime_for("snap.PNG"), "image/png");
        assert_eq!(mime_for("snap.jpg"), "image/jpeg");
        assert_eq!(mime_for("snap.JPEG"), "image/jpeg");
        assert_eq!(mime_for("snap.webp"), "image/webp");
        assert_eq!(mime_for("snap.unknown"), "image/png");
    }

    #[test]
    fn parse_response_pulls_first_choice_content() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"x^2 + y^2"}}]}"#;
        assert_eq!(parse_response(json).unwrap(), "x^2 + y^2");
    }

    #[test]
    fn parse_response_strips_grounding_marker_and_normalizes_math() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"text[[0, 0, 999, 999]]\nCâu 8: \\( (x+3)^{2}+(y-2)^{2}=36 \\)"}}]}"#;
        let result = parse_response(json).unwrap();
        assert_eq!(result, "Câu 8: $(x+3)^{2}+(y-2)^{2}=36$");
    }

    #[test]
    fn clean_grounding_output_strips_multiple_marker_types_and_display_math() {
        let raw = "text[[0, 0, 100, 100]]\nIntro.\nimage[[10, 20, 30, 40]]\n\\[ \\int_0^1 x dx = \\frac{1}{2} \\]\nEnd.";
        let cleaned = clean_grounding_output(raw);
        assert_eq!(cleaned, "Intro.\n$$\\int_0^1 x dx = \\frac{1}{2}$$\nEnd.");
    }

    #[test]
    fn clean_grounding_output_handles_inline_math_only() {
        let raw = "Mass is \\( m c^2 \\) per unit.";
        assert_eq!(clean_grounding_output(raw), "Mass is $m c^2$ per unit.");
    }

    #[test]
    fn clean_grounding_output_passes_through_clean_text() {
        let raw = "Already $clean$ text.";
        assert_eq!(clean_grounding_output(raw), "Already $clean$ text.");
    }

    #[test]
    fn parse_response_empty_content_errors() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":""}}]}"#;
        assert!(matches!(
            parse_response(json),
            Err(CloudNovitaError::EmptyResponse)
        ));
    }

    #[test]
    fn parse_response_no_choices_errors() {
        let json = r#"{"choices":[]}"#;
        assert!(matches!(
            parse_response(json),
            Err(CloudNovitaError::EmptyResponse)
        ));
    }

    #[test]
    fn redact_bearer_token() {
        let raw = "Authorization: Bearer sk_test_secret_value failed";
        assert_eq!(redact_key(raw), "Authorization: Bearer <redacted> failed");
    }

    #[test]
    fn redact_bare_sk_key() {
        let raw = "leaked sk_abcdefghijklmnopq in log";
        assert_eq!(redact_key(raw), "leaked <redacted-novita-key> in log");
    }
}
