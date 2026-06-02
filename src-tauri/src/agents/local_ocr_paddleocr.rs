use serde::Deserialize;

use crate::agents::local_ocr_client::{
    error_message, post_image, response_body, LocalOcrError, OCR_TIMEOUT,
};
use crate::ocr::smart_format::{detect_type, DetectedType};

const MIN_PADDLEOCR_CONFIDENCE: f32 = 0.50;
const TABLE_UNSUPPORTED: &str = "local does not support tables";

#[derive(Debug, Deserialize)]
struct PaddleOcrResponse {
    text: Option<String>,
    confidence: Option<f32>,
    detected: Option<String>,
    #[serde(default)]
    lines: Vec<PaddleOcrLine>,
}

#[derive(Debug, Deserialize)]
struct PaddleOcrLine {
    text: Option<String>,
    confidence: Option<f32>,
}

pub async fn paddleocr(base_url: &str, image_path: &str) -> Result<String, LocalOcrError> {
    let response = post_image(base_url, image_path, "/ocr/paddleocr").await?;
    let (status, text) = response_body(response).await?;
    parse_ocr_body(status, &text)
}

pub fn parse_ocr_body(status_code: u16, text: &str) -> Result<String, LocalOcrError> {
    if !(200..300).contains(&status_code) {
        return Err(match status_code {
            408 => LocalOcrError::Timeout(OCR_TIMEOUT.as_secs()),
            422 => map_unsupported(&error_message(text)),
            code => LocalOcrError::BadRequest(format!("HTTP {code}: {}", error_message(text))),
        });
    }

    let parsed: PaddleOcrResponse =
        serde_json::from_str(text).map_err(|e| LocalOcrError::Parse(e.to_string()))?;
    if detected_is_table(parsed.detected.as_deref()) {
        return Err(LocalOcrError::BadRequest(TABLE_UNSUPPORTED.into()));
    }

    let cleaned = normalized_text(&parsed);
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(LocalOcrError::EmptyResponse);
    }
    if detect_type(&cleaned) == DetectedType::TableOnly || looks_like_table(&cleaned) {
        return Err(LocalOcrError::BadRequest(TABLE_UNSUPPORTED.into()));
    }
    if let Some(confidence) = response_confidence(&parsed) {
        if confidence < MIN_PADDLEOCR_CONFIDENCE {
            return Err(LocalOcrError::LowConfidence(confidence));
        }
    }

    Ok(cleaned)
}

fn map_unsupported(error: &str) -> LocalOcrError {
    if error == "unsupported_table" {
        LocalOcrError::BadRequest(TABLE_UNSUPPORTED.into())
    } else {
        LocalOcrError::Unsupported(error.to_string())
    }
}

fn normalized_text(parsed: &PaddleOcrResponse) -> String {
    if let Some(text) = parsed.text.as_deref() {
        let cleaned = join_nonempty_lines(text.lines());
        if !cleaned.is_empty() {
            return cleaned;
        }
    }

    join_nonempty_lines(parsed.lines.iter().filter_map(|line| line.text.as_deref()))
}

fn response_confidence(parsed: &PaddleOcrResponse) -> Option<f32> {
    parsed
        .confidence
        .into_iter()
        .chain(parsed.lines.iter().filter_map(|line| line.confidence))
        .min_by(|a, b| a.total_cmp(b))
}

fn join_nonempty_lines<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn detected_is_table(detected: Option<&str>) -> bool {
    detected
        .map(|value| {
            let lower = value.trim().to_ascii_lowercase();
            lower == "table" || lower == "table_only"
        })
        .unwrap_or(false)
}

fn looks_like_table(text: &str) -> bool {
    let mut pipe_rows = 0;
    for line in text.lines().map(str::trim) {
        if line.contains("|---") || line.contains("| ---") {
            return true;
        }
        if line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 2 {
            pipe_rows += 1;
        }
    }
    pipe_rows >= 2
}
