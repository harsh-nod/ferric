//! Exact addressless launch recipes for complete M1 dispatch compositions.
//!
//! Each operation row already retains a canonical Ferric Qwen profile
//! identity. This module resolves that identity back through the finite profile
//! catalogs, selects the exact physical entry point, and checks its AQL launch
//! geometry. The resulting recipe still contains no addresses, kernarg values,
//! packets, allocation or queue authority, publication, completion, hardware
//! evidence, performance evidence, or refinement proof.

use fe2o3_aql::{AqlDispatchGeometryV1, AqlGeometryError};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::{Identity, Qwen3ExecutionMode, Qwen3Operator, Qwen3PlanSelection};

use crate::{
    AddresslessM1StepDispatchPlan, M1OperationDispatchKind, M1PhysicalProgramV1,
    M1StepDispatchStage, M1_MAX_STEP_DISPATCHES_V1,
};

/// Addressless physical-dispatch recipe format.
pub const M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1: u32 = 1;

/// Finite profile family whose canonical catalog could not be reconstructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalProfileFamilyV1 {
    /// Dense GEMM/GEMV profiles.
    Gemm,
    /// Token-embedding profiles.
    TokenEmbedding,
    /// RMSNorm/residual profiles.
    RmsNorm,
    /// `RoPE` and paged-KV-write profiles.
    RopeKv,
    /// Causal prefill-attention profiles.
    Prefill,
    /// Paged decode/verification profiles.
    PagedDecode,
    /// `SwiGLU` profiles.
    SwiGlu,
    /// Logits argmax/compact profiles.
    Logits,
}

/// One exact, checked, addressless physical dispatch row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PhysicalDispatchRecipeRowV1 {
    dispatch_index: u32,
    segment_index: u8,
    stage: M1StepDispatchStage,
    selection: Qwen3PlanSelection,
    logical_ordinal: u32,
    operator: Qwen3Operator,
    kind: M1OperationDispatchKind,
    profile_id: Identity,
    program: M1PhysicalProgramV1,
    geometry: AqlDispatchGeometryV1,
    kernarg_bytes: u64,
    dynamic_group_segment_bytes: u32,
}

impl M1PhysicalDispatchRecipeRowV1 {
    /// Zero-based position in the complete single-publication shape.
    #[must_use]
    pub const fn dispatch_index(self) -> u32 {
        self.dispatch_index
    }

    /// Zero-based operation-expansion segment position.
    #[must_use]
    pub const fn segment_index(self) -> u8 {
        self.segment_index
    }

    /// Exact semantic role of the containing segment.
    #[must_use]
    pub const fn stage(self) -> M1StepDispatchStage {
        self.stage
    }

    /// Exact role, execution mode, and finite bucket.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Operation ordinal within the selected generated plan.
    #[must_use]
    pub const fn logical_ordinal(self) -> u32 {
        self.logical_ordinal
    }

    /// Generated Qwen operation represented by this row.
    #[must_use]
    pub const fn operator(self) -> Qwen3Operator {
        self.operator
    }

    /// Whole-operation or exact K7 subdispatch role.
    #[must_use]
    pub const fn kind(self) -> M1OperationDispatchKind {
        self.kind
    }

    /// Canonical finite-profile identity resolved for this row.
    #[must_use]
    pub const fn profile_id(self) -> Identity {
        self.profile_id
    }

    /// Exact entry-point ordinal required from a future content-bound roster.
    #[must_use]
    pub const fn program(self) -> M1PhysicalProgramV1 {
        self.program
    }

    /// Stable program index used by a future fixed service batch.
    #[must_use]
    pub const fn program_index(self) -> usize {
        self.program.program_index()
    }

    /// Checked total-workitem grid and workgroup dimensions.
    #[must_use]
    pub const fn geometry(self) -> AqlDispatchGeometryV1 {
        self.geometry
    }

    /// Exact explicit plus COV6 hidden kernarg allocation length.
    #[must_use]
    pub const fn kernarg_bytes(self) -> u64 {
        self.kernarg_bytes
    }

    /// Dynamic group-segment bytes requested by the current artifacts.
    #[must_use]
    pub const fn dynamic_group_segment_bytes(self) -> u32 {
        self.dynamic_group_segment_bytes
    }

