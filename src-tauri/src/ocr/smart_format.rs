//! Classify cleaned OCR output so the UI can pick the right renderer
//! (raw LaTeX vs Markdown table vs full Markdown document).
//!
//! Order matters: LaTeX-density heuristic must run BEFORE the
//! blank-line heuristic, otherwise multi-line equations get
//! mis-classified as MIXED (see Session-3 regression guard test).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DetectedType {
    EquationOnly,
    TableOnly,
    Mixed,
}

pub fn detect_type(output: &str) -> DetectedType {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return DetectedType::Mixed;
    }

    // Heuristic 1: Markdown table syntax.
    if trimmed.contains("|---") || trimmed.contains("| ---") {
        let non_table_lines = trimmed
            .lines()
            .filter(|l| !l.contains('|') && !l.trim().is_empty())
            .count();
        if non_table_lines == 0 {
            return DetectedType::TableOnly;
        }
        return DetectedType::Mixed;
    }

    // Heuristic 2: raw LaTeX commands with little natural prose
    //              (runs BEFORE the blank-line check so multi-line
    //              equations stay EQUATION_ONLY).
    let has_latex_command = trimmed.contains("\\frac")
        || trimmed.contains("\\int")
        || trimmed.contains("\\sum")
        || trimmed.contains("\\sqrt")
        || trimmed.contains("\\lim");
    let has_latex_super_sub = trimmed.contains('^') || trimmed.contains('_');
    let has_latex = has_latex_command || (trimmed.contains('\\') && has_latex_super_sub);

    let natural_words = trimmed
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.len() > 2 && !w.chars().any(|c| c == '\\'))
        .count();

    if has_latex && natural_words < 3 && !trimmed.contains('$') {
        return DetectedType::EquationOnly;
    }

    // Heuristic 3: $ delimiters or Markdown structure → MIXED.
    if trimmed.contains('$')
        || trimmed.contains("\n\n")
        || trimmed.starts_with("# ")
        || trimmed.starts_with("- ")
    {
        return DetectedType::Mixed;
    }

    DetectedType::Mixed
}
