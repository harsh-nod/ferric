use fe2o3_device::Bf16;
use ferric_qwen3_down_f32_packed_u32_proposal::host::*;

fn traces(a: &[u16], weights: &[u16]) -> (Vec<LaneTrace>, Vec<LaneTrace>) {
    let packed_a = pack_rows(&a[..K], 1, K).unwrap();
    let packed_weights = pack_rows(weights, 1, K).unwrap();
    let baseline: Vec<_> = (0..LANES)
        .map(|lane| baseline_lane(a, weights, lane).unwrap())
        .collect();
    let packed: Vec<_> = (0..LANES)
        .map(|lane| packed_lane(&packed_a, &packed_weights, lane).unwrap())
        .collect();
    assert_eq!(baseline, packed);
    assert_eq!(finish_row(&baseline), finish_row(&packed));
    (baseline, packed)
}

fn geometry() -> KernelGeometry {
    KernelGeometry {
        rows: 1,
        n: N,
        k: K,
        world_size: 1,
        projection: 2,
        activation_words: WORDS,
        weight_words: N * WORDS,
        output_elements: N,
        launch_threads: N * LANES,
    }
}

#[test]
fn words_pair_original_lane_steps_64_apart_and_keep_rows_separate() {
    let source: Vec<u16> = (0..3 * K)
        .map(|index| u16::try_from(index).unwrap())
        .collect();
    let packed = pack_rows(&source, 3, K).unwrap();
    for row in 0..3 {
        for group in 0..GROUPS {
            for lane in 0..LANES {
                let index = row * WORDS + group * LANES + lane;
                let low = row * K + 2 * group * LANES + lane;
                assert_eq!(
                    unpack_word(packed[index]),
                    [source[low], source[low + LANES]]
                );
                assert_ne!(unpack_word(packed[index])[1], source[low + 1]);
                assert_eq!(word_index(3, K, row, group, lane), Ok(index));
            }
        }
    }
}

#[test]
fn every_bf16_bit_pattern_roundtrips_without_modifying_original_storage() {
    let source: Vec<_> = (0..6 * K)
        .map(|index| u16::try_from(index & 0xffff).unwrap())
        .collect();
    let original = source.clone();
    let packed = pack_rows(&source, 6, K).unwrap();
    assert_eq!(packed.len(), 6 * WORDS);
    assert_eq!(pack_rows(&source, 6, K).unwrap(), packed);
    assert_eq!(unpack_rows(&packed, 6, K).unwrap(), original);
    assert_eq!(source, original);
}

#[test]
fn little_endian_transfer_bytes_are_explicit_and_not_a_slice_reinterpretation() {
    let mut source = vec![0; K];
    source[0] = 0x1234;
    source[LANES] = 0xabcd;
    let packed = pack_rows(&source, 1, K).unwrap();
    let bytes = packed_le_bytes(&packed, 1, K).unwrap();
    assert_eq!(packed[0], 0xabcd_1234);
    assert_eq!(&bytes[..4], &[0x34, 0x12, 0xcd, 0xab]);
    assert_eq!(bytes.len(), K * 2);
    for (word, bytes) in packed.iter().zip(bytes.chunks_exact(4)) {
        assert_eq!(u32::from_le_bytes(bytes.try_into().unwrap()), *word);
    }
}

#[test]
fn coordinates_cover_last_weight_element_and_reject_every_endpoint_overrun() {
    assert_eq!(
        source_pair_indices(MAX_ROWS, K, MAX_ROWS - 1, GROUPS - 1, LANES - 1),
        Ok([MAX_ROWS * K - LANES - 1, MAX_ROWS * K - 1])
    );
    assert_eq!(
        word_index(MAX_ROWS, K, MAX_ROWS - 1, GROUPS - 1, LANES - 1),
        Ok(MAX_ROWS * WORDS - 1)
    );
    for (row, group, lane) in [
        (1, 0, 0),
        (0, GROUPS, 0),
        (0, 0, LANES),
        (usize::MAX, 0, 0),
        (0, usize::MAX, 0),
        (0, 0, usize::MAX),
    ] {
        assert_eq!(
            source_pair_indices(1, K, row, group, lane),
            Err(PackingError::Coordinate)
        );
        assert_eq!(
            word_index(1, K, row, group, lane),
            Err(PackingError::Coordinate)
        );
    }
}

