use std::collections::BTreeMap;
use std::io::Cursor;

use fe2o3_kfd::engineering_wire::{
    write_header_v1, BufferAccessV1, CommandV1, ExplicitArgumentV1, KernelMetadataV1, ResponseV1,
    MAX_TRANSFER_BYTES_V1,
};

use crate::artifact::{self, BYTES, GUARD};
use crate::session::{Packet, Transport};
use crate::{probe, Result};

fn metadata() -> KernelMetadataV1 {
    KernelMetadataV1 {
        symbol: artifact::SYMBOL.into(),
        object_sha256: artifact::digest(b"fixture object"),
        kernarg_bytes: 304,
        kernarg_alignment: 8,
        group_segment_bytes: 0,
        private_segment_bytes: 0,
        wavefront_size: 64,
        implicit_argument_offset: Some(48),
        implicit_argument_bytes: 256,
        explicit_arguments: (0..6)
            .map(|index| ExplicitArgumentV1 {
                offset: index * 8,
                bytes: 8,
                global_buffer: index % 2 == 0,
                pointee_alignment: (index % 2 == 0).then_some(4),
                access: (index % 2 == 0).then_some(if index == 4 {
                    BufferAccessV1::Write
                } else {
                    BufferAccessV1::Read
                }),
            })
            .collect(),
    }
}

struct Mock {
    buffers: BTreeMap<u64, Vec<u8>>,
    next_buffer: u64,
    mutation: u32,
    dispatches: u32,
    closed: bool,
    frees: Vec<u64>,
}

impl Mock {
    fn new(mutation: u32) -> Self {
        Self {
            buffers: BTreeMap::new(),
            next_buffer: 1,
            mutation,
            dispatches: 0,
            closed: false,
            frees: vec![],
        }
    }
}

impl Transport for Mock {
    fn request(&mut self, command: CommandV1, payload: Vec<u8>) -> Result<Packet> {
        assert_eq!(command.payload_bytes().unwrap(), payload.len());
        let mut response_payload = vec![];
        let response = match command {
            CommandV1::LoadKernel {
                object_sha256,
                symbol,
                ..
            } => {
                assert_eq!(object_sha256, artifact::digest(&payload));
                assert_eq!(symbol, artifact::SYMBOL);
                let mut metadata = metadata();
                if self.mutation == 1 {
                    metadata.object_sha256[0] ^= 1;
                }
                ResponseV1::LoadedKernel {
                    kernel: 5,
                    metadata,
                }
            }
            CommandV1::Allocate { bytes } => {
                let buffer = if self.mutation == 2 {
                    1
                } else {
                    self.next_buffer
                };
                self.next_buffer += 1;
                self.buffers
                    .insert(buffer, vec![0; usize::try_from(bytes).unwrap()]);
                ResponseV1::Allocated {
                    buffer,
                    bytes: bytes + u64::from(self.mutation == 3),
                }
            }
            CommandV1::Write { buffer, offset, .. } => {
                assert_eq!(offset, 0);
                *self.buffers.get_mut(&buffer).unwrap() = payload;
                ResponseV1::Written
            }
            CommandV1::Dispatch {
                workgroup,
                grid,
                pointers,
                timeout_ms,
                ..
            } => {
                assert_eq!(workgroup, [128, 1, 1]);
                assert_eq!(grid, [256, 1, 1]);
                assert_eq!(timeout_ms, 10_000);
                assert_eq!(pointers.len(), 3);
                assert_eq!(payload, artifact::kernarg(&metadata()).unwrap());
                for (index, pointer) in pointers.iter().enumerate() {
                    assert_eq!(pointer.kernarg_offset, u32::try_from(index * 16).unwrap());
                    assert_eq!(pointer.buffer, index as u64 + 1);
                    assert_eq!(pointer.extent_bytes, BYTES[index] as u64);
                    assert_eq!(
                        pointer.buffer_offset,
                        if index == 2 { GUARD as u64 } else { 0 }
                    );
                    assert_eq!(
                        pointer.access,
                        if index == 2 {
                            BufferAccessV1::Write
                        } else {
                            BufferAccessV1::Read
                        }
                    );
                }
                self.dispatches += 1;
                if self.mutation == 4 {
                    self.buffers.get_mut(&1).unwrap()[0] ^= 1;
                }
                let output = self.buffers.get_mut(&3).unwrap();
                if self.mutation != 6 {
                    output[GUARD..GUARD + BYTES[2]].fill(0);
                }
                if self.mutation == 5 {
                    output[0] ^= 1;
                }
                ResponseV1::Dispatched { elapsed_ns: 123 }
            }
            CommandV1::Read {
                buffer,
                offset,
                bytes,
            } => {
                assert_eq!(offset, 0);
                response_payload = self.buffers[&buffer].clone();
                if self.mutation == 7 {
                    response_payload.pop();
                }
                ResponseV1::Read {
                    payload_bytes: bytes,
                }
            }
            CommandV1::Free { buffer } => {
                self.buffers.remove(&buffer).unwrap();
                self.frees.push(buffer);
                if self.mutation == 8 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Freed
                }
            }
            CommandV1::Close => {
                assert!(self.buffers.is_empty());
                self.closed = true;
                if self.mutation == 9 {
                    ResponseV1::Written
                } else {
                    ResponseV1::Closed
                }
            }
            _ => panic!("unexpected command"),
        };
        Ok(Packet {
            response,
            payload: response_payload,
        })
    }
}

