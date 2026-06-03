use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProblemLabel {
    kind: String,
    number: String,
}

impl ProblemLabel {
    fn display(&self) -> String {
        format!("{} {}", self.kind, self.number)
    }
}

pub fn validate_rerun_consistency(previous: &str, candidate: &str) -> Result<(), String> {
    let previous_tokens = content_tokens(previous);
    let candidate_tokens = content_tokens(candidate);

    if previous_tokens.len() < 8 || candidate_tokens.len() < 8 {
        return Ok(());
    }

    let previous_labels = problem_labels(previous);
    if !previous_labels.is_empty() {
        if candidate_labels_are_disjoint(candidate, &previous_labels) {
            return Err(format!(
                "rerun output contains no matching problem label for `{}`",
                display_labels(&previous_labels)
            ));
        }
    }

    let previous_set: HashSet<_> = previous_tokens.iter().cloned().collect();
    let candidate_set: HashSet<_> = candidate_tokens.iter().cloned().collect();
    let shared = previous_set.intersection(&candidate_set).count();
    let base = previous_set.len().min(candidate_set.len());
    let token_ratio = shared as f32 / base as f32;
    let ngram_ratio = ngram_overlap_ratio(&previous_tokens, &candidate_tokens);

    if token_ratio >= 0.38 || ngram_ratio >= 0.22 {
        Ok(())
    } else {
        Err(format!(
            "low overlap with existing history row ({shared}/{base} content tokens, {ngram_ratio:.2} ordered overlap)"
        ))
    }
}

fn candidate_labels_are_disjoint(candidate: &str, expected: &[ProblemLabel]) -> bool {
    let candidate_labels = problem_labels(candidate);
    !candidate_labels.is_empty()
        && !candidate_labels
            .iter()
            .any(|candidate_label| expected.contains(candidate_label))
}

fn problem_labels(text: &str) -> Vec<ProblemLabel> {
    let tokens = raw_tokens(text);
    let mut labels = Vec::new();
    for pair in tokens.windows(2) {
        let kind = pair[0].as_str();
        let number = pair[1].as_str();
        if matches!(kind, "câu" | "cau" | "bài" | "bai")
            && number.chars().any(|c| c.is_ascii_digit())
        {
            labels.push(ProblemLabel {
                kind: canonical_label_kind(kind).to_string(),
                number: number.to_string(),
            });
        }
    }
    labels
}

fn canonical_label_kind(kind: &str) -> &str {
    match kind {
        "câu" | "cau" => "cau",
        "bài" | "bai" => "bai",
        _ => kind,
    }
}

fn display_labels(labels: &[ProblemLabel]) -> String {
    labels
        .iter()
        .map(ProblemLabel::display)
        .collect::<Vec<_>>()
        .join(", ")
}

fn content_tokens(text: &str) -> Vec<String> {
    raw_tokens(text)
        .into_iter()
        .filter(|token| is_content_token(token))
        .collect()
}

fn ngram_overlap_ratio(previous: &[String], candidate: &[String]) -> f32 {
    let previous_ngrams = ngrams(previous, 3);
    let candidate_ngrams = ngrams(candidate, 3);
    let base = previous_ngrams.len().min(candidate_ngrams.len());
    if base == 0 {
        return 0.0;
    }
    let shared = previous_ngrams.intersection(&candidate_ngrams).count();
    shared as f32 / base as f32
}

fn ngrams(tokens: &[String], size: usize) -> HashSet<String> {
    tokens
        .windows(size)
        .map(|window| window.join("\u{1f}"))
        .collect()
}

fn raw_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn is_content_token(token: &str) -> bool {
    if token.chars().any(|c| c.is_ascii_digit()) {
        return token.chars().count() > 1;
    }
    if token.chars().count() < 2 {
        return false;
    }
    !matches!(
        token,
        "câu"
            | "cau"
            | "bài"
            | "bai"
            | "cho"
            | "gọi"
            | "goi"
            | "là"
            | "la"
            | "có"
            | "co"
            | "của"
            | "cua"
            | "và"
            | "va"
            | "một"
            | "mot"
            | "trong"
            | "dưới"
            | "duoi"
            | "đây"
            | "day"
            | "nào"
            | "nao"
            | "đúng"
            | "dung"
            | "các"
            | "cac"
            | "ta"
    )
}

#[cfg(test)]
mod tests {
    use super::validate_rerun_consistency;

