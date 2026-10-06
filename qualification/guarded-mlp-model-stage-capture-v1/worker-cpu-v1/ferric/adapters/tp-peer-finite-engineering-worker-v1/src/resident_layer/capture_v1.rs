//! Opt-in position-zero/layer-zero observations, never numerical acceptance.

use super::{Allocation, Buffer, Group, LayerBindings, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(crate) const PAYLOAD_BYTES: usize = 256_136;
pub(crate) const PARTS: usize = 34;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Boundary {
    BeforePrefix,
    AfterPrefix,
    AfterFirstResidual,
    AfterMlp,
    AfterFinalResidual,
}

const BOUNDARIES: [Boundary; 5] = [
    Boundary::BeforePrefix,
    Boundary::AfterPrefix,
    Boundary::AfterFirstResidual,
    Boundary::AfterMlp,
    Boundary::AfterFinalResidual,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Input,
    CacheMetadata,
    Rotary,
    InputNormalized,
    RawQkv,
    Query,
    CurrentKey,
    CurrentValue,
    Attention,
    OutputPartial,
    FirstResidual,
    PostNormalized,
    Gate,
    Up,
    Activation,
    DownPartial,
    FinalHidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Scalar {
    Bf16,
    F32,
    U32,
}

impl Scalar {
    fn bytes(self) -> u32 {
        match self {
            Self::Bf16 => 2,
            Self::F32 | Self::U32 => 4,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Part {
    boundary: Boundary,
    rank: u32,
    role: Role,
    scalar: Scalar,
    elements: u32,
    offset: u32,
    bytes: u32,
    source_byte_offset: u64,
    sha256: [u8; 32],
}

/// Fixed stage-major, then rank-major, then role order. All numeric payloads
/// retain their original little-endian bits. This is not a full-cache readback.
#[derive(Debug, Serialize)]
pub(crate) struct Layer0CaptureV1 {
    schema: &'static str,
    generation: u64,
    position: u32,
    layer: u32,
    parts: Vec<Part>,
    payload_bytes: u32,
    payload_sha256: [u8; 32],
    payload: Vec<u8>,
    full_cache_capture: bool,
    native_close_confirmed: bool,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}

impl Layer0CaptureV1 {
    pub(crate) fn after_close(mut self, close: impl FnOnce() -> Result<()>) -> Result<Self> {
        close()?;
        self.native_close_confirmed = true;
        Ok(self)
    }
}

#[derive(Clone, Copy)]
enum Root {
    Prefix(usize),
    Mlp(usize),
    Final,
}

#[derive(Clone, Copy)]
struct Spec {
    role: Role,
    root: Root,
    scalar: Scalar,
    bytes: u32,
}

const fn spec(role: Role, root: Root, scalar: Scalar, bytes: u32) -> Spec {
    Spec {
        role,
        root,
        scalar,
        bytes,
    }
}

const BEFORE: [Spec; 3] = [
    spec(Role::Input, Root::Prefix(0), Scalar::Bf16, 8192),
    spec(Role::CacheMetadata, Root::Prefix(5), Scalar::U32, 580),
    spec(Role::Rotary, Root::Prefix(4), Scalar::F32, 512),
];
const PREFIX: [Spec; 7] = [
    spec(Role::InputNormalized, Root::Prefix(7), Scalar::Bf16, 8192),
    spec(Role::RawQkv, Root::Prefix(8), Scalar::Bf16, 6144),
    spec(Role::Query, Root::Prefix(9), Scalar::Bf16, 4096),
    spec(Role::CurrentKey, Root::Prefix(10), Scalar::Bf16, 1024),
    spec(Role::CurrentValue, Root::Prefix(11), Scalar::Bf16, 1024),
    spec(Role::Attention, Root::Prefix(12), Scalar::Bf16, 4096),
    spec(Role::OutputPartial, Root::Prefix(13), Scalar::F32, 16_384),
];
const FIRST: [Spec; 1] = [spec(Role::FirstResidual, Root::Mlp(0), Scalar::Bf16, 8192)];
const MLP: [Spec; 5] = [
    spec(Role::PostNormalized, Root::Mlp(5), Scalar::Bf16, 8192),
    spec(Role::Gate, Root::Mlp(6), Scalar::Bf16, 12_288),
    spec(Role::Up, Root::Mlp(7), Scalar::Bf16, 12_288),
    spec(Role::Activation, Root::Mlp(8), Scalar::Bf16, 12_288),
    spec(Role::DownPartial, Root::Mlp(9), Scalar::F32, 16_384),
];
const LAST: [Spec; 1] = [spec(Role::FinalHidden, Root::Final, Scalar::Bf16, 8192)];

fn specs(boundary: Boundary) -> &'static [Spec] {
    match boundary {
        Boundary::BeforePrefix => &BEFORE,
        Boundary::AfterPrefix => &PREFIX,
        Boundary::AfterFirstResidual => &FIRST,
        Boundary::AfterMlp => &MLP,
        Boundary::AfterFinalResidual => &LAST,
    }
}

fn finite(scalar: Scalar, bytes: &[u8]) -> Result<()> {
    let valid = match scalar {
        Scalar::Bf16 => bytes
            .chunks_exact(2)
            .all(|v| u16::from_le_bytes([v[0], v[1]]) & 0x7f80 != 0x7f80),
        Scalar::F32 => bytes
            .chunks_exact(4)
            .all(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]]).is_finite()),
        Scalar::U32 => true,
    };
    if valid {
        Ok(())
    } else {
        Err("nonfinite layer-zero diagnostic value".into())
    }
}

fn cache_offset(metadata: &[u8]) -> Result<u64> {
    if metadata.len() != 580 || metadata[..4] != [0; 4] {
        return Err("layer-zero diagnostic requires actual position zero".into());
    }
    let mut seen = [false; 144];
    for word in metadata[4..].chunks_exact(4) {
        let page = u32::from_le_bytes([word[0], word[1], word[2], word[3]]) as usize;
        let slot = seen.get_mut(page).ok_or("diagnostic page bounds")?;
        if *slot {
            return Err("diagnostic page alias".into());
        }
        *slot = true;
    }
    let page = u32::from_le_bytes(
        metadata[4..8]
            .try_into()
            .map_err(|_| "diagnostic page extent")?,
    );
    Ok(u64::from(page) * 16 * 512 * 2)
}

pub(super) trait Reader<B: Allocation> {
    fn read(&mut self, buffer: B, offset: u64, bytes: u32) -> Result<Vec<u8>>;
}

impl Reader<Buffer> for Group {
    fn read(&mut self, buffer: Buffer, offset: u64, bytes: u32) -> Result<Vec<u8>> {
        Group::read(self, buffer, offset, bytes)
    }
}

struct GuardedDownReader<'a, B, R> {
    reader: &'a mut R,
    old: [B; 2],
    actual: [B; 2],
}

