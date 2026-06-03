//! Cloud Goclaw OCR adapter tests. These cover deterministic parsing,
//! redaction, and dispatcher mapping without live network calls. Live
//! end-to-end testing against the VPS happens via the manual smoke flow
//! once Phase 4 wires the API key into the SnipTeX UI.

use sniptex_lib::agents::cloud_goclaw_api::{
    mime_for, parse_chat_response, redact_key, CloudGoclawError, GOCLAW_AGENT_ID, GOCLAW_API_BASE,
    GOCLAW_WS_URL,
};
use sniptex_lib::ocr::DispatchError;

#[test]
fn endpoint_constants_are_pinned() {
    assert_eq!(GOCLAW_API_BASE, "https://goclaw.tikz2svg.com/api");
    assert_eq!(GOCLAW_WS_URL, "wss://goclaw.tikz2svg.com/ws");
    assert_eq!(GOCLAW_AGENT_ID, "tex-ocr");
}

#[test]
fn test_redact_strips_goclaw_key() {
    let cleaned = redact_key("auth: goclaw_4c7540a5810249ce3f3ec9ba88b7fd98 invalid");
    assert!(!cleaned.contains("4c7540a5810249ce3f3ec9ba88b7fd98"));
    assert!(cleaned.contains("goclaw_<redacted>"));
}

#[test]
fn test_redact_leaves_non_key_strings_alone() {
    let raw = "connection refused at goclaw.tikz2svg.com:443";
    assert_eq!(redact_key(raw), raw);
}

#[test]
fn test_redact_short_pattern_not_matched() {
    // Anything shorter than 20 chars after "goclaw_" is not a valid key shape;
    // leave it untouched so we don't redact non-secret strings.
    let raw = "the goclaw_short label";
    assert_eq!(redact_key(raw), raw);
}

#[test]
fn test_mime_resolution() {
    assert_eq!(mime_for("capture.png"), "image/png");
    assert_eq!(mime_for("capture.jpg"), "image/jpeg");
    assert_eq!(mime_for("capture.jpeg"), "image/jpeg");
    assert_eq!(mime_for("capture.webp"), "image/webp");
    assert_eq!(mime_for("capture.bmp"), "image/png");
    assert_eq!(mime_for("document.pdf"), "application/pdf");
    assert_eq!(mime_for("document.PDF"), "application/pdf");
}

#[test]
fn parse_chat_response_extracts_content() {
    let frame = r#"{"type":"res","id":"2","ok":true,"payload":{"content":"\\int x dx"}}"#;
    assert_eq!(parse_chat_response(frame).unwrap(), "\\int x dx");
}

#[test]
fn parse_chat_response_propagates_rate_limited() {
    let frame =
        r#"{"type":"res","id":"2","ok":false,"error":{"code":"RATE_LIMITED","message":"x"}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    assert!(matches!(err, CloudGoclawError::RateLimited));
}

#[test]
fn parse_chat_response_propagates_unauthorized() {
    let frame =
        r#"{"type":"res","id":"2","ok":false,"error":{"code":"UNAUTHORIZED","message":"x"}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    assert!(matches!(err, CloudGoclawError::AuthFailed(401)));
}

#[test]
fn parse_chat_response_propagates_not_found() {
    let frame =
        r#"{"type":"res","id":"2","ok":false,"error":{"code":"NOT_FOUND","message":"no agent"}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    match err {
        CloudGoclawError::BadRequest(m) => assert!(m.contains("no agent")),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn parse_chat_response_empty_content_returns_empty_response() {
    let frame = r#"{"type":"res","id":"2","ok":true,"payload":{"content":""}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    assert!(matches!(err, CloudGoclawError::EmptyResponse));
}

#[test]
fn parse_chat_response_skips_evt_frame() {
    let frame = r#"{"type":"evt","payload":{"kind":"typing"}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    assert!(matches!(err, CloudGoclawError::Parse(_)));
}

#[test]
fn parse_chat_response_skips_connect_ack_id() {
    let frame = r#"{"type":"res","id":"1","ok":true,"payload":{}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    assert!(matches!(err, CloudGoclawError::Parse(_)));
}

#[test]
fn parse_chat_response_unknown_error_code_maps_to_server_error() {
    let frame = r#"{"type":"res","id":"2","ok":false,"error":{"code":"WEIRD","message":"x"}}"#;
    let err = parse_chat_response(frame).unwrap_err();
    match err {
        CloudGoclawError::ServerError(500, m) => assert!(m.contains("WEIRD")),
        other => panic!("expected ServerError(500), got {other:?}"),
    }
}

#[test]
fn rate_limited_maps_to_dispatch_rate_limited() {
    let err: DispatchError = CloudGoclawError::RateLimited.into();
    assert!(matches!(err, DispatchError::RateLimited));
}

#[test]
fn auth_failed_preserves_status_code() {
    let err: DispatchError = CloudGoclawError::AuthFailed(403).into();
    assert!(matches!(err, DispatchError::AuthFailed(403)));
}

#[test]
fn bad_request_propagates_message() {
    let err: DispatchError = CloudGoclawError::BadRequest("agent tex-ocr missing".into()).into();
    match err {
        DispatchError::BadRequest(m) => assert!(m.contains("tex-ocr")),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn empty_response_maps_to_empty_output() {
    let err: DispatchError = CloudGoclawError::EmptyResponse.into();
    assert!(matches!(err, DispatchError::EmptyOutput));
}

#[test]
fn server_error_maps_to_non_zero_exit() {
    let err: DispatchError = CloudGoclawError::ServerError(502, "bad gateway".into()).into();
    match err {
        DispatchError::NonZeroExit { code: 502, stderr } => assert_eq!(stderr, "bad gateway"),
        other => panic!("expected NonZeroExit(502), got {other:?}"),
    }
}

#[test]
fn network_error_maps_to_dispatch_network() {
    let err: DispatchError = CloudGoclawError::Network("dns fail".into()).into();
    match err {
        DispatchError::Network(m) => assert_eq!(m, "dns fail"),
        other => panic!("expected Network, got {other:?}"),
    }
}

#[test]
fn parse_error_maps_to_bad_request() {
    let err: DispatchError = CloudGoclawError::Parse("missing field".into()).into();
    match err {
        DispatchError::BadRequest(m) => assert_eq!(m, "missing field"),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}
