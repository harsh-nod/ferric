//! Private retained owner for finite setup. CatalogBound is not executable Ready.

use crate::finite_composition_wire as wire;
use crate::resident_artifacts::{self, LoadedResidentArtifacts, ResidentKind, ReviewedImages};
use crate::resident_layer::LayerBindings;
use crate::resident_layer::guarded_mlp_decode_v1 as guarded_layer;
use crate::resident_layer::mlp_tiles_v2::artifacts as tiles_artifacts;
use crate::resident_layer::prefix_tiles_v6::artifacts as prefix_artifacts;
use crate::state_roster::guarded_mlp_decode_v1 as guarded_states;
use crate::state_roster::prefix_tiles_decode_v6;
use crate::state_roster::prefix_tiles_v6;
use crate::state_roster::tiles_decode_v1;
use crate::state_roster::{PendingStates, ReservationIdentity, StateRoster, WorkerKind};
use crate::tail_artifacts::{self, LoadedTailArtifacts, ReviewedTailImages};
use crate::tail_bindings::TailBindings;
use crate::tail_head::{HEAD_BYTES, HeadManifest, HeadTranspose};
use fe2o3_kfd::{Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerGroupV1 as Group};
use sha2::{Digest, Sha256};

type Result<T> = core::result::Result<T, String>;
const RANKS: usize = 2;
const STATES_PER_RANK: usize = 144;
const BUFFERS_PER_RANK: [usize; RANKS] = [569, 566];
const MAX_BUFFERS: usize = 1135;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecutionProfile {
    BaseV1,
    TilesDecodeV1,
    PrefixTilesLayerV6,
    PrefixTilesDecodeV6,
    GuardedMlpDecodeV1,
    GuardedMlpReuseV1,
}
impl ExecutionProfile {
    fn prefix(self) -> WorkerKind {
        if matches!(
            self,
            Self::PrefixTilesLayerV6
                | Self::PrefixTilesDecodeV6
                | Self::GuardedMlpDecodeV1
                | Self::GuardedMlpReuseV1
        ) {
            WorkerKind::PrefixTilesV6
        } else {
            WorkerKind::PrefixV5
        }
    }
    fn mlp(self) -> WorkerKind {
        match self {
            Self::BaseV1 => WorkerKind::MlpV1,
            Self::GuardedMlpDecodeV1 | Self::GuardedMlpReuseV1 => WorkerKind::GuardedMlpCombinedV1,
            Self::TilesDecodeV1 | Self::PrefixTilesLayerV6 | Self::PrefixTilesDecodeV6 => {
                WorkerKind::MlpTilesV2
            }
        }
    }
    fn source_kind(self, kind: WorkerKind) -> Result<wire::StateKind> {
        if kind == self.prefix() {
            Ok(wire::StateKind::PrefixV5)
        } else if kind == self.mlp() {
            Ok(wire::StateKind::MlpV1)
        } else {
            Err("execution state kind does not match selected profile".into())
        }
    }
}
// Logical source slots are joined by the selected profile. A V2 token is never
// represented as a V1 typed token or claimed to come from the old source Program.
enum PendingExecution {
    Base(PendingStates),
    Tiles(tiles_decode_v1::Pending),
    PrefixTiles(prefix_tiles_v6::Pending),
    PrefixDecode(prefix_tiles_decode_v6::Pending),
    Guarded(guarded_states::Pending),
}

/// A source description, never a KFD token or an imported current-child ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BindingKey {
    Source {
        rank: u32,
        id: u64,
    },
    Pending {
        rank: u32,
        layer: Option<u32>,
        kind: wire::PendingKind,
    },
}

impl BindingKey {
    pub(super) fn rank(self) -> u32 {
        match self {
            Self::Source { rank, .. } | Self::Pending { rank, .. } => rank,
        }
    }
}

/// Read-only facts derived from retained records, not a handle factory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BindingFacts {
    key: BindingKey,
    catalog_id: u64,
    bytes: u64,
    allocation_bytes: u64,
    peer_readable: bool,
    immutable: bool,
    initialized_sha256: Option<[u8; 32]>,
}

