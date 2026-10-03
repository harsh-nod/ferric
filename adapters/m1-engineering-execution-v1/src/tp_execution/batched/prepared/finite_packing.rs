//! Setup-only uploads from authenticated original BF16 tensors. No GPU access.
//! QKV/head-norm concatenation preserves bytes; MLP/O remain original NxK.

use super::finite_composition::EngineeringTp2FiniteCompositionV1 as Composition;
use crate::tp_execution::{TpResult, section_bytes};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_engine::tensor_parallel::{Qwen3TensorParallelPlanV1, Qwen3TensorParallelTensorV1};
use ferric_spec::{QWEN3_NO_LAYER, Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata};
use sha2::{Digest, Sha256};

#[path = "finite_head_transpose.rs"]
mod head_transpose;
pub use head_transpose::EngineeringTp2FiniteHeadTransposeV1;

const MAX_CHUNK: usize = 4 * 1024 * 1024;
const LAYER_KINDS: [Qwen3TensorKind; 11] = [
    Qwen3TensorKind::InputLayerNorm,
    Qwen3TensorKind::QueryProjection,
    Qwen3TensorKind::KeyProjection,
    Qwen3TensorKind::ValueProjection,
    Qwen3TensorKind::QueryNorm,
    Qwen3TensorKind::KeyNorm,
    Qwen3TensorKind::OutputProjection,
    Qwen3TensorKind::PostAttentionLayerNorm,
    Qwen3TensorKind::GateProjection,
    Qwen3TensorKind::UpProjection,
    Qwen3TensorKind::DownProjection,
];
const GLOBAL_KINDS: [Qwen3TensorKind; 3] = [
    Qwen3TensorKind::TokenEmbedding,
    Qwen3TensorKind::FinalNorm,
    Qwen3TensorKind::LanguageModelHead,
];

/// Semantic upload destination; existing IDs are source IDs, not new child IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2FiniteUploadKeyV1 {
    /// Original NxK or replicated BF16 section. Globals use QWEN3_NO_LAYER.
    Source {
        rank: u32,
        id: u64,
        layer: u32,
        kind: Qwen3TensorKind,
    },
    /// New per-layer Q || K || V compact row-major allocation.
    PackedQkv { rank: u32, layer: u32 },
    /// New per-layer Q-norm || K-norm allocation.
    PackedHeadNorm { rank: u32, layer: u32 },
}

impl EngineeringTp2FiniteUploadKeyV1 {
    /// Logical rank; native owner independently maps this key to a fresh token.
    #[must_use]
    pub const fn rank(self) -> u32 {
        match self {
            Self::Source { rank, .. }
            | Self::PackedQkv { rank, .. }
            | Self::PackedHeadNorm { rank, .. } => rank,
        }
    }
}

/// Original authenticated section and exact compact source window, without pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTp2FiniteSourcePartV1 {
    /// Exact tensor coordinate retained by the authentic layout.
    pub metadata: Qwen3TensorMetadata,
    /// Digest of the full source tensor section, not of its TP shard.
    pub source_sha256: [u8; 32],
    /// Original rows and columns before sharding.
    pub source_dimensions: [u32; 2],
    /// First row and row count in the original source.
    pub rows: [u32; 2],
    /// First column and column count in the original source.
    pub columns: [u32; 2],
}

#[derive(Clone, Copy)]
struct Part<'a> {
    metadata: Qwen3TensorMetadata,
    digest: [u8; 32],
    source: &'a [u8],
    shard: Qwen3TensorParallelTensorV1,
}

impl Part<'_> {
    fn bytes(&self) -> TpResult<usize> {
        usize::try_from(
            u64::from(self.shard.rows().count) * u64::from(self.shard.columns().count) * 2,
        )
        .map_err(|_| "finite weight byte extent overflow".into())
    }

    fn check(&self) -> TpResult<()> {
        if self.metadata.role != Qwen3ModelRole::Target8B
            || self.shard.world_size() != 2
            || self.shard.rank() >= 2
            || self.shard.rows().count == 0
            || self.shard.columns().count == 0
        {
            return Err("finite weight source role or shard geometry".into());
        }
        self.shard
            .source_row_bytes(self.shard.rows().count - 1, self.source.len() as u64)
            .map_err(|error| format!("finite original source extent: {error:?}"))?;
        Ok(())
    }

    fn evidence(&self) -> EngineeringTp2FiniteSourcePartV1 {
        EngineeringTp2FiniteSourcePartV1 {
            metadata: self.metadata,
            source_sha256: self.digest,
            source_dimensions: [self.shard.source_rows(), self.shard.source_columns()],
            rows: [self.shard.rows().start, self.shard.rows().count],
            columns: [self.shard.columns().start, self.shard.columns().count],
        }
    }
}

