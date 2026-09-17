use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};

use crate::artifact::{self, ObservedArgumentQualifiers};
use crate::task_artifact as abi;
use crate::task_probe;

#[path = "task_mock.rs"]
mod mock;

fn metadata() -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: abi::SYMBOL.into(),
        object_sha256: artifact::digest(b"task fixture"),
        kernarg_bytes: 136,
        kernarg_alignment: 8,
        group_segment_bytes: 1024,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: None,
        implicit_argument_bytes: 0,
        explicit_arguments: (0..17)
            .map(|index| ExplicitArgumentV1 {
                offset: index * 8,
                bytes: 8,
                global_buffer: index != 1 && index != 3,
                pointee_alignment: None,
                access: None,
            })
            .collect(),
    }
}

#[test]
fn task_graph_exact_abi_and_optional_contradictions_reject() {
    abi::validate_metadata(&metadata()).unwrap();
    for mutation in 0..9 {
        let mut value = metadata();
        match mutation {
            0 => value.symbol.push('x'),
            1 => value.kernarg_bytes = 48,
            2 => value.group_segment_bytes = 512,
            3 => value.private_segment_bytes = 4,
            4 => value.wavefront_size = 32,
            5 => value.kernarg_alignment = 4,
            6 => value.implicit_argument_offset = Some(136),
            7 => value.implicit_argument_bytes = 256,
            _ => {
                value.explicit_arguments.pop();
            }
        }
        assert!(abi::validate_metadata(&value).is_err());
    }
    for index in 0..17 {
        for mutation in 0..5 {
            let mut value = metadata();
            let argument = &mut value.explicit_arguments[index];
            match mutation {
                0 => argument.offset += 1,
                1 => argument.bytes = 4,
                2 => argument.global_buffer = !argument.global_buffer,
                3 => argument.pointee_alignment = Some(8),
                _ => argument.access = Some(BufferAccessV1::Write),
            }
            assert!(
                abi::validate_metadata(&value).is_err(),
                "argument{index} mutation{mutation}"
            );
        }
    }
    let mut implicit = metadata();
    implicit.kernarg_bytes = 392;
    implicit.implicit_argument_offset = Some(136);
    implicit.implicit_argument_bytes = 256;
    abi::validate_metadata(&implicit).unwrap();
    let args = abi::kernarg(&implicit).unwrap();
    assert_eq!(&args[8..16], &896_u64.to_le_bytes());
    assert_eq!(&args[24..32], &1_u64.to_le_bytes());
    for index in 0..abi::BUFFER_COUNT {
        let offset = usize::try_from(abi::pointer_offset(index)).unwrap();
        assert_eq!(&args[offset..offset + 8], &[0; 8]);
    }
    assert!(args[136..].iter().all(|byte| *byte == 0));
}

#[test]
fn task_graph_qualifiers_preserve_shared_atomic_contract() {
    for index in 0..17 {
        let absent = || ObservedArgumentQualifiers {
            actual_access: None,
            is_const: None,
            is_restrict: None,
            is_volatile: None,
            is_pipe: None,
        };
        abi::validate_qualifier(index, &absent()).unwrap();
        let mut invalid = absent();
        invalid.is_restrict = Some(true);
        assert!(abi::validate_qualifier(index, &invalid).is_err());
        invalid = absent();
        invalid.is_volatile = Some(true);
        assert!(abi::validate_qualifier(index, &invalid).is_err());
        invalid = absent();
        invalid.actual_access = Some(BufferAccessV1::ReadWrite);
        assert_eq!(abi::validate_qualifier(index, &invalid).is_ok(), index >= 4);
    }
}

#[test]
fn task_graph_input_extent_and_value_caps_are_checked_before_transport() {
    let mut inputs = vec![0; abi::INPUT_WORDS * 4];
    inputs[..4].copy_from_slice(&1024_u32.to_le_bytes());
    abi::validate_inputs(&inputs).unwrap();
    inputs[..4].copy_from_slice(&1025_u32.to_le_bytes());
    let mut transport = mock::Mock::new(0);
    assert!(task_probe::run(
        &mut transport,
        b"task fixture".to_vec(),
        &metadata(),
        &inputs
    )
    .is_err());
    assert_eq!(transport.commands, 0);
    assert!(abi::validate_inputs(&inputs[..inputs.len() - 1]).is_err());
    assert!(task_probe::run(
        &mut transport,
        b"substituted".to_vec(),
        &metadata(),
        &vec![0; abi::INPUT_WORDS * 4]
    )
    .is_err());
    assert_eq!(transport.commands, 0);
}

