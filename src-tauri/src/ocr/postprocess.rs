//! Strip LLM scaffolding (preambles, fences, sign-offs, leaked thinking
//! transcripts) so downstream consumers see only the OCR body.

use regex::Regex;
use std::sync::OnceLock;

const PREAMBLES: &[&str] = &[
    "Here's",
    "Here is",
    "Sure!",
    "Sure,",
    "Of course",
    "Certainly",
    "Below is",
    "The image shows",
    "Đây là",
    "Sau đây là",
    "Dưới đây",
];

const SIGNOFFS: &[&str] = &[
    "Let me know",
    "Hope this helps",
    "Feel free to",
];

fn opening_fence_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^```(?:markdown|latex|md|tex)?\s*\n").unwrap())
}

fn closing_fence_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)\n```\s*$").unwrap())
}

fn leading_category_label_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)\A\s*(?:MIXED|EQUATION_ONLY|TABLE_ONLY)\s*\n+").unwrap())
}

fn thinking_marker_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:>(?:thought|instruction|reasoning|planning)|mekthought|section\}|CRITICAL INSTRUCTION|The user asked|I can see|I need to |I should |The image (?:has been|shows|contains)|The category is|Silently classify)").unwrap()
    })
}

fn ocr_content_start_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^(?:"|Let's do this directly\.)?(?:\*{0,2}(?:Câu|Bài|Ví dụ|Cho|Tìm|Xét|Trong|Gọi|Biết|Phương trình|Đường|Mặt|Hàm số|Tập|Giá trị|Số|Với|Một|Hai|Ba|Hỏi|Bao|Khi|Nếu|Có|Đề|Mốt|Trung bình|Phương sai)|\|[- ]|#{1,3} |\$\$|\\begin\{|\[UNREADABLE\])"#).unwrap()
    })
}

fn leading_junk_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\A[.>\s]+\n").unwrap())
}

/// Strip thinking/reasoning transcript from Gemini CLI output.
/// Strategy: if the output contains thinking markers, find the last
/// paragraph boundary before actual OCR content starts and discard
/// everything above it.
fn strip_thinking_transcript(raw: &str) -> String {
    if !thinking_marker_re().is_match(raw) {
        return raw.to_string();
    }

    if let Some(idx) = raw.rfind("Let's do this directly.") {
        return raw[idx..].to_string();
    }

    let content_re = ocr_content_start_re();
    if let Some(m) = content_re.find(raw) {
        let before = &raw[..m.start()];
        let last_boundary = before.rfind("\n\n").map(|i| i + 2)
            .or_else(|| before.rfind('\n').map(|i| i + 1))
            .unwrap_or(m.start());
        return raw[last_boundary..].to_string();
    }

    raw.to_string()
}

fn textbf_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\\textbf\{([^}]*)\}").unwrap())
}

fn textit_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\\textit\{([^}]*)\}").unwrap())
}

fn hspace_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\\hspace\{[^}]*\}").unwrap())
}

fn tabular_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)\\begin\{tabular\}\{[^}]*\}(.*?)\\end\{tabular\}").unwrap()
    })
}

fn enumerate_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)\\begin\{enumerate\}(?:\[[^\]]*\])?(.*?)\\end\{enumerate\}").unwrap()
    })
}

fn multicols_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)\\begin\{multicols\}\{[^}]*\}(.*?)\\end\{multicols\}").unwrap()
    })
}

fn normalize_latex_to_markdown(raw: &str) -> String {
    let mut s = raw.to_string();

    s = tabular_re()
        .replace_all(&s, |caps: &regex::Captures| {
            latex_tabular_to_md_table(&caps[1])
        })
        .to_string();

    s = enumerate_re()
        .replace_all(&s, |caps: &regex::Captures| {
            latex_enumerate_to_md(&caps[1])
        })
        .to_string();

    s = multicols_re().replace_all(&s, "$1").to_string();

    s = textbf_re().replace_all(&s, "**$1**").to_string();
    s = textit_re().replace_all(&s, "*$1*").to_string();
    s = hspace_re().replace_all(&s, " ").to_string();
    s = s.replace("\\par", "\n\n");
    s = s.replace("\\begin{center}", "");
    s = s.replace("\\end{center}", "");
    s = s.replace("\\\\", "\n");
    s
}

fn latex_enumerate_to_md(body: &str) -> String {
    let labels = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
    let mut idx = 0;
    let mut out = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\\item") {
            let label = labels.get(idx).copied().unwrap_or('?');
            idx += 1;
            out.push_str(&format!("**{label}.** {}\n\n", rest.trim()));
        }
    }
    out
}

