use fe2o3_device::Bf16;

#[derive(Debug, PartialEq, Eq)]
struct LaneTrace {
    reads: Vec<usize>,
    checkpoints: Vec<(u32, bool)>,
}

fn recorded_bits(value: f32) -> u32 {
    // Invalid inputs are rejected; their NaN payload is not an output contract.
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

fn baseline(left: &[u16], right: &[u16], k: usize, lane: usize, steps: usize) -> LaneTrace {
    let mut trace = LaneTrace {
        reads: Vec::new(),
        checkpoints: Vec::new(),
    };
    let mut partial = 0.0_f32;
    let mut finite = true;
    for step in 0..steps {
        let inner = step * 64 + lane;
        if inner < k {
            let a = Bf16::from_bits(left[inner]).to_f32();
            let b = Bf16::from_bits(right[inner]).to_f32();
            trace.reads.push(inner);
            let product = a * b;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
            trace.checkpoints.push((recorded_bits(partial), finite));
        }
    }
    trace
}

fn prefetch(left: &[u16], right: &[u16], k: usize, lane: usize, groups: usize) -> LaneTrace {
    let mut trace = LaneTrace {
        reads: Vec::new(),
        checkpoints: Vec::new(),
    };
    let mut partial = 0.0_f32;
    let mut finite = true;
    for group in 0..groups {
        let mut loaded = [(0_u16, 0_u16); 4];
        for (index, pair) in loaded.iter_mut().enumerate() {
            let inner = group * 256 + lane + index * 64;
            if inner < k {
                *pair = (left[inner], right[inner]);
                trace.reads.push(inner);
            }
        }
        for (index, (a, b)) in loaded.into_iter().enumerate() {
            let inner = group * 256 + lane + index * 64;
            if inner < k {
                let product = Bf16::from_bits(a).to_f32() * Bf16::from_bits(b).to_f32();
                partial += product;
                finite &= product.is_finite() & partial.is_finite();
                trace.checkpoints.push((recorded_bits(partial), finite));
            }
        }
    }
    trace
}

#[test]
fn every_admitted_reduction_width_and_lane_preserves_each_finite_fp32_update() {
    let patterns = [
        0x0000, 0x8000, 0x3f80, 0xbf80, 0x3f00, 0x4000, 0x3800, 0x0001, 0x8001,
    ];
    for k in [512, 1536, 2048, 4096, 6144, 12288] {
        for seed in 0..3 {
            let left: Vec<_> = (0..k)
                .map(|index| patterns[(index * 7 + seed) % patterns.len()])
                .collect();
            let right: Vec<_> = (0..k)
                .map(|index| patterns[(index * 5 + seed + 1) % patterns.len()])
                .collect();
            for lane in 0..64 {
                let expected = baseline(&left, &right, k, lane, 192);
                assert_eq!(prefetch(&left, &right, k, lane, 48), expected);
                assert!(expected.checkpoints.iter().all(|(_, finite)| *finite));
                assert_eq!(expected.reads.len(), k / 64);
                if k == 4096 {
                    assert_eq!(
                        prefetch(&left, &right, k, lane, 16),
                        baseline(&left, &right, k, lane, 64)
                    );
                }
            }
        }
    }
}

#[test]
fn cancellation_fixture_detects_reassociation_across_the_four_prefetched_pairs() {
    let mut left = vec![0_u16; 4096];
    let right = vec![0x3f80; 4096];
    for (index, value) in [0x4b80, 0x3f80, 0xcb80, 0x3f80].into_iter().enumerate() {
        left[index * 64] = value;
    }
    let expected = baseline(&left, &right, 4096, 0, 64);
    assert_eq!(prefetch(&left, &right, 4096, 0, 16), expected);
    assert_eq!(
        expected.checkpoints.last(),
        Some(&(1.0_f32.to_bits(), true))
    );
    let reassociated = (16_777_216.0_f32 + -16_777_216.0_f32) + (1.0_f32 + 1.0_f32);
    assert_ne!(
        expected.checkpoints.last().unwrap().0,
        reassociated.to_bits()
    );
}

#[test]
fn nonfinite_and_overflow_flags_remain_sticky_without_early_lane_exit() {
    for (bad, multiplier) in [
        (0x7f80, 0x3f80),
        (0xff80, 0x3f80),
        (0x7fc1, 0x3f80),
        (0x7f81, 0x3f80),
        (0x7f7f, 0x4000),
    ] {
        for step in [0, 1, 2, 3, 4, 63, 191] {
            let mut left = vec![0_u16; 12288];
            let right = vec![multiplier; 12288];
            left[step * 64 + 63] = bad;
            let expected = baseline(&left, &right, 12288, 63, 192);
            assert_eq!(prefetch(&left, &right, 12288, 63, 48), expected);
            assert_eq!(expected.reads.len(), 192);
            assert!(
                expected.checkpoints[..step]
                    .iter()
                    .all(|(_, finite)| *finite)
            );
            assert!(
                expected.checkpoints[step..]
                    .iter()
                    .all(|(_, finite)| !*finite)
            );
        }
    }
    let mut left = vec![0_u16; 4096];
    left[0] = 0x7f7f;
    left[64] = 0x7f7f;
    let right = vec![0x3f80; 4096];
    let trace = prefetch(&left, &right, 4096, 0, 16);
    assert_eq!(trace, baseline(&left, &right, 4096, 0, 64));
    assert!(trace.checkpoints[0].1);
    assert!(!trace.checkpoints[1].1);
}

#[test]
fn partial_tail_never_reads_or_accumulates_poisoned_inactive_storage() {
    for k in [512, 1536, 2048, 4096, 6144, 12288] {
        let mut left = vec![0x7fc1_u16; 12288];
        let mut right = vec![0x7f80_u16; 12288];
        left[..k].fill(0x3f80);
        right[..k].fill(0x3f80);
        for lane in 0..64 {
            let actual = prefetch(&left, &right, k, lane, 48);
            assert_eq!(actual, baseline(&left, &right, k, lane, 192));
            assert!(actual.reads.iter().all(|inner| *inner < k));
            assert_eq!(actual.checkpoints.len(), k / 64);
            assert_eq!(
                actual.checkpoints.last(),
                Some(&((k as f32 / 64.0).to_bits(), true))
            );
        }
    }
}

#[test]
fn prefetch_addresses_fit_every_admitted_tp_projection_and_endpoint_row() {
    for world in [1_usize, 2, 8] {
        for projection in 1..=6 {
            let n = match projection {
                1 => 4096 / world,
                2 | 3 => 1024 / world,
                4 | 5 => 12288 / world,
                _ => 151936,
            };
            for rows in [1, 32] {
                for row in [0, rows - 1] {
                    for column in [0, n - 1] {
                        for group in 0..16 {
                            for lane in 0..64 {
                                for index in 0..4 {
                                    let inner = group * 256 + lane + index * 64;
                                    assert!(inner < 4096);
                                    assert!(row * 4096 + inner < rows * 4096);
                                    assert!(column * 4096 + inner < n * 4096);
                                    assert!(row * n + column < rows * n);
                                }
                            }
                        }
                    }
                }
            }
        }
        for k in [4096 / world, 12288 / world] {
            for group in 0..48 {
                for lane in 0..64 {
                    for index in 0..4 {
                        let inner = group * 256 + lane + index * 64;
                        assert!(inner < 12288);
                        if inner < k {
                            assert!(31 * k + inner < 32 * k);
                            assert!(4095 * k + inner < 4096 * k);
                            assert_eq!(usize::from(inner as u16), inner);
                        }
                    }
                }
            }
        }
    }
}
