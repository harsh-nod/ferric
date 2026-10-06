//! Metadata-only backend tests. No model allocation, device opening or dispatch.
use super::*;

#[test]
fn guarded_catalog_retains_distinct_combined_kind_without_mutating_legacy_source() {
    let mut owner = tiny(Mock::new(), false, 4);
    let before = owner.source.clone();
    owner.select_guarded_mlp_decode().unwrap();
    owner.allocate_zeroed(tiny_key()).unwrap();
    owner.bind().unwrap();
    assert_eq!(owner.profile, ExecutionProfile::GuardedMlpDecodeV1);
    assert_eq!(owner.source, before);
    let rows = owner.states.as_ref().unwrap();
    assert_eq!(rows.len(), 288);
    assert_eq!(
        rows.iter()
            .filter(|s| s.kind == WorkerKind::PrefixTilesV6)
            .count(),
        144
    );
    assert_eq!(
        rows.iter()
            .filter(|s| s.kind == WorkerKind::GuardedMlpCombinedV1)
            .count(),
        144
    );
    assert!(
        !rows
            .iter()
            .any(|s| matches!(s.kind, WorkerKind::MlpTilesV2 | WorkerKind::MlpV1))
    );
    for old in [
        WorkerKind::MlpTilesV2,
        WorkerKind::MlpV1,
        WorkerKind::PrefixV5,
    ] {
        assert!(
            ExecutionProfile::GuardedMlpDecodeV1
                .source_kind(old)
                .is_err()
        );
    }
    for old in [
        ExecutionProfile::BaseV1,
        ExecutionProfile::TilesDecodeV1,
        ExecutionProfile::PrefixTilesLayerV6,
        ExecutionProfile::PrefixTilesDecodeV6,
    ] {
        assert!(old.source_kind(WorkerKind::GuardedMlpCombinedV1).is_err());
    }
}
#[test]
fn guarded_catalog_selector_refuses_all_late_or_cross_profile_transitions() {
    for mode in 0..7 {
        let mut owner = tiny(Mock::new(), false, 4);
        match mode {
            0 => owner.select_tiles_decode().unwrap(),
            1 => owner.select_prefix_tiles_layer().unwrap(),
            2 => owner.select_prefix_tiles_decode().unwrap(),
            3 => owner.select_guarded_mlp_decode().unwrap(),
            4 => {
                owner.allocate_zeroed(tiny_key()).unwrap();
            }
            5 => owner.phase = Phase::CatalogBound,
            _ => owner.phase = Phase::Terminal,
        }
        assert!(owner.select_guarded_mlp_decode().is_err());
        assert_eq!(owner.phase, Phase::Terminal);
    }
    for selector in [
        Catalog::<Mock>::select_tiles_decode,
        Catalog::<Mock>::select_prefix_tiles_layer,
        Catalog::<Mock>::select_prefix_tiles_decode,
    ] {
        let mut owner = tiny(Mock::new(), false, 4);
        owner.select_guarded_mlp_decode().unwrap();
        assert!(selector(&mut owner).is_err());
        assert_eq!(owner.phase, Phase::Terminal);
    }
}

