use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};

use crate::artifact::{self, ObservedArgumentQualifiers};
use crate::{atomic_artifact as abi, atomic_probe};

#[path = "atomic_mock.rs"]
mod mock;

fn metadata() -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: abi::SYMBOL.into(),
        object_sha256: artifact::digest(b"atomic fixture"),
        kernarg_bytes: 48,
        kernarg_alignment: 8,
        group_segment_bytes: 0,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: None,
        implicit_argument_bytes: 0,
        explicit_arguments: (0..6)
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

#[test]
fn atomic_channel_exact_metadata_and_all_argument_mutations() {
    abi::validate_metadata(&metadata()).unwrap();
    for mutation in 0..9 {
        let mut value = metadata();
        match mutation {
            0 => value.symbol.push('x'),
            1 => value.kernarg_bytes = 136,
            2 => value.group_segment_bytes = 4,
            3 => value.private_segment_bytes = 4,
            4 => value.wavefront_size = 32,
            5 => value.kernarg_alignment = 4,
            6 => value.implicit_argument_offset = Some(48),
            7 => value.implicit_argument_bytes = 256,
            _ => {
                value.explicit_arguments.pop();
            }
        }
        assert!(abi::validate_metadata(&value).is_err());
    }
    for index in 0..6 {
        for mutation in 0..4 {
            let mut value = metadata();
            let arg = &mut value.explicit_arguments[index];
            match mutation {
                0 => arg.offset += 8,
                1 => arg.bytes = 4,
                2 => arg.global_buffer = !arg.global_buffer,
                _ => arg.pointee_alignment = Some(8),
            }
            assert!(abi::validate_metadata(&value).is_err());
        }
        for access in [
            BufferAccessV1::Read,
            BufferAccessV1::Write,
            BufferAccessV1::ReadWrite,
        ] {
            let mut value = metadata();
            value.explicit_arguments[index].access = Some(access);
            assert_eq!(
                abi::validate_metadata(&value).is_ok(),
                index % 2 == 0 && access == abi::ACCESS[index / 2]
            );
        }
    }
    let mut implicit = metadata();
    implicit.kernarg_bytes = 304;
    implicit.implicit_argument_offset = Some(48);
    implicit.implicit_argument_bytes = 256;
    let args = abi::kernarg(&implicit).unwrap();
    for offset in [0, 16, 32] {
        assert_eq!(&args[offset..offset + 8], &[0; 8]);
        assert_eq!(&args[offset + 8..offset + 16], &256_u64.to_le_bytes());
    }
    assert!(args[48..].iter().all(|byte| *byte == 0));
}

#[test]
fn atomic_channel_cannot_be_const_readonly_or_restrict() {
    let absent = || ObservedArgumentQualifiers {
        actual_access: None,
        is_const: None,
        is_restrict: None,
        is_volatile: None,
        is_pipe: None,
    };
    for index in 0..6 {
        abi::validate_qualifier(index, &absent()).unwrap();
        for mutation in 0..5 {
            let mut value = absent();
            match mutation {
                0 => value.is_volatile = Some(true),
                1 => value.is_pipe = Some(true),
                2 => value.is_const = Some(true),
                3 => value.is_restrict = Some(index != 2 && index != 4),
                _ => {
                    value.actual_access = Some(if index == 2 {
                        BufferAccessV1::Write
                    } else {
                        BufferAccessV1::Read
                    });
                }
            }
            assert!(
                abi::validate_qualifier(index, &value).is_err(),
                "index {index}, mutation {mutation}"
            );
        }
    }
    assert!(abi::validate_qualifier(6, &absent()).is_err());
    let mut readonly_effect = absent();
    readonly_effect.actual_access = Some(BufferAccessV1::Read);
    abi::validate_qualifier(2, &readonly_effect).unwrap();
    assert!(abi::validate_qualifier(0, &readonly_effect).is_err());
}

#[test]
fn atomic_channel_checks_inputs_and_object_before_transport() {
    let mut transport = mock::Mock::new(0);
    for bytes in [0, abi::BYTES - 1, abi::BYTES + 1] {
        assert!(atomic_probe::run(
            &mut transport,
            b"atomic fixture".to_vec(),
            &metadata(),
            &vec![0; bytes]
        )
        .is_err());
    }
    assert!(atomic_probe::run(
        &mut transport,
        b"substituted".to_vec(),
        &metadata(),
        &vec![0; abi::BYTES]
    )
    .is_err());
    assert_eq!(transport.commands, 0);
    abi::validate_inputs(&vec![255; abi::BYTES]).unwrap();
}

#[test]
fn atomic_channel_reads_both_buffers_and_releases_all_allocations() {
    let inputs: Vec<u8> = (0_u8..=255).cycle().take(abi::BYTES).collect();
    let mut transport = mock::Mock::new(0);
    let result = atomic_probe::run(
        &mut transport,
        b"atomic fixture".to_vec(),
        &metadata(),
        &inputs,
    )
    .unwrap();
    assert_eq!(result.channels, inputs);
    assert_eq!(result.output, inputs);
    assert_eq!(result.elapsed_ns, 77);
    assert_eq!(transport.dispatches, 1);
    assert_eq!(transport.allocations, 3);
    assert_eq!(transport.frees, [3, 2, 1]);
    assert!(transport.closed);
}

#[test]
fn atomic_channel_transport_mutations_and_every_guard_fail_closed() {
    for mutation in 1..=18 {
        let mut transport = mock::Mock::new(mutation);
        assert!(
            atomic_probe::run(
                &mut transport,
                b"atomic fixture".to_vec(),
                &metadata(),
                &vec![0; abi::BYTES]
            )
            .is_err(),
            "mutation {mutation}"
        );
        assert!(transport.dispatches <= 1);
        if mutation <= 4 || mutation == 18 {
            assert_eq!(transport.dispatches, 0);
        }
    }
}

#[test]
fn atomic_channel_numerical_outputs_are_left_for_independent_reference() {
    // A numerically wrong but protocol-valid result is not a probe PASS claim.
    for mutation in [19, 20] {
        let mut transport = mock::Mock::new(mutation);
        let inputs = vec![0; abi::BYTES];
        let result = atomic_probe::run(
            &mut transport,
            b"atomic fixture".to_vec(),
            &metadata(),
            &inputs,
        )
        .unwrap();
        assert!(result.channels != inputs || result.output != inputs);
        assert!(transport.closed);
    }
}

#[test]
fn atomic_channel_cli_requires_ack_and_rejects_arbitrary_launch_parameters() {
    let base = [
        "atomic-channel-run",
        "--object",
        "object",
        "--source-file",
        "source",
        "--metadata",
        "metadata",
        "--worker",
        "worker",
        "--inputs",
        "inputs",
        "--device-id-file",
        "selector",
        "--run-dir",
        "new-run",
    ];
    let parse = |args: Vec<&str>| crate::Options::parse(args.into_iter().map(String::from));
    assert!(parse(base.to_vec()).is_err());
    let mut admitted = base.to_vec();
    admitted.push("--allow-unauthenticated-machine-code");
    assert_eq!(parse(admitted.clone()).unwrap().mode, "atomic-channel-run");
    for flag in [
        "--workgroup",
        "--grid",
        "--kernel",
        "--epochs",
        "--weights",
        "--object",
    ] {
        let mut invalid = admitted.clone();
        invalid.extend([flag, "arbitrary"]);
        assert!(parse(invalid).is_err());
    }
    admitted.push("--allow-unauthenticated-machine-code");
    assert!(parse(admitted).is_err());
}
