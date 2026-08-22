//! Addressless composition of complete M1 inference-step dispatch sequences.
//!
//! This layer orders already checked operation expansions into one bounded
//! publication shape. In a speculative round, each draft decode consumes the
//! immediately preceding draft argmax before target verification consumes the
//! complete ordered draft-choice prefix. A K-wide teacher-forced draft graph is
//! therefore not interchangeable with the declared autoregressive sequence.
//!
//! These declarations contain no addresses, packets, allocation authority,
//! authenticated artifacts, queue authority, completion evidence, hardware
//! result, performance result, or refinement proof.

use ferric_spec::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
};
use sha2::{Digest, Sha256};

use crate::{
    derive_m1_operation_dispatch_expansion, DeclaredM1OperationDispatchExpansion,
    DeclaredOperationKernelPlan, M1OperationDispatchExpansionError, M1OperationDispatchRow,
};

const STEP_DISPATCH_IDENTITY_DOMAIN: &[u8] = b"ferric.m1.step-dispatch-composition.v1";

/// Addressless full-step composition format.
pub const M1_STEP_DISPATCH_COMPOSITION_VERSION: u32 = 1;
/// Maximum dispatch rows admitted by one complete M1 step declaration.
pub const M1_MAX_STEP_DISPATCHES_V1: u32 = 8_192;

/// Requested complete inference-step shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepDispatchIntent {
    /// One target prefill or decode graph.
    TargetOnly(Qwen3PlanSelection),
    /// Draft prefill followed by target prefill for the same finite bucket.
    PairedPrefill(Qwen3PlanSelection),
    /// K autoregressive draft decodes followed by one target verification.
    SpeculativeRound(Qwen3PlanSelection),
}

impl M1StepDispatchIntent {
    /// Returns the target selection named by the request.
    #[must_use]
    pub const fn target_selection(self) -> Qwen3PlanSelection {
        match self {
            Self::TargetOnly(selection)
            | Self::PairedPrefill(selection)
            | Self::SpeculativeRound(selection) => selection,
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::TargetOnly(_) => 1,
            Self::PairedPrefill(_) => 2,
            Self::SpeculativeRound(_) => 3,
        }
    }
}

/// Exact role of one ordered operation-expansion segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepDispatchStage {
    /// Sole target graph in a target-only step.
    TargetOnly,
    /// Draft cache initialization before target cache initialization.
    DraftPrefill,
    /// Target cache initialization and final compact completion.
    TargetPrefill,
    /// One autoregressive draft decode and argmax.
    DraftDecode { iteration: u8 },
    /// Target verification and final compact completion.
    TargetVerification { draft_iterations: u8 },
}

impl M1StepDispatchStage {
    fn encode(self, record: &mut Vec<u8>) {
        match self {
            Self::TargetOnly => record.extend_from_slice(&[1, 0]),
            Self::DraftPrefill => record.extend_from_slice(&[2, 0]),
            Self::TargetPrefill => record.extend_from_slice(&[3, 0]),
            Self::DraftDecode { iteration } => record.extend_from_slice(&[4, iteration]),
            Self::TargetVerification { draft_iterations } => {
                record.extend_from_slice(&[5, draft_iterations]);
            }
        }
    }
}

/// Required logical data dependency at one segment boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepDispatchDependency {
    /// Inputs originate outside this publication.
    ExternalInputs,
    /// The segment's token input is the named prior draft argmax.
    PriorDraftArgmax { producer_segment: u8 },
    /// Target verification consumes every ordered draft choice in this range.
    DraftChoicePrefix {
        first_segment: u8,
        segment_count: u8,
    },
}

impl M1StepDispatchDependency {
    fn encode(self, record: &mut Vec<u8>) {
        match self {
            Self::ExternalInputs => record.extend_from_slice(&[1, 0, 0]),
            Self::PriorDraftArgmax { producer_segment } => {
                record.extend_from_slice(&[2, producer_segment, 0]);
            }
            Self::DraftChoicePrefix {
                first_segment,
                segment_count,
            } => record.extend_from_slice(&[3, first_segment, segment_count]),
        }
    }
}

/// One contiguous checked operation expansion in the full-step sequence.
#[derive(Debug, Eq, PartialEq)]
pub struct M1StepDispatchSegment {
    segment_index: u8,
    dispatch_start: u32,
    stage: M1StepDispatchStage,
    dependency: M1StepDispatchDependency,
    expansion: DeclaredM1OperationDispatchExpansion,
}

