use ferric_qwen3_tp_packed_gate_up_kernels_v8::host::*;

fn pair(a: &[u16], gate: &[u16], up: &[u16]) -> (Vec<PairTrace>, PairResult) {
    let (packed_a, packed_gate, packed_up) = (
        pack_row(a).unwrap(),
        pack_row(gate).unwrap(),
        pack_row(up).unwrap(),
    );
    let traces = (0..LANES)
        .map(|lane| {
            let actual = fused_lane(&packed_a, &packed_gate, &packed_up, lane).unwrap();
            assert_eq!(actual.gate, baseline_lane(a, gate, lane).unwrap());
            assert_eq!(actual.up, baseline_lane(a, up, lane).unwrap());
            actual
        })
        .collect::<Vec<_>>();
    let result = finish_pair(&traces).unwrap();
    let reference = |weights: &[u16]| {
        finish_row(
            &(0..LANES)
                .map(|lane| baseline_lane(a, weights, lane).unwrap())
                .collect::<Vec<_>>(),
        )
        .unwrap()
    };
    assert_eq!(result.gate, reference(gate));
    assert_eq!(result.up, reference(up));
    (traces, result)
}

#[test]
fn exact_geometry_rejects_every_short_long_or_foreign_extent() {
    let good = Geometry {
        activation_words: WORDS,
        gate_weight_words: N * WORDS,
        up_weight_words: N * WORDS,
        gate_output_elements: N,
        up_output_elements: N,
        launch_threads: N * LANES,
    };
    assert_eq!(good.validate(), Ok(()));
    for field in 0..6 {
        for delta in [-1_isize, 1] {
            let mut bad = good;
            let selected = match field {
                0 => &mut bad.activation_words,
                1 => &mut bad.gate_weight_words,
                2 => &mut bad.up_weight_words,
                3 => &mut bad.gate_output_elements,
                4 => &mut bad.up_output_elements,
                _ => &mut bad.launch_threads,
            };
            *selected = selected.checked_add_signed(delta).unwrap();
            assert_eq!(bad.validate(), Err(Error::Length));
        }
    }
}

#[test]
fn pack_keeps_every_bf16_bit_and_both_original_lane_time_positions() {
    for first in (0..65536_u32).step_by(K) {
        let source = (first..first + K as u32)
            .map(|bits| bits as u16)
            .collect::<Vec<_>>();
        let packed = pack_row(&source).unwrap();
        for group in 0..GROUPS {
            for lane in 0..LANES {
                let [low, high] = source_pair(group, lane).unwrap();
                let word = packed[group * LANES + lane];
                assert_eq!(
                    [word as u16, (word >> 16) as u16],
                    [source[low], source[high]]
                );
            }
        }
    }
}