fn latex_tabular_to_md_table(body: &str) -> String {
    let cleaned = body
        .replace("\\hline", "")
        .replace("\\\\", "\n");

    let rows: Vec<&str> = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if rows.is_empty() {
        return String::new();
    }

    let mut md_rows: Vec<String> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let cells: Vec<&str> = row.split('&').map(|c| c.trim()).collect();
        let md_row = format!("| {} |", cells.join(" | "));
        md_rows.push(md_row);
        if i == 0 {
            let sep = cells.iter().map(|_| "---").collect::<Vec<_>>();
            md_rows.push(format!("| {} |", sep.join(" | ")));
        }
    }
    md_rows.join("\n")
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub fn post_process(raw: &str) -> String {
    let mut s = raw.trim().to_string();

    // 0a. Decode HTML entities (Mistral OCR returns &lt; &gt; in math).
    if s.contains("&lt;") || s.contains("&gt;") || s.contains("&amp;") {
        s = decode_html_entities(&s);
    }

    // 0b. Strip thinking/reasoning transcripts (Gemini CLI).
    s = strip_thinking_transcript(&s);
    s = s.trim().to_string();

    // 0b. Convert raw LaTeX to Markdown when agent outputs LaTeX instead of MD.
    if s.contains("\\begin{") || s.contains("\\textbf{") || s.contains("\\item") {
        s = normalize_latex_to_markdown(&s);
    }

    // 1. Strip leaked category labels from the very top.
    s = leading_category_label_re().replace(&s, "").to_string();

    // 1b. Strip stray leading punctuation junk.
    s = leading_junk_re().replace(&s, "").to_string();
    if let Some(rest) = s.strip_prefix("Let's do this directly.") {
        s = rest.trim_start().to_string();
    }
    if let Some(rest) = s.strip_prefix('"') {
        s = rest.trim_start().to_string();
    }
    if let Some(head) = s.strip_suffix('"') {
        s = head.trim_end().to_string();
    }

    // 2. Strip a preamble line if present.
    for p in PREAMBLES {
        if s.starts_with(p) {
            if let Some(idx) = s.find('\n') {
                s = s[idx + 1..].trim_start().to_string();
            } else {
                s.clear();
            }
            break;
        }
    }

    // 3. Strip opening/closing code fences.
    s = opening_fence_re().replace(&s, "").to_string();
    s = closing_fence_re().replace(&s, "").to_string();
    if let Some(rest) = s.strip_prefix("```\n") {
        s = rest.to_string();
    }
    if let Some(head) = s.strip_suffix("\n```") {
        s = head.to_string();
    }

    // 4. Strip sign-off blocks that follow a blank line.
    for so in SIGNOFFS {
        let marker = format!("\n\n{}", so);
        if let Some(idx) = s.find(&marker) {
            s.truncate(idx);
        }
    }

    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_html_entities_in_math() {
        let input = "$x_1 &lt; x_2 \\Rightarrow f(x_1) &gt; f(x_2)$";
        let result = post_process(input);
        assert_eq!(result, "$x_1 < x_2 \\Rightarrow f(x_1) > f(x_2)$");
    }

    #[test]
    fn strips_gemini_thinking_with_numbered_marker() {
        let input = "-894>thought\nThe image has been provided. Let me analyze it.\nThe category is MIXED.\nI need to format it properly.\n\n**Câu 10.** Đồ thị hàm số $y=\\frac{2x}{4}$\n\nA. $y=2x+6$.";
        let result = post_process(input);
        assert!(result.starts_with("**Câu 10.**"), "got: {result}");
        assert!(!result.contains("thought"));
        assert!(!result.contains("category"));
    }

    #[test]
    fn strips_mekthought_with_prompt_echo() {
        let input = "mekthought\nCRITICAL INSTRUCTION 1: ALWAYS prioritize.\nThe user asked me to convert the image.\nThis is a MIXED category.\nOutput Markdown.\nInline math: $...$\nDo NOT translate.\nMath symbols: use standard LaTeX.\n\n**Câu 6.** Trong không gian $Oxyz$";
        let result = post_process(input);
        assert!(result.starts_with("**Câu 6.**"), "got: {result}");
    }

    #[test]
    fn strips_leading_dot() {
        let input = ".\n**Câu 4.** Trong không gian";
        let result = post_process(input);
        assert!(result.starts_with("**Câu 4.**"), "got: {result}");
    }

    #[test]
    fn strips_section_prefix() {
        let input = "section}\nCho hàm số $y=x^2$.";
        let result = post_process(input);
        assert!(result.starts_with("Cho hàm số"), "got: {result}");
    }

    #[test]
    fn preserves_clean_ocr() {
        let input = "**Câu 4.** Trong không gian $Oxyz$\n\nA. $M(3;5;-2)$.";
        let result = post_process(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strips_category_label() {
        let input = "MIXED\n\n**Câu 4.** Nội dung";
        let result = post_process(input);
        assert!(result.starts_with("**Câu 4.**"), "got: {result}");
    }

    #[test]
    fn strips_verbose_reasoning_before_table() {
        let input = "I should classify this as TABLE_ONLY.\nThe image contains a table.\nLet me format it.\n\n| A | B |\n|---|---|\n| 1 | 2 |";
        let result = post_process(input);
        assert!(result.starts_with("| A | B |"), "got: {result}");
    }

    #[test]
    fn strips_thinking_before_latex() {
        let input = "42>thought\nI need to extract the equation.\n\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$";
        let result = post_process(input);
        assert!(result.starts_with("$$\\int"), "got: {result}");
    }

    #[test]
    fn no_false_positive_on_clean_vietnamese() {
        let input = "Trong không gian $Oxyz$, cho điểm $A(1;2;3)$.";
        let result = post_process(input);
        assert_eq!(result, input);
    }

    #[test]
    fn handles_thinking_without_double_newline() {
        let input = "-5>thought\nThe image shows a math problem.\nCâu 4. Tìm $x$ sao cho $x^2=4$.";
        let result = post_process(input);
        assert!(result.starts_with("Câu 4."), "got: {result}");
    }

    #[test]
    fn converts_latex_tabular_to_markdown_table() {
        let input = "\\textbf{Câu 12.} Một vườn thú ghi lại tuổi của 20 con hổ\n\\begin{center}\n\\begin{tabular}{|l|c|c|c|c|c|}\n\\hline\nTuổi thọ ( năm) & [14;15) & [15;16) & [16;17) & [17;18) & [18;19) \\\\\\hline\nSố con hổ & 1 & 3 & 8 & 6 & 2 \\\\\\hline\n\\end{tabular}\n\\end{center}\nKhoảng biến thiên của mẫu số liệu trên bảng\n\\par\n\\textbf{A.} 6. \\hspace{2cm} \\textbf{B.} 8. \\hspace{2cm} \\textbf{C.} 5. \\hspace{2cm} \\textbf{D.} 7.";
        let result = post_process(input);
        assert!(result.contains("**Câu 12.**"), "missing bold: {result}");
        assert!(result.contains("| Tuổi thọ ( năm) |"), "missing table: {result}");
        assert!(result.contains("| --- |"), "missing separator: {result}");
        assert!(result.contains("**A.** 6."), "missing options: {result}");
        assert!(!result.contains("\\textbf"), "raw latex leaked: {result}");
        assert!(!result.contains("\\begin"), "raw latex leaked: {result}");
    }

    #[test]
    fn converts_latex_enumerate_multicols_to_markdown() {
        let input = "Câu 12. Nội dung\n\\begin{multicols}{4}\n\\begin{enumerate}[label=\\Alph*.]\n\\item 6.\n\\item 8.\n\\item 5.\n\\item 7.\n\\end{enumerate}\n\\end{multicols}";
        let result = post_process(input);
        assert!(result.contains("**A.** 6."), "missing A: {result}");
        assert!(result.contains("**B.** 8."), "missing B: {result}");
        assert!(result.contains("**D.** 7."), "missing D: {result}");
        assert!(!result.contains("\\begin"), "raw latex leaked: {result}");
        assert!(!result.contains("\\item"), "raw item leaked: {result}");
        assert!(!result.contains("\\end"), "raw end leaked: {result}");
    }

    #[test]
    fn strips_gemini_cli_image_read_scaffold() {
        let input = ".  I can see the image now.\nThe image contains the following text:\n\"Câu 1. Tiệm cận đứng của đồ thị hàm số $y=\\frac{x^2+4x+1}{x-1}$ là\nA. $y=1$.\"\n\nThe user asked to OCR the image.\n\nLet's do this directly.Câu 1. Tiệm cận đứng của đồ thị hàm số $y=\\frac{x^2+4x+1}{x-1}$ là\nA. $y=1$.";
        let result = post_process(input);
        assert!(result.starts_with("Câu 1."), "got: {result}");
        assert!(!result.contains("I can see"));
        assert!(!result.contains("The user asked"));
    }
}
