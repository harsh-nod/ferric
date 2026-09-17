use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};

use crate::{artifact, knorm_artifact as abi, knorm_probe, kproj_artifact, Options};

#[path = "knorm_mock.rs"]
mod mock;

fn metadata(consumer: bool) -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: if consumer {
            abi::SYMBOL
        } else {
            kproj_artifact::WAVE64_SYMBOL
        }
        .into(),
        object_sha256: artifact::digest(if consumer { b"norm" } else { b"proj" }),
        kernarg_bytes: if consumer { 64 } else { 48 },
        kernarg_alignment: 8,
        group_segment_bytes: 0,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: None,
        implicit_argument_bytes: 0,
        explicit_arguments: (0..if consumer { 8 } else { 6 })
            .map(|index| ExplicitArgumentV1 {
                offset: index * 8,
                bytes: 8,
                global_buffer: index % 2 == 0,
                pointee_alignment: None,
                access: None,
            })
            .collect(),
    }
}

fn run(mock: &mut mock::Mock) -> crate::Result<knorm_probe::Observation> {
    knorm_probe::run(
        mock,
        knorm_probe::Kernel {
            object: b"proj".to_vec(),
            metadata: &metadata(false),
        },
        knorm_probe::Kernel {
            object: b"norm".to_vec(),
            metadata: &metadata(true),
        },
        &knorm_probe::Inputs {
            hidden: &vec![0; abi::BYTES[0]],
            weights: &vec![0; abi::BYTES[1]],
            norm_weights: &vec![0; abi::BYTES[3]],
        },
    )
}

#[test]
fn knorm_chain_reuses_completed_projection_without_writes_and_releases_every_buffer() {
    let mut mock = mock::Mock::new(0);
    let observed = run(&mut mock).unwrap();
    assert_eq!(mock.dispatches, 2);
    assert_eq!(mock.loads, 2);
    assert_eq!(mock.allocations, 6);
    assert_eq!(mock.writes, 6);
    assert!(mock.intermediate_read);
    assert_eq!(mock.frees, vec![6, 5, 4, 3, 2, 1]);
    assert!(mock.closed);
    assert_eq!(observed.projection, 1.0_f32.to_le_bytes().repeat(1024));
    assert_eq!(observed.quantized, 0x3f80_u16.to_le_bytes().repeat(1024));
    assert_eq!(observed.output, 0x4000_u16.to_le_bytes().repeat(1024));
    assert_eq!(observed.elapsed_ns, [71, 72]);
}

#[test]
fn knorm_chain_protocol_corruption_never_returns_success_or_retries() {
    for mutation in 1..=31 {
        let mut mock = mock::Mock::new(mutation);
        assert!(run(&mut mock).is_err(), "mutation {mutation}");
        assert!(mock.commands < 45, "mutation {mutation}");
        assert!(mock.dispatches <= 2, "mutation {mutation}");
        if matches!(mutation, 7 | 9 | 18 | 19 | 31) {
            assert_eq!(
                mock.dispatches, 1,
                "consumer ran after bad producer: {mutation}"
            );
        }
    }
}

#[test]
fn knorm_chain_rejects_input_sizes_nonfinite_and_wrong_stage_before_allocating() {
    for mutation in 0..5 {
        let mut mock = mock::Mock::new(0);
        let mut hidden = vec![0; abi::BYTES[0]];
        let mut weights = vec![0; abi::BYTES[1]];
        let mut gamma = vec![0; abi::BYTES[3]];
        let mut producer = metadata(false);
        match mutation {
            0 => {
                hidden.pop();
            }
            1 => {
                weights.push(0);
            }
            2 => {
                gamma.pop();
            }
            3 => hidden[..2].copy_from_slice(&0x7fc0_u16.to_le_bytes()),
            _ => producer.symbol = kproj_artifact::SYMBOL.into(),
        }
        assert!(knorm_probe::run(
            &mut mock,
            knorm_probe::Kernel {
                object: b"proj".to_vec(),
                metadata: &producer
            },
            knorm_probe::Kernel {
                object: b"norm".to_vec(),
                metadata: &metadata(true)
            },
            &knorm_probe::Inputs {
                hidden: &hidden,
                weights: &weights,
                norm_weights: &gamma
            }
        )
        .is_err());
        assert_eq!(mock.commands, 0);
    }
}