fn run(mock: &mut Mock) -> Result<probe::Observation> {
    probe::run(
        mock,
        b"fixture object".to_vec(),
        &metadata(),
        &vec![0; BYTES[0]],
        &vec![0; BYTES[1]],
    )
}

#[test]
fn one_finite_dispatch_has_exact_owned_pointers_and_reverse_cleanup() {
    let mut mock = Mock::new(0);
    let observed = run(&mut mock).unwrap();
    assert_eq!(observed.output, vec![0; BYTES[2]]);
    assert_eq!(observed.elapsed_ns, 123);
    assert_eq!(mock.dispatches, 1);
    assert_eq!(mock.frees, [3, 2, 1]);
    assert!(mock.closed);
}

#[test]
fn worker_mismatch_corruption_unwritten_output_and_cleanup_failure_reject() {
    for mutation in 1..=9 {
        let mut mock = Mock::new(mutation);
        assert!(run(&mut mock).is_err(), "mutation {mutation}");
        assert!(mock.dispatches <= 1);
        if mutation <= 3 {
            assert_eq!(mock.dispatches, 0);
        }
    }
}

#[test]
fn artifact_identity_and_input_extents_fail_before_any_commands() {
    let mut mock = Mock::new(0);
    assert!(probe::run(
        &mut mock,
        b"other object".to_vec(),
        &metadata(),
        &vec![0; BYTES[0]],
        &vec![0; BYTES[1]]
    )
    .is_err());
    assert!(probe::run(
        &mut mock,
        b"fixture object".to_vec(),
        &metadata(),
        &[],
        &vec![0; BYTES[1]]
    )
    .is_err());
    assert!(mock.buffers.is_empty());
    assert_eq!(mock.next_buffer, 1);
}

