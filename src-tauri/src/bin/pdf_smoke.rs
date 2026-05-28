//! Smoke test: render a PDF to PNGs and verify each page has content.
//!
//! Usage:
//!   cargo run --bin pdf_smoke -- --pdf PATH [--dpi 200]

use std::path::Path;
use std::process::ExitCode;

use sniptex_lib::ocr::pdf_render;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut pdf: Option<String> = None;
    let mut dpi: f64 = 200.0;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--pdf" => pdf = args.next(),
            "--dpi" => dpi = args.next().and_then(|s| s.parse().ok()).unwrap_or(200.0),
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(pdf) = pdf else {
        eprintln!("--pdf PATH is required");
        return ExitCode::from(2);
    };

    let out = std::env::temp_dir().join("sniptex-pdf-smoke");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();

    eprintln!("rendering {} at {}dpi → {}", pdf, dpi, out.display());

    match pdf_render::page_count(&pdf) {
        Ok(n) => eprintln!("page_count = {n}"),
        Err(e) => {
            eprintln!("page_count failed: {e}");
            return ExitCode::from(3);
        }
    }

    let paths = match pdf_render::render_pages_to_pngs(&pdf, Path::new(&out), Some(dpi)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("render failed: {e}");
            return ExitCode::from(4);
        }
    };

    let mut all_ok = true;
    for (i, p) in paths.iter().enumerate() {
        let img = match image::open(p) {
            Ok(im) => im.to_rgba8(),
            Err(e) => {
                eprintln!("page {}: decode failed: {e}", i + 1);
                all_ok = false;
                continue;
            }
        };
        let total = img.pixels().count();
        let non_white = img
            .pixels()
            .filter(|px| !(px[0] > 240 && px[1] > 240 && px[2] > 240))
            .count();
        let pct = (non_white as f64 / total as f64) * 100.0;
        let blank = non_white < total / 100;
        eprintln!(
            "page {:>3}: {}x{}  non-white={}/{} ({:.2}%){}  {}",
            i + 1,
            img.width(),
            img.height(),
            non_white,
            total,
            pct,
            if blank { " ← BLANK!" } else { "" },
            p.display()
        );
        if blank {
            all_ok = false;
        }
    }

    if all_ok {
        eprintln!("\n✓ all {} pages rendered with content", paths.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("\n✗ at least one page is blank");
        ExitCode::from(1)
    }
}
