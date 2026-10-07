use crate::Qwen3ModelRole;
use core::fmt;
use vstd::prelude::*;

verus! {

pub const QWEN3_TENSORS_PER_LAYER: u32 = 11;
pub const QWEN3_TARGET_TENSOR_COUNT: u32 = 399;
pub const QWEN3_DRAFT_TENSOR_COUNT: u32 = 311;
pub const QWEN3_TARGET_TENSOR_DATA_BYTES: u64 = 16_381_470_720;
pub const QWEN3_DRAFT_TENSOR_DATA_BYTES: u64 = 1_503_264_768;
pub const QWEN3_NO_LAYER: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TensorDType {
    Bf16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3TensorKind {
    LanguageModelHead,
    TokenEmbedding,
    FinalNorm,
    InputLayerNorm,
    PostAttentionLayerNorm,
    QueryNorm,
    KeyNorm,
    QueryProjection,
    KeyProjection,
    ValueProjection,
    OutputProjection,
    GateProjection,
    UpProjection,
    DownProjection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3TensorMetadata {
    pub role: Qwen3ModelRole,
    pub kind: Qwen3TensorKind,
    pub layer: u32,
    pub dtype: TensorDType,
    pub rank: u32,
    pub dimension_0: u32,
    pub dimension_1: u32,
}

impl Qwen3ModelRole {
    #[must_use]
    pub const fn layers(self) -> u32 {
        match self {
            Self::Target8B => 36,
            Self::Draft06B => 28,
        }
    }

    #[must_use]
    pub const fn tensor_count(self) -> u32 {
        match self {
            Self::Target8B => QWEN3_TARGET_TENSOR_COUNT,
            Self::Draft06B => QWEN3_DRAFT_TENSOR_COUNT,
        }
    }

    #[must_use]
    pub const fn tensor_data_bytes(self) -> u64 {
        match self {
            Self::Target8B => QWEN3_TARGET_TENSOR_DATA_BYTES,
            Self::Draft06B => QWEN3_DRAFT_TENSOR_DATA_BYTES,
        }
    }
}

impl Qwen3TensorMetadata {
    pub closed spec fn valid(self) -> bool {
        self.dtype == TensorDType::Bf16
            && match self.role {
                Qwen3ModelRole::Target8B => match self.kind {
                    Qwen3TensorKind::LanguageModelHead
                    | Qwen3TensorKind::TokenEmbedding => {
                        self.layer == QWEN3_NO_LAYER
                            && self.rank == 2
                            && self.dimension_0 == 151_936
                            && self.dimension_1 == 4_096
                    }
                    Qwen3TensorKind::FinalNorm => {
                        self.layer == QWEN3_NO_LAYER
                            && self.rank == 1
                            && self.dimension_0 == 4_096
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::InputLayerNorm
                    | Qwen3TensorKind::PostAttentionLayerNorm => {
                        self.layer < 36
                            && self.rank == 1
                            && self.dimension_0 == 4_096
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::QueryNorm | Qwen3TensorKind::KeyNorm => {
                        self.layer < 36
                            && self.rank == 1
                            && self.dimension_0 == 128
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::QueryProjection
                    | Qwen3TensorKind::OutputProjection => {
                        self.layer < 36
                            && self.rank == 2
                            && self.dimension_0 == 4_096
                            && self.dimension_1 == 4_096
                    }
                    Qwen3TensorKind::KeyProjection | Qwen3TensorKind::ValueProjection => {
                        self.layer < 36
                            && self.rank == 2
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 4_096
                    }
                    Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection => {
                        self.layer < 36
                            && self.rank == 2
                            && self.dimension_0 == 12_288
                            && self.dimension_1 == 4_096
                    }
                    Qwen3TensorKind::DownProjection => {
                        self.layer < 36
                            && self.rank == 2
                            && self.dimension_0 == 4_096
                            && self.dimension_1 == 12_288
                    }
                },
                Qwen3ModelRole::Draft06B => match self.kind {
                    Qwen3TensorKind::LanguageModelHead
                    | Qwen3TensorKind::TokenEmbedding => {
                        self.layer == QWEN3_NO_LAYER
                            && self.rank == 2
                            && self.dimension_0 == 151_936
                            && self.dimension_1 == 1_024
                    }
                    Qwen3TensorKind::FinalNorm => {
                        self.layer == QWEN3_NO_LAYER
                            && self.rank == 1
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::InputLayerNorm
                    | Qwen3TensorKind::PostAttentionLayerNorm => {
                        self.layer < 28
                            && self.rank == 1
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::QueryNorm | Qwen3TensorKind::KeyNorm => {
                        self.layer < 28
                            && self.rank == 1
                            && self.dimension_0 == 128
                            && self.dimension_1 == 1
                    }
                    Qwen3TensorKind::QueryProjection => {
                        self.layer < 28
                            && self.rank == 2
                            && self.dimension_0 == 2_048
                            && self.dimension_1 == 1_024
                    }
                    Qwen3TensorKind::KeyProjection | Qwen3TensorKind::ValueProjection => {
                        self.layer < 28
                            && self.rank == 2
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 1_024
                    }
                    Qwen3TensorKind::OutputProjection => {
                        self.layer < 28
                            && self.rank == 2
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 2_048
                    }
                    Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection => {
                        self.layer < 28
                            && self.rank == 2
                            && self.dimension_0 == 3_072
                            && self.dimension_1 == 1_024
                    }
                    Qwen3TensorKind::DownProjection => {
                        self.layer < 28
                            && self.rank == 2
                            && self.dimension_0 == 1_024
                            && self.dimension_1 == 3_072
                    }
                },
            }
    }

    /// Validates one tensor against the exact admitted Qwen3 role and layer.
    ///
    /// # Errors
    ///
    /// Returns [`Qwen3TensorError`] if the tensor rank, shape, global
    /// placement, or layer index differs from the admitted model schema. The
    /// typed metadata can represent only BF16 tensors.
    pub fn validate(self) -> (result: Result<(), Qwen3TensorError>)
        ensures result.is_ok() == self.valid(),
    {
        match self.dtype {
            TensorDType::Bf16 => {}
        }
        match self.role {
            Qwen3ModelRole::Target8B => match self.kind {
                Qwen3TensorKind::LanguageModelHead | Qwen3TensorKind::TokenEmbedding => {
                    self.require_global_shape(2, 151_936, 4_096)?;
                }
                Qwen3TensorKind::FinalNorm => {
                    self.require_global_shape(1, 4_096, 1)?;
                }
                Qwen3TensorKind::InputLayerNorm
                | Qwen3TensorKind::PostAttentionLayerNorm => {
                    self.require_layer_shape(36, 1, 4_096, 1)?;
                }
                Qwen3TensorKind::QueryNorm | Qwen3TensorKind::KeyNorm => {
                    self.require_layer_shape(36, 1, 128, 1)?;
                }
                Qwen3TensorKind::QueryProjection | Qwen3TensorKind::OutputProjection => {
                    self.require_layer_shape(36, 2, 4_096, 4_096)?;
                }
                Qwen3TensorKind::KeyProjection | Qwen3TensorKind::ValueProjection => {
                    self.require_layer_shape(36, 2, 1_024, 4_096)?;
                }
                Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection => {
                    self.require_layer_shape(36, 2, 12_288, 4_096)?;
                }
                Qwen3TensorKind::DownProjection => {
                    self.require_layer_shape(36, 2, 4_096, 12_288)?;
                }
            },
            Qwen3ModelRole::Draft06B => match self.kind {
                Qwen3TensorKind::LanguageModelHead | Qwen3TensorKind::TokenEmbedding => {
                    self.require_global_shape(2, 151_936, 1_024)?;
                }
                Qwen3TensorKind::FinalNorm => {
                    self.require_global_shape(1, 1_024, 1)?;
                }
                Qwen3TensorKind::InputLayerNorm
                | Qwen3TensorKind::PostAttentionLayerNorm => {
                    self.require_layer_shape(28, 1, 1_024, 1)?;
                }
                Qwen3TensorKind::QueryNorm | Qwen3TensorKind::KeyNorm => {
                    self.require_layer_shape(28, 1, 128, 1)?;
                }
                Qwen3TensorKind::QueryProjection => {
                    self.require_layer_shape(28, 2, 2_048, 1_024)?;
                }
                Qwen3TensorKind::KeyProjection | Qwen3TensorKind::ValueProjection => {
                    self.require_layer_shape(28, 2, 1_024, 1_024)?;
                }
                Qwen3TensorKind::OutputProjection => {
                    self.require_layer_shape(28, 2, 1_024, 2_048)?;
                }
                Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection => {
                    self.require_layer_shape(28, 2, 3_072, 1_024)?;
                }
                Qwen3TensorKind::DownProjection => {
                    self.require_layer_shape(28, 2, 1_024, 3_072)?;
                }
            },
        }
        Ok(())
    }

    fn require_global_shape(
        self,
        rank: u32,
        dimension_0: u32,
        dimension_1: u32,
    ) -> (result: Result<(), Qwen3TensorError>)
        ensures
            result.is_ok() == (self.layer == QWEN3_NO_LAYER
                && self.rank == rank
                && self.dimension_0 == dimension_0
                && self.dimension_1 == dimension_1),
    {
        if self.layer != QWEN3_NO_LAYER {
            return Err(Qwen3TensorError::UnexpectedLayer);
        }
        self.require_shape(rank, dimension_0, dimension_1)
    }

    fn require_layer_shape(
        self,
        layers: u32,
        rank: u32,
        dimension_0: u32,
        dimension_1: u32,
    ) -> (result: Result<(), Qwen3TensorError>)
        ensures
            result.is_ok() == (self.layer < layers
                && self.rank == rank
                && self.dimension_0 == dimension_0
                && self.dimension_1 == dimension_1),
    {
        if self.layer >= layers {
            return Err(Qwen3TensorError::UnexpectedLayer);
        }
        self.require_shape(rank, dimension_0, dimension_1)
    }

    fn require_shape(
        self,
        rank: u32,
        dimension_0: u32,
        dimension_1: u32,
    ) -> (result: Result<(), Qwen3TensorError>)
        ensures
            result.is_ok() == (self.rank == rank
                && self.dimension_0 == dimension_0
                && self.dimension_1 == dimension_1),
    {
        if self.rank != rank {
            return Err(Qwen3TensorError::UnexpectedRank);
        }
        if self.dimension_0 != dimension_0 || self.dimension_1 != dimension_1 {
            return Err(Qwen3TensorError::UnexpectedShape);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3TensorError {
    UnexpectedLayer,
    UnexpectedRank,
    UnexpectedShape,
}

} // verus!

impl fmt::Display for Qwen3TensorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedLayer => formatter.write_str("tensor belongs to an invalid layer"),
            Self::UnexpectedRank => formatter.write_str("tensor rank does not match Qwen3"),
            Self::UnexpectedShape => formatter.write_str("tensor shape does not match Qwen3"),
        }
    }
}

impl std::error::Error for Qwen3TensorError {}

#[cfg(test)]
mod tests {
    use super::{
        Qwen3TensorError, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType,
        QWEN3_DRAFT_TENSOR_COUNT, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_NO_LAYER,
        QWEN3_TARGET_TENSOR_COUNT, QWEN3_TARGET_TENSOR_DATA_BYTES,
    };
    use crate::Qwen3ModelRole;

    #[test]
    fn pinned_tensor_counts_and_data_bytes_match_artifacts() {
        assert_eq!(Qwen3ModelRole::Target8B.tensor_count(), 399);
        assert_eq!(Qwen3ModelRole::Draft06B.tensor_count(), 311);
        assert_eq!(QWEN3_TARGET_TENSOR_COUNT, 36 * 11 + 3);
        assert_eq!(QWEN3_DRAFT_TENSOR_COUNT, 28 * 11 + 3);
        assert_eq!(QWEN3_TARGET_TENSOR_DATA_BYTES, 16_381_470_720);
        assert_eq!(QWEN3_DRAFT_TENSOR_DATA_BYTES, 1_503_264_768);
    }

    #[test]
    fn representative_target_and_draft_tensors_validate() {
        let target_query = Qwen3TensorMetadata {
            role: Qwen3ModelRole::Target8B,
            kind: Qwen3TensorKind::QueryProjection,
            layer: 35,
            dtype: TensorDType::Bf16,
            rank: 2,
            dimension_0: 4_096,
            dimension_1: 4_096,
        };
        assert_eq!(target_query.validate(), Ok(()));

        let draft_output = Qwen3TensorMetadata {
            role: Qwen3ModelRole::Draft06B,
            kind: Qwen3TensorKind::OutputProjection,
            layer: 27,
            dtype: TensorDType::Bf16,
            rank: 2,
            dimension_0: 1_024,
            dimension_1: 2_048,
        };
        assert_eq!(draft_output.validate(), Ok(()));

        let draft_head = Qwen3TensorMetadata {
            role: Qwen3ModelRole::Draft06B,
            kind: Qwen3TensorKind::LanguageModelHead,
            layer: QWEN3_NO_LAYER,
            dtype: TensorDType::Bf16,
            rank: 2,
            dimension_0: 151_936,
            dimension_1: 1_024,
        };
        assert_eq!(draft_head.validate(), Ok(()));
    }

    #[test]
    fn role_layer_rank_and_shape_substitutions_fail_closed() {
        let base = Qwen3TensorMetadata {
            role: Qwen3ModelRole::Target8B,
            kind: Qwen3TensorKind::KeyProjection,
            layer: 0,
            dtype: TensorDType::Bf16,
            rank: 2,
            dimension_0: 1_024,
            dimension_1: 4_096,
        };

        let mut wrong_layer = base;
        wrong_layer.layer = 36;
        assert_eq!(
            wrong_layer.validate(),
            Err(Qwen3TensorError::UnexpectedLayer)
        );

        let mut wrong_rank = base;
        wrong_rank.rank = 1;
        assert_eq!(wrong_rank.validate(), Err(Qwen3TensorError::UnexpectedRank));

        let mut wrong_shape = base;
        wrong_shape.dimension_0 = 4_096;
        assert_eq!(
            wrong_shape.validate(),
            Err(Qwen3TensorError::UnexpectedShape)
        );

        let mut wrong_role = base;
        wrong_role.role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            wrong_role.validate(),
            Err(Qwen3TensorError::UnexpectedShape)
        );
    }

    #[test]
    fn global_tensors_reject_layer_aliasing() {
        let tensor = Qwen3TensorMetadata {
            role: Qwen3ModelRole::Target8B,
            kind: Qwen3TensorKind::TokenEmbedding,
            layer: 0,
            dtype: TensorDType::Bf16,
            rank: 2,
            dimension_0: 151_936,
            dimension_1: 4_096,
        };
        assert_eq!(tensor.validate(), Err(Qwen3TensorError::UnexpectedLayer));
    }
}
