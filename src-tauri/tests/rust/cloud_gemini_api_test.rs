//! Cloud Gemini adapter tests.
//!
//! Network calls are exercised by `cli_test --agent cloud-gemini` (manual
//! smoke). At unit-test scope we cover the deterministic logic only:
//! the dispatcher error mapping from `CloudGeminiError`. Full HTTP-mock
//! coverage is deferred until we adopt `wiremock` in a later phase.

use sniptex_lib::agents::cloud_gemini_api::{CloudGeminiError, CLOUD_GEMINI_MODEL};
use sniptex_lib::ocr::DispatchError;

#[test]
fn cloud_model_constant_is_pinned() {
    // Pin guards against silent model swaps that would invalidate every
    // prompt-validation benchmark we've collected.
    assert_eq!(CLOUD_GEMINI_MODEL, "gemini-2.0-flash");
}

#[test]
fn rate_limited_maps_to_dispatch_rate_limited() {
    let err: DispatchError = CloudGeminiError::RateLimited("quota exhausted".into()).into();
    assert!(matches!(err, DispatchError::RateLimited));
}

#[test]
fn auth_failed_preserves_status_code() {
    let err: DispatchError = CloudGeminiError::AuthFailed(401).into();
    assert!(matches!(err, DispatchError::AuthFailed(401)));
}

#[test]
fn empty_response_maps_to_empty_output() {
    let err: DispatchError = CloudGeminiError::EmptyResponse.into();
    assert!(matches!(err, DispatchError::EmptyOutput));
}

#[test]
fn bad_request_payload_propagates_message() {
    let err: DispatchError = CloudGeminiError::BadRequest("malformed".into()).into();
    match err {
        DispatchError::BadRequest(m) => assert_eq!(m, "malformed"),
        other => panic!("unexpected variant: {other:?}"),
    }
}