impl M1StepDispatchSegment {
    /// Zero-based segment position.
    #[must_use]
    pub const fn segment_index(&self) -> u8 {
        self.segment_index
    }

    /// First global dispatch row in this segment.
    #[must_use]
    pub const fn dispatch_start(&self) -> u32 {
        self.dispatch_start
    }

    /// Exact role of this segment.
    #[must_use]
    pub const fn stage(&self) -> M1StepDispatchStage {
        self.stage
    }

    /// Required cross-segment data dependency.
    #[must_use]
    pub const fn dependency(&self) -> M1StepDispatchDependency {
        self.dependency
    }

    /// Exact role, mode, and bucket expanded in this segment.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.expansion.selection()
    }

    /// Number of physical dispatch rows in this segment.
    #[must_use]
    pub const fn dispatch_count(&self) -> u32 {
        self.expansion.physical_dispatch_count()
    }

    /// Exact addressless rows in segment-local order.
    #[must_use]
    pub fn rows(&self) -> &[M1OperationDispatchRow] {
        self.expansion.rows()
    }

    /// Identity of the independently checked operation expansion.
    #[must_use]
    pub const fn expansion_id(&self) -> Identity {
        self.expansion.expansion_id()
    }
}

/// Complete addressless step composition derived from checked operation plans.
///
/// This owner intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1StepDispatchPlan;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1StepDispatchPlan>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1StepDispatchPlan {
    version: u32,
    composition_id: Identity,
    intent: M1StepDispatchIntent,
    runner_declaration_id: Identity,
    kernel_catalog_id: Identity,
    dispatch_count: u32,
    segments: Box<[M1StepDispatchSegment]>,
}

impl AddresslessM1StepDispatchPlan {
    /// Composition format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Deterministic domain-separated composition identity.
    #[must_use]
    pub const fn composition_id(&self) -> Identity {
        self.composition_id
    }

    /// Requested full-step shape.
    #[must_use]
    pub const fn intent(&self) -> M1StepDispatchIntent {
        self.intent
    }

    /// Retained generated-runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.runner_declaration_id
    }

    /// Retained kernel catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.kernel_catalog_id
    }

    /// Total physical dispatch rows in the single publication shape.
    #[must_use]
    pub const fn dispatch_count(&self) -> u32 {
        self.dispatch_count
    }

    /// Ordered expansion segments.
    #[must_use]
    pub fn segments(&self) -> &[M1StepDispatchSegment] {
        &self.segments
    }

    /// The declaration describes exactly one bounded queue publication.
    #[must_use]
    pub const fn publication_count(&self) -> u8 {
        1
    }

    /// Addressless composition grants no packet, queue, or launch authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// Addressless composition authenticates no executable artifact bytes.
    #[must_use]
    pub const fn authenticates_artifacts(&self) -> bool {
        false
    }

    /// Structural ordering alone proves no operator or machine refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Fail-closed full-step composition error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepDispatchCompositionError {
    /// Every full-step intent must name the target model role.
    TargetRole,
    /// Target-only steps admit prefill or decode, never speculative mode.
    TargetOnlyMode,
    /// Paired prefill requires a prefill selection.
    PairedPrefillMode,
    /// The speculative target selection or finite bucket is unsupported.
    SpeculativeSelection,
    /// One checked operation expansion could not be derived.
    Expansion(M1OperationDispatchExpansionError),
    /// Checked count or identity arithmetic overflowed.
    ArithmeticOverflow,
    /// The complete step exceeds the reviewed addressless bound.
    Capacity { required: u32, capacity: u32 },
}

