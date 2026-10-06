use super::*;

#[test]
fn finite_observations_reject_nan_infinity_and_bad_extents() {
    for bits in [0x7f80u16, 0xff80, 0x7fc0, 0xffc1] {
        assert!(finite_bf16(&bits.to_le_bytes()).is_err());
    }
    assert!(finite_bf16(&[]).is_err());
    assert!(finite_bf16(&[0]).is_err());
    for bits in [0u16, 0x8000, 0x0001, 0x7f7f, 0xff7f] {
        assert!(finite_bf16(&bits.to_le_bytes()).is_ok());
    }
}

#[test]
fn independent_argmax_uses_lowest_index_for_equal_values_and_signed_zero() {
    let mut logits = vec![0; 151_936 * 2];
    logits[0..2].copy_from_slice(&0x8000u16.to_le_bytes());
    assert!(checked_argmax(&logits, 0).is_ok());
    assert!(checked_argmax(&logits, 1).is_err());
    for index in [4, 22, 151_935] {
        logits[index * 2..index * 2 + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
    }
    assert!(checked_argmax(&logits, 4).is_ok());
    assert!(checked_argmax(&logits, 22).is_err());
    assert!(checked_argmax(&logits, 151_936).is_err());
}

#[test]
fn output_token_cannot_hide_nonfinite_logits_or_truncated_read() {
    let mut logits = vec![0; 151_936 * 2];
    logits[14..16].copy_from_slice(&0x7fc0u16.to_le_bytes());
    assert!(checked_argmax(&logits, 0).is_err());
    logits[14..16].copy_from_slice(&0u16.to_le_bytes());
    logits.pop();
    assert!(checked_argmax(&logits, 0).is_err());
}