#[test]
fn prefix_decode_catalog_is_distinct_and_preserves_original_source_slots() {
    let mut owner = tiny(Mock::new(), false, 4);
    let before = owner.source.clone();
    owner.select_prefix_tiles_decode().unwrap();
    assert_eq!(owner.profile, ExecutionProfile::PrefixTilesDecodeV6);
    assert_ne!(owner.profile, ExecutionProfile::PrefixTilesLayerV6);
    owner.allocate_zeroed(tiny_key()).unwrap();
    owner.bind().unwrap();
    assert_eq!(owner.source, before);
    assert_eq!(owner.phase, Phase::CatalogBound);
    let rows = owner.states.as_ref().unwrap();
    assert_eq!(rows.len(), 288);
    assert_eq!(
        rows.iter()
            .filter(|s| s.kind == WorkerKind::PrefixTilesV6)
            .count(),
        144
    );
    assert_eq!(
        rows.iter()
            .filter(|s| s.kind == WorkerKind::MlpTilesV2)
            .count(),
        144
    );
    assert!(
        ExecutionProfile::PrefixTilesDecodeV6
            .source_kind(WorkerKind::PrefixV5)
            .is_err()
    );
    assert!(
        ExecutionProfile::PrefixTilesDecodeV6
            .source_kind(WorkerKind::MlpV1)
            .is_err()
    );
}
#[test]
fn prefix_decode_selector_refuses_late_repeated_and_other_profile_routes() {
    for mode in 0..6 {
        let mut owner = tiny(Mock::new(), false, 4);
        match mode {
            0 => owner.select_tiles_decode().unwrap(),
            1 => owner.select_prefix_tiles_layer().unwrap(),
            2 => owner.select_prefix_tiles_decode().unwrap(),
            3 => {
                owner.allocate_zeroed(tiny_key()).unwrap();
            }
            4 => owner.phase = Phase::CatalogBound,
            _ => owner.phase = Phase::Terminal,
        }
        assert!(owner.select_prefix_tiles_decode().is_err());
        assert_eq!(owner.phase, Phase::Terminal);
    }
    for layer in [false, true] {
        let mut owner = tiny(Mock::new(), false, 4);
        owner.select_prefix_tiles_decode().unwrap();
        assert!(
            if layer {
                owner.select_prefix_tiles_layer()
            } else {
                owner.select_tiles_decode()
            }
            .is_err()
        );
        assert_eq!(owner.phase, Phase::Terminal);
    }
}

#[test]
fn prefix_tiles_profile_cannot_alias_old_prefix22_or_mlp11_tokens() {
    let p = ExecutionProfile::PrefixTilesLayerV6;
    assert_eq!(
        p.source_kind(WorkerKind::PrefixTilesV6).unwrap(),
        wire::StateKind::PrefixV5
    );
    assert_eq!(
        p.source_kind(WorkerKind::MlpTilesV2).unwrap(),
        wire::StateKind::MlpV1
    );
    assert!(p.source_kind(WorkerKind::PrefixV5).is_err());
    assert!(p.source_kind(WorkerKind::MlpV1).is_err());
    for old in [ExecutionProfile::BaseV1, ExecutionProfile::TilesDecodeV1] {
        assert!(old.source_kind(WorkerKind::PrefixTilesV6).is_err());
    }
}
#[test]
fn prefix_tiles_selector_joins_original_logical_catalog_without_mutating_it() {
    let mut owner = tiny(Mock::new(), false, 4);
    let before = owner.source.clone();
    owner.select_prefix_tiles_layer().unwrap();
    owner.allocate_zeroed(tiny_key()).unwrap();
    owner.bind().unwrap();
    assert_eq!(owner.source, before);
    assert_eq!(owner.phase, Phase::CatalogBound);
    let rows = owner.states.as_ref().unwrap();
    assert_eq!(rows.len(), 288);
    assert_eq!(
        rows.iter()
            .filter(|v| v.kind == WorkerKind::PrefixTilesV6)
            .count(),
        144
    );
    assert_eq!(
        rows.iter()
            .filter(|v| v.kind == WorkerKind::MlpTilesV2)
            .count(),
        144
    );
}
#[test]
fn prefix_tiles_selector_refuses_old_late_repeated_and_terminal_routes() {
    for mode in 0..4 {
        let mut owner = tiny(Mock::new(), false, 4);
        match mode {
            0 => owner.select_tiles_decode().unwrap(),
            1 => {
                owner.allocate_zeroed(tiny_key()).unwrap();
            }
            2 => owner.select_prefix_tiles_layer().unwrap(),
            _ => owner.phase = Phase::Terminal,
        }
        assert!(owner.select_prefix_tiles_layer().is_err());
        assert_eq!(owner.phase, Phase::Terminal);
    }
}