/// Derives one exact addressless full-step dispatch composition.
///
/// Draft prefill precedes target prefill so the target compact completion is
/// last. Speculative steps repeat the matching draft decode graph K times;
/// they never substitute the draft model's K-wide speculative graph. Each
/// target verification expansion is last and ends in compact completion.
///
/// # Errors
///
/// Returns [`M1StepDispatchCompositionError`] for role/mode/bucket drift,
/// operation expansion failure, checked arithmetic overflow, or capacity
/// excess.
pub fn derive_m1_step_dispatch_plan(
    operation_plan: &DeclaredOperationKernelPlan,
    intent: M1StepDispatchIntent,
) -> Result<AddresslessM1StepDispatchPlan, M1StepDispatchCompositionError> {
    let target = intent.target_selection();
    if target.role != Qwen3ModelRole::Target8B {
        return Err(M1StepDispatchCompositionError::TargetRole);
    }

    let mut segments = Vec::with_capacity(match intent {
        M1StepDispatchIntent::TargetOnly(_) => 1,
        M1StepDispatchIntent::PairedPrefill(_) => 2,
        M1StepDispatchIntent::SpeculativeRound(_) => 17,
    });
    let mut dispatch_count = 0_u32;

    match intent {
        M1StepDispatchIntent::TargetOnly(selection) => {
            if !matches!(
                selection.mode,
                Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode
            ) {
                return Err(M1StepDispatchCompositionError::TargetOnlyMode);
            }
            push_segment(
                operation_plan,
                &mut segments,
                &mut dispatch_count,
                M1StepDispatchStage::TargetOnly,
                M1StepDispatchDependency::ExternalInputs,
                selection,
            )?;
        }
        M1StepDispatchIntent::PairedPrefill(selection) => {
            if selection.mode != Qwen3ExecutionMode::Prefill {
                return Err(M1StepDispatchCompositionError::PairedPrefillMode);
            }
            let draft = Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Prefill,
                bucket: selection.bucket,
            };
            push_segment(
                operation_plan,
                &mut segments,
                &mut dispatch_count,
                M1StepDispatchStage::DraftPrefill,
                M1StepDispatchDependency::ExternalInputs,
                draft,
            )?;
            push_segment(
                operation_plan,
                &mut segments,
                &mut dispatch_count,
                M1StepDispatchStage::TargetPrefill,
                M1StepDispatchDependency::ExternalInputs,
                selection,
            )?;
        }
        M1StepDispatchIntent::SpeculativeRound(selection) => {
            let (draft_bucket, iterations) = speculative_draft_decode(selection)?;
            let draft = Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: draft_bucket,
            };
            for iteration in 0..iterations {
                let dependency = if iteration == 0 {
                    M1StepDispatchDependency::ExternalInputs
                } else {
                    M1StepDispatchDependency::PriorDraftArgmax {
                        producer_segment: iteration - 1,
                    }
                };
                push_segment(
                    operation_plan,
                    &mut segments,
                    &mut dispatch_count,
                    M1StepDispatchStage::DraftDecode { iteration },
                    dependency,
                    draft,
                )?;
            }
            push_segment(
                operation_plan,
                &mut segments,
                &mut dispatch_count,
                M1StepDispatchStage::TargetVerification {
                    draft_iterations: iterations,
                },
                M1StepDispatchDependency::DraftChoicePrefix {
                    first_segment: 0,
                    segment_count: iterations,
                },
                selection,
            )?;
        }
    }

    let mut plan = AddresslessM1StepDispatchPlan {
        version: M1_STEP_DISPATCH_COMPOSITION_VERSION,
        composition_id: Identity::new([0; 32]),
        intent,
        runner_declaration_id: operation_plan.runner_declaration_id(),
        kernel_catalog_id: operation_plan.kernel_catalog_id(),
        dispatch_count,
        segments: segments.into_boxed_slice(),
    };
    plan.composition_id = composition_identity(&plan)?;
    Ok(plan)
}

fn speculative_draft_decode(
    selection: Qwen3PlanSelection,
) -> Result<(Qwen3PlanBucket, u8), M1StepDispatchCompositionError> {
    if selection.mode != Qwen3ExecutionMode::Speculative {
        return Err(M1StepDispatchCompositionError::SpeculativeSelection);
    }
    match selection.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 => Ok((Qwen3PlanBucket::DecodeS1C8192, 4)),
        Qwen3PlanBucket::SpeculativeS8K4C8192 => Ok((Qwen3PlanBucket::DecodeS8C8192, 4)),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => Ok((Qwen3PlanBucket::DecodeS1C8192, 8)),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => Ok((Qwen3PlanBucket::DecodeS1C8192, 16)),
        _ => Err(M1StepDispatchCompositionError::SpeculativeSelection),
    }
}

