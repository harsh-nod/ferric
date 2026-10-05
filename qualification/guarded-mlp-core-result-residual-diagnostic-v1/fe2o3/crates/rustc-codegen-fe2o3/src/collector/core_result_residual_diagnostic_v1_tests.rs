use super::*;
use std::fmt::Write as _;

#[test]
fn residual_diagnostic_accepts_only_four_render_slots() {
    let mut census = Census::default();
    for _ in 0..MATCH_CAP {
        assert!(census.matched());
    }
    assert!(!census.match_truncated);
    assert!(!census.matched());
    assert!(census.match_truncated);
    assert_eq!(census.matched, MATCH_CAP + 1);
}

#[test]
fn residual_diagnostic_scan_is_bounded_by_existing_function_limit() {
    let mut census = Census::default();
    for _ in 0..SCAN_CAP {
        assert!(census.scan());
    }
    assert!(!census.scan_truncated);
    assert!(!census.scan());
    assert!(census.scan_truncated);
    assert_eq!(census.scanned, SCAN_CAP);
}

#[test]
fn residual_diagnostic_empty_inventory_has_no_invented_match() {
    let census = Census::default();
    assert_eq!(census.scanned, 0);
    assert_eq!(census.matched, 0);
    assert!(!census.scan_truncated && !census.match_truncated);
}

#[test]
fn residual_diagnostic_uses_the_remaining_shared_text_budget() {
    let text = diagnostic_text(|output| {
        output.write_str(&"b".repeat(BYTE_CAP - TRAILER_RESERVE - 8))?;
        writeln!(output, "residual-record-does-not-fit")?;
        Ok(true)
    });
    assert!(text.len() <= BYTE_CAP);
    assert!(text.contains("status=truncated_or_render_error"));
    assert!(text.ends_with("authority=none\n"));
}