#[test]
fn exact_kernel_resources_and_every_argument_field_are_checked() {
    artifact::validate_abi(&metadata()).unwrap();
    for mutation in 0..8 {
        let mut value = metadata();
        match mutation {
            0 => value.symbol.push('x'),
            1 => value.kernarg_alignment = 16,
            2 => value.group_segment_bytes = 4,
            3 => value.private_segment_bytes = 4,
            4 => value.wavefront_size = 32,
            5 => value.kernarg_bytes += 1,
            6 => value.implicit_argument_offset = Some(56),
            _ => {
                value.explicit_arguments.pop();
            }
        }
        assert!(artifact::validate_abi(&value).is_err());
    }
    for index in 0..6 {
        for mutation in 0..5 {
            let mut value = metadata();
            let argument = &mut value.explicit_arguments[index];
            match mutation {
                0 => argument.offset += 1,
                1 => argument.bytes = 4,
                2 => argument.global_buffer = !argument.global_buffer,
                3 => argument.pointee_alignment = Some(8),
                _ => argument.access = Some(BufferAccessV1::ReadWrite),
            }
            assert!(
                artifact::validate_abi(&value).is_err(),
                "argument {index}, mutation {mutation}"
            );
        }
    }
}

#[test]
fn optional_native_qualifiers_remain_absent_and_conflicting_values_reject() {
    let mut native = metadata();
    native.kernarg_bytes = 48;
    native.implicit_argument_offset = None;
    native.implicit_argument_bytes = 0;
    for argument in &mut native.explicit_arguments {
        argument.pointee_alignment = None;
        argument.access = None;
    }
    artifact::validate_abi(&native).unwrap();
    assert_eq!(artifact::kernarg(&native).unwrap().len(), 48);
    let serialized = serde_json::to_value(&native).unwrap();
    assert!(serialized["explicit_arguments"][0]["access"].is_null());
    assert!(serialized["explicit_arguments"][4]["pointee_alignment"].is_null());
    for index in [0, 2, 4] {
        let mut conflicting = native.clone();
        conflicting.explicit_arguments[index].access = Some(if index == 4 {
            BufferAccessV1::Read
        } else {
            BufferAccessV1::Write
        });
        assert!(artifact::validate_abi(&conflicting).is_err());
        conflicting = native.clone();
        conflicting.explicit_arguments[index].pointee_alignment = Some(8);
        assert!(artifact::validate_abi(&conflicting).is_err());
    }
}

#[test]
fn optional_max_workgroups_do_not_replace_fixed_launch_constraints() {
    for max_groups in [
        [None; 3],
        [Some(2), None, Some(1)],
        [Some(2), Some(1), Some(1)],
    ] {
        artifact::validate_launch(Some([128, 1, 1]), 128, max_groups, None).unwrap();
    }
    for dimension in 0..3 {
        let mut contradictory = [Some(2), Some(1), Some(1)];
        contradictory[dimension] = Some(3);
        assert!(artifact::validate_launch(Some([128, 1, 1]), 128, contradictory, None).is_err());
    }
    assert!(artifact::validate_launch(None, 128, [None; 3], None).is_err());
    assert!(artifact::validate_launch(Some([64, 1, 1]), 128, [None; 3], None).is_err());
    assert!(artifact::validate_launch(Some([128, 1, 1]), 256, [None; 3], None).is_err());
    assert!(artifact::validate_launch(Some([128, 1, 1]), 128, [None; 3], Some([1; 3])).is_err());
    let declared = serde_json::to_value(artifact::declared_abi()).unwrap();
    assert_eq!(declared["max_workgroups"], serde_json::json!([2, 1, 1]));
    assert_eq!(declared["grid_work_items"], serde_json::json!([256, 1, 1]));
}

#[test]
fn optional_actual_access_and_source_qualifiers_reject_contradictions() {
    for index in 0..6 {
        let absent = || artifact::ObservedArgumentQualifiers {
            actual_access: None,
            is_const: None,
            is_restrict: None,
            is_volatile: None,
            is_pipe: None,
        };
        absent().validate(index).unwrap();
        let pointer = index % 2 == 0;
        for mutation in 0..5 {
            let mut observed = absent();
            match mutation {
                0 => observed.actual_access = Some(BufferAccessV1::ReadWrite),
                1 => observed.is_const = Some(!pointer || index == 4),
                2 => observed.is_restrict = Some(!pointer || index != 4),
                3 => observed.is_volatile = Some(true),
                _ => observed.is_pipe = Some(true),
            }
            assert!(
                observed.validate(index).is_err(),
                "argument {index} mutation {mutation}"
            );
        }
        let mut present = absent();
        present.actual_access = pointer.then_some(if index == 4 {
            BufferAccessV1::Write
        } else {
            BufferAccessV1::Read
        });
        present.is_const = Some(pointer && index != 4);
        present.is_restrict = Some(pointer && index == 4);
        present.validate(index).unwrap();
    }
}

