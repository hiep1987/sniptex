use serde::Deserialize;

use crate::agents::local_ocr_api::health_with_cache;
use crate::agents::local_ocr_client::{
    error_message, post_image_with_timeout, response_body, LocalOcrError, CLASSIFY_TIMEOUT,
};
use crate::agents::local_ocr_paddleocr;
use crate::agents::local_ocr_pix2tex;
use crate::agents::registry::{LOCAL_FAST_ID, LOCAL_PADDLEOCR_ID, LOCAL_PIX2TEX_ID};

const LOCAL_UNSUPPORTED: &str = "local unsupported";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalRoute {
    Pix2Tex,
    PaddleOcr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedLocalText {
    pub text: String,
    pub agent_id: &'static str,
    pub via: &'static str,
}

#[derive(Debug, Deserialize)]
struct ClassifyResponse {
    kind: Option<String>,
    confidence: Option<f32>,
}

pub async fn auto_route(base_url: &str, image_path: &str) -> Result<RoutedLocalText, LocalOcrError> {
    let health = health_with_cache(base_url).await;
    if !health.healthy {
        return Err(LocalOcrError::Unavailable("local OCR daemon unhealthy".into()));
    }

    let route = classify(base_url, image_path).await?;
    match route {
        LocalRoute::Pix2Tex => Ok(RoutedLocalText {
            text: local_ocr_pix2tex::pix2tex(base_url, image_path).await?,
            agent_id: LOCAL_PIX2TEX_ID,
            via: LOCAL_FAST_ID,
        }),
        LocalRoute::PaddleOcr => Ok(RoutedLocalText {
            text: local_ocr_paddleocr::paddleocr(base_url, image_path).await?,
            agent_id: LOCAL_PADDLEOCR_ID,
            via: LOCAL_FAST_ID,
        }),
    }
}

pub async fn classify(base_url: &str, image_path: &str) -> Result<LocalRoute, LocalOcrError> {
    let response = post_image_with_timeout(base_url, image_path, "/classify", CLASSIFY_TIMEOUT)
        .await?;
    let (status, text) = response_body(response).await?;
    parse_classify_body(status, &text)
}

pub fn parse_classify_body(status_code: u16, text: &str) -> Result<LocalRoute, LocalOcrError> {
    if !(200..300).contains(&status_code) {
        return Err(match status_code {
            408 => LocalOcrError::Timeout(CLASSIFY_TIMEOUT.as_secs().max(1)),
            422 => LocalOcrError::BadRequest(LOCAL_UNSUPPORTED.into()),
            code => LocalOcrError::BadRequest(format!("HTTP {code}: {}", error_message(text))),
        });
    }

    let parsed: ClassifyResponse =
        serde_json::from_str(text).map_err(|e| LocalOcrError::Parse(e.to_string()))?;
    if matches!(parsed.confidence, Some(c) if c <= 0.0) {
        return Err(LocalOcrError::LowConfidence(parsed.confidence.unwrap()));
    }

    match parsed.kind.as_deref().map(normalize_kind) {
        Some(kind) if kind == "equation" || kind == "equation_only" => Ok(LocalRoute::Pix2Tex),
        Some(kind) if kind == "text" => Ok(LocalRoute::PaddleOcr),
        Some(_) => Err(LocalOcrError::BadRequest(LOCAL_UNSUPPORTED.into())),
        None => Err(LocalOcrError::Parse("classifier response missing kind".into())),
    }
}

fn normalize_kind(kind: &str) -> String {
    kind.trim().to_ascii_lowercase().replace('-', "_")
}