/// Immutable borrowed source recipe with a digest of the exact resulting bytes.
/// This authenticates data only; it cannot allocate, admit an image or launch.
pub struct EngineeringTp2FiniteUploadV1<'a> {
    key: EngineeringTp2FiniteUploadKeyV1,
    bytes: usize,
    digest: [u8; 32],
    parts: Vec<Part<'a>>,
}

impl EngineeringTp2FiniteUploadV1<'_> {
    /// Source-owned logical destination, never a native address or imported token.
    #[must_use]
    pub const fn key(&self) -> EngineeringTp2FiniteUploadKeyV1 {
        self.key
    }
    /// Exact initialized allocation byte count.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.bytes
    }
    /// SHA256 over all output bytes in upload order.
    #[must_use]
    pub const fn sha256(&self) -> [u8; 32] {
        self.digest
    }
    /// Original section digests and sharding geometry, in concatenation order.
    pub fn sources(&self) -> impl ExactSizeIterator<Item = EngineeringTp2FiniteSourcePartV1> + '_ {
        self.parts.iter().map(Part::evidence)
    }

    /// Emit contiguous, gap-free chunks no larger than the chosen 2-byte to 4-MiB cap.
    /// The cap must be even and fit one complete compact BF16 row. No arithmetic
    /// conversion, transpose or rounding occurs. A sink error stops immediately;
    /// native callers must terminal/quarantine any partially initialized owner.
    /// # Errors
    /// Refuses malformed source/row/chunk bounds before calling the sink, or
    /// returns the first copy/sink error without retrying any write.
    pub fn visit_chunks(
        &self,
        chunk_bytes: usize,
        mut sink: impl FnMut(usize, &[u8]) -> TpResult<()>,
    ) -> TpResult<()> {
        stream_parts(&self.parts, chunk_bytes, &mut sink)
    }
}