fn push_segment(
    operation_plan: &DeclaredOperationKernelPlan,
    segments: &mut Vec<M1StepDispatchSegment>,
    dispatch_count: &mut u32,
    stage: M1StepDispatchStage,
    dependency: M1StepDispatchDependency,
    selection: Qwen3PlanSelection,
) -> Result<(), M1StepDispatchCompositionError> {
    let segment_index = u8::try_from(segments.len())
        .map_err(|_| M1StepDispatchCompositionError::ArithmeticOverflow)?;
    let expansion = derive_m1_operation_dispatch_expansion(operation_plan, selection)
        .map_err(M1StepDispatchCompositionError::Expansion)?;
    let next = dispatch_count
        .checked_add(expansion.physical_dispatch_count())
        .ok_or(M1StepDispatchCompositionError::ArithmeticOverflow)?;
    if next > M1_MAX_STEP_DISPATCHES_V1 {
        return Err(M1StepDispatchCompositionError::Capacity {
            required: next,
            capacity: M1_MAX_STEP_DISPATCHES_V1,
        });
    }
    segments.push(M1StepDispatchSegment {
        segment_index,
        dispatch_start: *dispatch_count,
        stage,
        dependency,
        expansion,
    });
    *dispatch_count = next;
    Ok(())
}

fn composition_identity(
    plan: &AddresslessM1StepDispatchPlan,
) -> Result<Identity, M1StepDispatchCompositionError> {
    let segment_count = u64::try_from(plan.segments.len())
        .map_err(|_| M1StepDispatchCompositionError::ArithmeticOverflow)?;
    let mut record = Vec::with_capacity(128 + plan.segments.len() * 64);
    record.extend_from_slice(&plan.version.to_le_bytes());
    record.push(plan.intent.tag());
    encode_selection(&mut record, plan.intent.target_selection());
    record.extend_from_slice(plan.runner_declaration_id.as_bytes());
    record.extend_from_slice(plan.kernel_catalog_id.as_bytes());
    record.extend_from_slice(&plan.dispatch_count.to_le_bytes());
    record.extend_from_slice(&segment_count.to_le_bytes());
    for segment in &plan.segments {
        record.push(segment.segment_index);
        record.extend_from_slice(&segment.dispatch_start.to_le_bytes());
        segment.stage.encode(&mut record);
        segment.dependency.encode(&mut record);
        encode_selection(&mut record, segment.selection());
        record.extend_from_slice(&segment.dispatch_count().to_le_bytes());
        record.extend_from_slice(segment.expansion_id().as_bytes());
    }
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, STEP_DISPATCH_IDENTITY_DOMAIN)?;
    hash_field(&mut hasher, &record)?;
    Ok(Identity::new(hasher.finalize().into()))
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), M1StepDispatchCompositionError> {
    let length = u64::try_from(bytes.len())
        .map_err(|_| M1StepDispatchCompositionError::ArithmeticOverflow)?;
    hasher.update(length.to_le_bytes());
    hasher.update(bytes);
    Ok(())
}

fn encode_selection(record: &mut Vec<u8>, selection: Qwen3PlanSelection) {
    record.push(match selection.role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    });
    record.push(match selection.mode {
        Qwen3ExecutionMode::Prefill => 1,
        Qwen3ExecutionMode::Decode => 2,
        Qwen3ExecutionMode::Speculative => 3,
    });
    record.push(match selection.bucket {
        Qwen3PlanBucket::PrefillS1T128 => 1,
        Qwen3PlanBucket::PrefillS8T128 => 2,
        Qwen3PlanBucket::PrefillS1T512 => 3,
        Qwen3PlanBucket::PrefillS1T2048 => 4,
        Qwen3PlanBucket::DecodeS1C8192 => 5,
        Qwen3PlanBucket::DecodeS8C8192 => 6,
        Qwen3PlanBucket::DecodeS32C8192 => 7,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 10,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 11,
    });
}

#[cfg(test)]
mod tests {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

    use super::{
        derive_m1_step_dispatch_plan, M1StepDispatchCompositionError, M1StepDispatchDependency,
        M1StepDispatchIntent, M1StepDispatchStage,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::M1OperationDispatchKind;

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    #[test]
    fn target_only_and_paired_prefill_end_in_target_compact_completion() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let target_decode = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
        )
        .expect("canonical target decode composes");
        assert_eq!(target_decode.dispatch_count(), 545);
        assert_eq!(target_decode.publication_count(), 1);
        assert_eq!(target_decode.segments().len(), 1);
        assert_eq!(
            target_decode.segments()[0].rows().last().unwrap().kind(),
            M1OperationDispatchKind::K7Compact
        );

