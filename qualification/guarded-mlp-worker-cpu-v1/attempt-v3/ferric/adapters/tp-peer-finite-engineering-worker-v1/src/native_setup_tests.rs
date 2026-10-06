//! Metadata/mock tests only; no authentic model upload, device opening or dispatch.
use super::*;

fn scope(registration: &source::Registration) -> wire::Scope {
    wire::Scope {
        bundle_id: registration.bundle_id,
        model_id: registration.model_id,
        session: registration.session,
        pool_identity: registration.pool_identity,
        group_id: registration.group_id,
        child_identity: registration.child_identity,
    }
}

fn manifest(registration: &source::Registration) -> wire::UploadManifest {
    let mut uploads = Vec::new();
    for b in registration
        .layers
        .iter()
        .flat_map(|l| l.weights.iter().map(|w| w.buffer))
        .chain(registration.globals.iter().map(|g| g.buffer))
    {
        uploads.push(wire::Upload {
            key: wire::Key::Source {
                rank: b.rank,
                id: b.id,
            },
            bytes: b.elements * u64::from(b.element_bytes),
            sha256: [9; 32],
        });
    }
    for p in &registration.pending_buffers {
        if matches!(
            p.kind,
            source::PendingKind::PackedQkvWeight | source::PendingKind::PackedHeadNormWeight
        ) {
            uploads.push(wire::Upload {
                key: wire::Key::Pending {
                    rank: p.rank,
                    layer: p.layer,
                    role: p.kind,
                },
                bytes: p.elements * u64::from(p.element_bytes),
                sha256: [10; 32],
            });
        }
    }
    let head = registration
        .globals
        .iter()
        .find(|g| g.kind == source::GlobalKind::LanguageModelHead)
        .unwrap()
        .buffer;
    wire::UploadManifest {
        version: 1,
        uploads,
        tail: Some(wire::TailHeadTranspose {
            source: wire::Key::Source {
                rank: 0,
                id: head.id,
            },
            source_sha256: [9; 32],
            bytes: HEAD_BYTES,
            sha256: [11; 32],
        }),
    }
}

#[test]
fn complete_manifest_binds_939_uploads_and_196_zeroed_roles_before_open() {
    let registration = registration();
    let rows = plan(&registration, &manifest(&registration)).unwrap();
    assert_eq!(rows.len(), 1135);
    assert_eq!(rows.iter().filter(|r| r.digest.is_some()).count(), 939);
    assert_eq!(rows.iter().filter(|r| r.digest.is_none()).count(), 196);
    assert_eq!(rows.iter().filter(|r| r.key.rank() == 0).count(), 569);
    assert_eq!(rows.iter().filter(|r| r.key.rank() == 1).count(), 566);
    assert_eq!(
        rows.iter()
            .filter(|r| r.bytes == 0 && r.digest.is_none())
            .count(),
        2
    );
}

#[test]
fn missing_extra_duplicate_mutable_or_wrong_extent_uploads_are_rejected() {
    let registration = registration();
    for mutation in 0..7 {
        let mut value = manifest(&registration);
        match mutation {
            0 => {
                value.uploads.pop();
            }
            1 => value.uploads.push(value.uploads[0].clone()),
            2 => value.uploads[1] = value.uploads[0].clone(),
            3 => {
                value.uploads[0].key = wire::Key::Source {
                    rank: 0,
                    id: 999_999,
                }
            }
            4 => value.uploads[0].bytes += 2,
            5 => value.uploads[0].sha256 = [0; 32],
            6 => {
                let b = registration.layers[0].caches[0];
                value.uploads[0].key = wire::Key::Source {
                    rank: b.rank,
                    id: b.id,
                };
            }
            _ => unreachable!(),
        }
        assert!(plan(&registration, &value).is_err(), "mutation {mutation}");
    }
}

#[test]
fn tail_cannot_be_substituted_by_a_source_alias_or_wrong_shape_digest() {
    let registration = registration();
    for mutation in 0..5 {
        let mut value = manifest(&registration);
        let tail = value.tail.as_mut().unwrap();
        match mutation {
            0 => tail.source = value.uploads[0].key,
            1 => tail.bytes -= 2,
            2 => tail.source_sha256 = [8; 32],
            3 => tail.sha256 = [0; 32],
            4 => tail.source = wire::Key::Source { rank: 1, id: 1 },
            _ => unreachable!(),
        }
        assert!(plan(&registration, &value).is_err());
    }
    let mut layer_only = manifest(&registration);
    layer_only.tail = None;
    assert!(plan(&registration, &layer_only).is_ok());
}

