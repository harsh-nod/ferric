use super::*;
use std::vec;

#[test]
fn batched_argmax_keeps_row_identity_and_lowest_id_ties() {
    let mut logits = vec![0_u16; 3 * 151936];
    logits[4] = 0x4000;
    logits[100] = 0x4000;
    logits[151936 + 19] = 0x4080;
    logits[2 * 151936 + 151935] = 0x3f80;
    assert_eq!(batch_argmax_v2!(&logits, 0), 4);
    assert_eq!(batch_argmax_v2!(&logits, 151936), 19);
    assert_eq!(batch_argmax_v2!(&logits, 2 * 151936), 151935);
}

#[test]
#[should_panic]
fn nonfinite_last_logit_is_not_ignored() {
    let mut logits = vec![0_u16; 151936];
    logits[151935] = 0x7fc0;
    let _ = batch_argmax_v2!(&logits, 0);
}