    /// Recipe data alone grants no packet or queue authority.
    #[must_use]
    pub const fn grants_dispatch_authority(self) -> bool {
        false
    }
}

/// Complete exact addressless recipe for one M1 step composition.
///
/// This owner intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1PhysicalDispatchRecipeV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1PhysicalDispatchRecipeV1>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1PhysicalDispatchRecipeV1 {
    version: u32,
    composition_id: Identity,
    dispatch_count: u32,
    rows: Box<[M1PhysicalDispatchRecipeRowV1]>,
}

impl AddresslessM1PhysicalDispatchRecipeV1 {
    /// Recipe format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Identity of the exact addressless step composition lowered here.
    #[must_use]
    pub const fn composition_id(&self) -> Identity {
        self.composition_id
    }

    /// Number of exact physical dispatch recipes.
    #[must_use]
    pub const fn dispatch_count(&self) -> u32 {
        self.dispatch_count
    }

    /// Exact recipes in global publication order.
    #[must_use]
    pub fn rows(&self) -> &[M1PhysicalDispatchRecipeRowV1] {
        &self.rows
    }

    /// Addressless recipes authenticate no executable artifact bytes.
    #[must_use]
    pub const fn authenticates_artifacts(&self) -> bool {
        false
    }

    /// Addressless recipes grant no packet, queue, or launch authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// Geometry and entry-point selection prove no operator refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Fail-closed physical-recipe derivation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalDispatchRecipeErrorV1 {
    /// One finite canonical Ferric profile catalog could not be reconstructed.
    CanonicalProfileCatalog(M1PhysicalProfileFamilyV1),
    /// A retained row profile identity was absent from its required catalog.
    ProfileIdentity {
        /// Global dispatch position of the rejected row.
        dispatch_index: u32,
        /// Generated operator whose profile could not be resolved.
        operator: Qwen3Operator,
    },
    /// A row used a K7-only subdispatch kind for another operator or vice versa.
    DispatchKind {
        /// Global dispatch position of the rejected row.
        dispatch_index: u32,
        /// Generated operator of the rejected row.
        operator: Qwen3Operator,
        /// Rejected physical subdispatch kind.
        kind: M1OperationDispatchKind,
    },
    /// Target compact completion was unavailable for the resolved profile.
    CompactGeometry { dispatch_index: u32 },
    /// Segment-local and complete-step row positions did not agree.
    DispatchOrder {
        /// Required next global dispatch position.
        expected: u32,
        /// Rejected derived global dispatch position.
        actual: u32,
    },
    /// Checked row-position arithmetic overflowed.
    ArithmeticOverflow,
    /// Derived row count differed from the retained complete-step count.
    DispatchCount { expected: u32, actual: u32 },
    /// The retained complete step exceeded the reviewed fixed-batch ceiling.
    Capacity { required: u32, capacity: u32 },
    /// A finite profile produced an invalid generic AQL launch geometry.
    Geometry {
        /// Global dispatch position of the rejected row.
        dispatch_index: u32,
        /// Exact generic geometry rejection.
        error: AqlGeometryError,
    },
}

struct CanonicalPhysicalProfiles {
    gemm: gemm::Qwen3GemmProfileCatalogV1,
    embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1,
    rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1,
    rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1,
    prefill: prefill::Qwen3PrefillProfileCatalogV1,
    paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1,
    swiglu: swiglu::Qwen3SwiGluProfileCatalogV1,
    logits: logits::Qwen3LogitsProfileCatalogV1,
}

impl CanonicalPhysicalProfiles {
    fn new() -> Result<Self, M1PhysicalDispatchRecipeErrorV1> {
        Ok(Self {
            gemm: gemm::Qwen3GemmProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Gemm,
                )
            })?,
            embedding: gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::TokenEmbedding,
                )
            })?,
            rmsnorm: rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::RmsNorm,
                )
            })?,
            rope_kv: rope_kv::Qwen3RopeKvProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::RopeKv,
                )
            })?,
            prefill: prefill::Qwen3PrefillProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Prefill,
                )
            })?,
            paged_decode: paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical().map_err(
                |_| {
                    M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                        M1PhysicalProfileFamilyV1::PagedDecode,
                    )
                },
            )?,
            swiglu: swiglu::Qwen3SwiGluProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::SwiGlu,
                )
            })?,
            logits: logits::Qwen3LogitsProfileCatalogV1::canonical().map_err(|_| {
                M1PhysicalDispatchRecipeErrorV1::CanonicalProfileCatalog(
                    M1PhysicalProfileFamilyV1::Logits,
                )
            })?,
        })
    }
}