#[test]
fn tiles_execution_role_is_explicit_and_cannot_alias_old_typed_state() {
    assert_eq!(
        ExecutionProfile::TilesDecodeV1
            .source_kind(WorkerKind::MlpTilesV2)
            .unwrap(),
        wire::StateKind::MlpV1
    );
    assert!(
        ExecutionProfile::TilesDecodeV1
            .source_kind(WorkerKind::MlpV1)
            .is_err()
    );
    assert!(
        ExecutionProfile::BaseV1
            .source_kind(WorkerKind::MlpTilesV2)
            .is_err()
    );
    assert_eq!(
        ExecutionProfile::BaseV1
            .source_kind(WorkerKind::MlpV1)
            .unwrap(),
        wire::StateKind::MlpV1
    );
}

#[test]
fn actual_catalog_reservation_uses_selected_roster_without_changing_source_program() {
    let mut owner = tiny(Mock::new(), false, 4);
    let source_before = owner.source.clone();
    owner.select_tiles_decode().unwrap();
    owner.allocate_zeroed(tiny_key()).unwrap();
    owner.bind().unwrap();
    assert_eq!(owner.phase, Phase::CatalogBound);
    assert_eq!(owner.source, source_before);
    let states = owner.states.as_ref().unwrap();
    assert_eq!(states.len(), 288);
    assert_eq!(
        states
            .iter()
            .filter(|r| r.kind == WorkerKind::MlpTilesV2)
            .count(),
        144
    );
    assert!(!states.iter().any(|r| r.kind == WorkerKind::MlpV1));
}

#[test]
fn tiles_profile_selector_refuses_late_repeated_or_terminal_selection() {
    let mut owner = tiny(Mock::new(), false, 4);
    owner.select_tiles_decode().unwrap();
    assert!(owner.select_tiles_decode().is_err());
    assert_eq!(owner.phase, Phase::Terminal);
    let mut owner = tiny(Mock::new(), false, 4);
    owner.allocate_zeroed(tiny_key()).unwrap();
    assert!(owner.select_tiles_decode().is_err());
    assert_eq!(owner.phase, Phase::Terminal);
    let mut owner = tiny(Mock::new(), false, 4);
    owner.phase = Phase::CatalogBound;
    assert!(owner.select_tiles_decode().is_err());
    assert_eq!(owner.phase, Phase::Terminal);
}
use std::cell::RefCell;
use std::rc::Rc;

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
        source_program_sha256: Sha256::digest([0_u8; 123]).into(),
    }
}

fn scope(source: &wire::Registration) -> SourceScope {
    SourceScope {
        bundle_id: source.bundle_id,
        model_id: source.model_id,
        session: source.session,
        pool_identity: source.pool_identity,
        group_id: source.group_id,
        child_identity: source.child_identity,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Preflight(Vec<usize>),
    Allocate(usize, bool, u64),
    Write(usize, u64, usize),
    States,
    Close,
    Drop,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MockBuffer {
    id: usize,
    rank: usize,
    bytes: u64,
}
struct Mock {
    log: Rc<RefCell<Vec<Event>>>,
    occupied: Vec<usize>,
    allocated: usize,
    writes: usize,
    fail_preflight: bool,
    fail_allocate: Option<usize>,
    fail_write: Option<usize>,
    fail_states: bool,
    malformed_states: bool,
    fail_close: bool,
}
impl Mock {
    fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(Vec::new())),
            occupied: vec![0, 0],
            allocated: 0,
            writes: 0,
            fail_preflight: false,
            fail_allocate: None,
            fail_write: None,
            fail_states: false,
            malformed_states: false,
            fail_close: false,
        }
    }
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.log.borrow_mut().push(Event::Drop);
    }
}
impl Backend for Mock {
    type Buffer = MockBuffer;
    type States = Vec<ReservationIdentity>;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>> {
        self.log
            .borrow_mut()
            .push(Event::Preflight(counts.to_vec()));
        if self.fail_preflight {
            Err("native preflight failure".into())
        } else {
            Ok(self.occupied.clone())
        }
    }
    fn allocate(&mut self, rank: usize, peer: bool, bytes: u64) -> Result<MockBuffer> {
        self.log
            .borrow_mut()
            .push(Event::Allocate(rank, peer, bytes));
        if self.fail_allocate == Some(self.allocated) {
            return Err("native allocation failure".into());
        }
        self.allocated += 1;
        Ok(MockBuffer {
            id: self.allocated,
            rank,
            bytes,
        })
    }
    fn write(&mut self, token: &MockBuffer, offset: u64, bytes: &[u8]) -> Result<()> {
        self.log
            .borrow_mut()
            .push(Event::Write(token.id, offset, bytes.len()));
        self.writes += 1;
        assert!(!bytes.is_empty() && bytes.len() <= wire::MAX_TRANSFER);
        assert!(offset.checked_add(bytes.len() as u64).unwrap() <= token.bytes);
        if self.fail_write == Some(self.writes) {
            Err("native write failure".into())
        } else {
            Ok(())
        }
    }
    fn reserve_states(
        &mut self,
        model: [u8; 32],
        profile: ExecutionProfile,
    ) -> Result<Self::States> {
        self.log.borrow_mut().push(Event::States);
        if self.fail_states {
            return Err("typed state allocation failure".into());
        }
        let mut result = Vec::new();
        for forward in 0..2 {
            for layer in 0..36 {
                for kind in [profile.prefix(), profile.mlp()] {
                    for rank in 0..2 {
                        result.push(ReservationIdentity {
                            reservation_id: result.len() as u64 + 1,
                            model,
                            forward,
                            layer,
                            rank,
                            kind,
                        });
                    }
                }
            }
        }
        if self.malformed_states {
            result[287].reservation_id = result[0].reservation_id;
        }
        Ok(result)
    }
    fn state_catalog<'a>(&self, states: &'a Self::States) -> &'a [ReservationIdentity] {
        states
    }
    fn close(&mut self) -> Result<()> {
        self.log.borrow_mut().push(Event::Close);
        if self.fail_close {
            Err("native close failure".into())
        } else {
            Ok(())
        }
    }
}

