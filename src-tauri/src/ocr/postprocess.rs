//! Strip LLM scaffolding (preambles, fences, sign-offs, leaked category
//! labels) so downstream consumers see only the OCR body.
//!
//! Defensive against agents that occasionally ignore the prompt's
//! "do not emit category name" rule — see test
//! `post_process_strips_leading_category_label`.

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

pub fn post_process(raw: &str) -> String {
    let mut s = raw.trim().to_string();

    // 1. Strip leaked category labels from the very top.
    s = leading_category_label_re().replace(&s, "").to_string();

    // 2. Strip a preamble line if present (only the first line, only if it
    //    starts with a known preamble — keeps real content intact).
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

    // 3. Strip opening/closing code fences (``` / ```markdown / ```latex / ```md / ```tex).
    s = opening_fence_re().replace(&s, "").to_string();
    s = closing_fence_re().replace(&s, "").to_string();
    // Edge case: file is wrapped in bare ``` ... ``` on first/last line.
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
