use super::*;

#[test]
fn full_single_step_routes_all_o_and_down_boundaries_through_selected_arithmetic() {
    for world in [1, 2, 8] {
        let mut driver = fixture(world);
        driver
            .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
            .unwrap();
        for transport in &mut driver.transports {
            transport.partial_override = Some(if transport.rank == 0 {
                f32::from_bits(0x3b80_8000)
            } else {
                0.0
            });
        }
        assert_eq!(driver.step(1).unwrap(), 42);
        assert!(driver.hidden.iter().all(|value| *value == 0x3f80));
        for transport in &driver.transports {
            for operation in [1, 2] {
                assert_eq!(
                    transport
                        .commands
                        .iter()
                        .filter(|command| command.kernel == PARTIAL
                            && RecordingTransport::scalar(command, 7) == operation)
                        .count(),
                    36
                );
            }
        }
        assert_eq!(driver.dispatch_counts()[0], 544);
        driver.close().unwrap();
    }
}

#[test]
fn single_row_o_and_down_share_explicit_projection_rounding() {
    for world in [1, 2, 8] {
        for mode in [
            EngineeringTpResidualArithmeticV1::Fp32ResidualV1,
            EngineeringTpResidualArithmeticV1::ProjectionBf16V1,
        ] {
            for reuse in [false, true] {
                let mut driver = fixture(world);
                if reuse {
                    driver
                        .configure_reduction(EngineeringTpReductionModeV3::HostStagedReuseV3)
                        .unwrap();
                }
                driver.configure_host_residual_arithmetic(mode).unwrap();
                assert_eq!(driver.residual_arithmetic(), mode);
                for operation in [
                    Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
                    Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
                ] {
                    driver.hidden.fill(0x3b80);
                    for (rank, transport) in driver.ranks.iter().zip(&mut driver.transports) {
                        let value = if rank.geometry.rank == 0 {
                            f32::from_bits(0x3f80_8000)
                        } else {
                            0.0
                        };
                        for bytes in transport
                            .buffers
                            .get_mut(&rank.partial.id)
                            .unwrap()
                            .chunks_exact_mut(4)
                        {
                            bytes.copy_from_slice(&value.to_le_bytes());
                        }
                    }
                    driver.reduce(0, operation).unwrap();
                    let expected = if mode == EngineeringTpResidualArithmeticV1::Fp32ResidualV1 {
                        0x3f81
                    } else {
                        0x3f80
                    };
                    assert!(driver.hidden.iter().all(|value| *value == expected));
                    for (rank, transport) in driver.ranks.iter().zip(&driver.transports) {
                        assert_eq!(
                            decode_bf16(&transport.buffers[&rank.hidden.id]).unwrap(),
                            driver.hidden
                        );
                        assert!(transport.commands.is_empty());
                    }
                }
                driver.close().unwrap();
            }
        }
    }
}

#[test]
fn reduction_and_arithmetic_configuration_are_one_shot_and_fresh_only() {
    let mut driver = fixture(1);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::HostStagedV1)
        .unwrap();
    for mode in [
        EngineeringTpReductionModeV3::HostStagedV1,
        EngineeringTpReductionModeV3::HostStagedReuseV3,
        EngineeringTpReductionModeV3::DeviceTp1V3,
    ] {
        assert!(driver.configure_reduction(mode).is_err());
    }
    driver
        .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
        .unwrap();
    assert!(
        driver
            .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::Fp32ResidualV1)
            .is_err()
    );
    driver.close().unwrap();

    let mut driver = fixture(1);
    driver
        .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
        .unwrap();
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    driver.close().unwrap();

    let mut driver = fixture(1);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    assert!(
        driver
            .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
            .is_err()
    );
    driver.close().unwrap();

    let mut driver = fixture(1);
    driver.step(1).unwrap();
    assert!(
        driver
            .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
            .is_err()
    );
    driver.close().unwrap();
    assert!(
        driver
            .configure_host_residual_arithmetic(EngineeringTpResidualArithmeticV1::ProjectionBf16V1)
            .is_err()
    );
}
