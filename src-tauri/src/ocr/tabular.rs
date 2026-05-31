//! Markdown table → LaTeX `tabular` converter.
//!
//! Counterpart to `postprocess::latex_tabular_to_md_table`. The OCR
//! pipeline normalises every TABLE_ONLY / MIXED output to Markdown so
//! the preview renderer (markdown-it + KaTeX) has a single shape to
//! consume. When the user picks "Copy as TeX", this module flips
//! tables back to `\begin{tabular}` so the paste lands cleanly in a
//! `.tex` file. Non-table prose and `$…$` math are passed through
//! verbatim — pandoc-style heavy Markdown→LaTeX conversion is out of
//! scope.

use regex::Regex;
use std::sync::OnceLock;

use super::tabular_complex_grid::convert_flattened_complex_grid;

/// Matches a contiguous GitHub Markdown table block:
/// header row, separator row (`|---|---|` with optional alignment colons),
/// and one-or-more body rows. All rows must start with `|` at line start.
fn md_table_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?m)^\|[^\n]*\|\s*\n\|[\s:|-]+\|\s*\n(?:\|[^\n]*\|\s*\n?)+",
        )
        .unwrap()
    })
}

/// Convert every GitHub Markdown table inside `text` to a LaTeX
/// `tabular` environment. Surrounding text is preserved verbatim.
pub fn markdown_tables_to_latex_tabular(text: &str) -> String {
    md_table_block_re()
        .replace_all(text, |caps: &regex::Captures| {
            convert_one_table(caps.get(0).unwrap().as_str())
        })
        .to_string()
}

fn convert_one_table(block: &str) -> String {
    let rows: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
    if rows.len() < 2 {
        return block.to_string(); // malformed — leave alone
    }

    let header = split_cells(rows[0]);
    let aligns = parse_alignment_row(rows[1], header.len());
    let body: Vec<Vec<String>> = rows[2..].iter().map(|r| split_cells(r)).collect();
    if let Some(tex) = convert_flattened_complex_grid(&header, &aligns, &body) {
        return tex;
    }

    let col_spec: String = aligns.iter().map(|a| format!("|{a}")).collect::<String>() + "|";

    let mut out = String::new();
    out.push_str(&format!("\\begin{{tabular}}{{{col_spec}}}\n\\hline\n"));
    out.push_str(&join_row(&header));
    out.push_str(" \\\\ \\hline\n");
    for row in &body {
        // Pad short rows with empty cells so column count matches.
        let mut padded = row.clone();
        while padded.len() < header.len() {
            padded.push(String::new());
        }
        out.push_str(&join_row(&padded));
        out.push_str(" \\\\ \\hline\n");
    }
    out.push_str("\\end{tabular}\n");
    out
}

fn split_cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    // Strip exactly one leading and one trailing pipe; interior pipes
    // are the cell separators. Markdown does not support escaped pipes
    // inside math here (the OCR prompt produces plain `$...$` cells).
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or(trimmed);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_alignment_row(row: &str, expected: usize) -> Vec<char> {
    let cells = split_cells(row);
    let mut aligns: Vec<char> = cells
        .iter()
        .map(|c| {
            let s = c.trim();
            let left = s.starts_with(':');
            let right = s.ends_with(':');
            match (left, right) {
                (true, true) => 'c',
                (false, true) => 'r',
                _ => 'l',
            }
        })
        .collect();
    while aligns.len() < expected {
        aligns.push('l');
    }
    aligns.truncate(expected);
    aligns
}

fn join_row(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join(" & ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_cau_7_price_table() {
        // Direct from the user's "câu 7" fixture: Markdown output the
        // SnipTeX agents produce today for the price-range table.
        let md = "| Mức giá | $[6;9)$ | $[9;12)$ | $[12;15)$ | $[15;18)$ | $[18;21)$ |\n|---|:---:|:---:|:---:|:---:|:---:|\n| Số khách hàng | 20 | 75 | 48 | 23 | 12 |\n";
        let tex = markdown_tables_to_latex_tabular(md);
        assert!(
            tex.starts_with("\\begin{tabular}{|l|c|c|c|c|c|}"),
            "got: {tex}"
        );
        assert!(tex.contains("Mức giá & $[6;9)$ & $[9;12)$"), "got: {tex}");
        assert!(
            tex.contains("Số khách hàng & 20 & 75 & 48 & 23 & 12 \\\\ \\hline"),
            "got: {tex}"
        );
        assert!(tex.trim_end().ends_with("\\end{tabular}"), "got: {tex}");
    }

    #[test]
    fn preserves_non_table_prose_around_table() {
        let md = "**Câu 7.** Một cửa hàng khảo sát.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nTìm khoảng tứ phân vị.";
        let out = markdown_tables_to_latex_tabular(md);
        assert!(out.starts_with("**Câu 7.**"), "lost prose: {out}");
        assert!(out.contains("\\begin{tabular}{|l|l|}"), "no tabular: {out}");
        assert!(out.contains("Tìm khoảng tứ phân vị."), "lost trailing prose: {out}");
    }

    #[test]
    fn defaults_alignment_to_left_when_separator_unmarked() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let out = markdown_tables_to_latex_tabular(md);
        assert!(out.contains("\\begin{tabular}{|l|l|}"), "got: {out}");
    }

    #[test]
    fn handles_mixed_alignment_left_center_right() {
        let md = "| a | b | c |\n|:---|:---:|---:|\n| 1 | 2 | 3 |\n";
        let out = markdown_tables_to_latex_tabular(md);
        assert!(out.contains("\\begin{tabular}{|l|c|r|}"), "got: {out}");
    }

    #[test]
    fn converts_multiple_tables_independently() {
        let md = "intro\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nbetween\n\n| x | y |\n|---|---|\n| 9 | 8 |\n";
        let out = markdown_tables_to_latex_tabular(md);
        let count = out.matches("\\begin{tabular}").count();
        assert_eq!(count, 2, "expected 2 tabulars: {out}");
        assert!(out.contains("intro"));
        assert!(out.contains("between"));
    }

    #[test]
    fn leaves_text_without_tables_unchanged() {
        let md = "Just prose with $x^2$ inline math, no tables here.";
        let out = markdown_tables_to_latex_tabular(md);
        assert_eq!(out, md);
    }

    #[test]
    fn passes_through_raw_latex_tabular_unchanged() {
        // When an agent follows the new master-prompt branch and emits
        // LaTeX directly for a merged-cell table, `convert_to_tex`
        // must NOT try to re-wrap it. The MD-table regex doesn't
        // match `\begin{tabular}` blocks, so the body passes through
        // verbatim. This locks that contract.
        let tex = "\\begin{tabular}{|c|c|c|c|}\n\\hline \\multirow{2}{*}{ Nhóm } & \\multirow{2}{*}{Số máy mỗi nhóm} & \\multicolumn{2}{|c|}{Số máy trong từng nhóm} \\\\\n\\cline { 3 - 4 } & & Loại I & Loại II \\\\\n\\hline$A$ & 10 & 2 & 2 \\\\\n\\hline\n\\end{tabular}";
        let out = markdown_tables_to_latex_tabular(tex);
        assert_eq!(out, tex, "raw tabular must round-trip unchanged");
    }

    #[test]
    fn pads_short_body_rows_with_empty_cells() {
        let md = "| a | b | c |\n|---|---|---|\n| 1 | 2 |\n";
        let out = markdown_tables_to_latex_tabular(md);
        assert!(out.contains("1 & 2 &  \\\\ \\hline"), "got: {out}");
    }
}