fn owner(backend: Mock) -> Catalog<Mock> {
    let source = registration();
    let json = serde_json::to_vec(&source).unwrap();
    Catalog::new(backend, scope(&source), source, &json, &[0; 123]).unwrap()
}

// Narrow upload mechanics use tiny private requirements, never a valid launch
// profile. Complete-profile tests below keep the full real requirement roster.
fn tiny(backend: Mock, immutable: bool, bytes: u64) -> Catalog<Mock> {
    let mut owner = owner(backend);
    owner.requirements = vec![
        requirement(
            BindingKey::Source { rank: 0, id: 1 },
            bytes / 2,
            2,
            immutable,
            false,
        )
        .unwrap(),
    ];
    owner
}
fn tiny_key() -> BindingKey {
    BindingKey::Source { rank: 0, id: 1 }
}

// Fabricated initialized facts are confined to this metadata-only fixture.
// It tests binding/phase control, not uploads, authentic weights or GPU values.
fn metadata_initialized(owner: &mut Catalog<Mock>) {
    for (index, requirement) in owner.requirements.iter().enumerate() {
        owner.records.push(Record {
            facts: BindingFacts {
                key: requirement.key,
                catalog_id: index as u64 + 1,
                bytes: requirement.bytes,
                allocation_bytes: requirement.allocation_bytes,
                peer_readable: requirement.peer_readable,
                immutable: requirement.immutable,
                initialized_sha256: Some([1; 32]),
            },
            token: MockBuffer {
                id: index + 1,
                rank: requirement.key.rank() as usize,
                bytes: requirement.allocation_bytes,
            },
            upload: None,
        });
    }
}

#[test]
fn complete_source_roles_have_asymmetric_capacity_and_closed_peer_mappings() {
    let source = registration();
    let rows = requirements(&source).unwrap();
    assert_eq!(rows.len(), 1135);
    assert_eq!(rows.iter().filter(|r| r.immutable).count(), 939);
    assert_eq!(rows.iter().filter(|r| !r.immutable).count(), 196);
    assert_eq!(rows.iter().filter(|r| r.peer_readable).count(), 6);
    for rank in 0..2 {
        assert_eq!(
            rows.iter().filter(|r| r.key.rank() == rank).count(),
            BUFFERS_PER_RANK[rank as usize]
        );
    }
    let empty: Vec<_> = rows.iter().filter(|r| r.bytes == 0).collect();
    assert_eq!(empty.len(), 2);
    assert!(
        empty
            .iter()
            .all(|r| r.allocation_bytes == 2 && !r.immutable && !r.peer_readable)
    );
    let owner = owner(Mock::new());
    assert_eq!(
        owner.backend.log.borrow()[0],
        Event::Preflight(vec![713, 710])
    );
    assert!(owner.records.is_empty() && owner.states.is_none());
}