#[test]
fn kernarg_has_only_lengths_with_zero_pointer_and_implicit_slots() {
    let bytes = artifact::kernarg(&metadata()).unwrap();
    for index in 0..3 {
        assert_eq!(&bytes[index * 16..index * 16 + 8], &[0; 8]);
        assert_eq!(
            &bytes[index * 16 + 8..index * 16 + 16],
            &artifact::LENGTHS[index].to_le_bytes()
        );
    }
    assert!(bytes[48..].iter().all(|byte| *byte == 0));
}

#[test]
fn ready_rejects_every_identity_authority_and_protocol_mutation() {
    for mutation in 0..6 {
        let packet = Packet {
            response: ResponseV1::Ready {
                protocol: if mutation == 1 { 2 } else { 1 },
                target: if mutation == 2 {
                    "gfx942"
                } else {
                    "gfx950:xnack-"
                }
                .into(),
                device_unique_id: if mutation == 3 { 8 } else { 7 },
                authority: if mutation == 4 { "production" } else { "none" }.into(),
            },
            payload: if mutation == 5 { vec![1] } else { vec![] },
        };
        assert_eq!(probe::ready(&packet, 7).is_ok(), mutation == 0);
    }
}

#[test]
fn framed_transport_bounds_payload_and_redacts_worker_error() {
    let mut frame = vec![];
    write_header_v1(&mut frame, &ResponseV1::Read { payload_bytes: 3 }).unwrap();
    frame.extend_from_slice(&[1, 2, 3]);
    assert_eq!(
        crate::session::receive(&mut Cursor::new(frame))
            .unwrap()
            .payload,
        [1, 2, 3]
    );
    for response in [
        ResponseV1::Read {
            payload_bytes: MAX_TRANSFER_BYTES_V1 + 1,
        },
        ResponseV1::Read { payload_bytes: 3 },
        ResponseV1::Error {
            message: "SYNTHETIC_PRIVATE_IDENTIFIER".into(),
            fatal: true,
        },
    ] {
        let mut frame = vec![];
        write_header_v1(&mut frame, &response).unwrap();
        let error = crate::session::receive(&mut Cursor::new(frame))
            .err()
            .unwrap();
        assert!(!error.contains("SYNTHETIC_PRIVATE_IDENTIFIER"));
    }
    assert!(crate::session::receive(&mut Cursor::new(vec![1, 0])).is_err());
}

#[test]
fn cli_requires_acknowledgement_and_rejects_unknown_duplicate_options() {
    let options = [
        "run",
        "--object",
        "a",
        "--source-file",
        "b",
        "--metadata",
        "c",
        "--worker",
        "d",
        "--inputs",
        "e",
        "--weights",
        "f",
        "--device-id-file",
        "g",
        "--run-dir",
        "h",
    ];
    assert!(crate::Options::parse(options.into_iter().map(str::to_owned)).is_err());
    let mut acknowledged = options.to_vec();
    acknowledged.push("--allow-unauthenticated-machine-code");
    assert!(crate::Options::parse(acknowledged.iter().map(|value| (*value).to_owned())).is_ok());
    for flag in [
        "--unknown",
        "--object",
        "--allow-unauthenticated-machine-code",
    ] {
        let mut changed = acknowledged.clone();
        changed.push(flag);
        assert!(crate::Options::parse(changed.into_iter().map(str::to_owned)).is_err());
    }
}
