use super::*;

#[test]
fn ordinary_hidden_read_keeps_two_rank_ordered_reads_and_stops_on_failure() {
    for failure in [None, Some(0), Some(1)] {
        let mut calls = Vec::new();
        let result = scalar_layer_hidden_pair(|rank| {
            calls.push(rank);
            if failure == Some(rank) { Err("injected scalar read failure".into()) }
            else { Ok(vec![rank as u8; 8192]) }
        });
        assert_eq!(calls, if failure == Some(0) { vec![0] } else { vec![0, 1] });
        assert_eq!(result.is_ok(), failure.is_none());
        if let Ok([rank0, rank1]) = result {
            assert_eq!(rank0, vec![0; 8192]);
            assert_eq!(rank1, vec![1; 8192]);
        }
    }
}

#[test]
fn paired_hidden_read_publishes_only_equal_finite_complete_transaction_bytes() {
    let mut good = vec![0; 8192];
    good[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
    good[6..8].copy_from_slice(&0x3f80u16.to_le_bytes());
    assert_eq!(checked_layer_hidden_pair(35, Ok([good.clone(), good.clone()])).unwrap(), good);
    assert!(checked_layer_hidden_pair(0, Err("trailing group fence failed".into())).is_err());
    for rank in 0..2 {
        for case in 0..5 {
            let mut values = [good.clone(), good.clone()];
            match case {
                0 => { values[rank].pop(); },
                1 => values[rank].push(0),
                2 => values[rank][0] = 1,
                3 => values[rank][0..2].copy_from_slice(&0x7f80u16.to_le_bytes()),
                _ => values[rank][0..2].copy_from_slice(&0x7fc0u16.to_le_bytes()),
            }
            assert!(checked_layer_hidden_pair(0, Ok(values)).is_err());
        }
    }
    for bits in [0x7f80u16, 0xff80, 0x7fc0, 0xffc1] {
        let mut bad = good.clone();
        bad[0..2].copy_from_slice(&bits.to_le_bytes());
        assert!(checked_layer_hidden_pair(0, Ok([bad.clone(), bad])).is_err());
    }
}

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