#[test]
fn scope_registration_and_source_bytes_are_checked_before_native_accounting() {
    for mutation in 0..6 {
        let mut source = registration();
        let mut source_scope = scope(&source);
        let mut json = serde_json::to_vec(&source).unwrap();
        let mut program = vec![0; 123];
        match mutation {
            0 => source_scope.model_id = [9; 32],
            1 => source_scope.session = [9; 32],
            2 => source_scope.child_identity += 1,
            3 => json[0] = b'!',
            4 => program[12] = 1,
            5 => source.globals.pop().map(|_| ()).unwrap(),
            _ => unreachable!(),
        }
        let backend = Mock::new();
        let log = backend.log.clone();
        assert!(Catalog::new(backend, source_scope, source, &json, &program).is_err());
        assert_eq!(&*log.borrow(), &[Event::Drop]);
    }
}

#[test]
fn unavailable_capacity_nonfresh_or_wrong_owner_count_never_allocate() {
    for occupied in [vec![1, 0], vec![0, 1], vec![0], vec![0; 3]] {
        let mut backend = Mock::new();
        backend.occupied = occupied;
        let log = backend.log.clone();
        let source = registration();
        let json = serde_json::to_vec(&source).unwrap();
        assert!(Catalog::new(backend, scope(&source), source, &json, &[0; 123]).is_err());
        assert_eq!(
            &*log.borrow(),
            &[Event::Preflight(vec![713, 710]), Event::Drop]
        );
    }
    let mut backend = Mock::new();
    backend.fail_preflight = true;
    let log = backend.log.clone();
    let source = registration();
    let json = serde_json::to_vec(&source).unwrap();
    assert!(Catalog::new(backend, scope(&source), source, &json, &[0; 123]).is_err());
    assert_eq!(
        &*log.borrow(),
        &[Event::Preflight(vec![713, 710]), Event::Drop]
    );
}

#[test]
fn sequential_immutable_upload_hashes_exact_bytes_and_forbids_postcompletion_write() {
    let mut owner = tiny(Mock::new(), true, 8);
    let data = [1_u8, 2, 3, 4, 5, 6, 7, 8];
    let digest: [u8; 32] = Sha256::digest(data).into();
    let id = owner.allocate_upload(tiny_key(), digest).unwrap();
    owner.write_upload(id, 0, &data[..4]).unwrap();
    assert_eq!(owner.records[0].facts.initialized_sha256(), None);
    owner.write_upload(id, 4, &data[4..]).unwrap();
    assert_eq!(owner.records[0].facts.initialized_sha256(), Some(digest));
    assert!(owner.records[0].upload.is_none());
    let writes = owner.backend.writes;
    assert!(owner.write_upload(id, 0, &data).is_err());
    assert_eq!(owner.phase, Phase::Terminal);
    assert_eq!(owner.backend.writes, writes);
}

#[test]
fn unknown_duplicate_wrong_initialization_and_catalog_id_overflow_fail_closed() {
    for mutation in 0..6 {
        let mut owner = tiny(Mock::new(), true, 4);
        let digest: [u8; 32] = Sha256::digest([1_u8; 4]).into();
        let result = match mutation {
            0 => owner.allocate_upload(BindingKey::Source { rank: 1, id: 1 }, digest),
            1 => owner.allocate_zeroed(tiny_key()),
            2 => owner.allocate_upload(tiny_key(), [0; 32]),
            3 => {
                owner.next_id = u64::MAX;
                owner.allocate_upload(tiny_key(), digest)
            }
            4 => {
                owner.allocate_upload(tiny_key(), digest).unwrap();
                owner.allocate_upload(tiny_key(), digest)
            }
            5 => {
                owner.requirements[0].immutable = false;
                owner.allocate_upload(tiny_key(), digest)
            }
            _ => unreachable!(),
        };
        assert!(result.is_err());
        assert_eq!(owner.phase, Phase::Terminal);
        assert_eq!(owner.backend.allocated, usize::from(mutation == 4));
        assert!(owner.close().is_err());
    }
}