#[test]
fn knorm_abi_and_optional_hidden_arguments_are_exact() {
    abi::validate_consumer(&metadata(true)).unwrap();
    assert!(abi::validate_consumer(&metadata(false)).is_err());
    for mutation in 0..9 {
        let mut changed = metadata(true);
        match mutation {
            0 => changed.symbol.push('x'),
            1 => changed.kernarg_bytes = 48,
            2 => changed.kernarg_alignment = 4,
            3 => changed.group_segment_bytes = 4,
            4 => changed.private_segment_bytes = 4,
            5 => changed.wavefront_size = 32,
            6 => changed.implicit_argument_offset = Some(64),
            7 => changed.implicit_argument_bytes = 256,
            _ => {
                changed.explicit_arguments.pop();
            }
        }
        assert!(abi::validate_consumer(&changed).is_err());
    }
    for index in 0..8 {
        for mutation in 0..4 {
            let mut changed = metadata(true);
            let argument = &mut changed.explicit_arguments[index];
            match mutation {
                0 => argument.offset += 8,
                1 => argument.bytes = 4,
                2 => argument.global_buffer = !argument.global_buffer,
                _ => argument.pointee_alignment = Some(8),
            }
            assert!(abi::validate_consumer(&changed).is_err());
        }
        for access in [
            BufferAccessV1::Read,
            BufferAccessV1::Write,
            BufferAccessV1::ReadWrite,
        ] {
            let mut changed = metadata(true);
            changed.explicit_arguments[index].access = Some(access);
            assert_eq!(
                abi::validate_consumer(&changed).is_ok(),
                index % 2 == 0
                    && access
                        == if index < 4 {
                            BufferAccessV1::Read
                        } else {
                            BufferAccessV1::Write
                        }
            );
        }
    }
    let mut hidden = metadata(true);
    hidden.kernarg_bytes = 320;
    hidden.implicit_argument_offset = Some(64);
    hidden.implicit_argument_bytes = 256;
    let args = abi::kernarg(&hidden).unwrap();
    for (index, length) in abi::LENGTHS.into_iter().enumerate() {
        assert_eq!(&args[index * 16..index * 16 + 8], &[0; 8]);
        assert_eq!(
            &args[index * 16 + 8..index * 16 + 16],
            &length.to_le_bytes()
        );
    }
    assert!(args[64..].iter().all(|byte| *byte == 0));
}

#[test]
fn knorm_launch_and_qualifiers_reject_contradictions() {
    let exact = artifact::ObservedLaunchMetadata {
        required_workgroup_size: Some([128, 1, 1]),
        max_flat_workgroup_size: 128,
        max_workgroups: [Some(4), Some(1), Some(1)],
        cluster_dims: None,
    };
    abi::validate_launch(&exact).unwrap();
    for mutation in 0..7 {
        let mut value = artifact::ObservedLaunchMetadata {
            required_workgroup_size: exact.required_workgroup_size,
            max_flat_workgroup_size: 128,
            max_workgroups: exact.max_workgroups,
            cluster_dims: None,
        };
        match mutation {
            0 => value.required_workgroup_size = None,
            1 => value.required_workgroup_size = Some([64, 1, 1]),
            2 => value.max_flat_workgroup_size = 256,
            3..=5 => value.max_workgroups[mutation - 3] = Some(9),
            _ => value.cluster_dims = Some([1, 1, 1]),
        }
        assert!(abi::validate_launch(&value).is_err());
    }
    for index in 0..8 {
        let absent = || artifact::ObservedArgumentQualifiers {
            actual_access: None,
            is_const: None,
            is_restrict: None,
            is_volatile: None,
            is_pipe: None,
        };
        abi::validate_qualifier(index, &absent()).unwrap();
        for mutation in 0..5 {
            let mut value = absent();
            match mutation {
                0 => value.is_volatile = Some(true),
                1 => value.is_pipe = Some(true),
                2 => value.is_const = Some(index % 2 != 0 || index >= 4),
                3 => value.is_restrict = Some(index % 2 != 0 || index < 4),
                _ => value.actual_access = Some(BufferAccessV1::ReadWrite),
            }
            assert!(
                abi::validate_qualifier(index, &value).is_err(),
                "{index}/{mutation}"
            );
        }
    }
}

#[test]
fn knorm_command_requires_both_artifacts_all_inputs_and_explicit_opt_in() {
    let mut args = vec!["qwen3-knorm-chain-run".to_owned()];
    for name in [
        "producer-object",
        "producer-source",
        "consumer-object",
        "consumer-source",
        "metadata",
        "worker",
        "inputs",
        "weights",
        "norm-weights",
        "device-id-file",
        "run-dir",
    ] {
        args.extend([format!("--{name}"), "path".into()]);
    }
    assert!(Options::parse(args.clone().into_iter()).is_err());
    args.push("--allow-unauthenticated-machine-code".into());
    assert!(Options::parse(args.clone().into_iter()).is_ok());
    for start in (1..23).step_by(2) {
        let mut missing = args.clone();
        missing.drain(start..start + 2);
        assert!(Options::parse(missing.into_iter()).is_err());
    }
    args.push("--allow-unauthenticated-machine-code".into());
    assert!(Options::parse(args.into_iter()).is_err());
}
