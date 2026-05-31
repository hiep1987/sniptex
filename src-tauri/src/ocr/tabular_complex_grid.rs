//! Reconstruct LaTeX merged-cell tables from flattened Markdown grids.
//!
//! Cloud OCR APIs can preserve text accurately while flattening rowspan /
//! colspan information into a blank "subheader" row. This module handles
//! that narrow shape before the generic Markdown-table converter runs.

pub(super) fn convert_flattened_complex_grid(
    header: &[String],
    aligns: &[char],
    body: &[Vec<String>],
) -> Option<String> {
    if body.len() < 2 {
        return None;
    }

    let col_count = column_count(header, aligns, body);
    if col_count < 3 {
        return None;
    }

    if let Some(tex) = convert_title_span_grid(header, body, col_count) {
        return Some(tex);
    }

    let top = pad_row(header, col_count);
    let sub = pad_row(&body[0], col_count);
    let span_start = sub.iter().position(|c| !is_empty(c))?;
    if span_start == 0 || col_count - span_start < 2 {
        return None;
    }
    if !sub[..span_start].iter().all(|c| is_empty(c)) {
        return None;
    }
    if top[..span_start].iter().any(|c| is_empty(c)) || is_empty(&top[span_start]) {
        return None;
    }
    if sub[span_start..].iter().any(|c| is_empty(c)) {
        return None;
    }
    if top[span_start + 1..].iter().any(|c| !is_empty(c)) {
        return None;
    }

    let data_rows: Vec<Vec<String>> = body[1..].iter().map(|r| pad_row(r, col_count)).collect();
    if data_rows.is_empty() || data_rows.iter().any(|r| is_empty(&r[0])) {
        return None;
    }

    let mut out = String::new();
    out.push_str(&format!("\\begin{{tabular}}{{{}}}\n\\hline\n", centered_spec(col_count)));

    let mut first_row = Vec::new();
    for cell in &top[..span_start] {
        first_row.push(format!("\\multirow{{2}}{{*}}{{{}}}", header_cell(cell)));
    }
    first_row.push(format!(
        "\\multicolumn{{{}}}{{c|}}{{{}}}",
        col_count - span_start,
        header_cell(&top[span_start])
    ));
    out.push_str(&first_row.join(" & "));
    out.push_str(" \\\\\n");
    out.push_str(&format!("\\cline{{{}-{}}}\n", span_start + 1, col_count));

    let mut second_row = vec![String::new(); span_start];
    second_row.extend(sub[span_start..].iter().map(|c| clean_cell(c)));
    out.push_str(&second_row.join(" & "));
    out.push_str(" \\\\\n\\hline\n");

    for row in data_rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(idx, cell)| data_cell(idx, cell))
            .collect();
        out.push_str(&cells.join(" & "));
        out.push_str(" \\\\\n\\hline\n");
    }
    out.push_str("\\end{tabular}\n");
    Some(out)
}

fn convert_title_span_grid(
    header: &[String],
    body: &[Vec<String>],
    col_count: usize,
) -> Option<String> {
    let title = header.first().map(|c| clean_cell(c))?;
    if title.is_empty() {
        return None;
    }
    let padded_header = pad_row(header, col_count);
    if padded_header[1..].iter().any(|c| !is_empty(c)) {
        return None;
    }

    let labels = pad_row(body.first()?, col_count);
    if labels.iter().any(|c| is_empty(c)) {
        return None;
    }
    let data_rows: Vec<Vec<String>> = body[1..].iter().map(|r| pad_row(r, col_count)).collect();
    if data_rows.is_empty() || data_rows.iter().any(|r| is_empty(&r[0])) {
        return None;
    }

    let mut out = String::new();
    out.push_str(&format!("\\begin{{tabular}}{{{}}}\n", title_span_spec(col_count)));
    out.push_str("\\hline\n");
    out.push_str(&format!("\\multicolumn{{{col_count}}}{{|c|}}{{{title}}} \\\\\n"));
    out.push_str("\\hline\n");
    out.push_str(&join_clean_cells(&labels));
    out.push_str(" \\\\\n\\hline\n");
    for row in data_rows {
        out.push_str(&join_clean_cells(&row));
        out.push_str(" \\\\\n");
    }
    out.push_str("\\hline\n\\end{tabular}\n");
    Some(out)
}

fn column_count(header: &[String], aligns: &[char], body: &[Vec<String>]) -> usize {
    body.iter()
        .map(Vec::len)
        .chain([header.len(), aligns.len()])
        .max()
        .unwrap_or(0)
}

fn pad_row(row: &[String], width: usize) -> Vec<String> {
    let mut out = row.to_vec();
    out.resize(width, String::new());
    out
}

fn title_span_spec(cols: usize) -> String {
    let mut spec = String::from("|l|");
    for _ in 1..cols {
        spec.push_str("c|");
    }
    spec
}

fn join_clean_cells(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| clean_cell(c))
        .collect::<Vec<_>>()
        .join(" & ")
}

fn centered_spec(cols: usize) -> String {
    let mut spec = String::new();
    for _ in 0..cols {
        spec.push_str("|c");
    }
    spec.push('|');
    spec
}

fn is_empty(cell: &str) -> bool {
    clean_cell(cell).is_empty()
}

fn clean_cell(cell: &str) -> String {
    let trimmed = cell.trim();
    trimmed
        .strip_prefix("**")
        .and_then(|s| s.strip_suffix("**"))
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

fn header_cell(cell: &str) -> String {
    let clean = clean_cell(cell);
    let words: Vec<&str> = clean.split_whitespace().collect();
    if words.len() < 4 {
        return clean;
    }
    let lines = split_header_lines(&words);
    format!("\\begin{{tabular}}{{c}}{}\\end{{tabular}}", lines.join("\\\\"))
}

fn split_header_lines<'a>(words: &[&'a str]) -> Vec<String> {
    if let Some(idx) = words.iter().position(|w| *w == "để") {
        if idx > 0 {
            return vec![words[..idx].join(" "), words[idx..].join(" ")];
        }
    }
    if words.len() <= 4 {
        return vec![words[..words.len() - 1].join(" "), words[words.len() - 1].into()];
    }
    let mid = (words.len() + 1) / 2;
    vec![words[..mid].join(" "), words[mid..].join(" ")]
}

fn data_cell(col: usize, cell: &str) -> String {
    let clean = clean_cell(cell);
    if col == 0 && clean.starts_with('$') && clean.ends_with('$') {
        return clean;
    }
    if col == 0 && is_single_latin_label(&clean) {
        return format!("${clean}$");
    }
    clean
}

fn is_single_latin_label(cell: &str) -> bool {
    let unwrapped = cell.trim().trim_matches('$');
    unwrapped.len() == 1 && unwrapped.chars().all(|c| c.is_ascii_alphabetic())
}
