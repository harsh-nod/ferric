//! Shared source-only wire: parent and child compile this without KFD imports.
//! All IDs and digests below are descriptions, never imported native authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{self, Read, Write};

pub const PROTOCOL: u32 = 1;
pub const PROFILE: &str = "qwen3-8b-tp2-finite-prefix-v5-mlp-v1-two-forward-context2304-v1";
pub const MAX_HEADER: usize = 65_536;
pub const MAX_REGISTRATION: usize = 4 * 1024 * 1024;
pub const MAX_TRANSFER: usize = 4 * 1024 * 1024;
pub const MAX_IMAGE: usize = 64 * 1024 * 1024;

fn check(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    PrefixV5,
    MlpV1,
    OrderedResidualV18,
    OrdinaryTail,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Setup {
    Allocate {
        bytes: u64,
        peer_readable: bool,
    },
    Write {
        buffer: u64,
        offset: u64,
        bytes: u32,
        sha256: [u8; 32],
    },
    LoadImage {
        kind: ImageKind,
        bytes: u32,
        sha256: [u8; 32],
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Setup {
        id: u64,
        rank: u32,
        command: Setup,
    },
    Register {
        id: u64,
        bytes: u32,
        sha256: [u8; 32],
    },
    Execute {
        id: u64,
        registration_sha256: [u8; 32],
        generation: u64,
        token: u32,
        position: u32,
        cache_metadata: Vec<u32>,
        rotary_bits: Vec<u32>,
    },
    Close {
        id: u64,
        registration_sha256: [u8; 32],
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u32,
    pub profile: String,
    pub device_ids: [u64; 2],
    pub request: Operation,
}

impl Request {
    pub fn id(&self) -> u64 {
        match &self.request {
            Operation::Setup { id, .. }
            | Operation::Register { id, .. }
            | Operation::Execute { id, .. }
            | Operation::Close { id, .. } => *id,
        }
    }

    pub fn payload_bytes(&self) -> io::Result<usize> {
        check(
            self.protocol == PROTOCOL
                && self.profile == PROFILE
                && self.id() != 0
                && self.device_ids[0] != 0
                && self.device_ids[1] != 0
                && self.device_ids[0] != self.device_ids[1],
            "finite envelope identity",
        )?;
        match &self.request {
            Operation::Setup { rank, command, .. } => {
                check(*rank < 2, "finite setup rank")?;
                match command {
                    Setup::Allocate { bytes, .. } => {
                        check(
                            *bytes > 0 && *bytes <= 2 * 1024 * 1024 * 1024,
                            "finite allocation extent",
                        )?;
                        Ok(0)
                    }
                    Setup::Write {
                        buffer,
                        offset,
                        bytes,
                        sha256,
                    } => {
                        check(
                            *buffer != 0
                                && *bytes > 0
                                && *bytes as usize <= MAX_TRANSFER
                                && offset.checked_add(u64::from(*bytes)).is_some()
                                && *sha256 != [0; 32],
                            "finite bounded write",
                        )?;
                        Ok(*bytes as usize)
                    }
                    Setup::LoadImage { bytes, sha256, .. } => {
                        check(
                            *bytes > 0 && *bytes as usize <= MAX_IMAGE && *sha256 != [0; 32],
                            "finite bounded image",
                        )?;
                        Ok(*bytes as usize)
                    }
                }
            }
            Operation::Register { bytes, sha256, .. } => {
                check(
                    *bytes > 0 && *bytes as usize <= MAX_REGISTRATION && *sha256 != [0; 32],
                    "finite bounded registration",
                )?;
                Ok(*bytes as usize)
            }
            Operation::Execute {
                registration_sha256,
                generation,
                token,
                position,
                cache_metadata,
                rotary_bits,
                ..
            } => {
                check(
                    *registration_sha256 != [0; 32]
                        && *generation == u64::from(*position) + 1
                        && *position < 2
                        && *token < 151_936,
                    "finite two-forward execution identity",
                )?;
                check(
                    cache_metadata.len() == 145
                        && cache_metadata[0] == *position
                        && cache_metadata[1..].iter().all(|page| *page < 144)
                        && cache_metadata[1..].iter().collect::<BTreeSet<_>>().len() == 144,
                    "finite complete page permutation",
                )?;
                check(
                    rotary_bits.len() == 128
                        && rotary_bits
                            .iter()
                            .all(|bits| bits & 0x7f80_0000 != 0x7f80_0000),
                    "finite exact F32 cosine/sine payload",
                )?;
                Ok(0)
            }
            Operation::Close {
                registration_sha256,
                ..
            } => {
                check(*registration_sha256 != [0; 32], "finite close identity")?;
                Ok(0)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightKind {
    InputLayerNorm,
    QueryProjection,
    KeyProjection,
    ValueProjection,
    QueryNorm,
    KeyNorm,
    OutputProjection,
    PostAttentionLayerNorm,
    GateProjection,
    UpProjection,
    DownProjection,
}

pub const WEIGHTS: [(WeightKind, u64); 11] = [
    (WeightKind::InputLayerNorm, 4096),
    (WeightKind::QueryProjection, 2048 * 4096),
    (WeightKind::KeyProjection, 512 * 4096),
    (WeightKind::ValueProjection, 512 * 4096),
    (WeightKind::QueryNorm, 128),
    (WeightKind::KeyNorm, 128),
    (WeightKind::OutputProjection, 4096 * 2048),
    (WeightKind::PostAttentionLayerNorm, 4096),
    (WeightKind::GateProjection, 6144 * 4096),
    (WeightKind::UpProjection, 6144 * 4096),
    (WeightKind::DownProjection, 4096 * 6144),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buffer {
    pub rank: u32,
    pub id: u64,
    pub elements: u64,
    pub element_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weight {
    pub kind: WeightKind,
    pub buffer: Buffer,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub rank: u32,
    pub layer: u32,
    pub weights: Vec<Weight>,
    pub caches: [Buffer; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalKind {
    TokenEmbedding,
    FinalNorm,
    LanguageModelHead,
}

pub const GLOBALS: [(GlobalKind, u64); 3] = [
    (GlobalKind::TokenEmbedding, 151_936 * 4096),
    (GlobalKind::FinalNorm, 4096),
    (GlobalKind::LanguageModelHead, 151_936 * 4096),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Global {
    pub kind: GlobalKind,
    pub buffer: Buffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuxiliaryKind {
    QueryProjection,
    KeyProjection,
    ValueProjection,
    QueryNormalized,
    KeyNormalized,
    KeyRotated,
    Cosine,
    Sine,
    Empty,
    Token,
    Logits,
    Choice,
    Positions,
    PageTable,
}

/// Exact rank-local ordinary source buffers; scalar kinds follow the enum role.
/// Empty is a zero-length logical view, never permission for a null device root.
pub const fn auxiliary_roster(rank: u32) -> [(AuxiliaryKind, u64, u32); 14] {
    use AuxiliaryKind as A;
    [
        (A::QueryProjection, 32768, 2),
        (A::KeyProjection, 8192, 2),
        (A::ValueProjection, 8192, 2),
        (A::QueryNormalized, 32768, 2),
        (A::KeyNormalized, 8192, 2),
        (A::KeyRotated, 8192, 2),
        (A::Cosine, 1024, 4),
        (A::Sine, 1024, 4),
        (A::Empty, 0, 2),
        (A::Token, 16, 4),
        (A::Logits, if rank == 0 { 2_430_976 } else { 1 }, 2),
        (A::Choice, 16, 4),
        (A::Positions, 16, 4),
        (A::PageTable, 2304, 4),
    ]
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auxiliary {
    pub kind: AuxiliaryKind,
    pub buffer: Buffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScratchKind {
    Hidden,
    PostAttentionResidual,
    Normalized,
    Query,
    Attention,
    Gate,
    Up,
    Activation,
    Partial,
}

pub const SCRATCH: [(ScratchKind, u64, u32); 9] = [
    (ScratchKind::Hidden, 4096, 2),
    (ScratchKind::PostAttentionResidual, 4096, 2),
    (ScratchKind::Normalized, 4096, 2),
    (ScratchKind::Query, 2048, 2),
    (ScratchKind::Attention, 2048, 2),
    (ScratchKind::Gate, 6144, 2),
    (ScratchKind::Up, 6144, 2),
    (ScratchKind::Activation, 6144, 2),
    (ScratchKind::Partial, 4096, 4),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scratch {
    pub kind: ScratchKind,
    pub buffer: Buffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingKind {
    PackedQkvWeight,
    PackedHeadNormWeight,
    QkvOutput,
    Rotary,
    CacheMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingBuffer {
    pub rank: u32,
    pub layer: Option<u32>,
    pub kind: PendingKind,
    pub elements: u64,
    pub element_bytes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    PrefixV5,
    MlpV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateSlot {
    pub forward: u32,
    pub layer: u32,
    pub rank: u32,
    pub kind: StateKind,
    pub atomic_words: u32,
}

/// Prepare-only source description. Missing packed buffers, native states and
/// artifact/dataflow joins deliberately make this unusable as a launch program.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub profile: String,
    pub bundle_id: [u8; 32],
    pub model_id: [u8; 32],
    pub session: [u8; 32],
    pub pool_identity: u64,
    pub group_id: u64,
    pub child_identity: u32,
    pub layers: Vec<Layer>,
    pub globals: Vec<Global>,
    pub auxiliary: Vec<Auxiliary>,
    pub scratch: Vec<Scratch>,
    pub pending_buffers: Vec<PendingBuffer>,
    pub state_slots: Vec<StateSlot>,
    pub source_program_bytes: u32,
    pub source_program_sha256: [u8; 32],
}

impl Registration {
    pub fn validate(&self) -> io::Result<()> {
        check(
            self.profile == PROFILE
                && self.bundle_id != [0; 32]
                && self.model_id != [0; 32]
                && self.session != [0; 32]
                && self.pool_identity != 0
                && self.child_identity != 0
                && self.source_program_bytes > 0
                && self.source_program_bytes as usize <= MAX_REGISTRATION
                && self.source_program_sha256 != [0; 32],
            "finite inert registration identity",
        )?;
        check(
            self.layers.len() == 72
                && self.globals.len() == 3
                && self.auxiliary.len() == 28
                && self.scratch.len() == 18
                && self.pending_buffers.len() == 150
                && self.state_slots.len() == 288,
            "finite exact rosters",
        )?;
        let mut seen = BTreeSet::new();
        let mut bind = |buffer: Buffer, rank: u32, elements: u64, width: u32| {
            check(
                buffer.rank == rank
                    && buffer.id != 0
                    && buffer.elements == elements
                    && buffer.element_bytes == width
                    && seen.insert((rank, buffer.id)),
                "finite source buffer role, extent or alias",
            )
        };
        for (index, layer) in self.layers.iter().enumerate() {
            let rank = (index / 36) as u32;
            check(
                layer.rank == rank
                    && layer.layer == (index % 36) as u32
                    && layer.weights.len() == WEIGHTS.len(),
                "finite ordered rank/layer weights",
            )?;
            for (weight, (kind, elements)) in layer.weights.iter().zip(WEIGHTS) {
                check(weight.kind == kind, "finite original weight role")?;
                bind(weight.buffer, rank, elements, 2)?;
            }
            for cache in layer.caches {
                bind(cache, rank, 2304 * 512, 2)?;
            }
        }
        for (index, scratch) in self.scratch.iter().enumerate() {
            let (kind, elements, width) = SCRATCH[index % 9];
            check(scratch.kind == kind, "finite scratch role order")?;
            bind(scratch.buffer, (index / 9) as u32, elements, width)?;
        }
        for (global, (kind, elements)) in self.globals.iter().zip(GLOBALS) {
            check(global.kind == kind, "finite global role order")?;
            bind(global.buffer, 0, elements, 2)?;
        }
        for (rank, auxiliary) in self.auxiliary.chunks_exact(14).enumerate() {
            for (binding, (kind, elements, width)) in
                auxiliary.iter().zip(auxiliary_roster(rank as u32))
            {
                check(binding.kind == kind, "finite auxiliary role order")?;
                bind(binding.buffer, rank as u32, elements, width)?;
            }
        }
        let mut expected_pending = Vec::with_capacity(150);
        for rank in 0..2 {
            for layer in 0..36 {
                for (kind, elements) in [
                    (PendingKind::PackedQkvWeight, 3072 * 4096),
                    (PendingKind::PackedHeadNormWeight, 256),
                ] {
                    expected_pending.push(PendingBuffer {
                        rank,
                        layer: Some(layer),
                        kind,
                        elements,
                        element_bytes: 2,
                    });
                }
            }
            for (kind, elements, element_bytes) in [
                (PendingKind::QkvOutput, 3072, 2),
                (PendingKind::Rotary, 128, 4),
                (PendingKind::CacheMetadata, 145, 4),
            ] {
                expected_pending.push(PendingBuffer {
                    rank,
                    layer: None,
                    kind,
                    elements,
                    element_bytes,
                });
            }
        }
        check(
            self.pending_buffers == expected_pending,
            "finite pending buffer requirements",
        )?;
        let mut slots = BTreeSet::new();
        for slot in &self.state_slots {
            check(
                slot.forward < 2
                    && slot.layer < 36
                    && slot.rank < 2
                    && slot.atomic_words
                        == match slot.kind {
                            StateKind::PrefixV5 => 22,
                            StateKind::MlpV1 => 11,
                        }
                    && slots.insert((slot.forward, slot.layer, slot.rank, slot.kind)),
                "finite logical state tuple or atomic protocol",
            )?;
        }
        // Exhaustive unique tuple coverage, independent of parent/native ordering.
        check(slots.len() == 288, "finite full state use-site set")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    NativeBindingMissing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: u32,
    pub id: u64,
    pub reason: Refusal,
    pub native_opened: bool,
    pub gpu_execution: bool,
    pub production_authority: bool,
}

pub fn refuse_unbound_request(request: &Request, payload: &[u8]) -> io::Result<Response> {
    check(
        request.payload_bytes()? == payload.len(),
        "finite exact payload length",
    )?;
    let expected = match &request.request {
        Operation::Setup {
            command: Setup::Write { sha256, .. } | Setup::LoadImage { sha256, .. },
            ..
        }
        | Operation::Register { sha256, .. } => Some(sha256),
        _ => None,
    };
    if let Some(expected) = expected {
        check(
            <[u8; 32]>::from(Sha256::digest(payload)) == *expected,
            "finite payload digest",
        )?;
    }
    if matches!(request.request, Operation::Register { .. }) {
        let registration: Registration =
            serde_json::from_slice(payload).map_err(io::Error::other)?;
        registration.validate()?;
    }
    Ok(Response {
        protocol: PROTOCOL,
        id: request.id(),
        reason: Refusal::NativeBindingMissing,
        native_opened: false,
        gpu_execution: false,
        production_authority: false,
    })
}

fn read_header<T: serde::de::DeserializeOwned>(input: &mut impl Read) -> io::Result<Option<T>> {
    let mut prefix = [0; 4];
    if input.read(&mut prefix[..1])? == 0 {
        return Ok(None);
    }
    input.read_exact(&mut prefix[1..])?;
    let length = u32::from_le_bytes(prefix) as usize;
    check(length > 0 && length <= MAX_HEADER, "finite header bound")?;
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

struct Header(Vec<u8>);
impl Write for Header {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        check(
            self.0
                .len()
                .checked_add(bytes.len())
                .is_some_and(|size| size <= MAX_HEADER),
            "finite encoded header bound",
        )?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_header(output: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let mut header = Header(Vec::new());
    serde_json::to_writer(&mut header, value).map_err(io::Error::other)?;
    output.write_all(&(header.0.len() as u32).to_le_bytes())?;
    output.write_all(&header.0)
}

pub fn read_request(input: &mut impl Read) -> io::Result<Option<(Request, Vec<u8>)>> {
    let Some(request) = read_header::<Request>(input)? else {
        return Ok(None);
    };
    let mut payload = vec![0; request.payload_bytes()?];
    input.read_exact(&mut payload)?;
    Ok(Some((request, payload)))
}

pub fn write_request(output: &mut impl Write, request: &Request, payload: &[u8]) -> io::Result<()> {
    check(
        request.payload_bytes()? == payload.len(),
        "finite write payload length",
    )?;
    write_header(output, request)?;
    output.write_all(payload)?;
    output.flush()
}

pub fn read_response(input: &mut impl Read) -> io::Result<Option<Response>> {
    let value = read_header::<Response>(input)?;
    if let Some(value) = &value {
        check(
            value.protocol == PROTOCOL
                && value.id > 0
                && !value.native_opened
                && !value.gpu_execution
                && !value.production_authority,
            "finite refusal-only response",
        )?;
    }
    Ok(value)
}

pub fn write_response(output: &mut impl Write, response: &Response) -> io::Result<()> {
    check(
        response.protocol == PROTOCOL
            && response.id > 0
            && !response.native_opened
            && !response.gpu_execution
            && !response.production_authority,
        "finite refusal-only response",
    )?;
    write_header(output, response)?;
    output.flush()
}
