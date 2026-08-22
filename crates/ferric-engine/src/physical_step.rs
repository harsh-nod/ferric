//! Inert identity binding between a logical step and a structural physical plan.
//!
//! This module performs no packet construction, fusion proof, authentication,
//! allocation, queue operation, publication, completion, hardware action,
//! performance claim, or qualification. The retained values remain declaration
//! data. In particular, the reviewed capacity tag is not fe2o3 support evidence.

use ferric_spec::{
    PhysicalCapacitySource, Qwen3ModelRole, StepPlan, StructurallyValidatedPhysicalPlan,
    QWEN3_DRAFT_PLAN_STEPS, QWEN3_TARGET_PLAN_STEPS,
};
use vstd::prelude::*;

verus! {

/// Fail-closed structural binding error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuralPhysicalStepBindingError {
    /// A future capacity declaration cannot enter reviewed engine custody.
    FutureCapacityUntrusted,
    /// The physical declaration names a different logical plan identity.
    PlanIdentityMismatch,
    /// The physical declaration names a different role, mode, or bucket.
    SelectionMismatch,
    /// The physical declaration does not retain the exact role operation count.
    LogicalOperationCountMismatch,
}

/// Retry-safe failure retaining the exact inert inputs.
#[derive(Debug, PartialEq, Eq)]
pub struct StructuralPhysicalStepBindingFailure {
    error: StructuralPhysicalStepBindingError,
    step: StepPlan,
    physical: StructurallyValidatedPhysicalPlan,
}

impl StructuralPhysicalStepBindingFailure {
    pub closed spec fn error_spec(&self) -> StructuralPhysicalStepBindingError {
        self.error
    }

    pub closed spec fn step_spec(&self) -> StepPlan {
        self.step
    }

    pub closed spec fn physical_declaration_spec(&self) -> ferric_spec::PhysicalPlanDeclaration {
        self.physical.declaration_spec()
    }

    pub closed spec fn physical_capacity_source_spec(&self) -> PhysicalCapacitySource {
        self.physical.capacity_source_spec()
    }

    /// Returns the diagnostic without consuming the retained inputs.
    #[must_use]
    pub const fn error(&self) -> (error: StructuralPhysicalStepBindingError)
        ensures error == self.error_spec(),
    {
        self.error
    }

    /// Recovers the exact unchanged logical and physical declaration inputs.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (parts: (
        StructuralPhysicalStepBindingError,
        StepPlan,
        StructurallyValidatedPhysicalPlan,
    ))
        ensures
            parts.0 == self.error_spec(),
            parts.1 == self.step_spec(),
            parts.2.declaration_spec() == self.physical_declaration_spec(),
            parts.2.capacity_source_spec() == self.physical_capacity_source_spec(),
    {
        (self.error, self.step, self.physical)
    }
}

/// Engine custody of one exact structural logical/physical identity binding.
///
/// This value is intentionally not `Clone` and exposes no execution operation.
#[derive(Debug, PartialEq, Eq)]
pub struct StructurallyBoundPhysicalStep {
    step: StepPlan,
    physical: StructurallyValidatedPhysicalPlan,
}

impl StructurallyBoundPhysicalStep {
    pub closed spec fn step_spec(&self) -> StepPlan {
        self.step
    }

    pub closed spec fn physical_declaration_spec(&self) -> ferric_spec::PhysicalPlanDeclaration {
        self.physical.declaration_spec()
    }

    pub closed spec fn physical_capacity_source_spec(&self) -> PhysicalCapacitySource {
        self.physical.capacity_source_spec()
    }

    /// Returns the retained logical step declaration.
    #[must_use]
    pub const fn step(&self) -> (step: StepPlan)
        ensures step == self.step_spec(),
    {
        self.step
    }

    /// Returns the retained inert physical declaration.
    #[must_use]
    pub const fn physical_plan(&self) -> (physical: &StructurallyValidatedPhysicalPlan)
        ensures
            physical.declaration_spec() == self.physical_declaration_spec(),
            physical.capacity_source_spec() == self.physical_capacity_source_spec(),
    {
        &self.physical
    }
}

