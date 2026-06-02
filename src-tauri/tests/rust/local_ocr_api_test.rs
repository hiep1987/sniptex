use sniptex_lib::agents::local_ocr_api::agents_for_capabilities;
use sniptex_lib::agents::local_ocr_cache::{LocalHealthCache, LocalHealthStatus};
use sniptex_lib::agents::local_ocr_client::LocalOcrError;
use sniptex_lib::agents::local_ocr_paddleocr::parse_ocr_body as parse_paddleocr_body;
use sniptex_lib::agents::local_ocr_pix2tex::parse_ocr_body as parse_pix2tex_body;
use sniptex_lib::agents::registry::{LOCAL_FAST_ID, LOCAL_PADDLEOCR_ID, LOCAL_PIX2TEX_ID};
use sniptex_lib::settings::AppSettings;

#[test]
fn cache_uses_long_ttl_for_unhealthy_status() {
    let mut cache = LocalHealthCache::default();
    cache.update(
        "http://127.0.0.1:8765",
        LocalHealthStatus {
            healthy: false,
            version: None,
            capabilities: Vec::new(),
        },
        1_000,
    );

    assert!(cache.get("http://127.0.0.1:8765", 30_000).is_some());
    assert!(cache.get("http://127.0.0.1:8765", 31_001).is_none());
}

#[test]
fn cache_uses_short_ttl_for_healthy_status() {
    let mut cache = LocalHealthCache::default();
    cache.update(
        "http://127.0.0.1:8765",
        LocalHealthStatus {
            healthy: true,
            version: Some("0.1.0".to_string()),
            capabilities: vec!["pix2tex".to_string()],
        },
        1_000,
    );

    assert!(cache.get("http://127.0.0.1:8765", 6_000).is_some());
    assert!(cache.get("http://127.0.0.1:8765", 6_001).is_none());
}

#[test]
fn local_agent_mapping_respects_capabilities_and_settings() {
    let settings = AppSettings {
        local_ocr_enabled: true,
        local_ocr_formula_enabled: true,
        local_ocr_text_enabled: false,
        ..AppSettings::default()
    };
    let health = LocalHealthStatus {
        healthy: true,
        version: Some("daemon-test".to_string()),
        capabilities: vec![
            "pix2tex".to_string(),
            "paddleocr".to_string(),
            "classifier".to_string(),
        ],
    };

    let ids: Vec<String> = agents_for_capabilities(&settings, &health)
        .into_iter()
        .map(|agent| agent.spec.id.to_string())
        .collect();
    assert_eq!(ids, vec![LOCAL_PIX2TEX_ID]);
}

#[test]
fn local_agent_mapping_surfaces_auto_router_in_phase_4() {
    let settings = AppSettings {
        local_ocr_enabled: true,
        local_ocr_formula_enabled: true,
        local_ocr_text_enabled: true,
        ..AppSettings::default()
    };
    let health = LocalHealthStatus {
        healthy: true,
        version: Some("daemon-test".to_string()),
        capabilities: vec![
            "pix2tex".to_string(),
            "paddleocr".to_string(),
            "classifier".to_string(),
        ],
    };

    let ids: Vec<String> = agents_for_capabilities(&settings, &health)
        .into_iter()
        .map(|agent| agent.spec.id.to_string())
        .collect();
    assert_eq!(ids, vec![LOCAL_PIX2TEX_ID, LOCAL_PADDLEOCR_ID, LOCAL_FAST_ID]);
}

#[test]
fn pix2tex_response_parser_returns_raw_tex() {
    let result = parse_pix2tex_body(
        200,
        r#"{"text":" \\frac{a}{b} ","detected":"EQUATION_ONLY","confidence":0.98}"#,
    )
    .expect("valid pix2tex body should parse");

    assert_eq!(result, "\\frac{a}{b}");
}

#[test]
fn pix2tex_response_parser_rejects_empty_output() {
    let err = parse_pix2tex_body(200, r#"{"text":"   ","confidence":0.99}"#)
        .expect_err("empty output should be rejected");

    assert!(matches!(err, LocalOcrError::EmptyResponse));
}

#[test]
fn pix2tex_response_parser_rejects_low_confidence() {
    let err = parse_pix2tex_body(200, r#"{"text":"x","confidence":0.01}"#)
        .expect_err("low confidence output should be rejected");

    assert!(matches!(err, LocalOcrError::LowConfidence(_)));
}

#[test]
fn pix2tex_response_parser_maps_unsupported_table() {
    let err = parse_pix2tex_body(422, r#"{"error":"unsupported_table"}"#)
        .expect_err("422 should map to unsupported");

    assert!(matches!(err, LocalOcrError::Unsupported(msg) if msg == "unsupported_table"));
}

#[test]
fn paddleocr_response_parser_preserves_vietnamese_diacritics() {
    let result = parse_paddleocr_body(
        200,
        r#"{"text":" Câu 7. Cho mẫu số liệu ghép nhóm\nTính mốt của mẫu số liệu đã cho. ","confidence":0.97}"#,
    )
    .expect("valid paddleocr body should parse");

    assert_eq!(
        result,
        "Câu 7. Cho mẫu số liệu ghép nhóm\nTính mốt của mẫu số liệu đã cho."
    );
}

#[test]
fn paddleocr_response_parser_joins_line_blocks() {
    let result = parse_paddleocr_body(
        200,
        r#"{"lines":[{"text":"Dòng thứ nhất","confidence":0.91},{"text":"Dòng thứ hai","confidence":0.88}]}"#,
    )
    .expect("line blocks should parse");

    assert_eq!(result, "Dòng thứ nhất\nDòng thứ hai");
}

#[test]
fn paddleocr_response_parser_rejects_low_confidence() {
    let err = parse_paddleocr_body(200, r#"{"text":"Câu hỏi","confidence":0.2}"#)
        .expect_err("low confidence output should be rejected");

    assert!(matches!(err, LocalOcrError::LowConfidence(_)));
}

#[test]
fn paddleocr_response_parser_rejects_low_line_confidence() {
    let err = parse_paddleocr_body(
        200,
        r#"{"confidence":0.99,"lines":[{"text":"Câu hỏi","confidence":0.2}]}"#,
    )
    .expect_err("low line confidence should not be masked by aggregate confidence");

    assert!(matches!(err, LocalOcrError::LowConfidence(_)));
}

#[test]
fn paddleocr_response_parser_maps_unsupported_table_to_bad_request() {
    let err = parse_paddleocr_body(422, r#"{"error":"unsupported_table"}"#)
        .expect_err("tables should be rejected");

    assert!(matches!(err, LocalOcrError::BadRequest(msg) if msg == "local does not support tables"));
}

#[test]
fn paddleocr_response_parser_rejects_leaked_table_markers() {
    let err = parse_paddleocr_body(
        200,
        r#"{"text":"| Giá trị | [135;140) |\n|---|---|\n| Tần số | 6 |","confidence":0.96}"#,
    )
    .expect_err("local paddleocr must not emit tables");

    assert!(matches!(err, LocalOcrError::BadRequest(msg) if msg == "local does not support tables"));
}
