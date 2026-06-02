use sniptex_lib::agents::local_ocr_client::LocalOcrError;
use sniptex_lib::agents::local_ocr_router::{parse_classify_body, LocalRoute};

#[test]
fn classifier_parser_routes_equation_to_pix2tex() {
    let route = parse_classify_body(200, r#"{"kind":"equation","confidence":0.93}"#)
        .expect("equation classifier result should route");

    assert_eq!(route, LocalRoute::Pix2Tex);
}

#[test]
fn classifier_parser_routes_equation_only_to_pix2tex() {
    let route = parse_classify_body(200, r#"{"kind":"EQUATION_ONLY","confidence":0.93}"#)
        .expect("equation-only classifier result should route");

    assert_eq!(route, LocalRoute::Pix2Tex);
}

#[test]
fn classifier_parser_routes_text_to_paddleocr() {
    let route = parse_classify_body(200, r#"{"kind":"text","confidence":0.91}"#)
        .expect("text classifier result should route");

    assert_eq!(route, LocalRoute::PaddleOcr);
}

#[test]
fn classifier_parser_rejects_table_and_mixed() {
    let table = parse_classify_body(200, r#"{"kind":"table","confidence":0.99}"#)
        .expect_err("table should fall back");
    let mixed = parse_classify_body(200, r#"{"kind":"mixed","confidence":0.99}"#)
        .expect_err("mixed should fall back");

    assert!(matches!(table, LocalOcrError::BadRequest(msg) if msg == "local unsupported"));
    assert!(matches!(mixed, LocalOcrError::BadRequest(msg) if msg == "local unsupported"));
}

#[test]
fn classifier_parser_rejects_unknown_and_missing_kind() {
    let unknown = parse_classify_body(200, r#"{"kind":"unknown","confidence":0.9}"#)
        .expect_err("unknown should fall back");
    let missing = parse_classify_body(200, r#"{"confidence":0.9}"#)
        .expect_err("missing kind should fail");

    assert!(matches!(unknown, LocalOcrError::BadRequest(msg) if msg == "local unsupported"));
    assert!(matches!(missing, LocalOcrError::Parse(_)));
}
