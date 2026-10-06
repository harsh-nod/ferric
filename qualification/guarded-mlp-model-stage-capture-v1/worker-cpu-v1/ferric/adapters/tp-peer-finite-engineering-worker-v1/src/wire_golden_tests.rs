//! These literal goldens run independently in the parent and current-KFD child.
use super::finite_composition_wire as wire;
use sha2::{Digest, Sha256};
use std::io::Cursor;

fn request(operation: wire::Operation) -> wire::Request {
    wire::Request {
        protocol: 1,
        profile: wire::PROFILE.into(),
        device_ids: [11, 22],
        request: operation,
    }
}

fn allocate() -> wire::Request {
    request(wire::Operation::Setup {
        id: 1,
        rank: 0,
        command: wire::Setup::Allocate {
            bytes: 8192,
            peer_readable: false,
        },
    })
}

fn execute(position: u32) -> wire::Request {
    request(wire::Operation::Execute {
        id: 2,
        registration_sha256: [1; 32],
        generation: u64::from(position) + 1,
        token: 785,
        position,
        cache_metadata: std::iter::once(position).chain((0..144).rev()).collect(),
        rotary_bits: (0..128)
            .map(|index| {
                if index < 64 {
                    1.0_f32.to_bits()
                } else {
                    0x8000_0000
                }
            })
            .collect(),
    })
}

fn framed(bytes: &[u8]) -> Vec<u8> {
    let mut result = (bytes.len() as u32).to_le_bytes().to_vec();
    result.extend_from_slice(bytes);
    result
}

fn registration() -> wire::Registration {
    let mut layers = Vec::new();
    let mut scratch = Vec::new();
    let mut globals = Vec::new();
    let mut auxiliary = Vec::new();
    let mut pending_buffers = Vec::new();
    for rank in 0..2 {
        let mut next_id = 1;
        let mut buffer = |elements, element_bytes| {
            let value = wire::Buffer {
                rank,
                id: next_id,
                elements,
                element_bytes,
            };
            next_id += 1;
            value
        };
        for layer in 0..36 {
            let weights = wire::WEIGHTS
                .into_iter()
                .map(|(kind, elements)| wire::Weight {
                    kind,
                    buffer: buffer(elements, 2),
                })
                .collect();
            layers.push(wire::Layer {
                rank,
                layer,
                weights,
                caches: [buffer(2304 * 512, 2), buffer(2304 * 512, 2)],
            });
            for (kind, elements) in [
                (wire::PendingKind::PackedQkvWeight, 3072 * 4096),
                (wire::PendingKind::PackedHeadNormWeight, 256),
            ] {
                pending_buffers.push(wire::PendingBuffer {
                    rank,
                    layer: Some(layer),
                    kind,
                    elements,
                    element_bytes: 2,
                });
            }
        }
        for (kind, elements, element_bytes) in wire::SCRATCH {
            scratch.push(wire::Scratch {
                kind,
                buffer: buffer(elements, element_bytes),
            });
        }
        if rank == 0 {
            for (kind, elements) in wire::GLOBALS {
                globals.push(wire::Global {
                    kind,
                    buffer: buffer(elements, 2),
                });
            }
        }
        for (kind, elements, element_bytes) in wire::auxiliary_roster(rank) {
            auxiliary.push(wire::Auxiliary {
                kind,
                buffer: buffer(elements, element_bytes),
            });
        }
        for (kind, elements, element_bytes) in [
            (wire::PendingKind::QkvOutput, 3072, 2),
            (wire::PendingKind::Rotary, 128, 4),
            (wire::PendingKind::CacheMetadata, 145, 4),
        ] {
            pending_buffers.push(wire::PendingBuffer {
                rank,
                layer: None,
                kind,
                elements,
                element_bytes,
            });
        }
    }
    let mut state_slots = Vec::new();
    for forward in 0..2 {
        for layer in 0..36 {
            for (kind, atomic_words) in [
                (wire::StateKind::PrefixV5, 22),
                (wire::StateKind::MlpV1, 11),
            ] {
                for rank in 0..2 {
                    state_slots.push(wire::StateSlot {
                        forward,
                        layer,
                        rank,
                        kind,
                        atomic_words,
                    });
                }
            }
        }
    }
    wire::Registration {
        profile: wire::PROFILE.into(),
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: 4,
        group_id: 0,
        child_identity: 6,
        layers,
        globals,
        auxiliary,
        scratch,
        pending_buffers,
        state_slots,
        source_program_bytes: 123,
        source_program_sha256: [7; 32],
    }
}

