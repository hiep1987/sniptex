use sniptex_lib::agents::cloud_novita_hybrid_api::{
    looks_hallucinated, mime_for, needs_gpt_cleanup, normalize_intermediate_markdown,
    parse_gpt_oss_response, redact_key, redact_url_secrets, CloudNovitaHybridError,
    GPT_OSS_ENDPOINT, GPT_OSS_MODEL, MAX_GPT_TOKENS,
};
use sniptex_lib::ocr::DispatchError;

#[test]
fn constants_are_pinned() {
    assert_eq!(GPT_OSS_MODEL, "openai/gpt-oss-120b");
    assert_eq!(
        GPT_OSS_ENDPOINT,
        "https://api.novita.ai/openai/v1/chat/completions"
    );
    assert_eq!(MAX_GPT_TOKENS, 8192);
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
fn clean_table_output_skips_gpt_cleanup() {
    let text = r#"Câu 7.
\begin{tabular}{|l|l|l|}
\hline
Giá trị & [135;140) & [140;145) \\ \hline
Tần số & 6 & 10 \\ \hline
\end{tabular}"#;
    assert!(!needs_gpt_cleanup(text));
}

#[test]
fn escaped_latex_source_triggers_gpt_cleanup() {
    let text = r#"Câu 7.
\begin{tabular}{|l|l|l|l|}
\hline
\textbackslash{begin} & \textbackslash{tabular} & \{ & \} \\ \hline
\textbackslash{hline} & Giá trị & \ & [135;140) \\ \hline
\end{tabular}
\text{end{tabular}"#;
    assert!(needs_gpt_cleanup(text));
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
fn hallucinated_boxed_answer_is_rejected() {
    let source = "Câu 7. Mốt của mẫu số liệu bằng\nA. 151.75. B. 20. C. 152. D. 151.5.";
    let output = "Câu 7. Mốt của mẫu số liệu bằng\nA. 151.75. B. 20. C. 152. D. 151.5.\n\\boxed{\\bar{x}=151.5}";
    assert!(looks_hallucinated(source, output));
}

#[test]
fn hallucinated_aligned_derivation_is_rejected() {
    let source = "\\begin{tabular}{|c|c|}\n\\hline\n[135;140) & 6 \\\\ \\hline\n\\end{tabular}";
    let output = "\\begin{tabular}{|c|c|}\n\\hline\n[135;140) & 6 \\\\ \\hline\n\\end{tabular}\n\\begin{aligned}\nN &= 6+10+12 = 28\n\\end{aligned}";
    assert!(looks_hallucinated(source, output));
}

#[test]
fn hallucinated_bar_statistical_symbol_is_rejected() {
    let source = "Tính trung bình mẫu số liệu sau:";
    let output = "Tính trung bình mẫu số liệu sau:\n\\bar{x} = 151.5";
    assert!(looks_hallucinated(source, output));
}

#[test]
fn legitimate_boxed_already_in_source_passes() {
    // Image legitimately captured a \boxed{} answer; GPT keeping it is NOT a hallucination.
    let source = "Đáp số: \\boxed{42}";
    let output = "Đáp số: \\boxed{42}";
    assert!(!looks_hallucinated(source, output));
}

#[test]
fn clean_table_cleanup_passes_hallucination_check() {
    let source = "Câu 7. \\textbackslash{begin}{tabular} ... Tần số & 6 & 10 \\\\ A. 5. B. 6.";
    let output = "Câu 7. \\begin{tabular} ... Tần số & 6 & 10 \\\\ A. 5. B. 6.";
    assert!(!looks_hallucinated(source, output));
}

#[test]
fn hybrid_errors_map_to_dispatch_errors() {
    let err: DispatchError = CloudNovitaHybridError::RateLimited.into();
    assert!(matches!(err, DispatchError::RateLimited));

    let err: DispatchError = CloudNovitaHybridError::AuthFailed(401).into();
    assert!(matches!(err, DispatchError::AuthFailed(401)));
}