struct ResolvedPhysicalProfile {
    program: M1PhysicalProgramV1,
    grid: [u32; 3],
    workgroup: [u32; 3],
    kernarg_bytes: u64,
}

/// Derives exact entry-point, kernarg-size, and AQL-geometry recipes.
///
/// # Errors
///
/// Returns an error for catalog drift, unresolved retained profile identity,
/// invalid K7 subdispatch selection, row-order/count drift, arithmetic
/// overflow, capacity excess, or generic AQL geometry rejection.
pub fn derive_m1_physical_dispatch_recipe_v1(
    step: &AddresslessM1StepDispatchPlan,
) -> Result<AddresslessM1PhysicalDispatchRecipeV1, M1PhysicalDispatchRecipeErrorV1> {
    if step.dispatch_count() > M1_MAX_STEP_DISPATCHES_V1 {
        return Err(M1PhysicalDispatchRecipeErrorV1::Capacity {
            required: step.dispatch_count(),
            capacity: M1_MAX_STEP_DISPATCHES_V1,
        });
    }
    let profiles = CanonicalPhysicalProfiles::new()?;
    let capacity = usize::try_from(step.dispatch_count())
        .map_err(|_| M1PhysicalDispatchRecipeErrorV1::ArithmeticOverflow)?;
    let mut rows = Vec::with_capacity(capacity);
    let mut expected_dispatch_index = 0_u32;

    for segment in step.segments() {
        for row in segment.rows() {
            let dispatch_index = segment
                .dispatch_start()
                .checked_add(row.dispatch_index())
                .ok_or(M1PhysicalDispatchRecipeErrorV1::ArithmeticOverflow)?;
            if dispatch_index != expected_dispatch_index {
                return Err(M1PhysicalDispatchRecipeErrorV1::DispatchOrder {
                    expected: expected_dispatch_index,
                    actual: dispatch_index,
                });
            }
            let resolved = resolve_physical_profile(
                &profiles,
                dispatch_index,
                segment.selection().mode,
                row.operator(),
                row.kind(),
                row.operation().profile_id(),
            )?;
            let geometry =
                AqlDispatchGeometryV1::new(resolved.grid, resolved.workgroup).map_err(|error| {
                    M1PhysicalDispatchRecipeErrorV1::Geometry {
                        dispatch_index,
                        error,
                    }
                })?;
            rows.push(M1PhysicalDispatchRecipeRowV1 {
                dispatch_index,
                segment_index: segment.segment_index(),
                stage: segment.stage(),
                selection: segment.selection(),
                logical_ordinal: row.logical_ordinal(),
                operator: row.operator(),
                kind: row.kind(),
                profile_id: row.operation().profile_id(),
                program: resolved.program,
                geometry,
                kernarg_bytes: resolved.kernarg_bytes,
                dynamic_group_segment_bytes: 0,
            });
            expected_dispatch_index = expected_dispatch_index
                .checked_add(1)
                .ok_or(M1PhysicalDispatchRecipeErrorV1::ArithmeticOverflow)?;
        }
    }

    if expected_dispatch_index != step.dispatch_count() {
        return Err(M1PhysicalDispatchRecipeErrorV1::DispatchCount {
            expected: step.dispatch_count(),
            actual: expected_dispatch_index,
        });
    }
    Ok(AddresslessM1PhysicalDispatchRecipeV1 {
        version: M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
        composition_id: step.composition_id(),
        dispatch_count: expected_dispatch_index,
        rows: rows.into_boxed_slice(),
    })
}