#[test]
fn gaps_replays_overflows_empty_and_oversized_chunks_never_reach_native_write() {
    for mutation in 0..7 {
        let mut owner = tiny(Mock::new(), true, 8);
        let id = owner
            .allocate_upload(tiny_key(), Sha256::digest([1_u8; 8]).into())
            .unwrap();
        let result = match mutation {
            0 => owner.write_upload(id, 4, &[1; 4]),
            1 => owner.write_upload(id, u64::MAX, &[1]),
            2 => owner.write_upload(id, 0, &[]),
            3 => owner.write_upload(id, 0, &[1; 9]),
            4 => owner.write_upload(id + 1, 0, &[1; 4]),
            5 => owner.write_upload(id, 0, &vec![1; wire::MAX_TRANSFER + 1]),
            6 => {
                owner.write_upload(id, 0, &[1; 4]).unwrap();
                owner.write_upload(id, 0, &[1; 4])
            }
            _ => unreachable!(),
        };
        assert!(result.is_err());
        assert_eq!(owner.phase, Phase::Terminal);
        assert_eq!(owner.backend.writes, usize::from(mutation == 6));
    }
}

#[test]
fn wrong_final_digest_retains_owner_without_success_or_retry() {
    let mut owner = tiny(Mock::new(), true, 4);
    let id = owner
        .allocate_upload(tiny_key(), Sha256::digest([1_u8; 4]).into())
        .unwrap();
    assert!(owner.write_upload(id, 0, &[2; 4]).is_err());
    assert_eq!(owner.records[0].facts.initialized_sha256(), None);
    assert_eq!(owner.phase, Phase::Terminal);
    assert_eq!(owner.backend.allocated, 1);
    assert!(owner.write_upload(id, 0, &[1; 4]).is_err());
    assert!(owner.bind().is_err());
    assert!(owner.close().is_err());
    assert_eq!(owner.backend.writes, 1);
    assert!(!owner.backend.log.borrow().contains(&Event::Close));
}

#[test]
fn allocation_and_write_failures_are_terminal_with_retained_prior_records() {
    for write in [false, true] {
        let mut backend = Mock::new();
        if write {
            backend.fail_write = Some(1);
        } else {
            backend.fail_allocate = Some(0);
        }
        let mut owner = tiny(backend, true, 4);
        let id = owner.allocate_upload(tiny_key(), Sha256::digest([1_u8; 4]).into());
        if write {
            assert!(owner.write_upload(id.unwrap(), 0, &[1; 4]).is_err());
            assert_eq!(owner.records.len(), 1);
        } else {
            assert!(id.is_err());
            assert!(owner.records.is_empty());
        }
        assert_eq!(owner.phase, Phase::Terminal);
        assert!(owner.close().is_err());
        assert!(!owner.backend.log.borrow().contains(&Event::Close));
    }
}

#[test]
fn mutable_zero_initialization_covers_bounded_backing_including_empty_views() {
    for bytes in [0, 6, wire::MAX_TRANSFER as u64 + 2] {
        let mut owner = tiny(Mock::new(), false, bytes);
        let id = owner.allocate_zeroed(tiny_key()).unwrap();
        let physical = bytes.max(2);
        assert_eq!(owner.records[0].facts.catalog_id(), id);
        assert_eq!(owner.records[0].facts.bytes(), bytes);
        assert_eq!(owner.records[0].facts.allocation_bytes(), physical);
        let expected: [u8; 32] = Sha256::digest(vec![0_u8; physical as usize]).into();
        assert_eq!(owner.records[0].facts.initialized_sha256(), Some(expected));
        assert_eq!(
            owner.backend.writes,
            if physical > wire::MAX_TRANSFER as u64 {
                2
            } else {
                1
            }
        );
        assert!(owner.allocate_upload(tiny_key(), expected).is_err());
    }
}

