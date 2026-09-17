use crate::{artifact, publication_artifact as abi, publication_probe};
use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};
#[path = "publication_mock.rs"]
mod mock;

fn metadata() -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: abi::SYMBOL.into(),
        object_sha256: artifact::digest(b"publication fixture"),
        kernarg_bytes: 80,
        kernarg_alignment: 8,
        group_segment_bytes: 0,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: None,
        implicit_argument_bytes: 0,
        explicit_arguments: (0..10)
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
fn case(shape: abi::Shape) -> abi::Case {
    abi::Case {
        schema: "ferric-static-publication-case-v1".into(),
        pattern: abi::Pattern::Ready,
        repetition: 0,
        shape,
    }
}
fn inputs() -> Vec<u8> {
    (1_u16..=128)
        .flat_map(|value| f32::from(value).to_le_bytes())
        .collect()
}
fn flags() -> Vec<u8> {
    [2_u32; 128]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect()
}

#[test]
fn publication_metadata_and_argument_mutations_reject() {
    abi::validate_metadata(&metadata()).unwrap();
    for mutation in 0..9 {
        let mut item = metadata();
        match mutation {
            0 => item.symbol.push('x'),
            1 => item.kernarg_bytes = 48,
            2 => item.group_segment_bytes = 4,
            3 => item.private_segment_bytes = 4,
            4 => item.wavefront_size = 32,
            5 => item.kernarg_alignment = 4,
            6 => item.implicit_argument_offset = Some(80),
            7 => item.implicit_argument_bytes = 256,
            _ => {
                item.explicit_arguments.pop();
            }
        }
        assert!(abi::validate_metadata(&item).is_err());
    }
    for index in 0..10 {
        for mutation in 0..5 {
            let mut item = metadata();
            let arg = &mut item.explicit_arguments[index];
            match mutation {
                0 => arg.offset += 8,
                1 => arg.bytes = 4,
                2 => arg.global_buffer = !arg.global_buffer,
                3 => arg.pointee_alignment = Some(2),
                _ => arg.access = Some(BufferAccessV1::Read),
            }
            assert!(abi::validate_metadata(&item).is_err());
        }
    }
}

#[test]
fn publication_closed_cases_bind_logical_extents_without_raw_pointers() {
    let mut metadata = metadata();
    metadata.kernarg_bytes = 336;
    metadata.implicit_argument_offset = Some(80);
    metadata.implicit_argument_bytes = 256;
    for shape in [
        abi::Shape::Valid,
        abi::Shape::PayloadShort,
        abi::Shape::FlagsShort,
        abi::Shape::PayloadEmpty,
        abi::Shape::FlagsEmpty,
    ] {
        let case = case(shape);
        let args = abi::kernarg(&metadata, &case).unwrap();
        for (index, len) in case.lengths().iter().enumerate() {
            assert_eq!(&args[index * 16..index * 16 + 8], &[0; 8]);
            assert_eq!(&args[index * 16 + 8..index * 16 + 16], &len.to_le_bytes());
        }
        assert!(args[80..].iter().all(|byte| *byte == 0));
    }
    let mut invalid = case(abi::Shape::Valid);
    invalid.repetition = 8;
    assert!(invalid.validate().is_err());
    invalid = case(abi::Shape::PayloadShort);
    invalid.pattern = abi::Pattern::Zero;
    assert!(invalid.validate().is_err());
}

#[test]
fn publication_qualifiers_preserve_shared_atomic_and_exclusive_roots() {
    let absent = || artifact::ObservedArgumentQualifiers {
        actual_access: None,
        is_const: None,
        is_restrict: None,
        is_volatile: None,
        is_pipe: None,
    };
    for index in 0..10 {
        abi::validate_qualifier(index, &absent()).unwrap();
        let mut value = absent();
        value.is_const = Some(true);
        assert!(abi::validate_qualifier(index, &value).is_err());
        value = absent();
        value.is_restrict = Some(index == 2 || index % 2 != 0);
        assert!(abi::validate_qualifier(index, &value).is_err());
    }
    let mut value = absent();
    value.actual_access = Some(BufferAccessV1::Read);
    abi::validate_qualifier(4, &value).unwrap();
    assert!(abi::validate_qualifier(2, &value).is_err());
}

