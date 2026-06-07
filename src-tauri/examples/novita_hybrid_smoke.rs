//! Manual live smoke for the Novita DeepSeek-OCR 2 + GPT OSS 120B agent.
//!
//! Usage:
//!   NOVITA_API_KEY=sk_... cargo run --bin novita_hybrid_smoke -- <image-path> [--show-output]

use std::env;
use std::time::Instant;

use sniptex_lib::agents::cloud_novita_hybrid_api;
use sniptex_lib::ocr;

const GPT_INPUT_PER_1K_USD: f64 = 0.0001;
const GPT_OUTPUT_PER_1K_USD: f64 = 0.0005;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(image_path) = args.first() else {
        eprintln!("usage: novita_hybrid_smoke <image-path> [--show-output]");
        std::process::exit(2);
    };
    let show_output = args.iter().any(|a| a == "--show-output");
    let key = match env::var("NOVITA_API_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("NOVITA_API_KEY is required");
            std::process::exit(2);
        }
    };

    let started = Instant::now();
    match cloud_novita_hybrid_api::call_with_image_path(image_path, ocr::MASTER_PROMPT, &key).await
    {
        Ok(text) => {
            let elapsed = started.elapsed();
            let input_chars = std::fs::metadata(image_path).map(|m| m.len()).unwrap_or(0);
            let output_chars = text.chars().count();
            println!("ok: true");
            println!("latency_ms: {}", elapsed.as_millis());
            println!("image_bytes: {input_chars}");
            println!("output_chars: {output_chars}");
            println!(
                "gpt_cost_note: exact token usage unavailable; rough output-only estimate ${:.6}",
                estimate_output_cost(output_chars)
            );
            if show_output {
                println!("\n--- output ---\n{text}");
            }
        }
        Err(err) => {
            eprintln!("ok: false");
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}

fn estimate_output_cost(chars: usize) -> f64 {
    let rough_output_tokens = chars as f64 / 4.0;
    let fixed_prompt_tokens = 1200.0;
    (fixed_prompt_tokens / 1000.0 * GPT_INPUT_PER_1K_USD)
        + (rough_output_tokens / 1000.0 * GPT_OUTPUT_PER_1K_USD)
}
