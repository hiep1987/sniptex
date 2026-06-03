use sniptex_lib::agents::cloud_novita_hybrid_api::{
    mime_for, normalize_intermediate_markdown, parse_gpt_oss_response, redact_key,
    redact_url_secrets, CloudNovitaHybridError, GPT_OSS_ENDPOINT, GPT_OSS_MODEL,
};
use sniptex_lib::ocr::DispatchError;

#[test]
fn constants_are_pinned() {
    assert_eq!(GPT_OSS_MODEL, "openai/gpt-oss-120b");
    assert_eq!(
        GPT_OSS_ENDPOINT,
        "https://api.novita.ai/openai/v1/chat/completions"
    );
}

#[test]
fn mime_for_common_extensions() {
    assert_eq!(mime_for("snap.png"), "image/png");
    assert_eq!(mime_for("snap.JPG"), "image/jpeg");
    assert_eq!(mime_for("snap.jpeg"), "image/jpeg");
    assert_eq!(mime_for("snap.webp"), "image/webp");
    assert_eq!(mime_for("snap.bmp"), "image/png");
}

#[test]
fn parse_gpt_oss_response_extracts_first_choice() {
    let json = r#"{"choices":[{"message":{"content":"\\frac{1}{2}"}}]}"#;
    assert_eq!(parse_gpt_oss_response(json).unwrap(), "\\frac{1}{2}");
}

#[test]
fn empty_outputs_error() {
    assert!(matches!(
        parse_gpt_oss_response(r#"{"choices":[{"message":{"content":""}}]}"#),
        Err(CloudNovitaHybridError::EmptyResponse)
    ));
}

#[test]
fn normalize_intermediate_caps_and_collapses_blank_lines() {
    let raw = "a\n\n\nb\n";
    assert_eq!(normalize_intermediate_markdown(raw), "a\n\nb");
}

#[test]
fn redact_strips_bearer_and_bare_key() {
    let raw = "Bearer sk_secretabcdefghijkl failed; sk_anothersecretvalue";
    let cleaned = redact_key(raw);
    assert!(!cleaned.contains("sk_secretabcdefghijkl"));
    assert!(!cleaned.contains("sk_anothersecretvalue"));
    assert!(cleaned.contains("Bearer <redacted>"));
}

#[test]
fn redact_url_secrets_strips_userinfo_and_query() {
    let raw = "failed at https://user:token@example.com/v1/chat?api_key=abc";
    let cleaned = redact_url_secrets(raw);
    assert!(!cleaned.contains("token"));
    assert!(!cleaned.contains("api_key=abc"));
    assert!(cleaned.contains("https://example.com/v1/chat?<redacted>"));
}

#[test]
fn hybrid_errors_map_to_dispatch_errors() {
    let err: DispatchError = CloudNovitaHybridError::RateLimited.into();
    assert!(matches!(err, DispatchError::RateLimited));

    let err: DispatchError = CloudNovitaHybridError::AuthFailed(401).into();
    assert!(matches!(err, DispatchError::AuthFailed(401)));
}
