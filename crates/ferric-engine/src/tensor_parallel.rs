//! Addressless tensor-parallel planning for the admitted Qwen3 models.
//!
//! Matrices use safetensors `[output, input]` order. Embeddings, normalization,
//! and the language-model head are replicated; attention and feed-forward
//! projections are partitioned without replicating KV heads. The collective
//! state checks rank readiness, not device completion or transport correctness.
//! Group IDs must be unique within the caller's live coordinator. This API does
//! not authenticate rank ownership, enqueue collectives, sum numerical values,
//! establish device completion, or grant artifact/qualification authority.

use ferric_spec::{ModelConfig, Qwen3ModelRole, Qwen3TensorKind, Qwen3TensorMetadata};
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TensorParallelErrorV1 {
    InvalidModel,
    UnsupportedWorldSize,
    NondivisibleGeometry,
    InvalidRank,
    InvalidRow,
    InvalidTensor,
    ModelRoleMismatch,
    ArithmeticOverflow,
    InvalidLayer,
    EpochMismatch,
    CollectiveOrderMismatch,
    DuplicateRank,
    MissingRanks,
    InvalidSourceLength,
    InvalidDestinationLength,
    GroupMismatch,
}

fn same_model_role(left: Qwen3ModelRole, right: Qwen3ModelRole) -> (same: bool)
    ensures same == (left == right),
{
    matches!((left, right),
        (Qwen3ModelRole::Target8B, Qwen3ModelRole::Target8B)
        | (Qwen3ModelRole::Draft06B, Qwen3ModelRole::Draft06B))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TensorParallelRangeV1 {
    pub start: u32,
    pub count: u32,
}

pub open spec fn tensor_parallel_range_matches_v1(
    range: TensorParallelRangeV1,
    extent: u32,
    world_size: u32,
    rank: u32,
) -> bool {
    &&& world_size == 1 || world_size == 2 || world_size == 8
    &&& rank < world_size
    &&& range.count > 0
    &&& range.count as int * world_size as int == extent
    &&& range.start as int == range.count as int * rank as int
    &&& range.start as int + range.count as int <= extent
}

/// One contiguous equal-width partition, with no empty or truncated shards.
///
/// # Errors
/// Rejects unsupported world sizes, invalid ranks, and indivisible extents.
pub fn tensor_parallel_range_v1(
    extent: u32,
    world_size: u32,
    rank: u32,
) -> (result: Result<TensorParallelRangeV1, TensorParallelErrorV1>)
    ensures
        result.is_ok() ==> tensor_parallel_range_matches_v1(
            result.unwrap(), extent, world_size, rank),
        result.is_ok() ==> world_size == 1 || world_size == 2 || world_size == 8,
        result.is_ok() ==> rank < world_size,
        result.is_ok() ==> result.unwrap().count > 0,
        result.is_ok() ==> result.unwrap().count as int * world_size as int == extent,
        result.is_ok() ==> result.unwrap().start as int
            == result.unwrap().count as int * rank as int,
        result.is_ok() ==> result.unwrap().start as int + result.unwrap().count as int
            <= extent,
{
    if world_size != 1 && world_size != 2 && world_size != 8 {
        return Err(TensorParallelErrorV1::UnsupportedWorldSize);
    }
    if rank >= world_size {
        return Err(TensorParallelErrorV1::InvalidRank);
    }
    if extent == 0 || !extent.is_multiple_of(world_size) {
        return Err(TensorParallelErrorV1::NondivisibleGeometry);
    }
    let count = extent / world_size;
    proof {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(extent as int, world_size as int);
        assert(count > 0) by (nonlinear_arith)
            requires extent > 0, count as int * world_size as int == extent;
        assert(count as int * rank as int + count as int <= extent) by (nonlinear_arith)
            requires rank < world_size, count as int * world_size as int == extent;
    }
    Ok(TensorParallelRangeV1 { start: count * rank, count })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TensorParallelMatrixModeV1 {
    Replicated,
    ColumnParallel,
    RowParallelSum,
}

pub open spec fn tensor_parallel_matrix_mode_v1(kind: Qwen3TensorKind)
    -> TensorParallelMatrixModeV1
{
    match kind {
        Qwen3TensorKind::QueryProjection | Qwen3TensorKind::KeyProjection
        | Qwen3TensorKind::ValueProjection | Qwen3TensorKind::GateProjection
        | Qwen3TensorKind::UpProjection => TensorParallelMatrixModeV1::ColumnParallel,
        Qwen3TensorKind::OutputProjection | Qwen3TensorKind::DownProjection
            => TensorParallelMatrixModeV1::RowParallelSum,
        _ => TensorParallelMatrixModeV1::Replicated,
    }
}

/// A rectangle in the original dense BF16 tensor, before compacting local rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorParallelTensorV1 {
    role: Qwen3ModelRole,
    world_size: u32,
    rank: u32,
    source_rows: u32,
    source_columns: u32,
    rows: TensorParallelRangeV1,
    columns: TensorParallelRangeV1,
    mode: TensorParallelMatrixModeV1,
}

pub struct Qwen3TensorParallelTensorViewV1 {
    pub role: Qwen3ModelRole,
    pub world_size: u32,
    pub rank: u32,
    pub source_rows: u32,
    pub source_columns: u32,
    pub rows: TensorParallelRangeV1,
    pub columns: TensorParallelRangeV1,
    pub mode: TensorParallelMatrixModeV1,
}

impl Qwen3TensorParallelTensorV1 {
    pub closed spec fn view(self) -> Qwen3TensorParallelTensorViewV1 {
        Qwen3TensorParallelTensorViewV1 {
            role: self.role, world_size: self.world_size, rank: self.rank,
            source_rows: self.source_rows, source_columns: self.source_columns,
            rows: self.rows, columns: self.columns, mode: self.mode,
        }
    }

    pub closed spec fn valid(self) -> bool {
        &&& self.source_rows > 0
        &&& self.source_columns > 0
        &&& self.rows.count > 0
        &&& self.columns.count > 0
        &&& self.rows.start as int + self.rows.count as int <= self.source_rows
        &&& self.columns.start as int + self.columns.count as int <= self.source_columns
        &&& self.source_rows as int * self.source_columns as int * 2 <= u64::MAX
        &&& self.rank < self.world_size
        &&& (self.world_size == 1 || self.world_size == 2 || self.world_size == 8)
    }

    #[must_use]
    pub fn role(&self) -> (value: Qwen3ModelRole)
        ensures value == self.view().role,
    { self.role }

    #[must_use]
    pub fn world_size(&self) -> (value: u32)
        ensures value == self.view().world_size,
    { self.world_size }

    #[must_use]
    pub fn rank(&self) -> (value: u32)
        ensures value == self.view().rank,
    { self.rank }

    #[must_use]
    pub fn source_rows(&self) -> (value: u32)
        ensures value == self.view().source_rows,
    { self.source_rows }

    #[must_use]
    pub fn source_columns(&self) -> (value: u32)
        ensures value == self.view().source_columns,
    { self.source_columns }

    #[must_use]
    pub fn rows(&self) -> (value: TensorParallelRangeV1)
        ensures value == self.view().rows,
    { self.rows }

    #[must_use]
    pub fn columns(&self) -> (value: TensorParallelRangeV1)
        ensures value == self.view().columns,
    { self.columns }

    #[must_use]
    pub fn mode(&self) -> (value: TensorParallelMatrixModeV1)
        ensures value == self.view().mode,
    { self.mode }

    /// Returns one source byte interval for a compact shard row.
    ///
    /// # Errors
    /// Rejects a wrong payload length or a row outside this shard. No bytes are read.
    pub fn source_row_bytes(
        &self,
        local_row: u32,
        source_bytes: u64,
    ) -> (result: Result<(u64, u64), TensorParallelErrorV1>)
        requires self.valid(),
        ensures
            result.is_ok() ==> result.unwrap().0 as int + result.unwrap().1 as int
                <= source_bytes,
            result.is_ok() ==> result.unwrap().0 as int
                == ((self.view().rows.start as int + local_row as int) * self.view().source_columns as int
                    + self.view().columns.start as int) * 2,
            result.is_ok() ==> result.unwrap().1 == self.view().columns.count as int * 2,
            result.is_ok() ==> source_bytes == self.view().source_rows as int
                * self.view().source_columns as int * 2,
    {
        let expected = u64::from(self.source_rows) * u64::from(self.source_columns) * 2;
        if source_bytes != expected {
            return Err(TensorParallelErrorV1::InvalidSourceLength);
        }
        if local_row >= self.rows.count {
            return Err(TensorParallelErrorV1::InvalidRow);
        }
        let row = u64::from(self.rows.start) + u64::from(local_row);
        proof {
            assert((row as int * self.source_columns as int + self.columns.start as int)
                * 2 + self.columns.count as int * 2 <= expected) by (nonlinear_arith)
                requires
                    row < self.source_rows,
                    self.source_columns > 0,
                    self.columns.start as int + self.columns.count as int <= self.source_columns,
                    expected == self.source_rows as int * self.source_columns as int * 2;
        }
        let offset = (row * u64::from(self.source_columns) + u64::from(self.columns.start)) * 2;
        Ok((offset, u64::from(self.columns.count) * 2))
    }

    /// Copies one compact BF16 shard row without allocation or device access.
    ///
    /// # Errors
    /// Rejects wrong source/destination lengths or an invalid local row before
    /// changing any destination byte. Both source and destination are raw BF16.
    pub fn copy_bf16_row_into(
        &self,
        source: &[u8],
        local_row: u32,
        destination: &mut [u8],
    ) -> (result: Result<(), TensorParallelErrorV1>)
        requires self.valid(),
        ensures
            result.is_err() ==> final(destination)@ == old(destination)@,
            result.is_ok() ==> final(destination).len() == self.view().columns.count as int * 2,
            result.is_ok() ==> forall|index: int| 0 <= index < final(destination).len() ==>
                final(destination)@[index] == source@[
                    ((self.view().rows.start as int + local_row as int) * self.view().source_columns as int
                        + self.view().columns.start as int) * 2 + index],
    {
        let (offset, bytes) = self.source_row_bytes(local_row, source.len() as u64)?;
        if destination.len() as u64 != bytes {
            return Err(TensorParallelErrorV1::InvalidDestinationLength);
        }
        let offset_index = match usize::try_from(offset) {
            Ok(value) => value,
            Err(_) => return Err(TensorParallelErrorV1::ArithmeticOverflow),
        };
        let mut index: usize = 0;
        while index < destination.len()
            invariant
                0 <= index <= destination.len(),
                destination.len() == old(destination).len(),
                destination.len() == bytes,
                offset_index == offset,
                offset as int + bytes as int <= source.len(),
                offset == ((self.rows.start as int + local_row as int)
                    * self.source_columns as int + self.columns.start as int) * 2,
                bytes == self.columns.count as int * 2,
                forall|done: int| 0 <= done < index ==>
                    destination@[done] == source@[offset as int + done],
            decreases destination.len() - index,
        {
            destination[index] = source[offset_index + index];
            index += 1;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorParallelRankV1 {
    pub rank: u32,
    pub world_size: u32,
    pub query_heads: TensorParallelRangeV1,
    pub kv_heads: TensorParallelRangeV1,
    pub query_channels: TensorParallelRangeV1,
    pub kv_channels: TensorParallelRangeV1,
    pub intermediate: TensorParallelRangeV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorParallelPlanV1 {
    model: ModelConfig,
    world_size: u32,
}

pub struct Qwen3TensorParallelPlanViewV1 {
    pub model: ModelConfig,
    pub world_size: u32,
}

impl Qwen3TensorParallelPlanV1 {
    pub closed spec fn view(self) -> Qwen3TensorParallelPlanViewV1 {
        Qwen3TensorParallelPlanViewV1 { model: self.model, world_size: self.world_size }
    }

    pub closed spec fn valid(self) -> bool {
        &&& self.model.valid()
        &&& (self.world_size == 1 || self.world_size == 2 || self.world_size == 8)
        &&& 0 < self.model.query_heads <= 32
        &&& 0 < self.model.kv_heads <= 8
        &&& self.model.head_dim == 128
        &&& self.model.query_heads % self.world_size == 0
        &&& self.model.kv_heads % self.world_size == 0
        &&& self.model.query_heads % self.model.kv_heads == 0
        &&& 0 < self.model.intermediate_size <= 12_288
        &&& self.model.intermediate_size % self.world_size == 0
        &&& 0 < self.model.layers <= 36
    }

    /// Admits existing Qwen model identities and exact geometry for TP1/2/8.
    ///
    /// # Errors
    /// Rejects invalid model metadata and any unsupported or indivisible split.
    pub fn new(model: ModelConfig, world_size: u32)
        -> (result: Result<Self, TensorParallelErrorV1>)
        ensures result.is_ok() ==> result.unwrap().valid(),
            result.is_ok() ==> result.unwrap().view().model == model,
            result.is_ok() ==> result.unwrap().view().world_size == world_size,
    {
        if model.validate().is_err() {
            return Err(TensorParallelErrorV1::InvalidModel);
        }
        if world_size != 1 && world_size != 2 && world_size != 8 {
            return Err(TensorParallelErrorV1::UnsupportedWorldSize);
        }
        if model.query_heads == 0 || model.query_heads > 32
            || model.kv_heads == 0 || model.kv_heads > 8 || model.head_dim != 128
            || !model.query_heads.is_multiple_of(world_size) || !model.kv_heads.is_multiple_of(world_size)
            || !model.query_heads.is_multiple_of(model.kv_heads)
            || model.intermediate_size == 0 || model.intermediate_size > 12_288
            || !model.intermediate_size.is_multiple_of(world_size)
            || model.layers == 0 || model.layers > 36
        {
            return Err(TensorParallelErrorV1::NondivisibleGeometry);
        }
        Ok(Self { model, world_size })
    }

    #[must_use]
    pub fn model(&self) -> (value: ModelConfig)
        ensures value == self.view().model,
    { self.model }

    #[must_use]
    pub fn world_size(&self) -> (value: u32)
        ensures value == self.view().world_size,
    { self.world_size }

    /// Plans rank-local attention, KV-cache channels, and intermediate rows.
    ///
    /// # Errors
    /// Rejects a rank outside the admitted group.
    pub fn rank(&self, rank: u32)
        -> (result: Result<Qwen3TensorParallelRankV1, TensorParallelErrorV1>)
        requires self.valid(),
        ensures result.is_ok() ==> result.unwrap().rank == rank,
            result.is_ok() ==> result.unwrap().world_size == self.view().world_size,
            result.is_ok() ==> tensor_parallel_range_matches_v1(
                result.unwrap().query_heads, self.view().model.query_heads, self.view().world_size, rank),
            result.is_ok() ==> tensor_parallel_range_matches_v1(
                result.unwrap().kv_heads, self.view().model.kv_heads, self.view().world_size, rank),
            result.is_ok() ==> tensor_parallel_range_matches_v1(
                result.unwrap().intermediate, self.view().model.intermediate_size, self.view().world_size, rank),
            result.is_ok() ==> result.unwrap().query_heads.count as int * self.view().world_size
                == self.view().model.query_heads,
            result.is_ok() ==> result.unwrap().kv_heads.count as int * self.view().world_size
                == self.view().model.kv_heads,
            result.is_ok() ==> result.unwrap().query_channels.start as int
                == result.unwrap().query_heads.start as int * 128,
            result.is_ok() ==> result.unwrap().query_channels.count as int
                == result.unwrap().query_heads.count as int * 128,
            result.is_ok() ==> result.unwrap().kv_channels.start as int
                == result.unwrap().kv_heads.start as int * 128,
            result.is_ok() ==> result.unwrap().kv_channels.count as int
                == result.unwrap().kv_heads.count as int * 128,
            result.is_ok() ==> result.unwrap().intermediate.count as int * self.view().world_size
                == self.view().model.intermediate_size,
    {
        let query_heads = tensor_parallel_range_v1(self.model.query_heads, self.world_size, rank)?;
        let kv_heads = tensor_parallel_range_v1(self.model.kv_heads, self.world_size, rank)?;
        let intermediate = tensor_parallel_range_v1(
            self.model.intermediate_size, self.world_size, rank,
        )?;
        Ok(Qwen3TensorParallelRankV1 {
            rank,
            world_size: self.world_size,
            query_heads,
            kv_heads,
            query_channels: TensorParallelRangeV1 {
                start: query_heads.start * 128,
                count: query_heads.count * 128,
            },
            kv_channels: TensorParallelRangeV1 {
                start: kv_heads.start * 128,
                count: kv_heads.count * 128,
            },
            intermediate,
        })
    }

    /// Plans a rectangle in an independently validated full BF16 tensor.
    ///
    /// # Errors
    /// Rejects a malformed tensor, wrong model role, invalid rank, or bad split.
    pub fn tensor(&self, metadata: Qwen3TensorMetadata, rank: u32)
        -> (result: Result<Qwen3TensorParallelTensorV1, TensorParallelErrorV1>)
        requires self.valid(),
        ensures result.is_ok() ==> result.unwrap().valid(),
            result.is_ok() ==> metadata.valid(),
            result.is_ok() ==> metadata.role == self.view().model.role,
            result.is_ok() ==> result.unwrap().view().role == self.view().model.role,
            result.is_ok() ==> result.unwrap().view().rank == rank,
            result.is_ok() ==> result.unwrap().view().world_size == self.view().world_size,
            result.is_ok() ==> result.unwrap().view().source_rows == metadata.dimension_0,
            result.is_ok() ==> result.unwrap().view().source_columns == metadata.dimension_1,
            result.is_ok() ==> result.unwrap().view().mode == tensor_parallel_matrix_mode_v1(metadata.kind),
            result.is_ok() ==> (if result.unwrap().view().mode == TensorParallelMatrixModeV1::ColumnParallel {
                result.unwrap().view().rows.count as int * self.view().world_size == metadata.dimension_0
                    && result.unwrap().view().rows.start as int == result.unwrap().view().rows.count as int * rank
                    && result.unwrap().view().columns == (TensorParallelRangeV1 {
                        start: 0, count: metadata.dimension_1 })
            } else if result.unwrap().view().mode == TensorParallelMatrixModeV1::RowParallelSum {
                result.unwrap().view().columns.count as int * self.view().world_size == metadata.dimension_1
                    && result.unwrap().view().columns.start as int == result.unwrap().view().columns.count as int * rank
                    && result.unwrap().view().rows == (TensorParallelRangeV1 {
                        start: 0, count: metadata.dimension_0 })
            } else {
                result.unwrap().view().rows == (TensorParallelRangeV1 { start: 0, count: metadata.dimension_0 })
                    && result.unwrap().view().columns == (TensorParallelRangeV1 { start: 0, count: metadata.dimension_1 })
            }),
    {
        if metadata.validate().is_err() {
            return Err(TensorParallelErrorV1::InvalidTensor);
        }
        if !same_model_role(metadata.role, self.model.role) {
            return Err(TensorParallelErrorV1::ModelRoleMismatch);
        }
        if rank >= self.world_size {
            return Err(TensorParallelErrorV1::InvalidRank);
        }
        if metadata.dimension_0 == 0 || metadata.dimension_1 == 0
            || metadata.dimension_0 > 151_936 || metadata.dimension_1 > 151_936
        {
            return Err(TensorParallelErrorV1::InvalidTensor);
        }
        let full_rows = TensorParallelRangeV1 { start: 0, count: metadata.dimension_0 };
        let full_columns = TensorParallelRangeV1 { start: 0, count: metadata.dimension_1 };
        let (rows, columns, mode) = match metadata.kind {
            Qwen3TensorKind::QueryProjection | Qwen3TensorKind::KeyProjection
            | Qwen3TensorKind::ValueProjection | Qwen3TensorKind::GateProjection
            | Qwen3TensorKind::UpProjection => {
                (tensor_parallel_range_v1(metadata.dimension_0, self.world_size, rank)?,
                    full_columns, TensorParallelMatrixModeV1::ColumnParallel)
            }
            Qwen3TensorKind::OutputProjection | Qwen3TensorKind::DownProjection => {
                (full_rows,
                    tensor_parallel_range_v1(metadata.dimension_1, self.world_size, rank)?,
                    TensorParallelMatrixModeV1::RowParallelSum)
            }
            _ => (full_rows, full_columns, TensorParallelMatrixModeV1::Replicated),
        };
        proof {
            assert(metadata.dimension_0 as int * metadata.dimension_1 as int * 2 <= u64::MAX)
                by (nonlinear_arith)
                requires metadata.dimension_0 <= 151_936, metadata.dimension_1 <= 151_936;
        }
        Ok(Qwen3TensorParallelTensorV1 {
            role: self.model.role,
            world_size: self.world_size,
            rank,
            source_rows: metadata.dimension_0,
            source_columns: metadata.dimension_1,
            rows,
            columns,
            mode,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3TensorParallelCollectiveV1 {
    AttentionOutputSum,
    FeedForwardDownSum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorParallelCollectiveKeyV1 {
    pub group_id: u64,
    pub model_role: Qwen3ModelRole,
    pub epoch: u64,
    pub layer: u32,
    pub operation: Qwen3TensorParallelCollectiveV1,
}

/// Per-group host readiness barrier. Arrival does not assert GPU completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorParallelCollectiveStateV1 {
    group_id: u64,
    model_role: Qwen3ModelRole,
    world_size: u32,
    layers: u32,
    epoch: u64,
    ordinal: u32,
    arrived: u8,
}

pub struct Qwen3TensorParallelCollectiveViewV1 {
    pub group_id: u64,
    pub model_role: Qwen3ModelRole,
    pub world_size: u32,
    pub layers: u32,
    pub epoch: u64,
    pub ordinal: u32,
    pub arrived: u8,
}

impl Qwen3TensorParallelCollectiveStateV1 {
    pub closed spec fn view(self) -> Qwen3TensorParallelCollectiveViewV1 {
        Qwen3TensorParallelCollectiveViewV1 {
            group_id: self.group_id, model_role: self.model_role, world_size: self.world_size,
            layers: self.layers, epoch: self.epoch, ordinal: self.ordinal, arrived: self.arrived,
        }
    }

    pub closed spec fn valid(self) -> bool {
        &&& self.world_size == 1 || self.world_size == 2 || self.world_size == 8
        &&& 0 < self.layers <= 36
        &&& self.ordinal < self.layers * 2
        &&& self.arrived <= (if self.world_size == 1 { 1u8 }
            else if self.world_size == 2 { 3u8 } else { 255u8 })
    }

    #[must_use]
    pub fn new(plan: &Qwen3TensorParallelPlanV1, group_id: u64, epoch: u64) -> (value: Self)
        requires plan.valid(),
        ensures value.valid(), value.view().epoch == epoch, value.view().ordinal == 0,
            value.view().arrived == 0, value.view().world_size == plan.view().world_size,
            value.view().layers == plan.view().model.layers,
            value.view().group_id == group_id, value.view().model_role == plan.view().model.role,
    {
        Self { group_id, model_role: plan.model.role,
            world_size: plan.world_size, layers: plan.model.layers, epoch,
            ordinal: 0, arrived: 0 }
    }

    #[must_use]
    pub fn expected(&self) -> (key: Qwen3TensorParallelCollectiveKeyV1)
        requires self.valid(),
        ensures key.epoch == self.view().epoch, key.layer == self.view().ordinal / 2,
            key.group_id == self.view().group_id, key.model_role == self.view().model_role,
            key.operation == (if self.view().ordinal % 2 == 0 {
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            } else { Qwen3TensorParallelCollectiveV1::FeedForwardDownSum }),
    {
        Qwen3TensorParallelCollectiveKeyV1 {
            group_id: self.group_id,
            model_role: self.model_role,
            epoch: self.epoch,
            layer: self.ordinal / 2,
            operation: if self.ordinal.is_multiple_of(2) {
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            } else { Qwen3TensorParallelCollectiveV1::FeedForwardDownSum },
        }
    }

    /// Records exactly one rank's readiness for the current reduction.
    ///
    /// # Errors
    /// Rejects a stale epoch, wrong operation/layer, invalid or duplicate rank.
    /// Every error leaves the entire state unchanged.
    pub fn arrive(&mut self, rank: u32, key: Qwen3TensorParallelCollectiveKeyV1)
        -> (result: Result<(), TensorParallelErrorV1>)
        requires old(self).valid(),
        ensures final(self).valid(),
            result.is_err() ==> *final(self) == *old(self),
            final(self).view().world_size == old(self).view().world_size,
            final(self).view().group_id == old(self).view().group_id,
            final(self).view().model_role == old(self).view().model_role,
            final(self).view().layers == old(self).view().layers,
            final(self).view().epoch == old(self).view().epoch,
            final(self).view().ordinal == old(self).view().ordinal,
            result.is_ok() ==> rank < final(self).view().world_size,
            result.is_ok() ==> key.epoch == final(self).view().epoch,
            result.is_ok() ==> key.group_id == final(self).view().group_id,
            result.is_ok() ==> key.model_role == final(self).view().model_role,
            result.is_ok() ==> key.layer == final(self).view().ordinal / 2,
            result.is_ok() ==> key.operation == (if final(self).view().ordinal % 2 == 0 {
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            } else { Qwen3TensorParallelCollectiveV1::FeedForwardDownSum }),
            result.is_ok() ==> final(self).view().arrived == old(self).view().arrived | (1u8 << rank),
            result.is_ok() ==> old(self).view().arrived & (1u8 << rank) == 0,
    {
        if rank >= self.world_size {
            return Err(TensorParallelErrorV1::InvalidRank);
        }
        if key.group_id != self.group_id || !same_model_role(key.model_role, self.model_role) {
            return Err(TensorParallelErrorV1::GroupMismatch);
        }
        if key.epoch != self.epoch {
            return Err(TensorParallelErrorV1::EpochMismatch);
        }
        if key.layer >= self.layers {
            return Err(TensorParallelErrorV1::InvalidLayer);
        }
        let expected = self.expected();
        let same_operation = matches!((key.operation, expected.operation),
            (Qwen3TensorParallelCollectiveV1::AttentionOutputSum,
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
            | (Qwen3TensorParallelCollectiveV1::FeedForwardDownSum,
                Qwen3TensorParallelCollectiveV1::FeedForwardDownSum));
        if key.layer != expected.layer || !same_operation {
            return Err(TensorParallelErrorV1::CollectiveOrderMismatch);
        }
        let bit = 1u8 << rank;
        if self.arrived & bit != 0 {
            return Err(TensorParallelErrorV1::DuplicateRank);
        }
        let next = self.arrived | bit;
        let ghost world = self.world_size;
        let ghost previous = self.arrived;
        proof {
            assert(next <= (if world == 1 { 1u8 }
                else if world == 2 { 3u8 } else { 255u8 })) by (bit_vector)
                requires
                    world == 1 || world == 2 || world == 8,
                    rank < world,
                    previous <= (if world == 1 { 1u8 }
                        else if world == 2 { 3u8 } else { 255u8 }),
                    next == previous | (1u8 << rank);
        }
        self.arrived = next;
        Ok(())
    }

    /// Advances only after every rank arrived; the last layer advances the epoch.
    ///
    /// # Errors
    /// Missing ranks and exhausted epoch space leave the entire state unchanged.
    pub fn advance(&mut self) -> (result: Result<(), TensorParallelErrorV1>)
        requires old(self).valid(),
        ensures final(self).valid(),
            result.is_err() ==> *final(self) == *old(self),
            final(self).view().world_size == old(self).view().world_size,
            final(self).view().group_id == old(self).view().group_id,
            final(self).view().model_role == old(self).view().model_role,
            final(self).view().layers == old(self).view().layers,
            result.is_ok() ==> final(self).view().arrived == 0,
            result.is_ok() ==> old(self).view().arrived == (if final(self).view().world_size == 1 { 1u8 }
                else if final(self).view().world_size == 2 { 3u8 } else { 255u8 }),
            result.is_ok() ==> (if old(self).view().ordinal + 1 == final(self).view().layers * 2 {
                final(self).view().epoch == old(self).view().epoch + 1 && final(self).view().ordinal == 0
            } else { final(self).view().epoch == old(self).view().epoch && final(self).view().ordinal == old(self).view().ordinal + 1 }),
    {
        let all = if self.world_size == 1 { 1u8 }
            else if self.world_size == 2 { 3u8 } else { 255u8 };
        if self.arrived != all {
            return Err(TensorParallelErrorV1::MissingRanks);
        }
        let next = self.ordinal + 1;
        if next == self.layers * 2 {
            if self.epoch == u64::MAX {
                return Err(TensorParallelErrorV1::ArithmeticOverflow);
            }
            self.epoch += 1;
            self.ordinal = 0;
        } else {
            self.ordinal = next;
        }
        self.arrived = 0;
        Ok(())
    }
}

} // verus!

#[cfg(test)]
#[path = "tensor_parallel_tests.rs"]
mod tests;