/// Exact linear outcome of one structural binding attempt.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum StructuralPhysicalStepBindingOutcome {
    /// The exact inert inputs satisfy the structural binding relation.
    Bound(StructurallyBoundPhysicalStep),
    /// The exact unchanged inputs are retained for diagnosis or retry.
    Rejected(StructuralPhysicalStepBindingFailure),
}

impl StructuralPhysicalStepBindingOutcome {
    pub closed spec fn is_bound_spec(&self) -> bool {
        match self {
            Self::Bound(_) => true,
            Self::Rejected(_) => false,
        }
    }
}

pub closed spec fn structural_binding_error_matches(
    error: StructuralPhysicalStepBindingError,
    step: StepPlan,
    physical: &StructurallyValidatedPhysicalPlan,
) -> bool {
    let declaration = physical.declaration_spec();
    match error {
        StructuralPhysicalStepBindingError::FutureCapacityUntrusted => {
            !reviewed_capacity_source(physical.capacity_source_spec())
        },
        StructuralPhysicalStepBindingError::PlanIdentityMismatch => {
            reviewed_capacity_source(physical.capacity_source_spec())
                && step.plan_id_spec().bytes_spec() != declaration.logical_plan_id.bytes_spec()
        },
        StructuralPhysicalStepBindingError::SelectionMismatch => {
            reviewed_capacity_source(physical.capacity_source_spec())
                && step.plan_id_spec().bytes_spec() == declaration.logical_plan_id.bytes_spec()
                && step.selection_spec() != declaration.selection
        },
        StructuralPhysicalStepBindingError::LogicalOperationCountMismatch => {
            reviewed_capacity_source(physical.capacity_source_spec())
                && step.plan_id_spec().bytes_spec() == declaration.logical_plan_id.bytes_spec()
                && step.selection_spec() == declaration.selection
                && !role_operation_count_matches(step, physical)
        },
    }
}

closed spec fn reviewed_capacity_source(source: PhysicalCapacitySource) -> bool {
    source == PhysicalCapacitySource::ReviewedBatchArithmeticV1
        || source == PhysicalCapacitySource::ReviewedBatchArithmeticV2
        || source == PhysicalCapacitySource::ReviewedBatchArithmeticV3
}

closed spec fn role_operation_count_matches(
    step: StepPlan,
    physical: &StructurallyValidatedPhysicalPlan,
) -> bool {
    physical.declaration_spec().logical_operation_count
        == match step.selection_spec().role {
            Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
            Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
        }
}

/// Exact inert identity relation established by the binder.
///
/// This relation says nothing about authentication, fusion refinement, packet
/// execution, publication, completion, hardware, performance, or qualification.
pub closed spec fn structural_step_physical_binding_matches(
    step: StepPlan,
    physical: &StructurallyValidatedPhysicalPlan,
) -> bool {
    let declaration = physical.declaration_spec();
    reviewed_capacity_source(physical.capacity_source_spec())
        && step.plan_id_spec().bytes_spec() == declaration.logical_plan_id.bytes_spec()
        && step.selection_spec() == declaration.selection
        && role_operation_count_matches(step, physical)
}

fn is_reviewed_capacity_source(source: PhysicalCapacitySource) -> (reviewed: bool)
    ensures reviewed == reviewed_capacity_source(source),
{
    match source {
        PhysicalCapacitySource::ReviewedBatchArithmeticV1
        | PhysicalCapacitySource::ReviewedBatchArithmeticV2
        | PhysicalCapacitySource::ReviewedBatchArithmeticV3 => true,
        PhysicalCapacitySource::FutureUntrusted => false,
    }
}

fn failure(
    error: StructuralPhysicalStepBindingError,
    step: StepPlan,
    physical: StructurallyValidatedPhysicalPlan,
) -> (failure: StructuralPhysicalStepBindingFailure)
    ensures
        failure.error_spec() == error,
        failure.step_spec() == step,
        failure.physical_declaration_spec() == physical.declaration_spec(),
        failure.physical_capacity_source_spec() == physical.capacity_source_spec(),
{
    StructuralPhysicalStepBindingFailure { error, step, physical }
}