#[test]
fn identical_parent_child_request_and_refusal_literal_goldens() {
    let request = allocate();
    let golden = br#"{"protocol":1,"profile":"qwen3-8b-tp2-finite-prefix-v5-mlp-v1-two-forward-context2304-v1","device_ids":[11,22],"request":{"op":"setup","id":1,"rank":0,"command":{"op":"allocate","bytes":8192,"peer_readable":false}}}"#;
    let mut encoded = Vec::new();
    wire::write_request(&mut encoded, &request, &[]).unwrap();
    assert_eq!(encoded, framed(golden));
    assert_eq!(
        wire::read_request(&mut Cursor::new(encoded)).unwrap(),
        Some((request.clone(), vec![]))
    );
    let refusal = wire::refuse_unbound_request(&request, &[]).unwrap();
    let mut encoded = Vec::new();
    wire::write_response(&mut encoded, &refusal).unwrap();
    assert_eq!(encoded, framed(br#"{"protocol":1,"id":1,"reason":"native_binding_missing","native_opened":false,"gpu_execution":false,"production_authority":false}"#));
    assert_eq!(
        wire::read_response(&mut Cursor::new(encoded)).unwrap(),
        Some(refusal)
    );
}

#[test]
fn registration_shape_is_inert_and_refused_after_digest_validation() {
    let value = registration();
    value.validate().unwrap();
    assert_eq!(value.group_id, 0); // Valid existing collective group, not a minted state ID.
    let payload = serde_json::to_vec(&value).unwrap();
    let request = request(wire::Operation::Register {
        id: 3,
        bytes: payload.len() as u32,
        sha256: Sha256::digest(&payload).into(),
    });
    let response = wire::refuse_unbound_request(&request, &payload).unwrap();
    assert!(!response.native_opened && !response.gpu_execution && !response.production_authority);
    let mut encoded = Vec::new();
    wire::write_request(&mut encoded, &request, &payload).unwrap();
    assert_eq!(
        wire::read_request(&mut Cursor::new(encoded)).unwrap(),
        Some((request.clone(), payload.clone()))
    );
    let mut bad = payload;
    bad[0] ^= 1;
    assert!(wire::refuse_unbound_request(&request, &bad).is_err());
}

#[test]
fn logical_state_roster_is_tuple_bound_not_positional_native_identity() {
    let mut value = registration();
    value
        .state_slots
        .sort_by_key(|slot| (slot.forward, slot.layer, slot.rank, slot.kind));
    value.validate().unwrap();
    value.state_slots[1] = value.state_slots[0];
    assert!(value.validate().is_err());
    let mut value = registration();
    value.state_slots[0].atomic_words = 11;
    assert!(value.validate().is_err());
    let mut value = registration();
    value.state_slots[0].forward = 2;
    assert!(value.validate().is_err());
}

#[test]
fn original_weight_and_scratch_roles_reject_alias_shape_rank_or_packing_mutation() {
    let mut cases = Vec::new();
    let mut value = registration();
    value.layers.swap(0, 1);
    cases.push(value);
    let mut value = registration();
    value.layers[0].weights[1].buffer.elements -= 1;
    cases.push(value);
    let mut value = registration();
    value.layers[0].weights[0].buffer.rank = 1;
    cases.push(value);
    let mut value = registration();
    value.layers[0].caches[1] = value.layers[0].caches[0];
    cases.push(value);
    let mut value = registration();
    value.scratch[0].buffer.id = value.layers[0].weights[0].buffer.id;
    cases.push(value);
    let mut value = registration();
    value.scratch[8].buffer.element_bytes = 2;
    cases.push(value);
    let mut value = registration();
    value.pending_buffers[0].layer = None;
    cases.push(value);
    let mut value = registration();
    value.pending_buffers[73].element_bytes = 2;
    cases.push(value);
    let mut value = registration();
    value.state_slots.pop();
    cases.push(value);
    for value in cases {
        assert!(value.validate().is_err());
    }
}

#[test]
fn globals_and_auxiliary_rosters_are_exact_and_never_implicit_tail_authority() {
    let value = registration();
    value.validate().unwrap();
    assert_eq!(value.globals.len(), 3);
    assert_eq!(value.auxiliary.len(), 28);
    assert_eq!(
        value
            .globals
            .iter()
            .map(|g| g.buffer.elements)
            .collect::<Vec<_>>(),
        [622_329_856, 4096, 622_329_856]
    );
    for rank in 0..2 {
        let values = &value.auxiliary[rank * 14..(rank + 1) * 14];
        for (actual, (kind, elements, width)) in
            values.iter().zip(wire::auxiliary_roster(rank as u32))
        {
            assert_eq!(
                (
                    actual.kind,
                    actual.buffer.rank,
                    actual.buffer.elements,
                    actual.buffer.element_bytes
                ),
                (kind, rank as u32, elements, width)
            );
        }
        assert_eq!(values[8].buffer.elements, 0);
        assert_ne!(values[8].buffer.id, 0);
    }
    let mut json = serde_json::to_value(&value).unwrap();
    json.as_object_mut().unwrap().remove("globals");
    assert!(serde_json::from_value::<wire::Registration>(json).is_err());
    let mut json = serde_json::to_value(&value).unwrap();
    json.as_object_mut().unwrap().remove("auxiliary");
    assert!(serde_json::from_value::<wire::Registration>(json).is_err());
}

#[test]
fn global_and_auxiliary_rank_scalar_extent_alias_and_role_mutations_fail_closed() {
    for mutation in 0..14 {
        let mut value = registration();
        match mutation {
            0 => {
                value.globals.pop();
            }
            1 => value.globals.swap(0, 2),
            2 => value.globals[0].buffer.rank = 1,
            3 => value.globals[1].buffer.element_bytes = 4,
            4 => value.globals[2].buffer.elements -= 1,
            5 => value.globals[0].buffer.id = value.layers[0].weights[0].buffer.id,
            6 => {
                value.auxiliary.pop();
            }
            7 => value.auxiliary.swap(6, 7), // Same F32 type, distinct roles.
            8 => value.auxiliary[9].buffer.element_bytes = 2,
            9 => value.auxiliary[10].buffer.elements = 151_936,
            10 => value.auxiliary[24].buffer.elements = 2_430_976,
            11 => value.auxiliary[8].buffer.id = 0,
            12 => value.auxiliary[8].buffer.elements = 1,
            13 => value.auxiliary[12].buffer.id = value.auxiliary[9].buffer.id,
            _ => unreachable!(),
        }
        assert!(value.validate().is_err(), "mutation {mutation}");
    }
}

#[test]
fn complete_source_binding_counts_are_owner_scoped_and_asymmetric_only_at_globals() {
    use std::collections::BTreeSet;
    let value = registration();
    let bindings: Vec<_> = value
        .layers
        .iter()
        .flat_map(|l| l.weights.iter().map(|w| w.buffer).chain(l.caches))
        .chain(value.scratch.iter().map(|s| s.buffer))
        .chain(value.globals.iter().map(|g| g.buffer))
        .chain(value.auxiliary.iter().map(|a| a.buffer))
        .collect();
    assert_eq!(bindings.len(), 985);
    assert_eq!(
        bindings
            .iter()
            .map(|b| (b.rank, b.id))
            .collect::<BTreeSet<_>>()
            .len(),
        985
    );
    assert_eq!(bindings.iter().filter(|b| b.rank == 0).count() + 75, 569);
    assert_eq!(bindings.iter().filter(|b| b.rank == 1).count() + 75, 566);
    assert_eq!(value.pending_buffers.len(), 150);
    // Allocation counts describe a future owner, never caller-minted handles.
    assert_eq!(569 + 144, 713);
    assert_eq!(566 + 144, 710);
}

#[test]
fn every_well_formed_native_route_remains_refused() {
    let bytes = [1, 2, 3, 4];
    let sha256 = Sha256::digest(bytes).into();
    for request in [
        request(wire::Operation::Setup {
            id: 4,
            rank: 1,
            command: wire::Setup::Write {
                buffer: 2,
                offset: 0,
                bytes: 4,
                sha256,
            },
        }),
        request(wire::Operation::Setup {
            id: 5,
            rank: 0,
            command: wire::Setup::LoadImage {
                kind: wire::ImageKind::MlpV1,
                bytes: 4,
                sha256,
            },
        }),
    ] {
        assert_eq!(
            wire::refuse_unbound_request(&request, &bytes)
                .unwrap()
                .reason,
            wire::Refusal::NativeBindingMissing
        );
    }
    for request in [
        allocate(),
        execute(0),
        execute(1),
        request(wire::Operation::Close {
            id: 6,
            registration_sha256: [1; 32],
        }),
    ] {
        assert_eq!(
            wire::refuse_unbound_request(&request, &[]).unwrap().reason,
            wire::Refusal::NativeBindingMissing
        );
    }
}

#[test]
fn execute_checks_two_forward_bounds_full_permutation_and_exact_f32_payload() {
    assert!(execute(2).payload_bytes().is_err());
    for mutate in 0..7 {
        let mut value = execute(0);
        let wire::Operation::Execute {
            generation,
            token,
            cache_metadata,
            rotary_bits,
            ..
        } = &mut value.request
        else {
            unreachable!()
        };
        match mutate {
            0 => *generation = 2,
            1 => *token = 151_936,
            2 => cache_metadata[1] = u32::MAX,
            3 => cache_metadata[2] = cache_metadata[1],
            4 => {
                cache_metadata.pop();
            }
            5 => rotary_bits[0] = 0x7fc0_0001,
            _ => {
                rotary_bits.pop();
            }
        }
        assert!(value.payload_bytes().is_err());
    }
    let value = execute(1);
    let mut bytes = Vec::new();
    wire::write_request(&mut bytes, &value, &[]).unwrap();
    let actual = wire::read_request(&mut Cursor::new(bytes))
        .unwrap()
        .unwrap()
        .0;
    assert_eq!(actual, value); // Includes the exact negative-zero sine bits.
}

#[test]
fn malformed_unknown_duplicate_and_cross_profile_json_are_refused() {
    let value = serde_json::to_value(allocate()).unwrap();
    for field in ["unknown", "native_handle"] {
        let mut value = value.clone();
        value[field] = serde_json::json!(1);
        assert!(
            wire::read_request(&mut Cursor::new(framed(
                &serde_json::to_vec(&value).unwrap()
            )))
            .is_err()
        );
    }
    let duplicate = br#"{"protocol":1,"protocol":1,"profile":"x","device_ids":[11,22],"request":{"op":"close","id":1,"registration_sha256":[]}}"#;
    assert!(wire::read_request(&mut Cursor::new(framed(duplicate))).is_err());
    let mut value = allocate();
    value.profile = "device-peer-tp2-prepared-queued-graph-v1".into();
    assert!(value.payload_bytes().is_err());
    let mut value = allocate();
    value.device_ids[1] = value.device_ids[0];
    assert!(value.payload_bytes().is_err());
    let mut value = allocate();
    value.protocol = 2;
    assert!(value.payload_bytes().is_err());
}

#[test]
fn frame_lengths_are_checked_before_payload_allocation_and_truncation_is_error() {
    assert!(
        wire::read_request(&mut Cursor::new(
            ((wire::MAX_HEADER + 1) as u32).to_le_bytes()
        ))
        .is_err()
    );
    assert!(wire::read_request(&mut Cursor::new([0; 4])).is_err());
    assert!(wire::read_request(&mut Cursor::new([1, 0])).is_err());
    assert!(
        wire::read_request(&mut Cursor::new(Vec::<u8>::new()))
            .unwrap()
            .is_none()
    );
    let oversized = request(wire::Operation::Register {
        id: 1,
        bytes: wire::MAX_REGISTRATION as u32 + 1,
        sha256: [1; 32],
    });
    assert!(
        wire::read_request(&mut Cursor::new(framed(
            &serde_json::to_vec(&oversized).unwrap()
        )))
        .is_err()
    );
    let write = request(wire::Operation::Setup {
        id: 1,
        rank: 0,
        command: wire::Setup::Write {
            buffer: 1,
            offset: u64::MAX,
            bytes: 1,
            sha256: [1; 32],
        },
    });
    assert!(write.payload_bytes().is_err());
    let declared = request(wire::Operation::Register {
        id: 1,
        bytes: 1,
        sha256: [1; 32],
    });
    assert!(
        wire::read_request(&mut Cursor::new(framed(
            &serde_json::to_vec(&declared).unwrap()
        )))
        .is_err()
    );
}

#[test]
fn no_success_or_authority_response_can_be_encoded_or_accepted() {
    let refusal = wire::refuse_unbound_request(&allocate(), &[]).unwrap();
    for field in ["native_opened", "gpu_execution", "production_authority"] {
        let mut value = serde_json::to_value(&refusal).unwrap();
        value[field] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(wire::read_response(&mut Cursor::new(framed(&bytes))).is_err());
        let typed = serde_json::from_slice::<wire::Response>(&bytes).unwrap();
        assert!(wire::write_response(&mut Vec::new(), &typed).is_err());
    }
}
