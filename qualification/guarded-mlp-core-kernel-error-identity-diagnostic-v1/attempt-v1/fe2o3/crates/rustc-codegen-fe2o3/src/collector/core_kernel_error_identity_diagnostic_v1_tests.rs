use super::*;
use std::fmt::Write as _;

#[test]
fn core_kernel_error_identity_diagnostic_complete_has_explicit_non_authority_trailer() {
    let text = diagnostic_text(|output| {
        writeln!(output, "actual compiler metadata")?;
        Ok(true)
    });
    assert!(text.contains("status=complete"));
    assert!(text.ends_with("authority=none\n"));
    assert!(text.len() <= BYTE_CAP);
}

#[test]
fn core_kernel_error_identity_diagnostic_partial_is_not_complete() {
    let text = diagnostic_text(|_| Ok(false));
    assert!(text.contains("status=partial"));
    assert!(!text.contains("status=complete"));
}

#[test]
fn core_kernel_error_identity_diagnostic_large_write_is_bounded() {
    let text = diagnostic_text(|output| {
        output.write_str(&"x".repeat(BYTE_CAP))?;
        Ok(true)
    });
    assert!(text.len() <= BYTE_CAP);
    assert!(text.contains("status=truncated_or_render_error"));
    assert!(text.ends_with("authority=none\n"));
}

#[test]
fn core_kernel_error_identity_diagnostic_exact_payload_cap_leaves_trailer_space() {
    let text = diagnostic_text(|output| {
        output.write_str(&"x".repeat(BYTE_CAP - TRAILER_RESERVE))?;
        Ok(true)
    });
    assert!(text.len() <= BYTE_CAP);
    assert!(text.contains("status=complete"));
    assert!(text.ends_with("authority=none\n"));
}

#[test]
fn core_kernel_error_identity_diagnostic_utf8_cap_uses_bytes_and_keeps_valid_text() {
    let mut output = BoundedText {
        text: String::new(),
        limit: 3,
    };
    output.write_str("\u{00e9}").unwrap();
    assert!(output.write_str("\u{00e9}").is_err());
    assert_eq!(output.text, "\u{00e9}");
    assert_eq!(output.text.len(), 2);
}

#[test]
fn core_kernel_error_identity_diagnostic_render_failure_preserves_partial_evidence() {
    let text = diagnostic_text(|output| {
        writeln!(output, "retained prefix")?;
        Err(fmt::Error)
    });
    assert!(text.starts_with("retained prefix\n"));
    assert!(text.contains("status=truncated_or_render_error"));
}