fn stream_parts(
    parts: &[Part<'_>],
    chunk_bytes: usize,
    sink: &mut impl FnMut(usize, &[u8]) -> TpResult<()>,
) -> TpResult<()> {
    if !(2..=MAX_CHUNK).contains(&chunk_bytes) || !chunk_bytes.is_multiple_of(2) || parts.is_empty()
    {
        return Err("finite upload chunk bound".into());
    }
    // Validate the entire recipe before the first externally visible chunk.
    let mut total = 0usize;
    for part in parts {
        part.check()?;
        let row_bytes = part.shard.columns().count as usize * 2;
        if row_bytes > chunk_bytes {
            return Err("finite compact row exceeds chunk bound".into());
        }
        total = total
            .checked_add(part.bytes()?)
            .ok_or("finite concatenated extent overflow")?;
    }
    let mut offset = 0usize;
    for part in parts {
        let row_bytes = part.shard.columns().count as usize * 2;
        let rows = part.shard.rows().count as usize;
        let chunk_rows = (chunk_bytes / row_bytes).min(rows);
        let mut chunk = vec![0; chunk_rows * row_bytes];
        for first in (0..rows).step_by(chunk_rows) {
            let count = chunk_rows.min(rows - first);
            for local in 0..count {
                part.shard
                    .copy_bf16_row_into(
                        part.source,
                        (first + local) as u32,
                        &mut chunk[local * row_bytes..(local + 1) * row_bytes],
                    )
                    .map_err(|error| format!("finite BF16 source shard copy: {error:?}"))?;
            }
            let bytes = &chunk[..count * row_bytes];
            sink(offset, bytes)?;
            offset += bytes.len();
        }
    }
    if offset != total {
        return Err("finite upload coverage mismatch".into());
    }
    Ok(())
}

fn make_upload<'a>(
    key: EngineeringTp2FiniteUploadKeyV1,
    parts: Vec<Part<'a>>,
    expected_bytes: usize,
) -> TpResult<EngineeringTp2FiniteUploadV1<'a>> {
    use EngineeringTp2FiniteUploadKeyV1 as Key;
    let (layer, kinds): (u32, &[Qwen3TensorKind]) = match &key {
        Key::Source {
            id, layer, kind, ..
        } if *id != 0 => (*layer, std::slice::from_ref(kind)),
        Key::PackedQkv { layer, .. } => (
            *layer,
            &[
                Qwen3TensorKind::QueryProjection,
                Qwen3TensorKind::KeyProjection,
                Qwen3TensorKind::ValueProjection,
            ],
        ),
        Key::PackedHeadNorm { layer, .. } => (
            *layer,
            &[Qwen3TensorKind::QueryNorm, Qwen3TensorKind::KeyNorm],
        ),
        _ => return Err("finite missing original source identity".into()),
    };
    if key.rank() >= 2
        || parts.len() != kinds.len()
        || parts.iter().zip(kinds).any(|(part, kind)| {
            part.shard.rank() != key.rank()
                || part.metadata.layer != layer
                || part.metadata.kind != *kind
        })
    {
        return Err("finite upload coordinate or concatenation order".into());
    }
    let mut hasher = Sha256::new();
    let mut bytes = 0usize;
    stream_parts(&parts, MAX_CHUNK, &mut |offset, chunk| {
        if offset != bytes {
            return Err("finite digest chunk order".into());
        }
        bytes += chunk.len();
        hasher.update(chunk);
        Ok(())
    })?;
    if bytes != expected_bytes {
        return Err("finite destination versus source extent".into());
    }
    Ok(EngineeringTp2FiniteUploadV1 {
        key,
        bytes,
        digest: hasher.finalize().into(),
        parts,
    })
}

/// Borrows the same authenticated model layout, original bytes and execution
/// composition for all setup recipes. Construction verifies every target tensor
/// section once; later views cannot substitute weights or a different model.
pub struct EngineeringTp2FiniteWeightSourceV1<'a> {
    layout: &'a AuthenticatedModelWeightLayout,
    source: &'a [u8],
    composition: &'a Composition<'a>,
    plan: Qwen3TensorParallelPlanV1,
}

impl<'a> EngineeringTp2FiniteWeightSourceV1<'a> {
    /// Authenticate the immutable prepacked target bytes before upload planning.
    /// # Errors
    /// Refuses a different model/bundle, wrong total size or any section digest.
    pub fn new(
        layout: &'a AuthenticatedModelWeightLayout,
        source: &'a [u8],
        composition: &'a Composition<'a>,
    ) -> TpResult<Self> {
        let deployment = layout.admission().prepacked().deployment();
        let model = deployment.target_model.config;
        if model.role != Qwen3ModelRole::Target8B
            || composition.bundle_id() != deployment.bundle_id.as_bytes()
            || composition.model_id() != model.model_id.as_bytes()
            || source.len() as u64 != Qwen3ModelRole::Target8B.tensor_data_bytes()
        {
            return Err("finite packing authenticated bundle/model/source binding".into());
        }
        let plan = Qwen3TensorParallelPlanV1::new(model, 2)
            .map_err(|error| format!("finite authentic TP2 model: {error:?}"))?;
        for ordinal in 0..layout.section_count(Qwen3ModelRole::Target8B) {
            let binding = layout
                .by_ordinal(Qwen3ModelRole::Target8B, ordinal)
                .map_err(|e| e.to_string())?;
            let bytes = section_bytes(source, binding.destination_range())?;
            if <[u8; 32]>::from(Sha256::digest(bytes)) != binding.sha256() {
                return Err(format!(
                    "finite original target section {ordinal} digest mismatch"
                ));
            }
            for rank in 0..2 {
                plan.tensor(binding.metadata(), rank)
                    .map_err(|error| format!("finite target section geometry: {error:?}"))?;
            }
        }
        Ok(Self {
            layout,
            source,
            composition,
            plan,
        })
    }