impl<B: Allocation, R: Reader<B>> Reader<B> for GuardedDownReader<'_, B, R> {
    fn read(&mut self, buffer: B, offset: u64, bytes: u32) -> Result<Vec<u8>> {
        let selected = self.old.iter().position(|token| *token == buffer);
        if let Some(rank) = selected {
            if offset != 0 || bytes != 16_384 {
                return Err("guarded capture exact Down extent".into());
            }
            self.reader.read(self.actual[rank], offset, bytes)
        } else {
            self.reader.read(buffer, offset, bytes)
        }
    }
}

/// Private append-only collector. Any error permanently prevents publication.
pub(crate) struct Collector {
    next: usize,
    terminal: bool,
    selected_offset: [Option<u64>; 2],
    parts: Vec<Part>,
    payload: Vec<u8>,
}

impl Collector {
    pub(crate) fn new(generation: u64, position: u32, layer: usize) -> Result<Self> {
        if (generation, position, layer) != (1, 0, 0) {
            return Err(
                "diagnostic capture is only generation-one/position-zero/layer-zero".into(),
            );
        }
        Ok(Self {
            next: 0,
            terminal: false,
            selected_offset: [None; 2],
            parts: Vec::with_capacity(PARTS),
            payload: Vec::with_capacity(PAYLOAD_BYTES),
        })
    }