fn resolve_physical_profile(
    catalogs: &CanonicalPhysicalProfiles,
    dispatch_index: u32,
    mode: Qwen3ExecutionMode,
    operator: Qwen3Operator,
    kind: M1OperationDispatchKind,
    profile_id: Identity,
) -> Result<ResolvedPhysicalProfile, M1PhysicalDispatchRecipeErrorV1> {
    if operator != Qwen3Operator::ArgmaxCompactCompletion
        && kind != M1OperationDispatchKind::WholeOperation
    {
        return Err(M1PhysicalDispatchRecipeErrorV1::DispatchKind {
            dispatch_index,
            operator,
            kind,
        });
    }

    let unresolved = || M1PhysicalDispatchRecipeErrorV1::ProfileIdentity {
        dispatch_index,
        operator,
    };
    match operator {
        Qwen3Operator::TokenEmbedding => {
            let profile = catalogs
                .embedding
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            Ok(ResolvedPhysicalProfile {
                program: M1PhysicalProgramV1::TokenEmbedding,
                grid: profile.aql_grid_workitems(),
                workgroup: gemm::QWEN3_GEMM_WORKGROUP_V1,
                kernarg_bytes: gemm::QWEN3_TOKEN_EMBEDDING_TOTAL_KERNARG_BYTES_V1,
            })
        }
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => {
            let profile = catalogs
                .gemm
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            let program = match profile.schedule() {
                gemm::Qwen3GemmScheduleV1::ReferenceWave64V1 => M1PhysicalProgramV1::GemmReference,
                gemm::Qwen3GemmScheduleV1::VectorizedA4Wave64V1 => {
                    M1PhysicalProgramV1::GemmVectorized
                }
            };
            Ok(ResolvedPhysicalProfile {
                program,
                grid: profile.aql_grid_workitems(),
                workgroup: gemm::QWEN3_GEMM_WORKGROUP_V1,
                kernarg_bytes: gemm::QWEN3_GEMM_TOTAL_KERNARG_BYTES_V1,
            })
        }
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => {
            let profile = catalogs
                .rmsnorm
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            Ok(ResolvedPhysicalProfile {
                program: M1PhysicalProgramV1::RmsNorm,
                grid: profile.aql_grid_work_items(),
                workgroup: rmsnorm::QWEN3_RMSNORM_WORKGROUP_V1,
                kernarg_bytes: rmsnorm::QWEN3_RMSNORM_TOTAL_KERNARG_BYTES_V1,
            })
        }
        Qwen3Operator::Rope | Qwen3Operator::KvWrite => {
            let profile = catalogs
                .rope_kv
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            let (program, kernarg_bytes) = match operator {
                Qwen3Operator::Rope => (
                    M1PhysicalProgramV1::Rope,
                    rope_kv::QWEN3_ROPE_TOTAL_KERNARG_BYTES_V1,
                ),
                Qwen3Operator::KvWrite => (
                    M1PhysicalProgramV1::PagedKvWrite,
                    rope_kv::QWEN3_KV_WRITE_TOTAL_KERNARG_BYTES_V1,
                ),
                _ => unreachable!(),
            };
            Ok(ResolvedPhysicalProfile {
                program,
                grid: profile.aql_grid_work_items(),
                workgroup: rope_kv::QWEN3_ROPE_KV_WORKGROUP_V1,
                kernarg_bytes,
            })
        }
        Qwen3Operator::Attention => match mode {
            Qwen3ExecutionMode::Prefill => {
                let profile = catalogs
                    .prefill
                    .profiles()
                    .iter()
                    .copied()
                    .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                    .ok_or_else(unresolved)?;
                Ok(ResolvedPhysicalProfile {
                    program: M1PhysicalProgramV1::GqaPrefill,
                    grid: profile.launch_workitems(),
                    workgroup: prefill::QWEN3_PREFILL_WORKGROUP_V1,
                    kernarg_bytes: prefill::QWEN3_PREFILL_TOTAL_KERNARG_BYTES_V1,
                })
            }
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                let profile = catalogs
                    .paged_decode
                    .profiles()
                    .iter()
                    .copied()
                    .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                    .ok_or_else(unresolved)?;
                Ok(ResolvedPhysicalProfile {
                    program: M1PhysicalProgramV1::PagedGqaDecode,
                    grid: profile.launch_workitems(),
                    workgroup: paged_decode::QWEN3_PAGED_DECODE_WORKGROUP_V1,
                    kernarg_bytes: paged_decode::QWEN3_PAGED_DECODE_TOTAL_KERNARG_BYTES_V1,
                })
            }
        },
        Qwen3Operator::SwiGlu => {
            let profile = catalogs
                .swiglu
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            Ok(ResolvedPhysicalProfile {
                program: M1PhysicalProgramV1::SwiGlu,
                grid: profile.launch_workitems(),
                workgroup: swiglu::QWEN3_SWIGLU_WORKGROUP_V1,
                kernarg_bytes: swiglu::QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1,
            })
        }
        Qwen3Operator::ArgmaxCompactCompletion => {
            let profile = catalogs
                .logits
                .profiles()
                .iter()
                .copied()
                .find(|candidate| candidate.identity().as_bytes() == profile_id.as_bytes())
                .ok_or_else(unresolved)?;
            match kind {
                M1OperationDispatchKind::K7Argmax => Ok(ResolvedPhysicalProfile {
                    program: M1PhysicalProgramV1::LogitsArgmax,
                    grid: profile.argmax_grid_workitems(),
                    workgroup: logits::QWEN3_LOGITS_WORKGROUP_V1,
                    kernarg_bytes: logits::QWEN3_LOGITS_ARGMAX_TOTAL_KERNARG_BYTES_V1,
                }),
                M1OperationDispatchKind::K7Compact => Ok(ResolvedPhysicalProfile {
                    program: M1PhysicalProgramV1::LogitsCompact,
                    grid: profile.compact_grid_workitems().ok_or(
                        M1PhysicalDispatchRecipeErrorV1::CompactGeometry { dispatch_index },
                    )?,
                    workgroup: logits::QWEN3_LOGITS_WORKGROUP_V1,
                    kernarg_bytes: logits::QWEN3_LOGITS_COMPACT_TOTAL_KERNARG_BYTES_V1,
                }),
                M1OperationDispatchKind::WholeOperation => {
                    Err(M1PhysicalDispatchRecipeErrorV1::DispatchKind {
                        dispatch_index,
                        operator,
                        kind,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

    use super::{
        derive_m1_physical_dispatch_recipe_v1, M1PhysicalProgramV1,
        M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{derive_m1_step_dispatch_plan, M1StepDispatchIntent};

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    #[test]
    fn exact_recipes_cover_every_reviewed_complete_step_shape() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let mut seen_programs = HashSet::new();
        let intents = [
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            )),
        ];
        for intent in intents {
            let step = derive_m1_step_dispatch_plan(&operation_plan, intent)
                .expect("canonical complete step");
            let recipe = derive_m1_physical_dispatch_recipe_v1(&step)
                .expect("all retained profiles resolve to checked physical recipes");
            assert_eq!(recipe.version(), M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1);
            assert_eq!(recipe.composition_id(), step.composition_id());
            assert_eq!(recipe.dispatch_count(), step.dispatch_count());
            assert_eq!(recipe.rows().len(), step.dispatch_count() as usize);
            assert_eq!(recipe.rows()[0].dispatch_index(), 0);
            assert_eq!(
                recipe.rows().last().unwrap().dispatch_index(),
                step.dispatch_count() - 1
            );
            assert!(recipe
                .rows()
                .iter()
                .all(|row| row.dynamic_group_segment_bytes() == 0));
            seen_programs.extend(recipe.rows().iter().map(|row| row.program()));
            assert!(!recipe.authenticates_artifacts());
            assert!(!recipe.grants_execution_authority());
            assert!(!recipe.proves_refinement());
        }
        assert_eq!(
            seen_programs,
            M1PhysicalProgramV1::ALL.into_iter().collect()
        );
    }

    #[test]
    fn draft_argmax_and_target_compact_select_distinct_k7_programs() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let step = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
        )
        .unwrap();
        let recipe = derive_m1_physical_dispatch_recipe_v1(&step).unwrap();
        let k7 = recipe
            .rows()
            .iter()
            .filter(|row| {
                matches!(
                    row.program(),
                    M1PhysicalProgramV1::LogitsArgmax | M1PhysicalProgramV1::LogitsCompact
                )
            })
            .map(|row| row.program())
            .collect::<Vec<_>>();
        assert_eq!(
            k7,
            [
                M1PhysicalProgramV1::LogitsArgmax,
                M1PhysicalProgramV1::LogitsArgmax,
                M1PhysicalProgramV1::LogitsArgmax,
                M1PhysicalProgramV1::LogitsArgmax,
                M1PhysicalProgramV1::LogitsArgmax,
                M1PhysicalProgramV1::LogitsCompact,
            ]
        );
    }
}