    fn part(&self, rank: u32, layer: u32, kind: Qwen3TensorKind) -> TpResult<Part<'a>> {
        let binding = self
            .layout
            .lookup(Qwen3ModelRole::Target8B, kind, layer)
            .map_err(|e| e.to_string())?;
        let shard = self
            .plan
            .tensor(binding.metadata(), rank)
            .map_err(|error| format!("finite authentic source shard: {error:?}"))?;
        let part = Part {
            metadata: binding.metadata(),
            digest: binding.sha256(),
            source: section_bytes(self.source, binding.destination_range())?,
            shard,
        };
        part.check()?;
        Ok(part)
    }

    /// Eleven original weight uploads followed by packed QKV and packed norms.
    /// MLP gate/up shard rows and O/down shard columns use the existing tensor
    /// planner. The source IDs are never accepted as native allocation tokens.
    /// # Errors
    /// Refuses a missing/rank-swapped layer binding, wrong shape or packing order.
    pub fn layer_uploads(
        &self,
        rank: u32,
        layer: u32,
    ) -> TpResult<Vec<EngineeringTp2FiniteUploadV1<'a>>> {
        let binding = self
            .composition
            .layers()
            .iter()
            .find(|value| value.rank() == rank && value.layer() == layer)
            .ok_or("finite packing rank/layer missing")?;
        if rank >= 2 || layer >= 36 || binding.weights().len() != LAYER_KINDS.len() {
            return Err("finite packing closed layer roster".into());
        }
        let mut result = Vec::with_capacity(13);
        for ((kind, buffer), expected_kind) in binding.weights().iter().zip(LAYER_KINDS) {
            if *kind != expected_kind || buffer.rank() != rank || buffer.element_bytes() != 2 {
                return Err("finite original weight binding kind/rank/scalar".into());
            }
            result.push(make_upload(
                EngineeringTp2FiniteUploadKeyV1::Source {
                    rank,
                    id: buffer.id(),
                    layer,
                    kind: *kind,
                },
                vec![self.part(rank, layer, *kind)?],
                buffer
                    .elements()
                    .checked_mul(2)
                    .ok_or("finite original byte bound")?,
            )?);
        }
        result.push(make_upload(
            EngineeringTp2FiniteUploadKeyV1::PackedQkv { rank, layer },
            vec![
                self.part(rank, layer, Qwen3TensorKind::QueryProjection)?,
                self.part(rank, layer, Qwen3TensorKind::KeyProjection)?,
                self.part(rank, layer, Qwen3TensorKind::ValueProjection)?,
            ],
            3072 * 4096 * 2,
        )?);
        result.push(make_upload(
            EngineeringTp2FiniteUploadKeyV1::PackedHeadNorm { rank, layer },
            vec![
                self.part(rank, layer, Qwen3TensorKind::QueryNorm)?,
                self.part(rank, layer, Qwen3TensorKind::KeyNorm)?,
            ],
            256 * 2,
        )?);
        Ok(result)
    }

    /// Authentic rank-zero embedding, final norm and LM head in original layout.
    /// # Errors
    /// Refuses incomplete/mixed global bindings; this is not a tail image join.
    pub fn global_uploads(&self) -> TpResult<Vec<EngineeringTp2FiniteUploadV1<'a>>> {
        let bindings = self.composition.globals();
        if bindings.len() != GLOBAL_KINDS.len() {
            return Err("finite global packing roster".into());
        }
        bindings
            .iter()
            .zip(GLOBAL_KINDS)
            .map(|((kind, buffer), expected)| {
                if *kind != expected || buffer.rank() != 0 || buffer.element_bytes() != 2 {
                    return Err("finite global weight binding".into());
                }
                make_upload(
                    EngineeringTp2FiniteUploadKeyV1::Source {
                        rank: 0,
                        id: buffer.id(),
                        layer: QWEN3_NO_LAYER,
                        kind: *kind,
                    },
                    vec![self.part(0, QWEN3_NO_LAYER, *kind)?],
                    buffer
                        .elements()
                        .checked_mul(2)
                        .ok_or("finite global byte bound")?,
                )
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "finite_packing_tests.rs"]
mod tests;