#[test]
fn bind_requires_every_initialized_original_and_pending_binding_before_states() {
    for mutation in 0..4 {
        let mut owner = owner(Mock::new());
        metadata_initialized(&mut owner);
        match mutation {
            0 => {
                owner.records.pop();
            }
            1 => owner.records[0].facts.initialized_sha256 = None,
            2 => owner.records[0].facts.bytes += 2,
            3 => owner.records[0].facts.peer_readable = true,
            _ => unreachable!(),
        }
        assert!(owner.bind().is_err());
        assert_eq!(owner.phase, Phase::Terminal);
        assert!(owner.states.is_none());
        assert!(!owner.backend.log.borrow().contains(&Event::States));
    }
}

#[test]
fn bound_catalog_retains_full_native_records_and_unsealed_typed_states() {
    let mut owner = owner(Mock::new());
    metadata_initialized(&mut owner);
    let facts = owner.bind().unwrap();
    assert_eq!(facts.len(), MAX_BUFFERS);
    assert_eq!(owner.phase, Phase::CatalogBound);
    assert_eq!(owner.states.as_ref().unwrap().len(), 288);
    assert!(facts.iter().all(|r| r.initialized_sha256().is_some()));
    assert_eq!(facts.iter().filter(|r| r.peer_readable()).count(), 6);
    assert!(owner.bind().is_err());
    assert_eq!(owner.phase, Phase::Terminal);
    assert_eq!(
        owner
            .backend
            .log
            .borrow()
            .iter()
            .filter(|e| **e == Event::States)
            .count(),
        1
    );
}

#[test]
fn failed_or_foreign_typed_state_reservations_prevent_bound_receipt_and_retry() {
    for malformed in [false, true] {
        let mut backend = Mock::new();
        backend.fail_states = !malformed;
        backend.malformed_states = malformed;
        let mut owner = owner(backend);
        metadata_initialized(&mut owner);
        assert!(owner.bind().is_err());
        assert_eq!(owner.phase, Phase::Terminal);
        assert_eq!(owner.states.is_some(), malformed);
        assert!(owner.bind().is_err());
        assert!(owner.close().is_err());
        assert_eq!(
            owner
                .backend
                .log
                .borrow()
                .iter()
                .filter(|e| **e == Event::States)
                .count(),
            1
        );
    }
}