#[test]
fn actual_child_pid_and_bootstrap_scope_must_match_not_merely_be_nonzero() {
    let registration = registration();
    let expected = scope(&registration);
    let part = wire::Part {
        bytes: 1,
        sha256: [1; 32],
    };
    let begin = wire::Begin {
        scope: expected.clone(),
        registration: part,
        source_program: part,
        uploads: part,
        prefix_image: part,
        mlp_image: part,
        residual_image: part,
        tail_image: None,
    };
    validate_scope(&begin, &registration, &expected, expected.child_identity).unwrap();
    assert!(
        validate_scope(
            &begin,
            &registration,
            &expected,
            expected.child_identity + 1
        )
        .is_err()
    );
    for mutation in 0..6 {
        let mut changed = expected.clone();
        match mutation {
            0 => changed.bundle_id[0] ^= 1,
            1 => changed.model_id[0] ^= 1,
            2 => changed.session[0] ^= 1,
            3 => changed.pool_identity += 1,
            4 => changed.group_id += 1,
            5 => changed.child_identity += 1,
            _ => unreachable!(),
        }
        assert!(validate_scope(&begin, &registration, &changed, changed.child_identity).is_err());
    }
}

fn fake_begin() -> (wire::Request, Vec<u8>, wire::Scope) {
    let mut registration = registration();
    registration.child_identity = std::process::id();
    let scope = scope(&registration);
    let data = [
        serde_json::to_vec(&registration).unwrap(),
        vec![0; 123],
        serde_json::to_vec(&manifest(&registration)).unwrap(),
        vec![1],
        vec![2],
        vec![3],
        vec![4],
    ];
    let parts: Vec<_> = data
        .iter()
        .map(|b| wire::Part {
            bytes: b.len() as u32,
            sha256: hash(b),
        })
        .collect();
    let begin = wire::Begin {
        scope: scope.clone(),
        registration: parts[0],
        source_program: parts[1],
        uploads: parts[2],
        prefix_image: parts[3],
        mlp_image: parts[4],
        residual_image: parts[5],
        tail_image: Some(parts[6]),
    };
    (
        request(1, wire::Command::Begin(begin)),
        data.concat(),
        scope,
    )
}

#[test]
fn prepared_intake_rejects_payload_tamper_scope_and_device_before_any_owner_exists() {
    let (request, payload, scope) = fake_begin();
    let mut changed = payload.clone();
    changed[0] ^= 1;
    assert!(
        PreparedSetup::prepare(request.clone(), changed, [7, 9], &scope)
            .err()
            .unwrap()
            .contains("part digest")
    );
    assert!(
        PreparedSetup::prepare(request.clone(), payload.clone(), [9, 7], &scope)
            .err()
            .unwrap()
            .contains("envelope")
    );
    let mut wrong_scope = scope.clone();
    wrong_scope.child_identity += 1;
    assert!(
        PreparedSetup::prepare(request.clone(), payload.clone(), [7, 9], &wrong_scope)
            .err()
            .unwrap()
            .contains("bootstrap/process")
    );
    // Valid metadata and exact same-byte hashes do not make fabricated images
    // reviewed. This reaches the independent retained-image intake and refuses.
    assert!(PreparedSetup::prepare(request, payload, [7, 9], &scope).is_err());
}

fn key(id: u64) -> wire::Key {
    wire::Key::Source { rank: 0, id }
}
fn tiny_plan() -> Vec<Planned> {
    vec![
        Planned {
            key: key(71),
            bytes: 2,
            digest: Some(hash(&[1, 2])),
        },
        Planned {
            key: key(72),
            bytes: 2,
            digest: None,
        },
    ]
}
fn tiny_tail() -> wire::TailHeadTranspose {
    wire::TailHeadTranspose {
        source: key(71),
        source_sha256: hash(&[1, 2]),
        bytes: 2,
        sha256: hash(&[3, 4]),
    }
}