    const CAU_9: &str = r#"Câu 9. Cho hình phẳng $(H)$ giới hạn bởi các đường $y=x^{2}+3,\ y=0,\ x=0,\ x=5$. Gọi $V$ là thể tích của khối tròn xoay sinh ra khi quay $(H)$ xung quanh trục $Ox$. Mệnh đề nào dưới đây đúng?

A. $V=\pi\displaystyle\int_{0}^{5}(x^{2}+3)dx.$

B. $V=\displaystyle\int_{0}^{5}(x^{2}+3)dx.$

C. $V=\pi\displaystyle\int_{0}^{5}(x^{2}+3)^{2}dx.$

D. $V=\displaystyle\int_{0}^{5}(x^{2}+3)^{2}dx.$"#;

    #[test]
    fn accepts_same_problem_with_formatting_changes() {
        let candidate = r#"Câu 9. Cho hình phẳng $(H)$ giới hạn bởi các đường $y = x^2 + 3$, $y = 0$, $x = 0$, $x = 5$. Gọi $V$ là thể tích của khối tròn xoay sinh ra khi quay $(H)$ xung quanh trục $Ox$. Mệnh đề nào dưới đây đúng?

A. $V = \pi \int_0^5 (x^2 + 3) dx.$
B. $V = \int_0^5 (x^2 + 3) dx.$
C. $V = \pi \int_0^5 (x^2 + 3)^2 dx.$
D. $V = \int_0^5 (x^2 + 3)^2 dx.$"#;

        assert!(validate_rerun_consistency(CAU_9, candidate).is_ok());
    }

    #[test]
    fn rejects_different_problem_label() {
        let candidate = r#"Bài 5. (1,0 điểm) Một cửa hàng bán bánh kẹo nhập về 360 hộp bánh và 400 hộp kẹo.
a) Trong tháng đầu tiên, cửa hàng bán được $x$ hộp bánh và $y$ hộp kẹo.
b) Nếu cửa hàng bán được 100 hộp bánh và 150 hộp kẹo, hãy tính số lượng hộp bánh và kẹo còn lại."#;

        assert!(validate_rerun_consistency(CAU_9, candidate).is_err());
    }

    #[test]
    fn rejects_unrelated_text_with_no_problem_label() {
        let candidate = r#"Ta có: $x^2 - xy + y^2 > 0 \Rightarrow x^3 + y^3$ cùng dấu với $x + y$.
Suy ra biểu thức đã cho luôn dương trong điều kiện xác định."#;

        assert!(validate_rerun_consistency(CAU_9, candidate).is_err());
    }

    #[test]
    fn rejects_same_label_with_unrelated_math_content() {
        let candidate = r#"Câu 9. Cho phương trình $x^2 - 3x + 2 = 0$. Gọi $x_1, x_2$ là hai nghiệm của phương trình. Tính giá trị biểu thức $P = x_1^2 + x_2^2$.

A. $1$
B. $2$
C. $5$
D. $9$"#;

        assert!(validate_rerun_consistency(CAU_9, candidate).is_err());
    }

    #[test]
    fn accepts_same_content_when_label_is_omitted() {
        let candidate = r#"Cho hình phẳng $(H)$ giới hạn bởi các đường $y = x^2 + 3$, $y = 0$, $x = 0$, $x = 5$. Gọi $V$ là thể tích của khối tròn xoay sinh ra khi quay $(H)$ xung quanh trục $Ox$. Mệnh đề nào dưới đây đúng?

A. $V = \pi \int_0^5 (x^2 + 3) dx.$
B. $V = \int_0^5 (x^2 + 3) dx.$
C. $V = \pi \int_0^5 (x^2 + 3)^2 dx.$
D. $V = \int_0^5 (x^2 + 3)^2 dx.$"#;

        assert!(validate_rerun_consistency(CAU_9, candidate).is_ok());
    }

    #[test]
    fn accepts_multi_question_crop_when_expected_label_is_present() {
        let previous =
            format!("{CAU_9}\n\nCâu 10. Cho hàm số $f(x)=x^2$. Tính đạo hàm của hàm số tại $x=1$.");
        let candidate =
            format!("{CAU_9}\n\nCâu 10. Cho hàm số $f(x)=x^2$. Tính đạo hàm của hàm số tại $x=1$.");

        assert!(validate_rerun_consistency(&previous, &candidate).is_ok());
    }

    #[test]
    fn skips_short_outputs_to_avoid_false_rejection() {
        assert!(validate_rerun_consistency("x + y", "x + y").is_ok());
    }
}