impl BindingFacts {
    pub(super) fn key(&self) -> BindingKey {
        self.key
    }
    pub(super) fn catalog_id(&self) -> u64 {
        self.catalog_id
    }
    pub(super) fn bytes(&self) -> u64 {
        self.bytes
    }
    pub(super) fn allocation_bytes(&self) -> u64 {
        self.allocation_bytes
    }
    pub(super) fn peer_readable(&self) -> bool {
        self.peer_readable
    }
    pub(super) fn immutable(&self) -> bool {
        self.immutable
    }
    pub(super) fn initialized_sha256(&self) -> Option<[u8; 32]> {
        self.initialized_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceScope {
    pub(super) bundle_id: [u8; 32],
    pub(super) model_id: [u8; 32],
    pub(super) session: [u8; 32],
    pub(super) pool_identity: u64,
    pub(super) group_id: u64,
    pub(super) child_identity: u32,
}

impl SourceScope {
    fn matches(&self, source: &wire::Registration) -> bool {
        self.bundle_id == source.bundle_id
            && self.model_id == source.model_id
            && self.session == source.session
            && self.pool_identity == source.pool_identity
            && self.group_id == source.group_id
            && self.child_identity == source.child_identity
    }
}

struct Requirement {
    key: BindingKey,
    bytes: u64,
    allocation_bytes: u64,
    immutable: bool,
    peer_readable: bool,
}

fn requirement(
    key: BindingKey,
    elements: u64,
    width: u32,
    immutable: bool,
    peer_readable: bool,
) -> Result<Requirement> {
    let bytes = elements
        .checked_mul(u64::from(width))
        .ok_or("finite buffer byte overflow")?;
    if key.rank() >= RANKS as u32
        || !matches!(width, 2 | 4)
        || bytes == 0 && immutable
        || bytes > 2 * 1024 * 1024 * 1024
    {
        return Err("finite buffer requirement extent or rank".into());
    }
    Ok(Requirement {
        key,
        bytes,
        allocation_bytes: bytes.max(u64::from(width)),
        immutable,
        peer_readable,
    })
}

fn source_key(buffer: wire::Buffer) -> BindingKey {
    BindingKey::Source {
        rank: buffer.rank,
        id: buffer.id,
    }
}

fn requirements(source: &wire::Registration) -> Result<Vec<Requirement>> {
    source.validate().map_err(|e| e.to_string())?;
    let mut result = Vec::with_capacity(MAX_BUFFERS);
    for layer in &source.layers {
        for weight in &layer.weights {
            let b = weight.buffer;
            result.push(requirement(
                source_key(b),
                b.elements,
                b.element_bytes,
                true,
                false,
            )?);
        }
        for b in layer.caches {
            result.push(requirement(
                source_key(b),
                b.elements,
                b.element_bytes,
                false,
                false,
            )?);
        }
    }
    for scratch in &source.scratch {
        let b = scratch.buffer;
        result.push(requirement(
            source_key(b),
            b.elements,
            b.element_bytes,
            false,
            matches!(
                scratch.kind,
                wire::ScratchKind::Hidden
                    | wire::ScratchKind::PostAttentionResidual
                    | wire::ScratchKind::Partial
            ),
        )?);
    }
    for global in &source.globals {
        let b = global.buffer;
        result.push(requirement(
            source_key(b),
            b.elements,
            b.element_bytes,
            true,
            false,
        )?);
    }
    for auxiliary in &source.auxiliary {
        let b = auxiliary.buffer;
        result.push(requirement(
            source_key(b),
            b.elements,
            b.element_bytes,
            false,
            false,
        )?);
    }
    for pending in &source.pending_buffers {
        let immutable = matches!(
            pending.kind,
            wire::PendingKind::PackedQkvWeight | wire::PendingKind::PackedHeadNormWeight
        );
        result.push(requirement(
            BindingKey::Pending {
                rank: pending.rank,
                layer: pending.layer,
                kind: pending.kind,
            },
            pending.elements,
            pending.element_bytes,
            immutable,
            false,
        )?);
    }
    if result.len() != MAX_BUFFERS
        || (0..RANKS as u32).any(|rank| {
            result.iter().filter(|row| row.key.rank() == rank).count()
                != BUFFERS_PER_RANK[rank as usize]
        })
    {
        return Err("finite retained requirement cardinality".into());
    }
    Ok(result)
}

// Private static seam; no wire value can implement a backend or mint a token.
trait Backend {
    type Buffer: Copy + Eq;
    type States;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>>;
    fn allocate(&mut self, rank: usize, peer_readable: bool, bytes: u64) -> Result<Self::Buffer>;
    fn write(&mut self, buffer: &Self::Buffer, offset: u64, bytes: &[u8]) -> Result<()>;
    fn reserve_states(
        &mut self,
        model: [u8; 32],
        profile: ExecutionProfile,
    ) -> Result<Self::States>;
    fn state_catalog<'a>(&self, states: &'a Self::States) -> &'a [ReservationIdentity];
    fn close(&mut self) -> Result<()>;
}

impl Backend for Group {
    type Buffer = Buffer;
    type States = PendingExecution;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(counts)
    }
    fn allocate(&mut self, rank: usize, peer_readable: bool, bytes: u64) -> Result<Buffer> {
        let peer = [1 - rank];
        Group::allocate(self, rank, if peer_readable { &peer } else { &[] }, bytes)
    }
    fn write(&mut self, buffer: &Buffer, offset: u64, bytes: &[u8]) -> Result<()> {
        Group::write(self, *buffer, offset, bytes)
    }
    fn reserve_states(
        &mut self,
        model: [u8; 32],
        profile: ExecutionProfile,
    ) -> Result<PendingExecution> {
        match profile {
            ExecutionProfile::BaseV1 => {
                PendingStates::allocate(self, model).map(PendingExecution::Base)
            }
            ExecutionProfile::TilesDecodeV1 => {
                tiles_decode_v1::Pending::allocate(self, model).map(PendingExecution::Tiles)
            }
            ExecutionProfile::PrefixTilesLayerV6 => {
                prefix_tiles_v6::Pending::allocate(self, model).map(PendingExecution::PrefixTiles)
            }
            ExecutionProfile::PrefixTilesDecodeV6 => {
                prefix_tiles_decode_v6::Pending::allocate(self, model)
                    .map(PendingExecution::PrefixDecode)
            }
            ExecutionProfile::GuardedMlpDecodeV1 | ExecutionProfile::GuardedMlpReuseV1 => {
                guarded_states::Pending::allocate(self, model).map(PendingExecution::Guarded)
            }
        }
    }
    fn state_catalog<'a>(&self, states: &'a PendingExecution) -> &'a [ReservationIdentity] {
        match states {
            PendingExecution::Base(v) => v.catalog(),
            PendingExecution::Tiles(v) => v.catalog(),
            PendingExecution::PrefixTiles(v) => v.catalog(),
            PendingExecution::PrefixDecode(v) => v.catalog(),
            PendingExecution::Guarded(v) => v.catalog(),
        }
    }
    fn close(&mut self) -> Result<()> {
        Group::close(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Setup,
    Binding,
    CatalogBound,
    LayersSealed,
    Closed,
    Terminal,
}

struct Upload {
    expected: [u8; 32],
    written: u64,
    hash: Sha256,
}

struct Record<T> {
    facts: BindingFacts,
    token: T,
    upload: Option<Upload>,
}

struct Catalog<B: Backend> {
    backend: B,
    scope: SourceScope,
    source: wire::Registration,
    registration_sha256: [u8; 32],
    requirements: Vec<Requirement>,
    records: Vec<Record<B::Buffer>>,
    states: Option<B::States>,
    next_id: u64,
    phase: Phase,
    profile: ExecutionProfile,
}

impl<B: Backend> Catalog<B> {
    fn new(
        mut backend: B,
        scope: SourceScope,
        source: wire::Registration,
        registration_bytes: &[u8],
        source_program: &[u8],
    ) -> Result<Self> {
        // Validate the same bytes whose digest is retained. JSON equality alone
        // does not authenticate source intake; the owning parent supplies scope.
        if registration_bytes.is_empty()
            || registration_bytes.len() > wire::MAX_REGISTRATION
            || serde_json::from_slice::<wire::Registration>(registration_bytes)
                .map_err(|e| e.to_string())?
                != source
            || !scope.matches(&source)
            || source_program.len() != source.source_program_bytes as usize
            || <[u8; 32]>::from(Sha256::digest(source_program)) != source.source_program_sha256
        {
            return Err("finite source registration/scope/byte custody mismatch".into());
        }
        let requirements = requirements(&source)?;
        let occupied = backend.preflight(&BUFFERS_PER_RANK.map(|count| count + STATES_PER_RANK))?;
        if occupied != [0, 0] {
            return Err("finite setup requires a fresh, exclusively owned current group".into());
        }
        Ok(Self {
            backend,
            scope,
            source,
            registration_sha256: Sha256::digest(registration_bytes).into(),
            requirements,
            records: Vec::with_capacity(MAX_BUFFERS),
            states: None,
            next_id: 1,
            phase: Phase::Setup,
            profile: ExecutionProfile::BaseV1,
        })
    }

    fn require_setup(&mut self) -> Result<()> {
        if self.phase != Phase::Setup {
            self.phase = Phase::Terminal;
            return Err("finite catalog is not in setup".into());
        }
        Ok(())
    }

    fn select_tiles_decode(&mut self) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Setup
                || !self.records.is_empty()
                || self.states.is_some()
                || self.profile != ExecutionProfile::BaseV1
            {
                return Err("tiles selector requires pristine base setup".into());
            }
            self.profile = ExecutionProfile::TilesDecodeV1;
            Ok(())
        })();
        self.finish(result)
    }

    fn select_prefix_tiles_decode(&mut self) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Setup
                || !self.records.is_empty()
                || self.states.is_some()
                || self.profile != ExecutionProfile::BaseV1
            {
                return Err("prefix decode selector must precede all allocations".into());
            }
            self.profile = ExecutionProfile::PrefixTilesDecodeV6;
            Ok(())
        })();
        self.finish(result)
    }
    fn select_prefix_tiles_layer(&mut self) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Setup
                || !self.records.is_empty()
                || self.states.is_some()
                || self.profile != ExecutionProfile::BaseV1
            {
                return Err("prefix layer selector requires pristine base setup".into());
            }
            self.profile = ExecutionProfile::PrefixTilesLayerV6;
            Ok(())
        })();
        self.finish(result)
    }

    fn select_guarded_mlp_decode(&mut self) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Setup
                || !self.records.is_empty()
                || self.states.is_some()
                || self.profile != ExecutionProfile::BaseV1
            {
                return Err("guarded decode selector must precede all allocations".into());
            }
            self.profile = ExecutionProfile::GuardedMlpDecodeV1;
            Ok(())
        })();
        self.finish(result)
    }

    fn finish<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.phase = Phase::Terminal;
        }
        result
    }

    fn allocate_inner(&mut self, key: BindingKey, expected: Option<[u8; 32]>) -> Result<u64> {
        let required = self
            .requirements
            .iter()
            .find(|r| r.key == key)
            .ok_or("unknown source/pending binding key")?;
        if self.records.iter().any(|r| r.facts.key == key)
            || self.records.len() >= MAX_BUFFERS
            || required.immutable != expected.is_some()
            || expected == Some([0; 32])
        {
            return Err("duplicate binding or wrong initialization contract".into());
        }
        let id = self.next_id;
        let next = id.checked_add(1).ok_or("finite catalog ID overflow")?;
        let token = self.backend.allocate(
            key.rank() as usize,
            required.peer_readable,
            required.allocation_bytes,
        )?;
        self.records.push(Record {
            facts: BindingFacts {
                key,
                catalog_id: id,
                bytes: required.bytes,
                peer_readable: required.peer_readable,
                allocation_bytes: required.allocation_bytes,
                immutable: required.immutable,
                initialized_sha256: None,
            },
            token,
            upload: expected.map(|expected| Upload {
                expected,
                written: 0,
                hash: Sha256::new(),
            }),
        });
        self.next_id = next;
        Ok(id)
    }

    fn allocate_upload(&mut self, key: BindingKey, expected_sha256: [u8; 32]) -> Result<u64> {
        self.require_setup()?;
        let result = self.allocate_inner(key, Some(expected_sha256));
        self.finish(result)
    }

    fn write_upload(&mut self, catalog_id: u64, offset: u64, bytes: &[u8]) -> Result<()> {
        self.require_setup()?;
        let result = (|| {
            let row = self
                .records
                .iter_mut()
                .find(|r| r.facts.catalog_id == catalog_id)
                .ok_or("unknown current-child catalog ID")?;
            let upload = row
                .upload
                .as_mut()
                .ok_or("binding is not an unfinished immutable upload")?;
            let end = offset
                .checked_add(bytes.len() as u64)
                .ok_or("upload offset overflow")?;
            if bytes.is_empty()
                || bytes.len() > wire::MAX_TRANSFER
                || offset != upload.written
                || end > row.facts.bytes
                || row.facts.initialized_sha256.is_some()
            {
                return Err("nonsequential, overflowing or repeated finite upload".into());
            }
            self.backend.write(&row.token, offset, bytes)?;
            upload.hash.update(bytes);
            upload.written = end;
            if end == row.facts.bytes {
                let actual: [u8; 32] = upload.hash.clone().finalize().into();
                if actual != upload.expected {
                    return Err("complete finite upload digest mismatch".into());
                }
                row.facts.initialized_sha256 = Some(actual);
                row.upload = None;
            }
            Ok(())
        })();
        self.finish(result)
    }

    fn allocate_zeroed(&mut self, key: BindingKey) -> Result<u64> {
        self.require_setup()?;
        let result = (|| {
            let id = self.allocate_inner(key, None)?;
            let row = self
                .records
                .last_mut()
                .ok_or("missing freshly allocated mutable binding")?;
            let chunk =
                vec![
                    0_u8;
                    usize::try_from(row.facts.allocation_bytes.min(wire::MAX_TRANSFER as u64))
                        .map_err(|_| "zero initialization chunk extent")?
                ];
            let mut offset = 0_u64;
            let mut hash = Sha256::new();
            while offset < row.facts.allocation_bytes {
                let length =
                    usize::try_from((row.facts.allocation_bytes - offset).min(chunk.len() as u64))
                        .map_err(|_| "zero initialization remaining extent")?;
                self.backend.write(&row.token, offset, &chunk[..length])?;
                hash.update(&chunk[..length]);
                offset += length as u64;
            }
            row.facts.initialized_sha256 = Some(hash.finalize().into());
            Ok(id)
        })();
        self.finish(result)
    }

    fn bind(&mut self) -> Result<Vec<BindingFacts>> {
        self.require_setup()?;
        self.phase = Phase::Binding;
        let result = (|| {
            if self.records.len() != self.requirements.len()
                || self
                    .records
                    .iter()
                    .any(|r| r.upload.is_some() || r.facts.initialized_sha256.is_none())
            {
                return Err("finite catalog has missing or unfinished initialized bindings".into());
            }
            for required in &self.requirements {
                let record = self
                    .records
                    .iter()
                    .find(|r| r.facts.key == required.key)
                    .ok_or("finite catalog missing source role")?;
                if record.facts.bytes != required.bytes
                    || record.facts.allocation_bytes != required.allocation_bytes
                    || record.facts.immutable != required.immutable
                    || record.facts.peer_readable != required.peer_readable
                {
                    return Err("finite retained source role mismatch".into());
                }
            }
            // Keep the actual typed tokens in this same owner, even on a later
            // validation error. No seal/Ready or dispatch follows this method.
            let states = self
                .backend
                .reserve_states(self.scope.model_id, self.profile)?;
            self.states = Some(states);
            let rows = self
                .backend
                .state_catalog(self.states.as_ref().ok_or("missing state owner")?);
            if rows.len() != self.source.state_slots.len() {
                return Err("finite typed state cardinality".into());
            }
            let mut ids = std::collections::BTreeSet::new();
            let mut slots = std::collections::BTreeSet::new();
            for row in rows {
                let kind = self.profile.source_kind(row.kind)?;
                if row.model != self.scope.model_id
                    || row.reservation_id == 0
                    || !ids.insert(row.reservation_id)
                    || !slots.insert((row.forward, row.layer, row.rank, kind))
                    || !self.source.state_slots.iter().any(|s| {
                        (s.forward, s.layer, s.rank, s.kind)
                            == (row.forward, row.layer, row.rank, kind)
                    })
                {
                    return Err("finite typed state provenance, duplicate or foreign tuple".into());
                }
            }
            Ok(self.records.iter().map(|r| r.facts.clone()).collect())
        })();
        let result = self.finish(result);
        if result.is_ok() {
            self.phase = Phase::CatalogBound;
        }
        result
    }

    fn close(&mut self) -> Result<()> {
        if !matches!(
            self.phase,
            Phase::Setup | Phase::CatalogBound | Phase::LayersSealed
        ) {
            self.phase = Phase::Terminal;
            return Err("terminal, active or already closed catalog cannot close/retry".into());
        }
        let result = self.backend.close();
        let result = self.finish(result);
        if result.is_ok() {
            self.phase = Phase::Closed;
        }
        result
    }

    fn layer_bindings(&self, layer: usize) -> Result<LayerBindings<B::Buffer>> {
        if self.phase != Phase::CatalogBound || layer >= 36 {
            return Err("layer resolution requires bound catalog and exact layer ordinal".into());
        }
        let retained = |key: BindingKey| -> Result<B::Buffer> {
            let row = self
                .records
                .iter()
                .find(|row| row.facts.key == key)
                .ok_or("missing retained layer root")?;
            if row.facts.initialized_sha256.is_none() || row.upload.is_some() {
                return Err("layer root has incomplete initialization".into());
            }
            Ok(row.token)
        };
        let rank_roots = |rank: u32| -> Result<([B::Buffer; 14], [B::Buffer; 10])> {
            let source = self
                .source
                .layers
                .iter()
                .find(|row| row.rank == rank && row.layer == layer as u32)
                .ok_or("missing source layer")?;
            let weight = |kind| -> Result<B::Buffer> {
                let row = source
                    .weights
                    .iter()
                    .find(|row| row.kind == kind)
                    .ok_or("missing layer weight")?;
                retained(source_key(row.buffer))
            };
            let scratch = |kind| -> Result<B::Buffer> {
                let row = self
                    .source
                    .scratch
                    .iter()
                    .find(|row| row.buffer.rank == rank && row.kind == kind)
                    .ok_or("missing layer scratch")?;
                retained(source_key(row.buffer))
            };
            let pending = |kind, scope| {
                retained(BindingKey::Pending {
                    rank,
                    layer: scope,
                    kind,
                })
            };
            use wire::{PendingKind as P, ScratchKind as S, WeightKind as W};
            let prefix = [
                scratch(S::Hidden)?,
                weight(W::InputLayerNorm)?,
                pending(P::PackedQkvWeight, Some(layer as u32))?,
                pending(P::PackedHeadNormWeight, Some(layer as u32))?,
                pending(P::Rotary, None)?,
                pending(P::CacheMetadata, None)?,
                weight(W::OutputProjection)?,
                scratch(S::Normalized)?,
                pending(P::QkvOutput, None)?,
                scratch(S::Query)?,
                retained(source_key(source.caches[0]))?,
                retained(source_key(source.caches[1]))?,
                scratch(S::Attention)?,
                scratch(S::Partial)?,
            ];
            let mlp = [
                scratch(S::PostAttentionResidual)?,
                weight(W::PostAttentionLayerNorm)?,
                weight(W::GateProjection)?,
                weight(W::UpProjection)?,
                weight(W::DownProjection)?,
                scratch(S::Normalized)?,
                scratch(S::Gate)?,
                scratch(S::Up)?,
                scratch(S::Activation)?,
                scratch(S::Partial)?,
            ];
            Ok((prefix, mlp))
        };
        let (prefix0, mlp0) = rank_roots(0)?;
        let (prefix1, mlp1) = rank_roots(1)?;
        let final_hidden = [prefix0[0], prefix1[0]];
        Ok(LayerBindings {
            prefix: [prefix0, prefix1],
            mlp: [mlp0, mlp1],
            final_hidden,
        })
    }

    fn tail_source_bindings(&self) -> Result<[B::Buffer; 9]> {
        if self.phase != Phase::CatalogBound {
            return Err("tail root resolution requires bound source catalog".into());
        }
        let retained = |buffer: wire::Buffer| -> Result<B::Buffer> {
            let record = self
                .records
                .iter()
                .find(|record| record.facts.key == source_key(buffer))
                .ok_or("tail source role missing from retained catalog")?;
            if record.upload.is_some() || record.facts.initialized_sha256.is_none() {
                return Err("tail source root initialization incomplete".into());
            }
            Ok(record.token)
        };
        let global = |kind| {
            self.source
                .globals
                .iter()
                .find(|row| row.kind == kind)
                .ok_or_else(|| "tail global kind missing".to_string())
                .and_then(|row| retained(row.buffer))
        };
        let auxiliary = |kind| {
            self.source
                .auxiliary
                .iter()
                .find(|row| row.buffer.rank == 0 && row.kind == kind)
                .ok_or_else(|| "tail rank-zero auxiliary missing".to_string())
                .and_then(|row| retained(row.buffer))
        };
        let scratch = |rank, kind| {
            self.source
                .scratch
                .iter()
                .find(|row| row.buffer.rank == rank && row.kind == kind)
                .ok_or_else(|| "tail rank scratch missing".to_string())
                .and_then(|row| retained(row.buffer))
        };
        use wire::{AuxiliaryKind as A, GlobalKind as G, ScratchKind as S};
        Ok([
            auxiliary(A::Token)?,
            global(G::TokenEmbedding)?,
            scratch(0, S::Hidden)?,
            scratch(1, S::Hidden)?,
            global(G::FinalNorm)?,
            auxiliary(A::Empty)?,
            scratch(0, S::Normalized)?,
            auxiliary(A::Logits)?,
            auxiliary(A::Choice)?,
        ])
    }
}