#[derive(Default)]
struct Mock {
    events: Vec<&'static str>,
    fail: Option<&'static str>,
    allocations: Vec<(u64, Planned)>,
    uploaded: bool,
    tail_allocated: bool,
    tail_uploaded: bool,
    tail_requested: bool,
}
impl Mock {
    fn call(&mut self, event: &'static str) -> Result<()> {
        self.events.push(event);
        require(self.fail != Some(event), "mock native failure")
    }
}
impl Backend for Mock {
    fn allocate(&mut self, row: &Planned) -> Result<u64> {
        self.call("allocate")?;
        let id = self.allocations.len() as u64 + 101;
        self.allocations.push((id, row.clone()));
        Ok(id)
    }
    fn write(&mut self, id: u64, offset: u64, bytes: &[u8]) -> Result<()> {
        self.call("write")?;
        let row = &self
            .allocations
            .iter()
            .find(|(old, _)| *old == id)
            .ok_or("foreign native ID")?
            .1;
        require(
            !self.uploaded
                && offset == 0
                && bytes.len() as u64 == row.bytes
                && row.digest == Some(hash(bytes)),
            "mock inherited complete upload check",
        )?;
        self.uploaded = true;
        Ok(())
    }
    fn allocate_tail(&mut self) -> Result<()> {
        self.call("allocate_tail")?;
        require(self.uploaded, "original head not initialized")?;
        self.tail_allocated = true;
        Ok(())
    }
    fn write_tail(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        self.call("write_tail")?;
        require(
            self.tail_allocated && !self.tail_uploaded && offset == 0 && bytes == [3, 4],
            "mock inherited tail hash check",
        )?;
        self.tail_uploaded = true;
        Ok(())
    }
    fn load(&mut self) -> Result<()> {
        self.call("load")
    }
    fn bind_seal(&mut self) -> Result<()> {
        self.call("bind_seal")?;
        require(
            self.uploaded && (!self.tail_requested || self.tail_uploaded),
            "mock inherited incomplete-upload refusal",
        )
    }
    fn close(&mut self) -> Result<()> {
        self.call("close")
    }
}
fn tiny(tail: bool) -> Core<Mock> {
    Core::new(
        Mock {
            tail_requested: tail,
            ..Mock::default()
        },
        [7, 9],
        [3; 32],
        tiny_plan(),
        tail.then(tiny_tail),
    )
}
fn request(id: u64, command: wire::Command) -> wire::Request {
    wire::Request {
        protocol: wire::PROTOCOL,
        id,
        device_ids: [7, 9],
        session: [3; 32],
        command,
    }
}
fn write_command(bytes: &[u8]) -> wire::Command {
    wire::Command::Write {
        key: key(71),
        offset: 0,
        part: wire::Part {
            bytes: bytes.len() as u32,
            sha256: hash(bytes),
        },
    }
}
fn sequence(tail: bool) -> Vec<(wire::Command, Vec<u8>)> {
    let mut result = vec![
        (wire::Command::Allocate { key: key(71) }, vec![]),
        (write_command(&[1, 2]), vec![1, 2]),
        (wire::Command::Allocate { key: key(72) }, vec![]),
    ];
    if tail {
        result.extend([
            (wire::Command::AllocateTailHead, vec![]),
            (
                wire::Command::WriteTailHead {
                    offset: 0,
                    part: wire::Part {
                        bytes: 2,
                        sha256: hash(&[3, 4]),
                    },
                },
                vec![3, 4],
            ),
        ]);
    }
    result.extend([
        (wire::Command::LoadArtifacts, vec![]),
        (wire::Command::BindAndSeal, vec![]),
    ]);
    result
}
fn frames(tail: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (index, (command, payload)) in sequence(tail).into_iter().enumerate() {
        wire::write_request(&mut bytes, &request(index as u64 + 2, command), &payload).unwrap();
    }
    bytes
}