#[test]
fn publication_completion_reads_five_buffers_and_closes_without_reuse() {
    for shape in [
        abi::Shape::Valid,
        abi::Shape::PayloadShort,
        abi::Shape::FlagsShort,
        abi::Shape::PayloadEmpty,
        abi::Shape::FlagsEmpty,
    ] {
        let case = case(shape);
        let mut mock = mock::Mock::new(0, case.clone());
        let result = publication_probe::run(
            &mut mock,
            b"publication fixture".to_vec(),
            &metadata(),
            &case,
            &inputs(),
            &flags(),
        )
        .unwrap();
        assert_eq!(result.data[2], inputs());
        assert_eq!(result.elapsed_ns, 77);
        assert_eq!(mock.dispatches, 1);
        assert_eq!(mock.reads, 5);
        assert_eq!(mock.allocations, 5);
        assert_eq!(mock.frees, [5, 4, 3, 2, 1]);
        assert!(mock.closed);
    }
}

#[test]
fn publication_every_guard_and_transport_failure_is_terminal() {
    let case = case(abi::Shape::Valid);
    for mutation in (1..=22).filter(|value| *value != 21) {
        let mut mock = mock::Mock::new(mutation, case.clone());
        assert!(
            publication_probe::run(
                &mut mock,
                b"publication fixture".to_vec(),
                &metadata(),
                &case,
                &inputs(),
                &flags()
            )
            .is_err(),
            "mutation {mutation}"
        );
        assert!(mock.dispatches <= 1);
        if matches!(mutation, 16 | 22) {
            assert_eq!(mock.reads, 0);
            assert!(mock.frees.is_empty());
            assert!(!mock.closed);
        }
    }
}

#[test]
fn publication_invalid_inputs_never_reach_transport_and_numerics_are_independent() {
    let case = case(abi::Shape::Valid);
    let mut mock = mock::Mock::new(0, case.clone());
    for (input, flags) in [
        (vec![], flags()),
        (inputs(), vec![]),
        (inputs(), vec![0; 512]),
        (vec![0; 512], flags()),
    ] {
        assert!(publication_probe::run(
            &mut mock,
            b"publication fixture".to_vec(),
            &metadata(),
            &case,
            &input,
            &flags
        )
        .is_err());
    }
    assert!(publication_probe::run(
        &mut mock,
        b"wrong object".to_vec(),
        &metadata(),
        &case,
        &inputs(),
        &flags()
    )
    .is_err());
    assert_eq!(mock.commands, 0);
    let mut mock = mock::Mock::new(21, case.clone());
    let result = publication_probe::run(
        &mut mock,
        b"publication fixture".to_vec(),
        &metadata(),
        &case,
        &inputs(),
        &flags(),
    )
    .unwrap();
    assert_eq!(&result.data[4][512..516], &[0; 4]);
    assert!(mock.closed); // The independent checker, not transport, rejects this bad Ready value.
}

#[test]
fn publication_cli_has_no_launch_override_and_requires_explicit_ack() {
    let base = [
        "static-publication-run",
        "--object",
        "o",
        "--source-file",
        "s",
        "--metadata",
        "m",
        "--worker",
        "w",
        "--inputs",
        "i",
        "--initial-flags",
        "f",
        "--case-file",
        "c",
        "--device-id-file",
        "d",
        "--run-dir",
        "r",
    ];
    let parse = |args: Vec<&str>| crate::Options::parse(args.into_iter().map(String::from));
    assert!(parse(base.to_vec()).is_err());
    let mut args = base.to_vec();
    args.push("--allow-unauthenticated-machine-code");
    assert!(parse(args.clone()).is_ok());
    for flag in ["--grid", "--workgroup", "--epochs", "--retries", "--object"] {
        let mut invalid = args.clone();
        invalid.extend([flag, "arbitrary"]);
        assert!(parse(invalid).is_err());
    }
}