#[test]
fn task_graph_reuses_fifteen_allocations_for_four_epochs_and_stale_rejection() {
    let mut transport = mock::Mock::new(0);
    let observations = task_probe::run(
        &mut transport,
        b"task fixture".to_vec(),
        &metadata(),
        &vec![0; abi::INPUT_WORDS * 4],
    )
    .unwrap();
    assert_eq!(observations.len(), 5);
    assert_eq!(transport.allocations, 15);
    assert_eq!(transport.dispatches, 5);
    assert_eq!(transport.frees, (1..=15).rev().collect::<Vec<_>>());
    assert!(transport.closed);
    for (index, observed) in observations.iter().enumerate() {
        assert_eq!(observed.expected_epoch, u32::try_from(index + 1).unwrap());
        assert_eq!(observed.initialized_epoch, observed.expected_epoch.min(4));
        assert_eq!(observed.stale_epoch_negative, index == 4);
        assert_eq!(observed.worker_dispatch_interval_ns, 77);
    }
}

#[test]
fn task_graph_corruption_duplicate_allocation_and_cleanup_fail_closed() {
    for mutation in 1..=15 {
        let mut transport = mock::Mock::new(mutation);
        assert!(
            task_probe::run(
                &mut transport,
                b"task fixture".to_vec(),
                &metadata(),
                &vec![0; abi::INPUT_WORDS * 4]
            )
            .is_err(),
            "mutation{mutation}"
        );
        assert!(transport.dispatches <= 5);
        if mutation <= 3 {
            assert_eq!(transport.dispatches, 0);
        }
    }
}

#[test]
fn task_graph_cli_requires_acknowledgement_and_has_no_arbitrary_launch_options() {
    let base = [
        "task-graph-run",
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
    assert_eq!(parse(admitted.clone()).unwrap().mode, "task-graph-run");
    for flag in [
        "--epochs",
        "--workgroup",
        "--grid",
        "--kernel",
        "--weights",
        "--object",
    ] {
        let mut invalid = admitted.clone();
        invalid.extend([flag, "arbitrary"]);
        assert!(parse(invalid).is_err(), "option {flag}");
    }
    admitted.push("--allow-unauthenticated-machine-code");
    assert!(parse(admitted).is_err());
    let inspect = [
        "task-graph-inspect",
        "--object",
        "object",
        "--source-file",
        "source",
        "--metadata",
        "new-metadata",
    ];
    assert_eq!(parse(inspect.to_vec()).unwrap().mode, "task-graph-inspect");
}

#[test]
fn task_graph_state_masks_owners_epoch_and_stale_payload_are_exact() {
    let mut completed = [0; abi::STATE_WORDS];
    completed[..6].copy_from_slice(&[1, 0, 127, 127, 0x1555, 0]);
    task_probe::validate_state(&completed, 1, 1).unwrap();
    for index in 0..6 {
        let mut invalid = completed;
        invalid[index] ^= if index == 4 { 1 << 14 } else { 1 };
        assert!(task_probe::validate_state(&invalid, 1, 1).is_err());
    }
    for owner in [0, 3] {
        let mut invalid = completed;
        invalid[4] = (invalid[4] & !3) | owner;
        assert!(task_probe::validate_state(&invalid, 1, 1).is_err());
    }
    let mut rejected = task_probe::initial_state(4);
    rejected[5] = 1;
    task_probe::validate_state(&rejected, 5, 4).unwrap();
    for index in 0..abi::STATE_WORDS {
        let mut invalid = rejected;
        invalid[index] ^= 1;
        assert!(task_probe::validate_state(&invalid, 5, 4).is_err());
    }
}

#[test]
fn task_graph_observed_owner_edges_do_not_infer_handoffs_from_launch_geometry() {
    let (owners, distinct, edges) = task_probe::owner_evidence(0x1555);
    assert_eq!(owners, [1; 7]);
    assert_eq!(distinct, 1);
    assert!(edges.is_empty());
    let (_, distinct, edges) = task_probe::owner_evidence(0x1555 ^ (3 << 2));
    assert_eq!(distinct, 2);
    assert_eq!(edges, [[0, 1], [1, 3]]);
    let (owners, distinct, edges) = task_probe::owner_evidence(0);
    assert_eq!(owners, [0; 7]);
    assert_eq!(distinct, 0);
    assert!(edges.is_empty());
}
