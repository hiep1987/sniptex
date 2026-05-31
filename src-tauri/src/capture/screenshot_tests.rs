use super::super::{staging_path, SelectionRect};
use super::{clamp_selection_to_monitor, CaptureError};

#[test]
fn staging_path_lives_under_temp_sniptex() {
    let p = staging_path("test-name.png");
    assert!(p.starts_with(std::env::temp_dir()));
    assert_eq!(
        p.parent().and_then(|p| p.file_name()),
        Some(std::ffi::OsStr::new("sniptex"))
    );
    assert_eq!(p.file_name(), Some(std::ffi::OsStr::new("test-name.png")));
}

#[test]
fn clamps_selection_to_logical_monitor_bounds() {
    let sel = SelectionRect {
        x: 90,
        y: 80,
        w: 40,
        h: 50,
    };
    let region = clamp_selection_to_monitor(sel, 100, 100).unwrap();
    assert_eq!(region, (90, 80, 10, 20));
}

#[test]
fn rejects_selection_outside_logical_monitor() {
    let sel = SelectionRect {
        x: 120,
        y: 120,
        w: 20,
        h: 20,
    };
    let err = clamp_selection_to_monitor(sel, 100, 100).unwrap_err();
    assert!(matches!(err, CaptureError::SelectionOutOfBounds));
}
