use sniptex_lib::ocr::post_process;

#[test]
fn post_process_strips_preamble_then_returns_body() {
    let raw = "Here is the equation you requested:\n\\int_0^1 x^2 \\, dx = \\frac{1}{3}";
    let out = post_process(raw);
    assert_eq!(out, "\\int_0^1 x^2 \\, dx = \\frac{1}{3}");
}

#[test]
fn post_process_strips_vietnamese_preamble() {
    let raw = "Đây là kết quả OCR\n\\int_0^1 f(x) dx";
    let out = post_process(raw);
    assert_eq!(out, "\\int_0^1 f(x) dx");
}

#[test]
fn post_process_strips_fenced_latex_block() {
    let raw = "```latex\n\\frac{a}{b}\n```";
    let out = post_process(raw);
    assert_eq!(out, "\\frac{a}{b}");
}

#[test]
fn post_process_strips_fenced_markdown_block() {
    let raw = "```markdown\n# Heading\n\nbody\n```";
    let out = post_process(raw);
    assert_eq!(out, "# Heading\n\nbody");
}

#[test]
fn post_process_strips_leading_category_label() {
    // Session-3 regression guard: defense in depth against agents that
    // leak the silent classification label.
    for label in ["MIXED", "EQUATION_ONLY", "TABLE_ONLY"] {
        let raw = format!("{label}\n\\frac{{1}}{{2}}");
        let out = post_process(&raw);
        assert_eq!(out, "\\frac{1}{2}", "label `{label}` should be stripped");
    }
}

#[test]
fn post_process_strips_signoff_after_blank_line() {
    let raw = "answer line one\n\\frac{1}{2}\n\nHope this helps!";
    let out = post_process(raw);
    assert_eq!(out, "answer line one\n\\frac{1}{2}");
}

#[test]
fn post_process_leaves_clean_body_untouched() {
    let raw = "| a | b |\n|---|---|\n| 1 | 2 |";
    let out = post_process(raw);
    assert_eq!(out, raw);
}

#[test]
fn post_process_preserves_vietnamese_diacritics() {
    let raw = "Hàm số $f(x) = \\sin(x)$ có đạo hàm là $f'(x) = \\cos(x)$.";
    let out = post_process(raw);
    assert_eq!(out, raw);
}
