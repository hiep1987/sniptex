//! Cloud Mistral OCR adapter tests. These cover deterministic parsing,
//! redaction, and dispatcher mapping without live network calls.

use sniptex_lib::agents::cloud_mistral_api::{
    mime_for, parse_response, redact_key, CloudMistralError, CLOUD_MISTRAL_MODEL,
};
use sniptex_lib::ocr::DispatchError;

#[test]
fn cloud_model_constant_is_pinned() {
    assert_eq!(CLOUD_MISTRAL_MODEL, "mistral-ocr-latest");
}

#[test]
fn test_redact_strips_bearer_token() {
    let cleaned = redact_key("upstream saw Bearer sk-live-secret-token in request");
    assert_eq!(cleaned, "upstream saw Bearer <redacted> in request");
}

#[test]
fn test_redact_strips_lowercase_bearer_and_bare_key() {
    let cleaned = redact_key(
        "upstream saw bearer abc_1234567890123456789012345 and abc_1234567890123456789012345",
    );
    assert_eq!(
        cleaned,
        "upstream saw Bearer <redacted> and <redacted-mistral-key>"
    );
}

#[test]
fn test_mime_resolution() {
    assert_eq!(mime_for("capture.png"), "image/png");
    assert_eq!(mime_for("capture.jpg"), "image/jpeg");
    assert_eq!(mime_for("capture.jpeg"), "image/jpeg");
    assert_eq!(mime_for("capture.webp"), "image/webp");
    assert_eq!(mime_for("capture.bmp"), "image/png");
}

#[test]
fn test_parse_success_response() {
    let raw = r#"{
        "pages": [
            { "markdown": "\\frac{a}{b}" }
        ]
    }"#;
    assert_eq!(parse_response(raw).unwrap(), "\\frac{a}{b}");
}

#[test]
fn test_parse_empty_pages() {
    let err = parse_response(r#"{ "pages": [] }"#).unwrap_err();
    assert!(matches!(err, CloudMistralError::EmptyResponse));
}

#[test]
fn test_parse_null_markdown() {
    let err = parse_response(r#"{ "pages": [{ "markdown": null }] }"#).unwrap_err();
    assert!(matches!(err, CloudMistralError::EmptyResponse));
}

#[test]
fn test_parse_multi_page_concatenates_all() {
    let raw = r#"{
        "pages": [
            { "markdown": "page one" },
            { "markdown": "page two" }
        ]
    }"#;
    assert_eq!(parse_response(raw).unwrap(), "page one\n\npage two");
}

#[test]
fn rate_limited_maps_to_dispatch_rate_limited() {
    let err: DispatchError = CloudMistralError::RateLimited.into();
    assert!(matches!(err, DispatchError::RateLimited));
}

#[test]
fn auth_failed_preserves_status_code() {
    let err: DispatchError = CloudMistralError::AuthFailed(403).into();
    assert!(matches!(err, DispatchError::AuthFailed(403)));
}

#[test]
fn empty_response_maps_to_empty_output() {
    let err: DispatchError = CloudMistralError::EmptyResponse.into();
    assert!(matches!(err, DispatchError::EmptyOutput));
}
