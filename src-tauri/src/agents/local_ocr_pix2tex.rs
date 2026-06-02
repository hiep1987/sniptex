use serde::Deserialize;

use crate::agents::local_ocr_client::{error_message, post_image, response_body, OCR_TIMEOUT};

const MIN_PIX2TEX_CONFIDENCE: f32 = 0.15;

pub use crate::agents::local_ocr_client::LocalOcrError;

#[derive(Debug, Deserialize)]
struct OcrResponse {
    text: Option<String>,
    confidence: Option<f32>,
}

pub async fn pix2tex(base_url: &str, image_path: &str) -> Result<String, LocalOcrError> {
    let response = post_image(base_url, image_path, "/ocr/pix2tex").await?;
    let (status, text) = response_body(response).await?;
    parse_ocr_body(status, &text)
}

pub fn parse_ocr_body(status_code: u16, text: &str) -> Result<String, LocalOcrError> {
    if !(200..300).contains(&status_code) {
        return Err(match status_code {
            408 => LocalOcrError::Timeout(OCR_TIMEOUT.as_secs()),
            422 => LocalOcrError::Unsupported(error_message(text)),
            code => LocalOcrError::BadRequest(format!("HTTP {code}: {}", error_message(text))),
        });
    }

    let parsed: OcrResponse =
        serde_json::from_str(text).map_err(|e| LocalOcrError::Parse(e.to_string()))?;
    let cleaned = parsed.text.unwrap_or_default().trim().to_string();
    if cleaned.is_empty() || cleaned == "[UNREADABLE]" {
        return Err(LocalOcrError::EmptyResponse);
    }
    if let Some(confidence) = parsed.confidence {
        if confidence < MIN_PIX2TEX_CONFIDENCE {
            return Err(LocalOcrError::LowConfidence(confidence));
        }
    }
    Ok(cleaned)
}