#[test]
fn actual_backend_ids_replace_source_ids_and_tail_seals_without_ready() {
    let mut core = tiny(true);
    let mut replies = Vec::new();
    core.serve(&mut frames(true).as_slice(), &mut replies)
        .unwrap();
    assert!(core.forward_extractable());
    assert_eq!(core.allocated, vec![(key(71), 101), (key(72), 102)]);
    assert_eq!(
        core.backend.events,
        [
            "allocate",
            "write",
            "allocate",
            "allocate_tail",
            "write_tail",
            "load",
            "bind_seal"
        ]
    );
    let mut reader = replies.as_slice();
    let first = wire::read_response(&mut reader).unwrap().unwrap();
    assert_eq!(first.status, wire::Status::OwnerCreated);
    let mut last = first;
    while let Some(response) = wire::read_response(&mut reader).unwrap() {
        assert!(
            !response.gpu_execution && !response.forward_started && !response.production_authority
        );
        last = response;
    }
    assert_eq!(last.status, wire::Status::LayersAndTailSealed);
    let mut layer_only = tiny(false);
    layer_only
        .serve(&mut frames(false).as_slice(), &mut Vec::new())
        .unwrap();
    assert_eq!(layer_only.phase, Phase::Sealed);
    assert!(!layer_only.forward_extractable());
}

#[test]
fn every_native_failure_is_terminal_without_close_or_retry() {
    for event in [
        "allocate",
        "write",
        "allocate_tail",
        "write_tail",
        "load",
        "bind_seal",
    ] {
        let mut core = tiny(true);
        core.backend.fail = Some(event);
        assert!(
            core.serve(&mut frames(true).as_slice(), &mut Vec::new())
                .is_err(),
            "{event}"
        );
        assert_eq!(core.phase, Phase::Terminal);
        assert!(!core.forward_extractable());
        let events = core.backend.events.clone();
        assert!(
            core.handle(request(core.next, wire::Command::AbortClose), &[])
                .is_err()
        );
        assert_eq!(core.backend.events, events);
        assert!(!events.contains(&"close"));
    }
}

#[test]
fn sequence_scope_unknown_key_duplicate_and_chunk_hash_fail_before_side_effects() {
    for mutation in 0..5 {
        let mut core = tiny(false);
        let mut r = request(2, wire::Command::Allocate { key: key(71) });
        match mutation {
            0 => r.id = 3,
            1 => r.session[0] ^= 1,
            2 => r.device_ids.swap(0, 1),
            3 => r.command = wire::Command::Allocate { key: key(999) },
            4 => r.protocol += 1,
            _ => unreachable!(),
        }
        assert!(core.handle(r, &[]).is_err());
        assert!(core.backend.events.is_empty());
    }
    let mut core = tiny(false);
    core.handle(request(2, wire::Command::Allocate { key: key(71) }), &[])
        .unwrap();
    assert!(
        core.handle(request(3, wire::Command::Allocate { key: key(71) }), &[])
            .is_err()
    );
    assert_eq!(core.backend.events, ["allocate"]);
    let mut core = tiny(false);
    core.handle(request(2, wire::Command::Allocate { key: key(71) }), &[])
        .unwrap();
    assert!(
        core.handle(request(3, write_command(&[1, 2])), &[1, 3])
            .is_err()
    );
    assert_eq!(core.backend.events, ["allocate"]);
}

#[test]
fn missing_upload_tail_or_images_never_seals_and_original_head_is_required() {
    let mut core = tiny(true);
    assert!(
        core.handle(request(2, wire::Command::AllocateTailHead), &[])
            .is_err()
    );
    assert_eq!(core.backend.events, ["allocate_tail"]);
    let mut core = tiny(false);
    assert!(
        core.handle(request(2, wire::Command::AllocateTailHead), &[])
            .is_err()
    );
    assert!(core.backend.events.is_empty());
    let mut core = tiny(true);
    for id in [71, 72] {
        core.handle(
            request(core.next, wire::Command::Allocate { key: key(id) }),
            &[],
        )
        .unwrap();
    }
    core.handle(request(core.next, wire::Command::LoadArtifacts), &[])
        .unwrap();
    assert!(
        core.handle(request(core.next, wire::Command::BindAndSeal), &[])
            .is_err()
    );
    assert!(!core.backend.events.contains(&"bind_seal"));
    let mut core = tiny(false);
    for id in [71, 72] {
        core.handle(
            request(core.next, wire::Command::Allocate { key: key(id) }),
            &[],
        )
        .unwrap();
    }
    core.handle(request(core.next, wire::Command::LoadArtifacts), &[])
        .unwrap();
    assert!(
        core.handle(request(core.next, wire::Command::BindAndSeal), &[])
            .is_err()
    );
    assert_eq!(core.backend.events.last(), Some(&"bind_seal"));
    assert!(!core.forward_extractable());
}

