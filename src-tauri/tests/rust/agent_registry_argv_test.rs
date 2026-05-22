//! Lock the Session-3-verified argv shape for each adapter so a future
//! "cleanup" refactor can't silently break the CLI contract.

use sniptex_lib::agents::registry::{
    build_command_args, CLOUD_GEMINI_ID, CODEX_ID, GEMINI_CLI_ID,
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
fn gemini_cli_uses_plan_approval_mode_gate() {
    let args = build_command_args(GEMINI_CLI_ID, "/tmp/img.png", "PROMPT", None);
    // Plan mode is the entire safety contract — no --yolo, no other
    // approval modes. Drift here re-opens the Phase-1 tool-loop failure.
    assert!(args.contains(&"--approval-mode".to_string()));
    assert!(args.contains(&"plan".to_string()));
    assert!(!args.iter().any(|a| a == "--yolo" || a == "-y"));
    assert!(args.iter().any(|a| a.contains("@\"/tmp/img.png\"")));
}

#[test]
fn cloud_gemini_returns_empty_argv() {
    // Cloud adapter is dispatched in-process; the CLI argv builder must
    // refuse to produce arguments so a careless caller can't try to
    // spawn it as a process.
    let args = build_command_args(CLOUD_GEMINI_ID, "/tmp/img.png", "PROMPT", None);
    assert!(args.is_empty());
}
