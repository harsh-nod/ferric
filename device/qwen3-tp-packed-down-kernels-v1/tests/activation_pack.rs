use ferric_qwen3_down_f32_packed_u32_proposal::{
    activation_pack_host::{
        ActivationPackGeometry, activation_source_pair, model_activation_pack_into,
    },
    host::{K, LANES, MAX_ACTIVATION_ROWS, PackingError, WORDS, pack_rows, source_pair_indices},
};

fn geometry() -> ActivationPackGeometry {
    ActivationPackGeometry {
        rows: 1,
        k: K,
        source_elements: K,
        output_words: WORDS,
        launch_threads: WORDS,
    }
}

#[test]
fn endpoints_preserve_lane_stride_and_reject_every_word_overrun() {
    for (word, expected) in [
        (0, [0, 64]),
        (63, [63, 127]),
        (64, [128, 192]),
        (WORDS - 1, [12223, 12287]),
    ] {
        assert_eq!(activation_source_pair(word), Ok(expected));
    }
    for word in [WORDS, WORDS + 1, usize::MAX] {
        assert_eq!(activation_source_pair(word), Err(PackingError::Coordinate));
    }
}

#[test]
fn every_source_element_is_used_once_and_matches_existing_host_layout() {
    let mut seen = vec![false; K];
    for word in 0..WORDS {
        let pair = activation_source_pair(word).unwrap();
        assert_eq!(
            pair,
            source_pair_indices(1, K, 0, word / LANES, word % LANES).unwrap()
        );
        for index in pair {
            assert!(index < K);
            assert!(!seen[index]);
            seen[index] = true;
        }
    }
    assert!(seen.into_iter().all(|value| value));
}

#[test]
fn every_bf16_bit_pattern_matches_pack_rows_without_float_conversion() {
    for start in (0..=usize::from(u16::MAX)).step_by(K) {
        let source: Vec<_> = (0..K)
            .map(|index| u16::try_from((start + index) & 0xffff).unwrap())
            .collect();
        let original = source.clone();
        let mut output = vec![0xdead_beef; WORDS];
        model_activation_pack_into(&source, &mut output, 1, K, WORDS).unwrap();
        assert_eq!(output, pack_rows(&source, 1, K).unwrap());
        assert_eq!(source, original);
    }
}

#[test]
fn signed_zeros_nonfinite_payloads_and_subnormals_are_copied_exactly() {
    let bits = [
        0x0000_u16, 0x8000, 0x0001, 0x8001, 0x7f80, 0xff80, 0x7fc1, 0xffa5,
    ];
    let source: Vec<_> = (0..K)
        .map(|index| bits[(index / 64 + index) % bits.len()])
        .collect();
    let mut output = vec![0; WORDS];
    model_activation_pack_into(&source, &mut output, 1, K, WORDS).unwrap();
    for (word, packed) in output.into_iter().enumerate() {
        let [low, high] = activation_source_pair(word).unwrap();
        assert_eq!(packed as u16, source[low]);
        assert_eq!((packed >> 16) as u16, source[high]);
    }
}

#[test]
fn poisoned_source_capacity_is_unread_and_output_capacity_is_unwritten() {
    let prefix: Vec<_> = (0..K)
        .map(|bits| u16::try_from(bits).unwrap() ^ 0x8123)
        .collect();
    let expected = pack_rows(&prefix, 1, K).unwrap();
    for (source_len, output_len) in [
        (K, WORDS),
        (K + 1, WORDS + 1),
        (MAX_ACTIVATION_ROWS * K, MAX_ACTIVATION_ROWS * WORDS),
    ] {
        for poison in [0x7fc1_u16, 0xff80] {
            let mut source = vec![poison; source_len];
            source[..K].copy_from_slice(&prefix);
            let original = source.clone();
            let mut output = vec![0xdead_beef; output_len];
            model_activation_pack_into(&source, &mut output, 1, K, WORDS).unwrap();
            assert_eq!(&output[..WORDS], expected.as_slice());
            assert!(output[WORDS..].iter().all(|word| *word == 0xdead_beef));
            assert_eq!(source, original);
        }
    }
}

#[test]
fn host_geometry_rejects_wrong_rows_k_lengths_and_launch_extent() {
    let base = geometry();
    assert!(base.validate().is_ok());
    for changed in [
        ActivationPackGeometry { rows: 0, ..base },
        ActivationPackGeometry { rows: 2, ..base },
        ActivationPackGeometry { k: K - 1, ..base },
        ActivationPackGeometry { k: K + 1, ..base },
        ActivationPackGeometry {
            source_elements: K - 1,
            ..base
        },
        ActivationPackGeometry {
            source_elements: MAX_ACTIVATION_ROWS * K + 1,
            ..base
        },
        ActivationPackGeometry {
            output_words: WORDS - 1,
            ..base
        },
        ActivationPackGeometry {
            output_words: MAX_ACTIVATION_ROWS * WORDS + 1,
            ..base
        },
        ActivationPackGeometry {
            launch_threads: WORDS - 1,
            ..base
        },
        ActivationPackGeometry {
            launch_threads: WORDS + 1,
            ..base
        },
    ] {
        assert!(changed.validate().is_err(), "accepted {changed:?}");
    }
}

#[test]
fn rejected_host_requests_leave_all_output_bytes_unchanged() {
    for (source_len, output_len, rows, k, threads) in [
        (K - 1, WORDS, 1, K, WORDS),
        (K, WORDS - 1, 1, K, WORDS),
        (MAX_ACTIVATION_ROWS * K + 1, WORDS, 1, K, WORDS),
        (K, MAX_ACTIVATION_ROWS * WORDS + 1, 1, K, WORDS),
        (K, WORDS, 2, K, WORDS),
        (K, WORDS, 1, K - 1, WORDS),
        (K, WORDS, 1, K, WORDS - 1),
        (K, WORDS, 1, K, WORDS + 1),
    ] {
        let source = vec![0x7fc1; source_len];
        let mut output = vec![0x1234_5678; output_len];
        let before = output.clone();
        assert!(model_activation_pack_into(&source, &mut output, rows, k, threads).is_err());
        assert_eq!(output, before);
    }
}
