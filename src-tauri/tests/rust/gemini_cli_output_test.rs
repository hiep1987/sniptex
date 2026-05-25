use sniptex_lib::ocr::dispatcher::{
    looks_like_gemini_tool_error, parse_gemini_cli_json_response, DispatchError,
};

#[test]
fn parses_gemini_cli_response_field() {
    let raw = r#"{
        "response": "\\frac{1}{2}",
        "stats": { "tools": { "totalCalls": 0 } }
    }"#;

    let parsed = parse_gemini_cli_json_response(raw).unwrap();

    assert_eq!(parsed, "\\frac{1}{2}");
}

#[test]
fn ignores_unknown_gemini_cli_json_fields() {
    let raw = r#"{
        "response": "| a | b |\n|---|---|\n| 1 | 2 |",
        "unexpected": { "nested": true }
    }"#;

    let parsed = parse_gemini_cli_json_response(raw).unwrap();

    assert!(parsed.starts_with("| a | b |"));
}

#[test]
fn rejects_missing_gemini_cli_response_field() {
    let err = parse_gemini_cli_json_response(r#"{"stats":{}}"#).unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("missing response"));
}

#[test]
fn surfaces_structured_gemini_cli_error_field() {
    let err = parse_gemini_cli_json_response(
        r#"{"error":{"message":"model unavailable","code":"UNAVAILABLE"}}"#,
    )
    .unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("gemini-cli error"));
    assert!(err.to_string().contains("model unavailable"));
}

#[test]
fn rejects_gemini_cli_tool_call_stats() {
    let err = parse_gemini_cli_json_response(
        r#"{
            "response": "\\frac{1}{2}",
            "stats": { "tools": { "totalCalls": 1 } }
        }"#,
    )
    .unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("used tools"));
}

#[test]
fn allows_successful_read_file_used_for_image_attachment() {
    let raw = r#"{
        "response": "Câu 1. Nội dung OCR",
        "stats": {
            "tools": {
                "totalCalls": 1,
                "totalFail": 0,
                "byName": {
                    "read_file": {
                        "count": 1,
                        "fail": 0
                    }
                }
            }
        }
    }"#;

    let parsed = parse_gemini_cli_json_response(raw).unwrap();

    assert_eq!(parsed, "Câu 1. Nội dung OCR");
}

#[test]
fn rejects_non_read_file_tool_usage() {
    let err = parse_gemini_cli_json_response(
        r#"{
            "response": "\\frac{1}{2}",
            "stats": {
                "tools": {
                    "totalCalls": 2,
                    "totalFail": 0,
                    "byName": {
                        "read_file": { "count": 1, "fail": 0 },
                        "grep": { "count": 1, "fail": 0 }
                    }
                }
            }
        }"#,
    )
    .unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("used tools"));
}

#[test]
fn rejects_gemini_cli_snake_case_tool_call_stats() {
    let err = parse_gemini_cli_json_response(
        r#"{
            "response": "\\frac{1}{2}",
            "stats": { "tools": { "total_calls": 2 } }
        }"#,
    )
    .unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("used tools"));
}

#[test]
fn rejects_gemini_cli_tool_error_response_text() {
    let err = parse_gemini_cli_json_response(
        r#"{
            "response": "Error executing tool read_file: Path not in workspace"
        }"#,
    )
    .unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("tool execution error"));
}

#[test]
fn detects_known_gemini_tool_error_markers() {
    assert!(looks_like_gemini_tool_error(
        "Error executing tool read_file: Path not in workspace"
    ));
    assert!(looks_like_gemini_tool_error(
        "failed while calling default_api_read_file"
    ));
    assert!(!looks_like_gemini_tool_error(
        "Find the value of x in the equation x + 1 = 3."
    ));
    assert!(!looks_like_gemini_tool_error(
        "This workspace has a tool panel for editing equations."
    ));
    assert!(!looks_like_gemini_tool_error(
        "The read_file example returns an error when the input is invalid."
    ));
    assert!(!looks_like_gemini_tool_error(
        "The constant default_api_timeout appears in this code sample."
    ));
    assert!(!looks_like_gemini_tool_error(
        "The function default_api_read_file is documented below."
    ));
}

#[test]
fn rejects_empty_gemini_cli_response_field() {
    let err = parse_gemini_cli_json_response(r#"{"response":"   "}"#).unwrap_err();

    assert!(matches!(err, DispatchError::EmptyOutput));
}

#[test]
fn rejects_non_json_gemini_cli_stdout() {
    let err = parse_gemini_cli_json_response("YOLO mode is enabled\nnot json").unwrap_err();

    assert!(matches!(err, DispatchError::BadRequest(_)));
    assert!(err.to_string().contains("invalid JSON"));
}