/// Safe owner construction consumes a Group opened by the future separately
/// audited binary opener. No group/token escape or native-open implementation.
pub(super) struct NativeOwner {
    catalog: Catalog<Group>,
    artifacts: Option<LoadedResidentArtifacts>,
    layer_bindings: Vec<LayerBindings>,
    sealed_states: Option<StateRoster>,
    tiles_states: Option<tiles_decode_v1::Roster>,
    tiles_image: Option<tiles_artifacts::Image>,
    tiles_artifacts: Option<tiles_artifacts::LoadedKernels>,
    prefix_states: Option<prefix_tiles_v6::Roster>,
    prefix_decode_states: Option<prefix_tiles_decode_v6::Roster>,
    guarded_states: Option<guarded_states::Roster>,
    guarded_images: Option<guarded_layer::Images>,
    guarded_loaded: Option<guarded_layer::Loaded>,
    guarded_down: Option<[Buffer; 2]>,
    prefix_image: Option<prefix_artifacts::Image>,
    prefix_artifacts: Option<prefix_artifacts::Loaded>,
    tail_manifest: Option<HeadManifest>,
    tail_head: Option<HeadTranspose>,
    tail_artifacts: Option<LoadedTailArtifacts>,
    tail_bindings: Option<TailBindings>,
}

