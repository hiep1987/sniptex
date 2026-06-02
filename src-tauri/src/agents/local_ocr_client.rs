use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

use crate::settings::validate_local_ocr_url;

pub const OCR_TIMEOUT: Duration = Duration::from_secs(10);
pub const CLASSIFY_TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Debug, Error)]
pub enum LocalOcrError {
    #[error("local OCR daemon unavailable: {0}")]
    Unavailable(String),
    #[error("timeout after {0}s")]
    Timeout(u64),
    #[error("unsupported input: {0}")]
    Unsupported(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("empty response")]
    EmptyResponse,
    #[error("low confidence: {0:.2}")]
    LowConfidence(f32),
    #[error("response parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(String),
}

pub async fn post_image(
    base_url: &str,
    image_path: &str,
    endpoint_path: &str,
) -> Result<reqwest::Response, LocalOcrError> {
    post_image_with_timeout(base_url, image_path, endpoint_path, OCR_TIMEOUT).await
}

pub async fn post_image_with_timeout(
    base_url: &str,
    image_path: &str,
    endpoint_path: &str,
    timeout: Duration,
) -> Result<reqwest::Response, LocalOcrError> {
    if validate_local_ocr_url(base_url).is_err() {
        return Err(LocalOcrError::BadRequest("non-loopback local OCR URL".into()));
    }

    let bytes = tokio::fs::read(image_path)
        .await
        .map_err(|e| LocalOcrError::Io(e.to_string()))?;
    let file_name = std::path::Path::new(image_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("capture.png")
        .to_string();
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str(mime_for(image_path))
        .map_err(|e| LocalOcrError::BadRequest(e.to_string()))?;
    let form = reqwest::multipart::Form::new().part("image", part);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| LocalOcrError::Unavailable(e.to_string()))?;
    let endpoint = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        endpoint_path.trim_start_matches('/')
    );

    client.post(endpoint).multipart(form).send().await.map_err(|e| {
        if e.is_timeout() {
            LocalOcrError::Timeout(timeout.as_secs().max(1))
        } else {
            LocalOcrError::Unavailable(e.to_string())
        }
    })
}

pub async fn response_body(response: reqwest::Response) -> Result<(u16, String), LocalOcrError> {
    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| LocalOcrError::Unavailable(e.to_string()))?;
    Ok((status, text))
}

pub fn error_message(text: &str) -> String {
    #[derive(Deserialize)]
    struct ErrorBody {
        error: Option<String>,
    }
    serde_json::from_str::<ErrorBody>(text)
        .ok()
        .and_then(|body| body.error)
        .filter(|msg| !msg.trim().is_empty())
        .unwrap_or_else(|| text.trim().to_string())
}

fn mime_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}
