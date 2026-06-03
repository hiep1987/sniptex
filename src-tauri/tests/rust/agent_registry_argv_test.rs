//! Lock the Session-3-verified argv shape for each adapter so a future
//! "cleanup" refactor can't silently break the CLI contract.

use sniptex_lib::agents::registry::{
    build_command_args, spec_by_id, CLOUD_GEMINI_ID, CLOUD_MISTRAL_ID, CODEX_ID,
    DEFAULT_FALLBACK_CHAIN, GEMINI_CLI_ID,
};

#[test]
fn codex_argv_matches_session_3_verified_shape() {
    let args = build_command_args(CODEX_ID, "/tmp/img.png", "PROMPT", Some("/tmp/last.txt"));
    assert_eq!(
        args,
        vec![
            "exec".to_string(),
            "--skip-git-repo-check".to_string(),
            "--image".to_string(),
            "/tmp/img.png".to_string(),
            "--output-last-message".to_string(),
            "/tmp/last.txt".to_string(),
            "--".to_string(),
            "PROMPT".to_string(),
        ]
    );
}

#[test]
fn codex_argv_omits_output_last_message_flag_when_none() {
    let args = build_command_args(CODEX_ID, "/tmp/img.png", "PROMPT", None);
    assert!(!args.iter().any(|a| a == "--output-last-message"));
    assert!(args.contains(&"--skip-git-repo-check".to_string()));
    assert!(args.last() == Some(&"PROMPT".to_string()));
}

#[test]
fn gemini_cli_uses_interactive_like_headless_prompt() {
    let args = build_command_args(GEMINI_CLI_ID, "/tmp/img.png", "PROMPT", None);
    let prompt = args
        .windows(2)
        .find(|pair| pair[0] == "-p")
        .map(|pair| pair[1].as_str())
        .expect("gemini-cli should receive a prompt");
    assert_eq!(prompt, "PROMPT @\"/tmp/img.png\"");
    assert!(!args.iter().any(|a| a == "--approval-mode"));
    assert!(args.iter().any(|arg| arg == "--skip-trust"));
    assert!(args
        .windows(2)
        .any(|pair| pair == ["--include-directories", "/tmp"]));
    assert!(args.windows(2).any(|pair| pair == ["--output-format", "text"]));
    assert!(args.windows(2).any(|pair| pair == ["-e", "none"]));
    assert!(!args.iter().any(|a| a == "--session-id"));
    assert!(!args.iter().any(|a| a == "--yolo" || a == "-y"));
}

#[test]
fn cloud_gemini_returns_empty_argv() {
    // Cloud adapter is dispatched in-process; the CLI argv builder must
    // refuse to produce arguments so a careless caller can't try to
    // spawn it as a process.
    let args = build_command_args(CLOUD_GEMINI_ID, "/tmp/img.png", "PROMPT", None);
    assert!(args.is_empty());
}

#[test]
fn cloud_mistral_returns_empty_argv() {
    let args = build_command_args(CLOUD_MISTRAL_ID, "/tmp/img.png", "PROMPT", None);
    assert!(args.is_empty());
}

#[test]
fn fallback_chain_includes_mistral_after_gemini() {
    let gemini_idx = DEFAULT_FALLBACK_CHAIN
        .iter()
        .position(|id| *id == CLOUD_GEMINI_ID)
        .expect("fallback chain should include cloud-gemini");
    let mistral_idx = DEFAULT_FALLBACK_CHAIN
        .iter()
        .position(|id| *id == CLOUD_MISTRAL_ID)
        .expect("fallback chain should include cloud-mistral");
    assert!(mistral_idx > gemini_idx);
}

#[test]
fn spec_by_id_finds_mistral() {
    let spec = spec_by_id(CLOUD_MISTRAL_ID).expect("cloud-mistral spec should exist");
    assert_eq!(spec.display_name, "Mistral Vision API");
    assert!(spec.supports_vision);
}