impl NativeOwner {
    pub(super) fn from_reviewed_group(
        group: Group,
        scope: SourceScope,
        registration: wire::Registration,
        registration_bytes: &[u8],
        source_program: &[u8],
    ) -> Result<Self> {
        Ok(Self {
            catalog: Catalog::new(
                group,
                scope,
                registration,
                registration_bytes,
                source_program,
            )?,
            artifacts: None,
            layer_bindings: Vec::new(),
            sealed_states: None,
            tiles_states: None,
            tiles_image: None,
            tiles_artifacts: None,
            prefix_states: None,
            prefix_decode_states: None,
            guarded_states: None,
            guarded_images: None,
            guarded_loaded: None,
            guarded_down: None,
            prefix_image: None,
            prefix_artifacts: None,
            tail_manifest: None,
            tail_head: None,
            tail_artifacts: None,
            tail_bindings: None,
        })
    }
    pub(super) fn allocate_upload(
        &mut self,
        key: BindingKey,
        expected_sha256: [u8; 32],
    ) -> Result<u64> {
        self.catalog.allocate_upload(key, expected_sha256)
    }

    pub(super) fn select_tiles_decode(&mut self, image: tiles_artifacts::Image) -> Result<()> {
        let result = (|| {
            if self.tail_manifest.is_some() || self.tiles_image.is_some() {
                return Err("tiles profile must be selected before setup allocations".into());
            }
            self.catalog.select_tiles_decode()?;
            self.tiles_image = Some(image);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn select_prefix_tiles_decode(
        &mut self,
        prefix: prefix_artifacts::Image,
        mlp: tiles_artifacts::Image,
    ) -> Result<()> {
        let result = (|| {
            if self.tail_manifest.is_some()
                || self.tiles_image.is_some()
                || self.prefix_image.is_some()
            {
                return Err("prefix decode images must precede setup allocations".into());
            }
            self.catalog.select_prefix_tiles_decode()?;
            self.prefix_image = Some(prefix);
            self.tiles_image = Some(mlp);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn select_prefix_tiles_layer(
        &mut self,
        prefix: prefix_artifacts::Image,
        mlp: tiles_artifacts::Image,
    ) -> Result<()> {
        let result = (|| {
            if self.tail_manifest.is_some()
                || self.tiles_image.is_some()
                || self.prefix_image.is_some()
            {
                return Err("prefix layer images must precede setup allocations".into());
            }
            self.catalog.select_prefix_tiles_layer()?;
            self.prefix_image = Some(prefix);
            self.tiles_image = Some(mlp);
            Ok(())
        })();
        self.catalog.finish(result)
    }

    pub(super) fn select_guarded_mlp_decode(
        &mut self,
        prefix: prefix_artifacts::Image,
        mlp: tiles_artifacts::Image,
        guarded: guarded_layer::Images,
        reuse: bool,
    ) -> Result<()> {
        let result = (|| {
            if self.tail_manifest.is_some()
                || self.tiles_image.is_some()
                || self.prefix_image.is_some()
                || self.guarded_images.is_some()
            {
                return Err("guarded images must precede all setup allocations".into());
            }
            self.catalog.select_guarded_mlp_decode()?;
            if reuse {
                self.catalog.profile = ExecutionProfile::GuardedMlpReuseV1;
            }
            self.prefix_image = Some(prefix);
            self.tiles_image = Some(mlp);
            self.guarded_images = Some(guarded);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn write_upload(
        &mut self,
        catalog_id: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.catalog.write_upload(catalog_id, offset, bytes)
    }
    pub(super) fn allocate_zeroed(&mut self, key: BindingKey) -> Result<u64> {
        self.catalog.allocate_zeroed(key)
    }
    pub(super) fn bind_catalog(&mut self) -> Result<Vec<BindingFacts>> {
        if self.tail_manifest.is_some() {
            let result = self
                .tail_head
                .as_ref()
                .ok_or_else(|| "requested tail transpose missing".to_string())
                .and_then(HeadTranspose::buffer);
            self.catalog.finish(result)?;
        }
        self.catalog.bind()
    }
    /// Optional tail is declared before any physical source allocation. This
    /// preflight includes all original/pending roots and both forward state sets.
    pub(super) fn prepare_tail_setup(&mut self, manifest: HeadManifest) -> Result<()> {
        let result = (|| {
            if self.catalog.phase != Phase::Setup
                || !self.catalog.records.is_empty()
                || self.tail_manifest.is_some()
            {
                return Err("tail setup must be declared once before source allocations".into());
            }
            let source = self
                .catalog
                .source
                .globals
                .iter()
                .find(|row| row.kind == wire::GlobalKind::LanguageModelHead)
                .ok_or("tail original head source missing")?
                .buffer;
            if source.rank != 0
                || source.id == 0
                || source.id != manifest.source_id
                || source.elements.checked_mul(u64::from(source.element_bytes)) != Some(HEAD_BYTES)
                || source.element_bytes != 2
                || manifest.source_sha256 == [0; 32]
                || manifest.output_sha256 == [0; 32]
            {
                return Err("tail original head source identity or manifest mismatch".into());
            }
            let occupied =
                self.catalog.backend.preflight_additional_allocations_v1(
                    &if self.catalog.profile == ExecutionProfile::GuardedMlpReuseV1 {
                        guarded_states::REUSE_MAX_COUNTS
                    } else if self.catalog.profile == ExecutionProfile::GuardedMlpDecodeV1 {
                        guarded_states::MAX_COUNTS
                    } else {
                        [714, 710]
                    },
                )?;
            if occupied != [0, 0] {
                return Err("tail setup requires the same fresh native group".into());
            }
            self.tail_manifest = Some(manifest);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn allocate_tail_head(&mut self) -> Result<()> {
        let result = (|| {
            if self.catalog.phase != Phase::Setup || self.tail_head.is_some() {
                return Err("tail transpose allocation is unavailable or already retained".into());
            }
            let manifest = self.tail_manifest.ok_or("tail setup was not declared")?;
            let source = self
                .catalog
                .records
                .iter()
                .find(|row| {
                    row.facts.key
                        == (BindingKey::Source {
                            rank: 0,
                            id: manifest.source_id,
                        })
                })
                .ok_or("tail original head has not been initialized")?;
            if !source.facts.immutable || source.upload.is_some() {
                return Err("tail original head is not a completed immutable upload".into());
            }
            let digest = source
                .facts
                .initialized_sha256
                .ok_or("tail original head digest absent")?;
            let original = source.token;
            self.tail_head = Some(HeadTranspose::allocate(
                &mut self.catalog.backend,
                original,
                manifest.source_id,
                digest,
                manifest,
            )?);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn write_tail_head(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        let result = (|| {
            if self.catalog.phase != Phase::Setup {
                return Err("tail upload outside setup".into());
            }
            self.tail_head
                .as_mut()
                .ok_or("tail transpose is not allocated")?
                .write(&mut self.catalog.backend, offset, bytes)
        })();
        self.catalog.finish(result)
    }
    pub(super) fn load_tail_artifacts(&mut self, images: ReviewedTailImages) -> Result<()> {
        let result = (|| {
            if !matches!(self.catalog.phase, Phase::Setup | Phase::CatalogBound)
                || self.tail_manifest.is_none()
                || self.tail_artifacts.is_some()
            {
                return Err("tail images unavailable, undeclared or already loaded".into());
            }
            self.tail_artifacts = Some(tail_artifacts::load(&mut self.catalog.backend, images)?);
            Ok(())
        })();
        self.catalog.finish(result)
    }
    pub(super) fn load_resident_artifacts(&mut self, images: ReviewedImages) -> Result<()> {
        if !matches!(self.catalog.phase, Phase::Setup | Phase::CatalogBound)
            || self.artifacts.is_some()
        {
            self.catalog.phase = Phase::Terminal;
            return Err("resident artifacts already loaded or owner not available".into());
        }
        match resident_artifacts::load(&mut self.catalog.backend, images) {
            Ok(artifacts) => {
                self.artifacts = Some(artifacts);
                if matches!(
                    self.catalog.profile,
                    ExecutionProfile::TilesDecodeV1
                        | ExecutionProfile::PrefixTilesLayerV6
                        | ExecutionProfile::PrefixTilesDecodeV6
                        | ExecutionProfile::GuardedMlpDecodeV1
                        | ExecutionProfile::GuardedMlpReuseV1
                ) {
                    let result = (|| {
                        let image = self
                            .tiles_image
                            .take()
                            .ok_or("tiles image already consumed")?;
                        self.tiles_artifacts = Some(tiles_artifacts::load_decode_kernels(
                            &mut self.catalog.backend,
                            image,
                        )?);
                        Ok(())
                    })();
                    self.catalog.finish(result)?;
                }
                if matches!(
                    self.catalog.profile,
                    ExecutionProfile::PrefixTilesLayerV6
                        | ExecutionProfile::PrefixTilesDecodeV6
                        | ExecutionProfile::GuardedMlpDecodeV1
                        | ExecutionProfile::GuardedMlpReuseV1
                ) {
                    let result = (|| {
                        let image = self
                            .prefix_image
                            .take()
                            .ok_or("prefix layer image missing")?;
                        self.prefix_artifacts =
                            Some(prefix_artifacts::load(&mut self.catalog.backend, image)?);
                        Ok(())
                    })();
                    self.catalog.finish(result)?;
                }
                Ok(())
            }
            Err(error) => {
                self.catalog.phase = Phase::Terminal;
                Err(error)
            }
        }
    }
    pub(super) fn resident_kernel(
        &mut self,
        rank: usize,
        kind: ResidentKind,
    ) -> Result<&fe2o3_kfd::Gfx950EngineeringPeerKernelV1> {
        if !matches!(
            self.catalog.phase,
            Phase::CatalogBound | Phase::LayersSealed
        ) {
            self.catalog.phase = Phase::Terminal;
            return Err("resident kernel requires bound retained catalog".into());
        }
        let result = self
            .artifacts
            .as_ref()
            .ok_or_else(|| "resident images have not been loaded".to_string())
            .and_then(|artifacts| artifacts.kernel(rank, kind));
        if result.is_err() {
            self.catalog.phase = Phase::Terminal;
        }
        result
    }
    /// Called only after the parent source-owned upload plan has supplied the
    /// expected immutable bytes/digests. This validates actual retained roots
    /// and pinned loaded kernels, not the legacy Program's opaque hash. It seals
    /// only fixed layer state; no embedding, forward begin, launch or Ready is
    /// provided. A later real forward transition must complete metadata/embedding.
    #[allow(unsafe_code)]
    pub(super) fn seal_layer_bindings_after_source_intake(&mut self) -> Result<()> {
        let result = (|| {
            if self.catalog.phase != Phase::CatalogBound
                || self.sealed_states.is_some()
                || self.tiles_states.is_some()
                || self.prefix_states.is_some()
                || self.prefix_decode_states.is_some()
                || self.guarded_states.is_some()
                || !self.layer_bindings.is_empty()
            {
                return Err("layer state is not available for first sealing".into());
            }
            let artifacts = self
                .artifacts
                .as_ref()
                .ok_or("exact resident artifacts not loaded")?;
            for rank in 0..2 {
                for kind in [
                    ResidentKind::Prefix,
                    ResidentKind::Mlp,
                    ResidentKind::Residual,
                ] {
                    if artifacts.kernel(rank, kind)?.rank() != rank {
                        return Err("resident artifact belongs to wrong native rank".into());
                    }
                }
            }
            let mut bindings = Vec::with_capacity(36);
            for layer in 0..36 {
                let roots = self.catalog.layer_bindings(layer)?;
                roots.validate()?;
                bindings.push(roots);
            }
            let tail_bindings = if self.tail_manifest.is_some() {
                let _images = self
                    .tail_artifacts
                    .as_ref()
                    .ok_or("requested tail images are not loaded")?;
                let head = self
                    .tail_head
                    .as_ref()
                    .ok_or("requested tail transpose missing")?;
                let [
                    token,
                    embedding,
                    hidden0,
                    hidden1,
                    norm,
                    empty,
                    normalized,
                    logits,
                    choice,
                ] = self.catalog.tail_source_bindings()?;
                Some(TailBindings::new(
                    token,
                    embedding,
                    [hidden0, hidden1],
                    norm,
                    empty,
                    normalized,
                    head,
                    logits,
                    choice,
                )?)
            } else {
                None
            };
            let pending = self
                .catalog
                .states
                .take()
                .ok_or("typed states were not retained")?;
            match (self.catalog.profile, pending) {
                (ExecutionProfile::BaseV1, PendingExecution::Base(v)) => {
                    self.sealed_states = Some(v.seal(self.catalog.registration_sha256)?);
                }
                (ExecutionProfile::TilesDecodeV1, PendingExecution::Tiles(v)) => {
                    if self.tiles_artifacts.is_none()
                        || self.tiles_image.is_some()
                        || tail_bindings.is_none()
                    {
                        return Err("tiles decode missing exact image or tail".into());
                    }
                    self.tiles_states = Some(v.seal(self.catalog.registration_sha256)?);
                }
                (ExecutionProfile::PrefixTilesLayerV6, PendingExecution::PrefixTiles(v)) => {
                    if self.tiles_artifacts.is_none()
                        || self.prefix_artifacts.is_none()
                        || self.tiles_image.is_some()
                        || self.prefix_image.is_some()
                        || tail_bindings.is_none()
                    {
                        return Err("prefix layer missing distinct images or tail setup".into());
                    }
                    self.prefix_states = Some(v.seal(self.catalog.registration_sha256)?);
                }
                (ExecutionProfile::PrefixTilesDecodeV6, PendingExecution::PrefixDecode(v)) => {
                    if self.tiles_artifacts.is_none()
                        || self.prefix_artifacts.is_none()
                        || self.tiles_image.is_some()
                        || self.prefix_image.is_some()
                        || tail_bindings.is_none()
                    {
                        return Err("prefix decode missing distinct images or tail setup".into());
                    }
                    self.prefix_decode_states = Some(v.seal(self.catalog.registration_sha256)?);
                }
                (
                    ExecutionProfile::GuardedMlpDecodeV1 | ExecutionProfile::GuardedMlpReuseV1,
                    PendingExecution::Guarded(v),
                ) => {
                    if self.prefix_artifacts.is_none()
                        || self.tiles_image.is_some()
                        || self.prefix_image.is_some()
                        || tail_bindings.is_none()
                    {
                        return Err("guarded decode missing exact images or tail".into());
                    }
                    let images = guarded_layer::load(
                        &mut self.catalog.backend,
                        self.guarded_images
                            .take()
                            .ok_or("guarded images already consumed")?,
                    )?;
                    let mlp = self
                        .tiles_artifacts
                        .as_ref()
                        .ok_or("guarded MLP image missing")?;
                    let down =
                        crate::resident_layer::prefix_tiles_decode_v6::allocate_ordered_down(
                            &mut self.catalog.backend,
                            &bindings,
                        )?;
                    let mut inputs = Vec::with_capacity(72);
                    for _ in 0..2 {
                        for roots in &bindings {
                            inputs.push(guarded_layer::inputs(&images, mlp, roots, down)?);
                        }
                    }
                    // SAFETY: the private source-intake path owns the exact image
                    // and model roles. Binding consumes all72 actual unbound pairs;
                    // runtime verifies exact-own-residual identity and custody.
                    let states = unsafe {
                        match self.catalog.profile {
                            ExecutionProfile::GuardedMlpReuseV1 => v.seal_reusable(
                                &mut self.catalog.backend,
                                self.catalog.registration_sha256,
                                &inputs,
                                10_000,
                            )?,
                            _ => v.seal(
                                &mut self.catalog.backend,
                                self.catalog.registration_sha256,
                                &inputs,
                                10_000,
                            )?,
                        }
                    };
                    drop(inputs);
                    self.guarded_states = Some(states);
                    self.guarded_loaded = Some(images);
                    self.guarded_down = Some(down);
                }
                _ => return Err("typed state/profile mismatch".into()),
            }
            self.layer_bindings = bindings;
            self.tail_bindings = tail_bindings;
            Ok(())
        })();
        let result = self.catalog.finish(result);
        if result.is_ok() {
            self.catalog.phase = Phase::LayersSealed;
        }
        result
    }
    pub(super) fn registration_sha256(&self) -> [u8; 32] {
        self.catalog.registration_sha256
    }
    /// # Safety
    /// Called only by the root's bounded forward sequencer after metadata intake
    /// and before begin_states; no other forward may be active. On error this
    /// owner is terminal. This is not a public registration/launch capability.
    #[allow(unsafe_code)]
    pub(super) unsafe fn tail_embedding(
        &mut self,
        token: u32,
        timeout_ms: u32,
    ) -> Result<[u64; 2]> {
        unsafe { self.tail_embedding_with_recording(token, timeout_ms, None) }
    }
    /// # Safety
    /// Same exclusive forward-phase contract as tail_embedding. A recorder
    /// requires the caller's whole forward to use the fresh raw queue mode.
    #[allow(unsafe_code)]
    pub(super) unsafe fn tail_embedding_with_recording(
        &mut self,
        token: u32,
        timeout_ms: u32,
        recording: Option<(
            &mut crate::native_prefix_device_recorder_v1::Recorder,
            u64,
            u32,
        )>,
    ) -> Result<[u64; 2]> {
        let result = (|| {
            if self.catalog.phase != Phase::LayersSealed {
                return Err("tail embedding before source seal".into());
            }
            let artifacts = self
                .tail_artifacts
                .as_ref()
                .ok_or("tail artifacts missing")?;
            let bindings = self.tail_bindings.as_ref().ok_or("tail bindings missing")?;
            // SAFETY: caller supplies the additional forward-phase contract.
            unsafe {
                if let Some((recorder, generation, position)) = recording {
                    crate::tail_bindings::begin_embedding_recorded(
                        &mut self.catalog.backend,
                        artifacts,
                        bindings,
                        token,
                        timeout_ms,
                        recorder,
                        generation,
                        position,
                    )
                } else {
                    crate::tail_bindings::begin_embedding(
                        &mut self.catalog.backend,
                        artifacts,
                        bindings,
                        token,
                        timeout_ms,
                    )
                }
            }
        })();
        self.catalog.finish(result)
    }
    /// # Safety
    /// Called only after the same forward's 36 paired layers completed, before
    /// idle-fence/commit/publication. The result is still internal retained data.
    #[allow(unsafe_code)]
    pub(super) unsafe fn tail_finish(&mut self, timeout_ms: u32) -> Result<(u32, [u64; 3])> {
        unsafe { self.tail_finish_with_recording(timeout_ms, None) }
    }
    /// # Safety
    /// Same completed paired-layer and unpublished-output contract as tail_finish.
    #[allow(unsafe_code)]
    pub(super) unsafe fn tail_finish_with_recording(
        &mut self,
        timeout_ms: u32,
        recording: Option<(
            &mut crate::native_prefix_device_recorder_v1::Recorder,
            u64,
            u32,
        )>,
    ) -> Result<(u32, [u64; 3])> {
        let result = (|| {
            if self.catalog.phase != Phase::LayersSealed {
                return Err("tail execution before source seal".into());
            }
            let artifacts = self
                .tail_artifacts
                .as_ref()
                .ok_or("tail artifacts missing")?;
            let bindings = self.tail_bindings.as_ref().ok_or("tail bindings missing")?;
            // SAFETY: caller supplies the additional forward-phase contract.
            let result = unsafe {
                if let Some((recorder, generation, position)) = recording {
                    crate::tail_bindings::finish_tail_recorded(
                        &mut self.catalog.backend,
                        artifacts,
                        bindings,
                        timeout_ms,
                        recorder,
                        generation,
                        position,
                    )
                } else {
                    crate::tail_bindings::finish_tail(
                        &mut self.catalog.backend,
                        artifacts,
                        bindings,
                        timeout_ms,
                    )
                }
            }?;
            Ok((result.token, result.timing_ns))
        })();
        self.catalog.finish(result)
    }
    pub(super) fn close_setup(&mut self) -> Result<()> {
        self.catalog.close()
    }
}

#[cfg(test)]
#[path = "native_catalog_tests.rs"]
mod tests;

#[allow(unsafe_code)]
#[path = "native_forward.rs"]
pub(super) mod forward;
