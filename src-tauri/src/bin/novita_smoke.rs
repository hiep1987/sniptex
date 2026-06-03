//! Diagnostic smoke test for DeepSeek-OCR 2 via Novita.ai.
//!
//! Pass `--via-adapter` as the second arg to call through
//! `cloud_novita_api::call_with_image_path` instead of the raw HTTP test.
//!
//! Bypasses the adapter so we can see the raw request/response and try
//! different schema variants when the generic `invalid_request_error`
//! gives us nothing actionable.
//!
//! Usage:
//!   NOVITA_API_KEY=sk_xxx cargo run --bin novita_smoke -- <image-path> [variant]
//!
//! variant ∈ { 1=image-first+grounding (current adapter),
//!             2=image-only-no-text,
//!             3=image-first+free-ocr,
//!             4=text-first+grounding,
//!             5=image-first+grounding+detail=high }
//! Default: 1.

use base64::Engine;
use serde_json::{json, Value};
use std::process::ExitCode;
use std::time::Instant;

const ENDPOINT: &str = "https://api.novita.ai/openai/v1/chat/completions";
const MODEL: &str = "deepseek/deepseek-ocr-2";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let key = match std::env::var("NOVITA_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            eprintln!("error: set NOVITA_API_KEY env var");
            return ExitCode::from(2);
        }
    };
    let image_path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: NOVITA_API_KEY=... novita_smoke <image-path> [variant]");
            return ExitCode::from(2);
        }
    };
    let arg2 = std::env::args().nth(2).unwrap_or_default();
    if arg2 == "--via-adapter" {
        return run_via_adapter(&image_path, &key).await;
    }
    let variant: u8 = arg2.parse().unwrap_or(1);

    let bytes = match tokio::fs::read(&image_path).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error reading image: {e}");
            return ExitCode::from(2);
        }
    };
    let mime = if image_path.to_lowercase().ends_with(".jpg")
        || image_path.to_lowercase().ends_with(".jpeg")
    {
        "image/jpeg"
    } else {
        "image/png"
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_uri = format!("data:{mime};base64,{encoded}");
    eprintln!(
        "[smoke] image={image_path} mime={mime} bytes={} b64_len={}",
        bytes.len(),
        encoded.len()
    );

    let image_part = json!({
        "type": "image_url",
        "image_url": { "url": data_uri.clone() },
    });
    let image_part_hi = json!({
        "type": "image_url",
        "image_url": { "url": data_uri, "detail": "high" },
    });
    let text_grounding = json!({
        "type": "text",
        "text": "<|grounding|>Convert the document to markdown.",
    });
    let text_free = json!({
        "type": "text",
        "text": "Free OCR.",
    });

    let text_ocr = json!({ "type": "text", "text": "OCR." });
    let text_verbatim = json!({
        "type": "text",
        "text": "Transcribe the text verbatim, preserving all characters exactly as shown including backslashes.",
    });
    let content: Value = match variant {
        2 => json!([image_part.clone()]),
        3 => json!([image_part.clone(), text_free]),
        4 => json!([text_grounding.clone(), image_part.clone()]),
        5 => json!([image_part_hi, text_grounding.clone()]),
        6 => json!([image_part.clone(), text_ocr]),
        7 => json!([image_part.clone(), text_verbatim]),
        _ => json!([image_part, text_grounding]),
    };

    let body = json!({
        "model": MODEL,
        "messages": [{ "role": "user", "content": content }],
        "max_tokens": 4096,
        "temperature": 0.0,
    });

    eprintln!("[smoke] variant={variant} endpoint={ENDPOINT}");
    eprintln!(
        "[smoke] request body preview (image data truncated):\n{}",
        truncate_image_in(&body)
    );

    let client = reqwest::Client::new();
    let started = Instant::now();
    let resp = match client
        .post(ENDPOINT)
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[smoke] network error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let elapsed_ms = started.elapsed().as_millis();

    eprintln!("[smoke] HTTP {} in {elapsed_ms}ms", status.as_u16());
    println!("{text}");

    if status.is_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

async fn run_via_adapter(image_path: &str, key: &str) -> ExitCode {
    use sniptex_lib::agents::cloud_novita_api;
    eprintln!("[smoke] calling via adapter (cleaning applied)…");
    let started = Instant::now();
    match cloud_novita_api::call_with_image_path(image_path, "", key).await {
        Ok(text) => {
            eprintln!(
                "[smoke] OK in {}ms ({} chars)",
                started.elapsed().as_millis(),
                text.chars().count()
            );
            println!("---CLEANED OUTPUT---");
            println!("{text}");
            println!("---END---");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[smoke] FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}

fn truncate_image_in(body: &Value) -> String {
    let mut cloned = body.clone();
    if let Some(arr) = cloned
        .get_mut("messages")
        .and_then(|m| m.get_mut(0))
        .and_then(|m| m.get_mut("content"))
        .and_then(|c| c.as_array_mut())
    {
        for part in arr {
            if let Some(url) = part
                .get_mut("image_url")
                .and_then(|iu| iu.get_mut("url"))
                .and_then(|u| u.as_str().map(|s| s.to_string()))
            {
                let truncated = if url.len() > 80 {
                    format!("{}…(+{} chars)", &url[..80], url.len() - 80)
                } else {
                    url
                };
                part["image_url"]["url"] = json!(truncated);
            }
        }
    }
    serde_json::to_string_pretty(&cloned).unwrap_or_default()
}