#[test]
fn packed_coordinates_cover_exact_first_last_columns_without_tail_or_overflow() {
    for column in [0, 1, N - 1] {
        let indexes = (0..GROUPS)
            .flat_map(|group| {
                (0..LANES).map(move |lane| packed_weight_index(column, group, lane).unwrap())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            indexes,
            (column * WORDS..(column + 1) * WORDS).collect::<Vec<_>>()
        );
    }
    for (column, group, lane) in [(N, 0, 0), (usize::MAX, 0, 0), (0, GROUPS, 0), (0, 0, LANES)] {
        assert_eq!(
            packed_weight_index(column, group, lane),
            Err(Error::Coordinate)
        );
    }
    assert_eq!(
        packed_weight_index(N - 1, GROUPS - 1, LANES - 1),
        Ok(N * WORDS - 1)
    );
}

#[test]
fn distinct_gate_and_up_keep_each_lane_checkpoint_and_six_stage_reduction() {
    for seed in [1_u16, 7, 23, 101] {
        let a = (0..K)
            .map(|i| 0x3e00 | (((i as u16).wrapping_mul(seed)) & 0x01ff))
            .collect::<Vec<_>>();
        let gate = (0..K)
            .map(|i| 0x3d00 | ((i as u16).wrapping_mul(13) & 0x00ff))
            .collect::<Vec<_>>();
        let up = (0..K)
            .map(|i| 0xbc00 | ((i as u16).wrapping_mul(29) & 0x007f))
            .collect::<Vec<_>>();
        let (_, result) = pair(&a, &gate, &up);
        assert!(result.accepted());
        assert_ne!(result.gate.narrowed[0], result.up.narrowed[0]);
    }
}

#[test]
fn cancellation_case_detects_low_high_reordering_without_cross_output_reassociation() {
    let mut a = vec![0; K];
    a[0] = 0x60ad;
    a[64] = 0x3f80;
    a[128] = 0xe0ad;
    let gate = vec![0x3f80; K];
    let up = vec![0xbf80; K];
    let (traces, result) = pair(&a, &gate, &up);
    assert!(result.accepted());
    assert_eq!(
        traces[0]
            .gate
            .steps
            .iter()
            .take(3)
            .map(|step| step.inner)
            .collect::<Vec<_>>(),
        [0, 64, 128]
    );
    assert_eq!(f32::from_bits(traces[0].gate.steps[2].partial), 0.0);
    let large = f32::from_bits(u32::from(a[0]) << 16);
    let reordered = (large + -large) + 1.0;
    assert_ne!(traces[0].gate.steps[2].partial, reordered.to_bits());
}

#[test]
fn zeros_subnormals_and_ties_keep_finite_reference_bits() {
    let values = [
        0x0000_u16, 0x8000, 0x0001, 0x8001, 0x007f, 0x807f, 0x3f80, 0xbf80,
    ];
    let a = (0..K).map(|i| values[i % values.len()]).collect::<Vec<_>>();
    let gate = (0..K)
        .map(|i| values[(i + 3) % values.len()])
        .collect::<Vec<_>>();
    let up = (0..K)
        .map(|i| values[(i + 5) % values.len()])
        .collect::<Vec<_>>();
    assert!(pair(&a, &gate, &up).1.accepted());
}

#[test]
fn either_nonfinite_arm_rejects_both_stores_including_nonzero_lane() {
    for bad in [0x7f80_u16, 0xff80, 0x7fc1, 0x7fff] {
        for arm in 0..2 {
            for inner in [0, 63, 64, K - 1] {
                let a = vec![0x3f80; K];
                let mut gate = vec![0x3f80; K];
                let mut up = vec![0x3f00; K];
                if arm == 0 {
                    gate[inner] = bad;
                } else {
                    up[inner] = bad;
                }
                let (_, result) = pair(&a, &gate, &up);
                assert!(!result.accepted());
                let mut gate_out = vec![0x1234; N];
                let mut up_out = vec![0x5678; N];
                assert_eq!(
                    result.store(&mut gate_out, &mut up_out, N - 1),
                    Err(Error::NonFinite)
                );
                assert!(gate_out.iter().all(|bits| *bits == 0x1234));
                assert!(up_out.iter().all(|bits| *bits == 0x5678));
            }
        }
    }
}

#[test]
fn intermediate_overflow_is_sticky_and_gate_up_finite_states_are_independent() {
    let mut a = vec![0; K];
    a[17] = 0x7f7f;
    a[17 + 64] = 0x7f7f;
    a[17 + 128] = 0x7f7f;
    let mut gate = vec![0x3f80; K];
    gate[17 + 128] = 0xbf80;
    let up = vec![0; K];
    let (traces, result) = pair(&a, &gate, &up);
    assert!(traces[17].gate.steps[0].finite);
    assert!(traces[17].gate.steps[1..].iter().all(|step| !step.finite));
    assert!(traces.iter().all(|lane| lane.up.finite));
    assert!(!result.gate.accepted && result.up.accepted && !result.accepted());
}

#[test]
fn finite_f32_reduction_that_narrows_to_infinity_rejects_the_pair() {
    let mut a = vec![0; K];
    a[0] = 0x7f7f;
    a[1] = 0x7b00;
    let gate = vec![0x3f80; K];
    let up = vec![0; K];
    let (traces, result) = pair(&a, &gate, &up);
    assert!(traces.iter().all(|lane| lane.gate.finite));
    assert!(
        result
            .gate
            .sums
            .iter()
            .all(|bits| f32::from_bits(*bits).is_finite())
    );
    assert!(result.gate.narrowed.iter().all(|bits| *bits == 0x7f80));
    assert!(!result.gate.accepted && result.up.accepted && !result.accepted());
}

#[test]
fn successful_store_writes_only_selected_column_and_invalid_store_is_unchanged() {
    let a = vec![0x3f00; K];
    let gate = vec![0x3e80; K];
    let up = vec![0xbe00; K];
    let (_, result) = pair(&a, &gate, &up);
    for column in [0, 1, N - 1] {
        let mut gate_out = vec![0x1234; N];
        let mut up_out = vec![0x5678; N];
        result.store(&mut gate_out, &mut up_out, column).unwrap();
        assert_eq!(gate_out[column], result.gate.narrowed[0]);
        assert_eq!(up_out[column], result.up.narrowed[0]);
        assert!(
            gate_out
                .iter()
                .enumerate()
                .all(|(i, bits)| i == column || *bits == 0x1234)
        );
        assert!(
            up_out
                .iter()
                .enumerate()
                .all(|(i, bits)| i == column || *bits == 0x5678)
        );
    }
    let mut gate_out = vec![0x1234; N];
    let mut up_out = vec![0x5678; N - 1];
    assert_eq!(
        result.store(&mut gate_out, &mut up_out, 0),
        Err(Error::Length)
    );
    assert_eq!(gate_out, vec![0x1234; N]);
    up_out.push(0x5678);
    assert_eq!(
        result.store(&mut gate_out, &mut up_out, N),
        Err(Error::Coordinate)
    );
    assert_eq!(up_out, vec![0x5678; N]);
}

#[test]
fn malformed_host_views_and_incomplete_collectives_fail_closed() {
    assert_eq!(pack_row(&vec![0; K - 1]), Err(Error::Length));
    assert_eq!(pack_row(&vec![0; K + 1]), Err(Error::Length));
    assert_eq!(
        baseline_lane(&vec![0; K], &vec![0; K], LANES),
        Err(Error::Coordinate)
    );
    assert_eq!(
        fused_lane(&vec![0; WORDS], &vec![0; WORDS - 1], &vec![0; WORDS], 0),
        Err(Error::Length)
    );
    assert_eq!(
        fused_lane(&vec![0; WORDS], &vec![0; WORDS], &vec![0; WORDS + 1], 0),
        Err(Error::Length)
    );
    assert_eq!(finish_row(&[]), Err(Error::Length));
    assert_eq!(finish_pair(&[]), Err(Error::Length));
}
