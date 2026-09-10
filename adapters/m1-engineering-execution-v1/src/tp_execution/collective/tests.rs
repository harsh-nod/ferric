use super::{reduce_residual_bf16_v1 as reduce, HostStagedPartialV1 as Partial};

#[test]
fn exact_rank_order_residual_once_and_single_rounding() {
    let values = [[1.0_f32], [0.003_906_25]];
    let partials = [
        Partial {
            rank: 1,
            values: &values[1],
        },
        Partial {
            rank: 0,
            values: &values[0],
        },
    ];
    // 1 + half a BF16 ulp ties to even; residual added only once.
    assert_eq!(reduce(2, &partials, &[0]).unwrap(), [0x3f80]);
    assert_eq!(reduce(2, &partials, &[0x3f80]).unwrap(), [0x4000]);
    let values = [
        [16_777_216.0_f32],
        [1.0],
        [-16_777_216.0],
        [2.0],
        [0.0],
        [0.0],
        [0.0],
        [0.0],
    ];
    let partials = (0..8)
        .rev()
        .map(|rank| Partial {
            rank,
            values: &values[rank as usize],
        })
        .collect::<Vec<_>>();
    assert_eq!(reduce(8, &partials, &[0]).unwrap(), [0x4000]);
}

#[test]
fn rank_and_shape_errors_never_mutate_inputs() {
    let values = [1.0_f32];
    let duplicate = [
        Partial {
            rank: 0,
            values: &values,
        },
        Partial {
            rank: 0,
            values: &values,
        },
    ];
    assert!(reduce(2, &duplicate, &[0]).is_err());
    assert!(reduce(8, &duplicate, &[0]).is_err());
    assert!(reduce(3, &duplicate, &[0]).is_err());
    let outside = [Partial {
        rank: 2,
        values: &values,
    }];
    assert!(reduce(1, &outside, &[0]).is_err());
    let short = [Partial {
        rank: 0,
        values: &[],
    }];
    assert!(reduce(1, &short, &[0]).is_err());
    assert_eq!(values.map(f32::to_bits), [1.0_f32.to_bits()]);
}

#[test]
fn rejects_nonfinite_inputs_intermediates_and_bf16_overflow() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        assert!(reduce(
            1,
            &[Partial {
                rank: 0,
                values: &[value]
            }],
            &[0]
        )
        .is_err());
    }
    assert!(reduce(
        1,
        &[Partial {
            rank: 0,
            values: &[0.0]
        }],
        &[0x7fc0]
    )
    .is_err());
    assert!(reduce(
        2,
        &[
            Partial {
                rank: 0,
                values: &[f32::MAX]
            },
            Partial {
                rank: 1,
                values: &[f32::MAX]
            }
        ],
        &[0]
    )
    .is_err());
}