struct FailAcknowledgement(&'static [u8]);
impl Write for FailAcknowledgement {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.windows(self.0.len()).any(|v| v == self.0) {
            Err(std::io::Error::other("closed parent response stream"))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn failed_seal_ack_or_truncated_input_cannot_publish_the_owner() {
    let mut core = tiny(true);
    assert!(
        core.serve(
            &mut frames(true).as_slice(),
            &mut FailAcknowledgement(b"layers_and_tail_sealed")
        )
        .is_err()
    );
    assert!(core.backend.events.contains(&"bind_seal"));
    assert_eq!(core.phase, Phase::Terminal);
    assert!(!core.forward_extractable());
    let mut core = tiny(true);
    assert!(core.serve(&mut &[0_u8, 1][..], &mut Vec::new()).is_err());
    assert!(core.backend.events.is_empty());
    assert!(!core.forward_extractable());
}

#[test]
fn explicit_healthy_abort_closes_once_failed_close_is_terminal() {
    let mut bytes = Vec::new();
    wire::write_request(&mut bytes, &request(2, wire::Command::AbortClose), &[]).unwrap();
    for fail in [false, true] {
        let mut core = tiny(false);
        if fail {
            core.backend.fail = Some("close");
        }
        let result = core.serve(&mut bytes.as_slice(), &mut Vec::new());
        assert_eq!(result.is_err(), fail);
        assert_eq!(
            core.phase,
            if fail { Phase::Terminal } else { Phase::Closed }
        );
        assert!(
            core.handle(request(core.next, wire::Command::AbortClose), &[])
                .is_err()
        );
        assert_eq!(core.backend.events, ["close"]);
        assert!(!core.forward_extractable());
    }
    let mut core = tiny(false);
    assert!(
        core.serve(
            &mut bytes.as_slice(),
            &mut FailAcknowledgement(b"\"closed\"")
        )
        .is_err()
    );
    assert_eq!(core.backend.events, ["close"]);
    assert_eq!(core.phase, Phase::Terminal);
}

fn registration() -> source::Registration {
    let mut layers = Vec::new();
    let mut scratch = Vec::new();
    let mut globals = Vec::new();
    let mut auxiliary = Vec::new();
    let mut pending_buffers = Vec::new();
    for rank in 0..2 {
        let mut next_id = 1;
        let mut buffer = |elements, element_bytes| {
            let value = source::Buffer {
                rank,
                id: next_id,
                elements,
                element_bytes,
            };
            next_id += 1;
            value
        };
        for layer in 0..36 {
            let weights = source::WEIGHTS
                .into_iter()
                .map(|(kind, elements)| source::Weight {
                    kind,
                    buffer: buffer(elements, 2),
                })
                .collect();
            layers.push(source::Layer {
                rank,
                layer,
                weights,
                caches: [buffer(2304 * 512, 2), buffer(2304 * 512, 2)],
            });
            for (kind, elements) in [
                (source::PendingKind::PackedQkvWeight, 3072 * 4096),
                (source::PendingKind::PackedHeadNormWeight, 256),
            ] {
                pending_buffers.push(source::PendingBuffer {
                    rank,
                    layer: Some(layer),
                    kind,
                    elements,
                    element_bytes: 2,
                });
            }
        }
        for (kind, elements, element_bytes) in source::SCRATCH {
            scratch.push(source::Scratch {
                kind,
                buffer: buffer(elements, element_bytes),
            });
        }
        if rank == 0 {
            for (kind, elements) in source::GLOBALS {
                globals.push(source::Global {
                    kind,
                    buffer: buffer(elements, 2),
                });
            }
        }
        for (kind, elements, element_bytes) in source::auxiliary_roster(rank) {
            auxiliary.push(source::Auxiliary {
                kind,
                buffer: buffer(elements, element_bytes),
            });
        }
        for (kind, elements, element_bytes) in [
            (source::PendingKind::QkvOutput, 3072, 2),
            (source::PendingKind::Rotary, 128, 4),
            (source::PendingKind::CacheMetadata, 145, 4),
        ] {
            pending_buffers.push(source::PendingBuffer {
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
                (source::StateKind::PrefixV5, 22),
                (source::StateKind::MlpV1, 11),
            ] {
                for rank in 0..2 {
                    state_slots.push(source::StateSlot {
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
    source::Registration {
        profile: source::PROFILE.into(),
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
        source_program_sha256: Sha256::digest([0_u8; 123]).into(),
    }
}
