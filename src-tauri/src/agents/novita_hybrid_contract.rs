use serde::Deserialize;

pub const GPT_OSS_MODEL: &str = "openai/gpt-oss-120b";
pub const GPT_OSS_ENDPOINT: &str = "https://api.novita.ai/openai/v1/chat/completions";
pub const MAX_INTERMEDIATE_CHARS: usize = 12_000;
pub const MAX_GPT_TOKENS: u32 = 2048;

#[derive(Debug, thiserror::Error)]
pub enum CloudNovitaHybridError {
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

pub fn normalize_intermediate_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_INTERMEDIATE_CHARS));
    let mut previous_blank = false;
    for line in text.lines().map(str::trim_end) {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        previous_blank = blank;
        if out.len() + line.len() + 1 > MAX_INTERMEDIATE_CHARS {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

pub fn parse_gpt_oss_response(text: &str) -> Result<String, CloudNovitaHybridError> {
    serde_json::from_str::<ChatResponse>(text)
        .map_err(|e| CloudNovitaHybridError::Parse(e.to_string()))?
        .choices
        .and_then(|mut c| c.drain(..).next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or(CloudNovitaHybridError::EmptyResponse)
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

pub fn redact_url_secrets(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static URL_SECRET_RE: OnceLock<Regex> = OnceLock::new();
    let re = URL_SECRET_RE.get_or_init(|| {
        Regex::new(r"(https?://)([^/\s:@]+:)?([^/\s:@]+@)?([^?\s]+)(\?[^\s]+)?").unwrap()
    });
    re.replace_all(s, |c: &regex::Captures| {
        let scheme = c.get(1).map_or("", |m| m.as_str());
        let host_path = c.get(4).map_or("", |m| m.as_str());
        if c.get(2).is_some() || c.get(3).is_some() || c.get(5).is_some() {
            format!("{scheme}{host_path}?<redacted>")
        } else {
            format!("{scheme}{host_path}")
        }
    })
    .to_string()
}