    pub(super) fn observe<B: Allocation>(
        &mut self,
        boundary: Boundary,
        reader: &mut impl Reader<B>,
        roots: &LayerBindings<B>,
    ) -> Result<()> {
        let result = (|| {
            if self.terminal || BOUNDARIES.get(self.next) != Some(&boundary) {
                return Err("layer-zero diagnostic boundary order or terminal collector".into());
            }
            roots.validate()?;
            for rank in 0..2 {
                for spec in specs(boundary) {
                    let root = match spec.root {
                        Root::Prefix(index) => roots.prefix[rank][index],
                        Root::Mlp(index) => roots.mlp[rank][index],
                        Root::Final => roots.final_hidden[rank],
                    };
                    let offset = if matches!(spec.role, Role::CurrentKey | Role::CurrentValue) {
                        self.selected_offset[rank]
                            .ok_or("diagnostic cache metadata was not captured")?
                    } else {
                        0
                    };
                    if offset
                        .checked_add(u64::from(spec.bytes))
                        .is_none_or(|end| end > root.bytes())
                    {
                        return Err("diagnostic buffer bounds".into());
                    }
                    let data = reader.read(root, offset, spec.bytes)?;
                    if data.len() != spec.bytes as usize
                        || self.parts.len() >= PARTS
                        || self
                            .payload
                            .len()
                            .checked_add(data.len())
                            .is_none_or(|n| n > PAYLOAD_BYTES)
                    {
                        return Err("diagnostic exact bounded readback extent".into());
                    }
                    finite(spec.scalar, &data)?;
                    if spec.role == Role::CacheMetadata {
                        self.selected_offset[rank] = Some(cache_offset(&data)?);
                    }
                    self.parts.push(Part {
                        boundary,
                        rank: rank as u32,
                        role: spec.role,
                        scalar: spec.scalar,
                        elements: spec.bytes / spec.scalar.bytes(),
                        offset: self.payload.len() as u32,
                        bytes: spec.bytes,
                        source_byte_offset: offset,
                        sha256: Sha256::digest(&data).into(),
                    });
                    self.payload.extend_from_slice(&data);
                }
            }
            self.next += 1;
            Ok(())
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }

    pub(super) fn observe_guarded<B: Allocation>(
        &mut self,
        boundary: Boundary,
        reader: &mut impl Reader<B>,
        roots: &LayerBindings<B>,
        down: [B; 2],
    ) -> Result<()> {
        let result = (|| {
            super::guarded_mlp_decode_v1::selected_mlp(roots, down)?;
            if boundary == Boundary::AfterMlp {
                // Guarded Down has dedicated storage. Keep the validated legacy
                // bindings intact and redirect only these two exact readbacks.
                let mut selected = GuardedDownReader {
                    reader,
                    old: [roots.mlp[0][9], roots.mlp[1][9]],
                    actual: down,
                };
                self.observe(boundary, &mut selected, roots)
            } else {
                self.observe(boundary, reader, roots)
            }
        })();
        if result.is_err() {
            self.terminal = true;
        }
        result
    }

    pub(crate) fn finish(self) -> Result<Layer0CaptureV1> {
        if self.terminal
            || self.next != BOUNDARIES.len()
            || self.parts.len() != PARTS
            || self.payload.len() != PAYLOAD_BYTES
        {
            return Err("incomplete or terminal layer-zero diagnostic".into());
        }
        Ok(Layer0CaptureV1 {
            schema: "FerricFiniteLayerZeroCaptureV1",
            generation: 1,
            position: 0,
            layer: 0,
            parts: self.parts,
            payload_bytes: PAYLOAD_BYTES as u32,
            payload_sha256: Sha256::digest(&self.payload).into(),
            payload: self.payload,
            full_cache_capture: false,
            native_close_confirmed: false,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        })
    }
}

#[cfg(test)]
mod tests;
