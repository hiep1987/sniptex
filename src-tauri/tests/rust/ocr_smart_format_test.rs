use sniptex_lib::ocr::{detect_type, DetectedType};

#[test]
fn detect_type_returns_table_only_for_pure_markdown_table() {
    let s = "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
    assert_eq!(detect_type(s), DetectedType::TableOnly);
}

#[test]
fn detect_type_returns_mixed_for_table_with_surrounding_prose() {
    let s = "Below is the data:\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nNote the trend.";
    assert_eq!(detect_type(s), DetectedType::Mixed);
}

#[test]
fn detect_type_returns_equation_only_for_single_line_latex() {
    let s = "\\int_0^1 x^2 \\, dx = \\frac{1}{3}";
    assert_eq!(detect_type(s), DetectedType::EquationOnly);
}

#[test]
fn detect_type_returns_equation_only_for_raw_latex_even_with_newlines() {
    // Session-3 regression guard: multi-line equations must NOT be
    // mis-classified as MIXED just because they contain `\n\n`.
    let s = "\\frac{a}{b} = c \\\\\n\\sqrt{d^2 + e^2} = f";
    assert_eq!(detect_type(s), DetectedType::EquationOnly);
}

#[test]
fn detect_type_returns_mixed_for_text_with_inline_math() {
    let s = "The integral $\\int_0^1 x\\, dx$ evaluates to $\\frac{1}{2}$.";
    assert_eq!(detect_type(s), DetectedType::Mixed);
}

#[test]
fn detect_type_returns_mixed_for_markdown_heading() {
    let s = "# Section\n\nSome body text here.";
    assert_eq!(detect_type(s), DetectedType::Mixed);
}

#[test]
fn detect_type_returns_mixed_for_empty_input() {
    assert_eq!(detect_type(""), DetectedType::Mixed);
    assert_eq!(detect_type("   \n  "), DetectedType::Mixed);
}
