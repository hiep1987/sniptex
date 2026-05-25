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

pub fn post_process(raw: &str) -> String {
    let mut s = raw.trim().to_string();

    // 0. Strip thinking/reasoning transcripts (Gemini CLI).
    s = strip_thinking_transcript(&s);
    s = s.trim().to_string();

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
    fn strips_gemini_cli_image_read_scaffold() {
        let input = ".  I can see the image now.\nThe image contains the following text:\n\"Câu 1. Tiệm cận đứng của đồ thị hàm số $y=\\frac{x^2+4x+1}{x-1}$ là\nA. $y=1$.\"\n\nThe user asked to OCR the image.\n\nLet's do this directly.Câu 1. Tiệm cận đứng của đồ thị hàm số $y=\\frac{x^2+4x+1}{x-1}$ là\nA. $y=1$.";
        let result = post_process(input);
        assert!(result.starts_with("Câu 1."), "got: {result}");
        assert!(!result.contains("I can see"));
        assert!(!result.contains("The user asked"));
    }
}
