//! Smoke-test harness for the OCR pipeline.
//!
//! Usage:
//!   cargo run --bin cli_test -- --image PATH [--agent codex|gemini-cli|cloud-gemini]
//!
//! Exits non-zero on timeout / rate limit / empty output so CI can gate
//! on it once test fixtures + API keys are wired up.

use std::process::ExitCode;

use sniptex_lib::agents::{self, registry::AgentInfo};
use sniptex_lib::ocr::{detect_type, run_ocr, run_with_fallback};

#[derive(Default)]
struct Cli {
    image: Option<String>,
    agent: Option<String>,
}

fn parse_args() -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--image" => cli.image = args.next(),
            "--agent" => cli.agent = args.next(),
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if cli.image.is_none() {
        return Err("--image PATH is required".into());
    }
    Ok(cli)
}

fn print_usage() {
    eprintln!(
        "Usage: cli_test --image PATH [--agent codex|gemini-cli|cloud-gemini]\n\
         \n\
         When --agent is omitted, run_with_fallback is used over all detected\n\
         agents in the default priority order (codex → cloud-gemini → gemini-cli)."
    );
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let image = cli.image.expect("validated above");
    let installed = agents::detect_installed_agents();
    if installed.is_empty() {
        eprintln!("no agents detected (install codex / gemini, or set a Gemini API key)");
        return ExitCode::from(3);
    }

    eprintln!("detected agents:");
    for a in &installed {
        eprintln!(
            "  - {:<14} kind={:?}  bin={}  version={}",
            a.spec.id,
            a.spec.kind,
            a.binary_path.display(),
            a.version.as_deref().unwrap_or("?")
        );
    }

    let result = match cli.agent {
        Some(id) => {
            let agent: AgentInfo = match installed.iter().find(|a| a.spec.id == id) {
                Some(a) => a.clone(),
                None => {
                    eprintln!("agent not installed: {id}");
                    return ExitCode::from(4);
                }
            };
            eprintln!("\nrunning OCR with agent: {id}");
            run_ocr(&agent, &image).await.map(|t| (t, agent))
        }
        None => {
            eprintln!("\nrunning OCR with fallback chain");
            let default_priority: Vec<String> =
                sniptex_lib::agents::registry::DEFAULT_FALLBACK_CHAIN
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            run_with_fallback(&installed, &image, &default_priority)
                .await
                .map(|(t, a)| (t, a.clone()))
        }
    };

    match result {
        Ok((text, agent)) => {
            let detected = detect_type(&text);
            eprintln!(
                "\n✓ success via {} ({} chars, detected={:?})",
                agent.spec.id,
                text.chars().count(),
                detected
            );
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("\n✗ all agents failed: {e}");
            ExitCode::from(1)
        }
    }
}
