//! Ad-hoc smoke driver: feeds a raw agent output through the full
//! `post_process` → `markdown_tables_to_latex_tabular` pipeline and
//! prints the result, so live OCR responses can be verified against
//! the Mathpix reference without launching the GUI.
//!
//! Run: cargo run --bin tabular_e2e_smoke -- /tmp/codex-out.txt

use sniptex_lib::ocr;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).expect("usage: tabular_e2e_smoke <input-file>");
    let raw = fs::read_to_string(path).expect("read input");

    println!("=== RAW AGENT OUTPUT ({} chars) ===", raw.len());
    println!("{raw}\n");

    let after_pp = ocr::post_process(&raw);
    println!("=== AFTER post_process ({} chars) ===", after_pp.len());
    println!("{after_pp}\n");

    let after_tex = ocr::markdown_tables_to_latex_tabular(&after_pp);
    println!(
        "=== AFTER convert_to_tex (final clipboard payload, {} chars) ===",
        after_tex.len()
    );
    println!("{after_tex}\n");

    let checks: [(&str, bool); 7] = [
        ("\\multirow preserved", after_tex.contains("\\multirow")),
        (
            "\\multicolumn preserved",
            after_tex.contains("\\multicolumn"),
        ),
        ("\\cline preserved", after_tex.contains("\\cline")),
        (
            "\\begin{tabular} preserved",
            after_tex.contains("\\begin{tabular}"),
        ),
        ("Nhóm diacritic intact", after_tex.contains("Nhóm")),
        ("Loại I diacritic intact", after_tex.contains("Loại I")),
        ("NOT flattened to MD grid", !after_tex.contains("| Nhóm |")),
    ];
    println!("=== END-TO-END STRUCTURE CHECKS ===");
    let mut all_pass = true;
    for (name, ok) in checks {
        println!("  [{}] {name}", if ok { "PASS" } else { "FAIL" });
        if !ok {
            all_pass = false;
        }
    }
    std::process::exit(if all_pass { 0 } else { 1 });
}