        let paired = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
        )
        .expect("canonical paired prefill composes");
        assert_eq!(paired.dispatch_count(), 969);
        assert_eq!(paired.segments().len(), 2);
        assert_eq!(
            paired.segments()[0].stage(),
            M1StepDispatchStage::DraftPrefill
        );
        assert_eq!(paired.segments()[0].dispatch_count(), 424);
        assert_eq!(
            paired.segments()[1].stage(),
            M1StepDispatchStage::TargetPrefill
        );
        assert_eq!(paired.segments()[1].dispatch_start(), 424);
        assert_eq!(
            paired.segments()[1].rows().last().unwrap().kind(),
            M1OperationDispatchKind::K7Compact
        );
    }

    #[test]
    fn speculative_rounds_repeat_decode_and_bind_every_argmax_dependency() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let cases = [
            (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                4,
                2_241,
            ),
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                Qwen3PlanBucket::DecodeS8C8192,
                4,
                2_241,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                8,
                3_937,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                16,
                7_329,
            ),
        ];
        for (bucket, draft_bucket, iterations, dispatch_count) in cases {
            let plan = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    bucket,
                )),
            )
            .expect("canonical speculative round composes");
            assert_eq!(plan.dispatch_count(), dispatch_count);
            assert_eq!(plan.publication_count(), 1);
            assert_eq!(plan.segments().len(), usize::from(iterations) + 1);
            for iteration in 0..iterations {
                let segment = &plan.segments()[usize::from(iteration)];
                assert_eq!(
                    segment.stage(),
                    M1StepDispatchStage::DraftDecode { iteration }
                );
                assert_eq!(segment.selection().role, Qwen3ModelRole::Draft06B);
                assert_eq!(segment.selection().mode, Qwen3ExecutionMode::Decode);
                assert_eq!(segment.selection().bucket, draft_bucket);
                assert_eq!(segment.dispatch_count(), 424);
                assert_eq!(
                    segment.dependency(),
                    if iteration == 0 {
                        M1StepDispatchDependency::ExternalInputs
                    } else {
                        M1StepDispatchDependency::PriorDraftArgmax {
                            producer_segment: iteration - 1,
                        }
                    }
                );
                assert_eq!(
                    segment.rows().last().unwrap().kind(),
                    M1OperationDispatchKind::K7Argmax
                );
            }
            let target_segment = plan.segments().last().unwrap();
            assert_eq!(
                target_segment.stage(),
                M1StepDispatchStage::TargetVerification {
                    draft_iterations: iterations,
                }
            );
            assert_eq!(
                target_segment.dependency(),
                M1StepDispatchDependency::DraftChoicePrefix {
                    first_segment: 0,
                    segment_count: iterations,
                }
            );
            assert_eq!(target_segment.dispatch_count(), 545);
            assert_eq!(
                target_segment.rows().last().unwrap().kind(),
                M1OperationDispatchKind::K7Compact
            );
        }
    }

    #[test]
    fn role_mode_and_teacher_forced_substitutions_fail_closed() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let draft = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        assert_eq!(
            derive_m1_step_dispatch_plan(&operation_plan, M1StepDispatchIntent::TargetOnly(draft),),
            Err(M1StepDispatchCompositionError::TargetRole)
        );
        assert_eq!(
            derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::DecodeS1C8192,
                )),
            ),
            Err(M1StepDispatchCompositionError::SpeculativeSelection)
        );
        assert_eq!(
            derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::TargetOnly(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K4C8192,
                )),
            ),
            Err(M1StepDispatchCompositionError::TargetOnlyMode)
        );
    }

    #[test]
    fn composition_identity_is_deterministic_and_grants_no_authority() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let intent = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ));
        let first = derive_m1_step_dispatch_plan(&operation_plan, intent).unwrap();
        let second = derive_m1_step_dispatch_plan(&operation_plan, intent).unwrap();
        assert_eq!(first.composition_id(), second.composition_id());
        assert!(!first.authenticates_artifacts());
        assert!(!first.grants_execution_authority());
        assert!(!first.proves_refinement());
    }
}