fn bound(
    step: StepPlan,
    physical: StructurallyValidatedPhysicalPlan,
) -> (bound: StructurallyBoundPhysicalStep)
    ensures
        bound.step_spec() == step,
        bound.physical_declaration_spec() == physical.declaration_spec(),
        bound.physical_capacity_source_spec() == physical.capacity_source_spec(),
{
    StructurallyBoundPhysicalStep { step, physical }
}

/// Binds one logical step to one exact inert physical declaration.
///
/// The rejected outcome retains both unchanged inputs unless the physical value
/// has reviewed capacity provenance, the exact logical plan identity, the exact
/// role/mode/bucket selection, and the exact role operation count. Success
/// grants no physical execution or publication authority.
pub fn bind_structural_physical_step(
    step: StepPlan,
    physical: StructurallyValidatedPhysicalPlan,
) -> (result: StructuralPhysicalStepBindingOutcome)
    ensures
        result.is_bound_spec() == structural_step_physical_binding_matches(step, &physical),
        match result {
            StructuralPhysicalStepBindingOutcome::Bound(bound) => {
                &&& bound.step_spec() == step
                &&& bound.physical_declaration_spec() == physical.declaration_spec()
                &&& bound.physical_capacity_source_spec() == physical.capacity_source_spec()
            },
            StructuralPhysicalStepBindingOutcome::Rejected(failure) => {
                &&& structural_binding_error_matches(failure.error_spec(), step, &physical)
                &&& failure.step_spec() == step
                &&& failure.physical_declaration_spec() == physical.declaration_spec()
                &&& failure.physical_capacity_source_spec() == physical.capacity_source_spec()
            },
        },
{
    proof {
        reveal(structural_step_physical_binding_matches);
        reveal(structural_binding_error_matches);
        reveal(reviewed_capacity_source);
        reveal(role_operation_count_matches);
    }
    if !is_reviewed_capacity_source(physical.capacity_source()) {
        return StructuralPhysicalStepBindingOutcome::Rejected(failure(
            StructuralPhysicalStepBindingError::FutureCapacityUntrusted,
            step,
            physical,
        ));
    }
    if !step
        .plan_id()
        .equals(&physical.declaration().logical_plan_id)
    {
        return StructuralPhysicalStepBindingOutcome::Rejected(failure(
            StructuralPhysicalStepBindingError::PlanIdentityMismatch,
            step,
            physical,
        ));
    }
    if !step
        .selection()
        .matches(physical.declaration().selection)
    {
        return StructuralPhysicalStepBindingOutcome::Rejected(failure(
            StructuralPhysicalStepBindingError::SelectionMismatch,
            step,
            physical,
        ));
    }
    let expected_operations = match step.selection().role {
        Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
    };
    if physical.declaration().logical_operation_count != expected_operations {
        return StructuralPhysicalStepBindingOutcome::Rejected(failure(
            StructuralPhysicalStepBindingError::LogicalOperationCountMismatch,
            step,
            physical,
        ));
    }
    StructuralPhysicalStepBindingOutcome::Bound(bound(step, physical))
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        bind_structural_physical_step, StructuralPhysicalStepBindingError,
        StructuralPhysicalStepBindingOutcome,
    };
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::{
        validate_physical_plan_declaration, DeclaredFusionRefinementPremise, Identity,
        PhysicalCapacityExpectation, PhysicalCapacitySource, PhysicalCompletionDeclaration,
        PhysicalPacketIdentityBinding, PhysicalPacketSpanDeclaration, PhysicalPlanDeclaration,
        PhysicalPlanError, PhysicalPlanExpectation, PhysicalPublicationDeclaration,
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
        StepPlan, StructurallyValidatedPhysicalPlan, M1_PHYSICAL_PLAN_DECLARATION_VERSION,
        M1_REVIEWED_BATCH_PACKET_CAPACITY_V1, M1_REVIEWED_BATCH_PACKET_CAPACITY_V2,
        M1_REVIEWED_BATCH_PACKET_CAPACITY_V3, QWEN3_TARGET_PLAN_STEPS,
    };

    fn identity(role: u8, index: u32) -> Identity {
        let mut bytes = [0u8; 32];
        bytes[0] = role;
        bytes[1..5].copy_from_slice(&index.to_le_bytes());
        Identity::new(bytes)
    }

    const fn target_selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    const fn draft_selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn synthetic_unproved_physical_plan(
        source: PhysicalCapacitySource,
        selection: Qwen3PlanSelection,
        logical_operation_count: u32,
        logical_plan_id: Identity,
    ) -> Result<StructurallyValidatedPhysicalPlan, PhysicalPlanError> {
        let batch_packet_capacity = match source {
            PhysicalCapacitySource::ReviewedBatchArithmeticV1
            | PhysicalCapacitySource::FutureUntrusted => M1_REVIEWED_BATCH_PACKET_CAPACITY_V1,
            PhysicalCapacitySource::ReviewedBatchArithmeticV2 => {
                M1_REVIEWED_BATCH_PACKET_CAPACITY_V2
            }
            PhysicalCapacitySource::ReviewedBatchArithmeticV3 => {
                M1_REVIEWED_BATCH_PACKET_CAPACITY_V3
            }
        };
        let capacity = PhysicalCapacityExpectation {
            source,
            descriptor_id: identity(10, 0),
            batch_packet_capacity,
            ring_packet_capacity: batch_packet_capacity,
        };
        let declaration = PhysicalPlanDeclaration {
            version: M1_PHYSICAL_PLAN_DECLARATION_VERSION,
            declaration_id: identity(1, 0),
            source_closure_id: identity(2, 0),
            logical_plan_id,
            selection,
            logical_operation_count,
            capacity_descriptor_id: capacity.descriptor_id,
            declared_batch_packet_capacity: capacity.batch_packet_capacity,
            declared_ring_packet_capacity: capacity.ring_packet_capacity,
            packets: vec![PhysicalPacketSpanDeclaration {
                packet_index: 0,
                logical_start: 0,
                logical_count: logical_operation_count,
                identities: PhysicalPacketIdentityBinding {
                    kernel_contract_id: identity(20, 0),
                    artifact_id: identity(21, 0),
                    descriptor_id: identity(22, 0),
                    geometry_id: identity(23, 0),
                    kernarg_layout_id: identity(24, 0),
                    buffer_layout_id: identity(25, 0),
                    effect_contract_id: identity(26, 0),
                },
                predecessors: Vec::new(),
                fusion: Some(DeclaredFusionRefinementPremise {
                    relation_id: identity(27, 0),
                    evidence_requirement_id: identity(28, 0),
                }),
            }],
            publication: PhysicalPublicationDeclaration {
                contract_id: identity(4, 0),
                reservation_count: 1,
                reserved_packet_count: 1,
                release_header_count: 1,
                doorbell_count: 1,
                doorbell_packet_index: 0,
            },
            completion: PhysicalCompletionDeclaration {
                contract_id: identity(5, 0),
                completion_packet_index: 0,
                completion_signal_count: 1,
                declared_dominated_packet_count: 1,
            },
        };
        let expectation = PhysicalPlanExpectation {
            expected: declaration.clone(),
            capacity,
        };
        validate_physical_plan_declaration(declaration, &expectation)
    }

    fn step(plan_id: Identity, selection: Qwen3PlanSelection) -> StepPlan {
        StepPlan::new(
            RequestId::new(3, 7),
            CompletionEpoch::new(11),
            plan_id,
            selection,
        )
    }

    #[test]
    fn exact_reviewed_binding_retains_both_inert_inputs() {
        for source in [
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            PhysicalCapacitySource::ReviewedBatchArithmeticV2,
            PhysicalCapacitySource::ReviewedBatchArithmeticV3,
        ] {
            let plan_id = identity(3, 0);
            let physical = synthetic_unproved_physical_plan(
                source,
                target_selection(),
                QWEN3_TARGET_PLAN_STEPS,
                plan_id,
            )
            .unwrap();
            let expected_declaration = physical.declaration().clone();
            let expected_step = step(plan_id, target_selection());
            let bound = match bind_structural_physical_step(expected_step, physical) {
                StructuralPhysicalStepBindingOutcome::Bound(bound) => bound,
                StructuralPhysicalStepBindingOutcome::Rejected(failure) => {
                    panic!("unexpected structural binding rejection: {failure:?}")
                }
            };

            assert_eq!(bound.step(), expected_step);
            assert_eq!(bound.physical_plan().declaration(), &expected_declaration);
            assert_eq!(bound.physical_plan().capacity_source(), source);
        }
    }

    #[test]
    fn future_capacity_fails_and_returns_exact_inputs() {
        let plan_id = identity(3, 0);
        let physical = synthetic_unproved_physical_plan(
            PhysicalCapacitySource::FutureUntrusted,
            target_selection(),
            QWEN3_TARGET_PLAN_STEPS,
            plan_id,
        )
        .unwrap();
        let expected_declaration = physical.declaration().clone();
        let expected_step = step(plan_id, target_selection());
        let failure = match bind_structural_physical_step(expected_step, physical) {
            StructuralPhysicalStepBindingOutcome::Bound(bound) => {
                panic!("unexpected structural binding success: {bound:?}")
            }
            StructuralPhysicalStepBindingOutcome::Rejected(failure) => failure,
        };

        assert_eq!(
            failure.error(),
            StructuralPhysicalStepBindingError::FutureCapacityUntrusted
        );
        let (error, returned_step, returned_physical) = failure.into_parts();
        assert_eq!(
            error,
            StructuralPhysicalStepBindingError::FutureCapacityUntrusted
        );
        assert_eq!(returned_step, expected_step);
        assert_eq!(returned_physical.declaration(), &expected_declaration);
        assert_eq!(
            returned_physical.capacity_source(),
            PhysicalCapacitySource::FutureUntrusted
        );
    }

    #[test]
    fn plan_identity_and_selection_drift_fail_closed_with_retention() {
        let plan_id = identity(3, 0);
        for (step, expected_error) in [
            (
                step(identity(3, 999), target_selection()),
                StructuralPhysicalStepBindingError::PlanIdentityMismatch,
            ),
            (
                step(plan_id, draft_selection()),
                StructuralPhysicalStepBindingError::SelectionMismatch,
            ),
        ] {
            let physical = synthetic_unproved_physical_plan(
                PhysicalCapacitySource::ReviewedBatchArithmeticV1,
                target_selection(),
                QWEN3_TARGET_PLAN_STEPS,
                plan_id,
            )
            .unwrap();
            let expected_declaration = physical.declaration().clone();
            let failure = match bind_structural_physical_step(step, physical) {
                StructuralPhysicalStepBindingOutcome::Bound(bound) => {
                    panic!("unexpected structural binding success: {bound:?}")
                }
                StructuralPhysicalStepBindingOutcome::Rejected(failure) => failure,
            };
            assert_eq!(failure.error(), expected_error);
            let (returned_error, returned_step, returned_physical) = failure.into_parts();
            assert_eq!(returned_error, expected_error);
            assert_eq!(returned_step, step);
            assert_eq!(returned_physical.declaration(), &expected_declaration);
        }
    }

    #[test]
    fn wrong_role_operation_count_never_reaches_binding() {
        let result = synthetic_unproved_physical_plan(
            PhysicalCapacitySource::ReviewedBatchArithmeticV1,
            target_selection(),
            QWEN3_TARGET_PLAN_STEPS - 1,
            identity(3, 0),
        );
        assert!(matches!(result, Err(PhysicalPlanError::InvalidExpectation)));
    }
}
