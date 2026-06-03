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
        "You are an OCR escape-artifact cleaner. You are NOT a solver, tutor, or formatter. Your only job is to undo specific LaTeX over-escaping in the source. The source is OCR output captured for verbatim copy-paste, so anything you add to it is a regression.\n\n\
        STRICT RULES — violating any single rule means the output is wrong:\n\
        1. Output ONLY content derived from the source. Never invent text, headers, labels, or computations.\n\
        2. NEVER solve math problems. If the source contains a question (e.g. \"Mốt của mẫu số liệu...\"), copy it verbatim — do NOT compute the answer.\n\
        3. NEVER add: `\\boxed{{...}}`, `\\bar{{...}}`, `\\begin{{aligned}}`, midpoint formulas, mean / median / mode derivations, \"Đáp án:\", \"Answer:\", \"Solution:\", \"Tính ...\", \"Bảng ... đã sửa\", or any heading that is not literally present in the source.\n\
        4. Preserve EVERY question header (\"Câu N.\"), every multiple-choice option line (A./B./C./D./E.), and every numeric value, character-for-character. If a choice exists in source, it MUST exist in output.\n\
        5. ONLY fix these specific over-escape artifacts:\n\
           - `\\textbackslash{{begin}}` → `\\begin`\n\
           - `\\textbackslash{{tabular}}` → `\\tabular`\n\
           - `\\textbackslash{{hline}}` → `\\hline`\n\
           - `\\textbackslash{{end}}` → `\\end`\n\
           - `\\text{{end{{tabular}}}}` → `\\end{{tabular}}`\n\
           - Stray ` & \\` / `\\ &` cell separators inside tabular rows\n\
        6. Preserve table SHAPE from the source: if source shows a 1-row + 1-row tabular (Giá trị header row, Tần số data row across columns), output the SAME 2×N shape. Do NOT pivot rows-to-columns.\n\
        7. If artifacts cannot be cleaned without inventing content, return literally `[UNREADABLE]` and nothing else.\n\n\
        Few-shot example (note: no extra labels, no derivation, choices preserved):\n\
        Source:\n\
        ```\n\
        Câu 1. Cho bảng:\n\
        \\begin{{tabular}}{{|c|c|}}\n\
        \\textbackslash{{hline}} & Giá trị \\\\\n\
        \\end{{tabular}}\n\
        Mốt bằng\n\
        A. 5. B. 6.\n\
        ```\n\
        Cleaned:\n\
        ```\n\
        Câu 1. Cho bảng:\n\
        \\begin{{tabular}}{{|c|c|}}\n\
        \\hline\n\
        Giá trị \\\\\n\
        \\end{{tabular}}\n\
        Mốt bằng\n\
        A. 5. B. 6.\n\
        ```\n\n\
        Source from {CLOUD_NOVITA_MODEL}:\n{markdown}"
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
    if looks_hallucinated(markdown, &cleaned) {
        return Err(CloudNovitaHybridError::BadRequest(
            "GPT cleanup hallucinated content not in source".to_string(),
        ));
    }
    Ok(cleaned)
}

/// Detect GPT-introduced content not present in the source markdown.
///
/// GPT-OSS-120B has a strong "tutor" bias: shown a math question, it tries to
/// solve it even when explicitly told to only clean syntax. The fingerprints
/// of that failure mode are LaTeX constructs that appear in the cleaned output
/// but not the OCR source — `\boxed{...}` final answers, `\bar{x}` / `\bar{y}`
/// statistical symbols introduced for mean derivations, and `\begin{aligned}`
/// blocks holding multi-step computations. Any one of those appearing only on
/// the output side means GPT invented content; reject so the dispatcher can
/// fall back to a non-hallucinating agent.
pub fn looks_hallucinated(source: &str, output: &str) -> bool {
    const SUSPICIOUS: &[&str] = &[
        "\\boxed{",
        "\\bar{",
        "\\begin{aligned}",
    ];
    SUSPICIOUS
        .iter()
        .any(|pattern| output.contains(pattern) && !source.contains(pattern))
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
