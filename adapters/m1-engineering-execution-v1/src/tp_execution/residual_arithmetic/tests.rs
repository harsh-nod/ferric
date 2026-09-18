use super::{
    EngineeringTpResidualArithmeticV1 as Arithmetic, HostStagedPartialV1 as Partial, reduce,
    reduce_residual_bf16_v1,
};

#[test]
fn projection_materialization_is_distinct_and_after_the_complete_rank_sum() {
    let partials = [Partial {
        rank: 0,
        values: &[1.003_906_25],
    }];
    assert_eq!(
        reduce(Arithmetic::Fp32ResidualV1, 1, &partials, &[0x3b80]).unwrap(),
        [0x3f81]
    );
    assert_eq!(
        reduce(Arithmetic::ProjectionBf16V1, 1, &partials, &[0x3b80]).unwrap(),
        [0x3f80]
    );
    for world in [2, 8] {
        let mut values = vec![[0.0_f32]; world as usize];
        values[0] = [1.003_906_25];
        values[1] = [0.003_906_25];
        let partials = (0..world)
            .rev()
            .map(|rank| Partial {
                rank,
                values: &values[rank as usize],
            })
            .collect::<Vec<_>>();
        // Rounding rank zero separately would incorrectly produce 0x3f80.
        assert_eq!(
            reduce(Arithmetic::ProjectionBf16V1, world, &partials, &[0]).unwrap(),
            [0x3f81]
        );
    }
    let values = [
        [16_777_216.0_f32],
        [1.0],
        [-16_777_216.0],
        [1.003_906_25],
        [0.003_906_25],
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
    assert_eq!(
        reduce(Arithmetic::ProjectionBf16V1, 8, &partials, &[0]).unwrap(),
        [0x3f81]
    );
}

#[test]
fn projection_overflow_is_rejected_before_residual_cancellation() {
    let values = [f32::MAX];
    let partials = [Partial {
        rank: 0,
        values: &values,
    }];
    assert!(reduce_residual_bf16_v1(1, &partials, &[0xff7f]).is_ok());
    assert!(reduce(Arithmetic::ProjectionBf16V1, 1, &partials, &[0xff7f]).is_err());
    let max_bf16 = f32::from_bits(0x7f7f_0000);
    assert!(
        reduce(
            Arithmetic::ProjectionBf16V1,
            1,
            &[Partial {
                rank: 0,
                values: &[max_bf16]
            }],
            &[0x7f7f]
        )
        .is_err()
    );
    assert_eq!(values, [f32::MAX]);
}

#[test]
fn both_profiles_fail_closed_on_roster_shape_and_nonfinite_inputs() {
    for mode in [Arithmetic::Fp32ResidualV1, Arithmetic::ProjectionBf16V1] {
        for world in [0, 3, 9] {
            assert!(reduce(mode, world, &[], &[0]).is_err());
        }
        assert!(
            reduce(
                mode,
                1,
                &[Partial {
                    rank: 0,
                    values: &[]
                }],
                &[0]
            )
            .is_err()
        );
        assert!(
            reduce(
                mode,
                1,
                &[Partial {
                    rank: 1,
                    values: &[0.0]
                }],
                &[0]
            )
            .is_err()
        );
        assert!(
            reduce(
                mode,
                2,
                &[
                    Partial {
                        rank: 0,
                        values: &[0.0]
                    },
                    Partial {
                        rank: 0,
                        values: &[0.0]
                    }
                ],
                &[0]
            )
            .is_err()
        );
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(
                reduce(
                    mode,
                    1,
                    &[Partial {
                        rank: 0,
                        values: &[value]
                    }],
                    &[0]
                )
                .is_err()
            );
        }
        for residual in [0x7fc0, 0x7f80, 0xff80] {
            assert!(
                reduce(
                    mode,
                    1,
                    &[Partial {
                        rank: 0,
                        values: &[0.0]
                    }],
                    &[residual]
                )
                .is_err()
            );
        }
        assert!(
            reduce(
                mode,
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
            .is_err()
        );
    }
}

#[test]
fn projection_rounding_keeps_signed_zero_and_exact_labels() {
    let negative_tiny = -f32::from_bits(1);
    assert_eq!(
        reduce(
            Arithmetic::ProjectionBf16V1,
            1,
            &[Partial {
                rank: 0,
                values: &[negative_tiny]
            }],
            &[0x8000]
        )
        .unwrap(),
        [0x8000]
    );
    for mode in [Arithmetic::Fp32ResidualV1, Arithmetic::ProjectionBf16V1] {
        assert_eq!(Arithmetic::parse(mode.label()).unwrap(), mode);
    }
    assert!(Arithmetic::parse("auto").is_err());
    assert_eq!(Arithmetic::default(), Arithmetic::Fp32ResidualV1);
}