#[test]
fn partial_k_tail_shapes_and_inexact_row_lengths_are_rejected_not_zero_padded() {
    for k in [0, 1, 64, 128, 512, K - 1, K + 1, 8192, usize::MAX] {
        assert_eq!(pack_rows(&[], 1, k), Err(PackingError::Shape));
        assert_eq!(unpack_rows(&[], 1, k), Err(PackingError::Shape));
    }
    for rows in [0, MAX_ROWS + 1, usize::MAX] {
        assert_eq!(pack_rows(&[], rows, K), Err(PackingError::Shape));
    }
    for len in [0, K - 1, K + 1, 2 * K] {
        assert_eq!(pack_rows(&vec![0; len], 1, K), Err(PackingError::Length));
    }
    for len in [0, WORDS - 1, WORDS + 1, 2 * WORDS] {
        assert_eq!(unpack_rows(&vec![0; len], 1, K), Err(PackingError::Length));
        assert_eq!(
            packed_le_bytes(&vec![0; len], 1, K),
            Err(PackingError::Length)
        );
    }
}

#[test]
fn kernel_shape_length_and_launch_model_is_closed_but_preserves_capacity_tails() {
    assert_eq!(geometry().validate(), Ok(()));
    let mut tail = geometry();
    tail.activation_words = MAX_ACTIVATION_ROWS * WORDS;
    tail.output_elements = MAX_ACTIVATION_ROWS * N;
    assert_eq!(tail.validate(), Ok(()));
    for field in 0..9 {
        for value in [0, usize::MAX] {
            let mut wrong = geometry();
            match field {
                0 => wrong.rows = value,
                1 => wrong.n = value,
                2 => wrong.k = value,
                3 => wrong.world_size = value,
                4 => wrong.projection = value,
                5 => wrong.activation_words = value,
                6 => wrong.weight_words = value,
                7 => wrong.output_elements = value,
                8 => wrong.launch_threads = value,
                _ => unreachable!(),
            }
            assert!(wrong.validate().is_err(), "field {field}, value {value}");
        }
    }
    for wrong in [
        KernelGeometry {
            rows: 2,
            ..geometry()
        },
        KernelGeometry {
            n: 12288,
            ..geometry()
        },
        KernelGeometry {
            k: 4096,
            ..geometry()
        },
        KernelGeometry {
            world_size: 2,
            ..geometry()
        },
        KernelGeometry {
            world_size: 8,
            ..geometry()
        },
        KernelGeometry {
            projection: 1,
            ..geometry()
        },
    ] {
        assert!(wrong.validate().is_err(), "unadmitted geometry {wrong:?}");
    }
    for value in [WORDS - 1, MAX_ACTIVATION_ROWS * WORDS + 1] {
        assert!(
            KernelGeometry {
                activation_words: value,
                ..geometry()
            }
            .validate()
            .is_err()
        );
    }
    for value in [N - 1, MAX_ACTIVATION_ROWS * N + 1] {
        assert!(
            KernelGeometry {
                output_elements: value,
                ..geometry()
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn every_lane_checkpoint_and_wave_result_matches_for_multiple_finite_patterns() {
    for seed in [3_usize, 17, 59] {
        let a: Vec<_> = (0..K)
            .map(|i| {
                let sign = if (i + seed) % 3 == 0 { 0x8000 } else { 0 };
                sign | 0x3c00 | u16::try_from((i * seed) % 1024).unwrap()
            })
            .collect();
        let weights: Vec<_> = (0..K)
            .map(|i| {
                let sign = if (i + seed) % 5 == 0 { 0x8000 } else { 0 };
                sign | 0x3b00 | u16::try_from((i * (seed + 2)) % 1024).unwrap()
            })
            .collect();
        let (baseline, _) = traces(&a, &weights);
        assert!(finish_row(&baseline).unwrap().accepted);
        for (lane, trace) in baseline.iter().enumerate() {
            assert_eq!(trace.steps.len(), 192);
            assert_eq!(
                trace
                    .steps
                    .iter()
                    .map(|step| step.inner)
                    .collect::<Vec<_>>(),
                (0..192).map(|step| step * LANES + lane).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn cancellation_fixture_detects_pairwise_addition_or_low_high_reordering() {
    let mut a = vec![0; K];
    let weights = vec![0x3f80; K];
    for (step, bits) in [0x4b80, 0x3f80, 0xcb80, 0x3f80].into_iter().enumerate() {
        a[step * LANES] = bits;
    }
    let (baseline, _) = traces(&a, &weights);
    assert_eq!(baseline[0].partial_bits, 1.0_f32.to_bits());
    let reassociated = (16_777_216.0_f32 + -16_777_216.0_f32) + (1.0_f32 + 1.0_f32);
    assert_ne!(baseline[0].partial_bits, reassociated.to_bits());
    let swapped = ((1.0_f32 + 16_777_216.0_f32) + 1.0_f32) + -16_777_216.0_f32;
    assert_ne!(baseline[0].partial_bits, swapped.to_bits());
}

#[test]
fn signed_zero_subnormals_and_underflow_preserve_bits_and_contribution_order() {
    let patterns = [0_u16, 0x8000, 1, 0x8001, 0x0080, 0x8080, 0x3f00, 0xbf00];
    let a: Vec<_> = (0..K)
        .map(|i| patterns[(i / LANES) % patterns.len()])
        .collect();
    let weights = vec![0x3f80; K];
    let (baseline, _) = traces(&a, &weights);
    assert_eq!(baseline[0].steps[0].product_bits, 0.0_f32.to_bits());
    assert_eq!(baseline[0].steps[1].product_bits, (-0.0_f32).to_bits());
    let mut a = vec![0; K];
    let mut weights = vec![0; K];
    a[0] = 0x8001;
    weights[0] = 1;
    let (baseline, _) = traces(&a, &weights);
    assert_eq!(baseline[0].steps[0].product_bits, (-0.0_f32).to_bits());
}

#[test]
fn nan_infinity_and_product_overflow_remain_sticky_at_early_middle_and_last_steps() {
    for (left, right) in [
        (0x7f80, 0x3f80),
        (0xff80, 0x3f80),
        (0x7fc1, 0x3f80),
        (0x7f81, 0x3f80),
        (0x7f7f, 0x4000),
    ] {
        for step in [0, 1, 95, 96, 190, 191] {
            for lane in [0, 63] {
                let mut a = vec![0; K];
                let mut weights = vec![0x3f80; K];
                a[step * LANES + lane] = left;
                weights[step * LANES + lane] = right;
                let packed_a = pack_rows(&a, 1, K).unwrap();
                let packed_weights = pack_rows(&weights, 1, K).unwrap();
                let old = baseline_lane(&a, &weights, lane).unwrap();
                let new = packed_lane(&packed_a, &packed_weights, lane).unwrap();
                assert_eq!(old, new);
                assert!(new.steps[..step].iter().all(|point| point.finite));
                assert!(new.steps[step..].iter().all(|point| !point.finite));
            }
        }
    }
}

#[test]
fn accumulation_and_collective_overflow_reject_but_finite_fp32_without_bf16_narrowing_is_accepted()
{
    let weights = vec![0x3f80; K];
    let mut a = vec![0; K];
    a[0] = 0x7f7f;
    a[LANES] = 0x7f7f;
    let (baseline, _) = traces(&a, &weights);
    assert!(f32::from_bits(baseline[0].steps[1].product_bits).is_finite());
    assert!(!baseline[0].finite);
    assert!(!finish_row(&baseline).unwrap().accepted);

    a.fill(0);
    a[..LANES].fill(0x7cff);
    let (baseline, _) = traces(&a, &weights);
    assert!(baseline.iter().all(|lane| lane.finite));
    let result = finish_row(&baseline).unwrap();
    assert!(!result.accepted);
    assert!(
        result
            .sums
            .iter()
            .all(|bits| f32::from_bits(*bits).is_infinite())
    );

    a.fill(0);
    a[0] = 0x7f7f;
    a[LANES] = 0x7b00;
    let (baseline, _) = traces(&a, &weights);
    assert!(baseline.iter().all(|lane| lane.finite));
    let result = finish_row(&baseline).unwrap();
    assert!(
        result
            .sums
            .iter()
            .all(|bits| f32::from_bits(*bits).is_finite())
    );
    assert_eq!(result.sums, [0x7f7f_8000; LANES]);
    assert!(!Bf16::from_f32(f32::from_bits(result.sums[0])).is_finite());
    assert!(result.accepted);
}

#[test]
fn poisoned_unused_activation_capacity_does_not_become_a_contribution() {
    let mut a = vec![0x7fc1; MAX_ACTIVATION_ROWS * K];
    a[..K].fill(0x3f00);
    let before = a.clone();
    let packed = pack_rows(&a, MAX_ACTIVATION_ROWS, K).unwrap();
    let weights = vec![0x3f80; K];
    let packed_weights = pack_rows(&weights, 1, K).unwrap();
    for lane in 0..LANES {
        let expected = baseline_lane(&a, &weights, lane).unwrap();
        assert_eq!(
            packed_lane(&packed, &packed_weights, lane).unwrap(),
            expected
        );
        assert_eq!(
            packed_lane(&packed[..WORDS + 1], &packed_weights, lane).unwrap(),
            expected
        );
        assert!(expected.finite);
    }
    assert!(packed[WORDS..].iter().all(|word| *word == 0x7fc1_7fc1));
    assert_eq!(a, before);
}

#[test]
fn reference_models_reject_invalid_lengths_lanes_and_incomplete_collectives() {
    let a = vec![0; K];
    let packed = vec![0; WORDS];
    for lane in [LANES, usize::MAX] {
        assert_eq!(baseline_lane(&a, &a, lane), Err(PackingError::Coordinate));
        assert_eq!(
            packed_lane(&packed, &packed, lane),
            Err(PackingError::Coordinate)
        );
    }
    assert_eq!(baseline_lane(&a[..K - 1], &a, 0), Err(PackingError::Length));
    assert_eq!(baseline_lane(&a, &a[..K - 1], 0), Err(PackingError::Length));
    assert_eq!(
        packed_lane(&packed[..WORDS - 1], &packed, 0),
        Err(PackingError::Length)
    );
    assert_eq!(
        packed_lane(&packed, &packed[..WORDS - 1], 0),
        Err(PackingError::Length)
    );
    assert_eq!(
        packed_lane(&vec![0; MAX_ACTIVATION_ROWS * WORDS + 1], &packed, 0),
        Err(PackingError::Length)
    );
    assert_eq!(finish_row(&[]), Err(PackingError::Length));
    let lane = baseline_lane(&a, &a, 0).unwrap();
    assert_eq!(
        finish_row(&vec![lane; LANES - 1]),
        Err(PackingError::Length)
    );
}

#[test]
fn exact_down_geometry_and_last_high_half_keep_the_final_contribution() {
    assert_eq!((N, K, GROUPS, WORDS), (4096, 12288, 96, 6144));
    assert_eq!(source_pair_indices(1, K, 0, 95, 63), Ok([12223, 12287]));
    assert_eq!(word_index(1, K, 0, 95, 63), Ok(6143));
    let mut a = vec![0; K];
    let weights = vec![0x3f80; K];
    a[K - 1] = 0x3f80;
    let (baseline, _) = traces(&a, &weights);
    let last = &baseline[63].steps[191];
    assert_eq!(last.inner, 12287);
    assert_eq!(last.product_bits, 1.0_f32.to_bits());
    assert_eq!(
        finish_row(&baseline).unwrap().sums,
        [1.0_f32.to_bits(); LANES]
    );
}

#[test]
fn nonzero_positive_and_negative_subnormal_fp32_outputs_are_not_flushed() {
    for (bits, expected) in [(0x0001, 0x0001_0000), (0x8001, 0x8001_0000)] {
        let mut a = vec![0; K];
        let weights = vec![0x3f80; K];
        a[0] = bits;
        let (baseline, _) = traces(&a, &weights);
        let result = finish_row(&baseline).unwrap();
        assert!(result.accepted);
        assert_eq!(result.sums, [expected; LANES]);
        assert_ne!(expected & 0x7fff_ffff, 0);
    }
}

#[test]
fn fp32_store_keeps_non_bf16_bits_and_every_other_output_sentinel() {
    let mut a = vec![0; K];
    let weights = vec![0x3f80; K];
    a[0] = 0x3f80;
    a[LANES] = 0x3b00;
    let (baseline, packed) = traces(&a, &weights);
    let expected = 0x3f80_4000;
    assert_ne!(
        Bf16::from_f32(f32::from_bits(expected)).to_f32().to_bits(),
        expected
    );
    for column in [0, N - 1] {
        let poison = 0x7fc1_2345;
        let mut old_output = vec![f32::from_bits(poison); MAX_ACTIVATION_ROWS * N];
        let mut new_output = old_output.clone();
        assert_eq!(
            finish_row_into(&baseline, &mut old_output, column),
            finish_row_into(&packed, &mut new_output, column)
        );
        let actual: Vec<_> = new_output.iter().map(|value| value.to_bits()).collect();
        assert_eq!(
            actual,
            old_output
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(actual[column], expected);
        assert!(
            actual
                .iter()
                .enumerate()
                .all(|(index, bits)| *bits == if index == column { expected } else { poison })
        );
    }
}

#[test]
fn rejected_fp32_host_stores_leave_the_complete_buffer_unchanged() {
    let weights = vec![0x3f80; K];
    let mut a = vec![0; K];
    a[K - 1] = 0x7f80;
    let (bad, _) = traces(&a, &weights);
    a.fill(0);
    let (good, _) = traces(&a, &weights);
    for (lanes, len, column, expected) in [
        (&bad, N, 0, PackingError::NonFinite),
        (
            &bad,
            MAX_ACTIVATION_ROWS * N,
            N - 1,
            PackingError::NonFinite,
        ),
        (&good, N - 1, 0, PackingError::Length),
        (&good, MAX_ACTIVATION_ROWS * N + 1, 0, PackingError::Length),
        (&good, N, N, PackingError::Coordinate),
        (&good, N, usize::MAX, PackingError::Coordinate),
    ] {
        let mut output = vec![f32::from_bits(0x7fc1_2345); len];
        let before: Vec<_> = output.iter().map(|value| value.to_bits()).collect();
        assert_eq!(finish_row_into(lanes, &mut output, column), Err(expected));
        assert_eq!(
            output
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            before
        );
    }
    let mut output = vec![f32::from_bits(0x7fc1_2345); N];
    assert_eq!(
        finish_row_into(&good[..LANES - 1], &mut output, 0),
        Err(PackingError::Length)
    );
    assert!(output.iter().all(|value| value.to_bits() == 0x7fc1_2345));
}