#[test]
fn setup_or_bound_close_is_once_and_any_failure_quarantines_without_retry() {
    for bound in [false, true] {
        for fail_close in [false, true] {
            let mut backend = Mock::new();
            backend.fail_close = fail_close;
            let mut owner = owner(backend);
            if bound {
                metadata_initialized(&mut owner);
                owner.bind().unwrap();
            }
            assert_eq!(owner.close().is_ok(), !fail_close);
            assert_eq!(
                owner.phase,
                if fail_close {
                    Phase::Terminal
                } else {
                    Phase::Closed
                }
            );
            assert!(owner.close().is_err());
            assert!(owner.allocate_zeroed(tiny_key()).is_err());
            assert_eq!(
                owner
                    .backend
                    .log
                    .borrow()
                    .iter()
                    .filter(|e| **e == Event::Close)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn every_layer_resolves_original_rank_roots_and_shared_phase_allocations() {
    let mut owner = owner(Mock::new());
    metadata_initialized(&mut owner);
    owner.bind().unwrap();
    let bytes_prefix = [
        8192, 8192, 25_165_824, 512, 512, 580, 16_777_216, 8192, 6144, 4096, 2_359_296, 2_359_296,
        4096, 16_384,
    ];
    let bytes_mlp = [
        8192, 8192, 50_331_648, 50_331_648, 50_331_648, 8192, 12_288, 12_288, 12_288, 16_384,
    ];
    for layer in 0..36 {
        let roots = owner.layer_bindings(layer).unwrap();
        for rank in 0..2 {
            for (values, extents) in [
                (roots.prefix[rank].as_slice(), bytes_prefix.as_slice()),
                (roots.mlp[rank].as_slice(), bytes_mlp.as_slice()),
            ] {
                for (index, (token, bytes)) in values.iter().zip(extents).enumerate() {
                    assert_eq!((token.rank, token.bytes), (rank, *bytes));
                    assert!(!values[..index].contains(token));
                }
            }
            assert_eq!(roots.final_hidden[rank], roots.prefix[rank][0]);
            assert_eq!(roots.mlp[rank][5], roots.prefix[rank][7]);
            assert_eq!(roots.mlp[rank][9], roots.prefix[rank][13]);
            assert!(!roots.prefix[rank].contains(&roots.mlp[rank][0]));
            let layer_source = &owner.source.layers[rank * 36 + layer];
            for (slot, key) in [
                (10, source_key(layer_source.caches[0])),
                (11, source_key(layer_source.caches[1])),
            ] {
                assert_eq!(
                    roots.prefix[rank][slot],
                    owner
                        .records
                        .iter()
                        .find(|r| r.facts.key == key)
                        .unwrap()
                        .token
                );
            }
        }
    }
    let first = owner.layer_bindings(0).unwrap();
    let last = owner.layer_bindings(35).unwrap();
    assert_eq!(first.prefix[0][0], last.prefix[0][0]);
    assert_eq!(first.mlp[1][9], last.mlp[1][9]);
    assert_ne!(first.prefix[0][2], last.prefix[0][2]);
    assert_ne!(first.prefix[0][10], last.prefix[0][10]);
}

#[test]
fn layer_resolution_rejects_unbound_bad_layer_or_missing_initialized_root() {
    let mut owner = owner(Mock::new());
    assert!(owner.layer_bindings(0).is_err());
    metadata_initialized(&mut owner);
    owner.bind().unwrap();
    assert!(owner.layer_bindings(36).is_err());
    assert!(owner.layer_bindings(usize::MAX).is_err());
    let key = source_key(owner.source.layers[0].weights[0].buffer);
    owner
        .records
        .iter_mut()
        .find(|r| r.facts.key == key)
        .unwrap()
        .facts
        .initialized_sha256 = None;
    assert!(owner.layer_bindings(0).is_err());
}

#[test]
fn tail_source_roles_resolve_original_globals_and_both_actual_hidden_owners() {
    let mut owner = owner(Mock::new());
    assert!(owner.tail_source_bindings().is_err());
    metadata_initialized(&mut owner);
    owner.bind().unwrap();
    let roots = owner.tail_source_bindings().unwrap();
    let record = |index: usize| {
        owner
            .records
            .iter()
            .find(|r| r.token == roots[index])
            .unwrap()
    };
    assert_eq!(
        roots.map(|r| r.bytes),
        [64, HEAD_BYTES, 8192, 8192, 8192, 2, 8192, 4_861_952, 64]
    );
    assert_eq!(roots.map(|r| r.rank), [0, 0, 0, 1, 0, 0, 0, 0, 0]);
    for index in [1, 4] {
        assert!(record(index).facts.immutable);
    }
    for index in [0, 2, 3, 5, 6, 7, 8] {
        assert!(!record(index).facts.immutable);
    }
    let original_head = owner
        .source
        .globals
        .iter()
        .find(|g| g.kind == wire::GlobalKind::LanguageModelHead)
        .unwrap();
    assert!(roots.iter().all(|root| {
        owner
            .records
            .iter()
            .find(|r| r.token == *root)
            .unwrap()
            .facts
            .key
            != source_key(original_head.buffer)
    }));
    let layers = owner.layer_bindings(35).unwrap();
    assert_eq!(roots[2..4], layers.final_hidden);
    assert_eq!(roots[6], layers.prefix[0][7]);
}

#[test]
fn tail_source_resolution_rejects_each_missing_initialization() {
    for index in 0..9 {
        let mut owner = owner(Mock::new());
        metadata_initialized(&mut owner);
        owner.bind().unwrap();
        let root = owner.tail_source_bindings().unwrap()[index];
        owner
            .records
            .iter_mut()
            .find(|r| r.token == root)
            .unwrap()
            .facts
            .initialized_sha256 = None;
        assert!(owner.tail_source_bindings().is_err());
    }
}
