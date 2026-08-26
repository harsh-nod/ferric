//! Ordered production inputs for the dynamic M1 physical-serving adapter.
//!
//! The adapter advances scheduler authority before asking for physical inputs.
//! This provider therefore admits an exact, move-only generation queue up
//! front, rejects a wrong next phase or batch binding without dequeueing it,
//! and retains every subsequently consumed owner in terminal phase custody.
//! Completion semantics are never derived from compact completion bytes: the
//! provider attaches the independent direct or S1/K4 diagnostic allocation
//! required by the selected serving plan.

use core::fmt;
use std::collections::{TryReserveError, VecDeque};

use ferric_spec::{completion::CompletionEpoch, RequestId};

use crate::{
    prepare_m1_long_lived_queue_rearm_v1, prepare_m1_s1_k4_queue_rollover_v1,
    reserve_m1_long_lived_queue_rearm_kv_v1, reserve_m1_s1_k4_queue_rollover_kv_v1,
    ActiveDeviceKvCache, Engine, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1LongLivedQueueRearmKvInputsV1, M1PartitionedModelMemoryKvPoolV1,
    M1PhysicalFixedBatchShapeV1, M1PhysicalRunnerRecipeOutcomeV1, M1PhysicalRunnerV1,
    M1S1K4QueueRolloverKvInputsV1, M1ScheduledDispatchV1, M1ScheduledLongLivedQueueRearmV1,
    M1ScheduledS1K4QueueRolloverV1, M1ServingBatchPlanV1, M1ServingPhysicalInputProviderV1,
    M1ServingPlanV1, M1ServingPreparedFirstPublicationV1, M1ServingPreparedS1K4RolloverV1,
    M1ServingPreparedSameShapeRearmV1, M1ServingPreparedSemanticEvidenceV1, M1StepDispatchIntent,
};

/// Exact operation for which one queued physical input is valid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingQueuedGenerationPhaseV1 {
    FirstPublication,
    SameShapeRearm,
    S1K4Rollover,
}

/// Immutable registry binding carried beside every move-only physical input.
#[must_use = "generation binding must remain paired with its physical inputs"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingQueuedGenerationBindingV1 {
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
    epoch: CompletionEpoch,
}

impl M1ServingQueuedGenerationBindingV1 {
    /// Binds an input to one exact plan, ordered request roster, and epoch.
    pub fn new(plan: M1ServingPlanV1, requests: Box<[RequestId]>, epoch: CompletionEpoch) -> Self {
        Self {
            plan,
            requests,
            epoch,
        }
    }

    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.requests
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    fn matches(
        &self,
        plan: M1ServingPlanV1,
        requests: &[RequestId],
        epoch: CompletionEpoch,
    ) -> bool {
        self.plan == plan && self.epoch == epoch && self.requests.as_ref() == requests
    }
}

/// First-generation owners queued before exact scheduler dispatch.
#[must_use = "first-publication inputs own linear memory, KV, and cache custody"]
#[derive(Debug)]
pub struct M1ServingQueuedFirstPublicationV1 {
    binding: M1ServingQueuedGenerationBindingV1,
    memory: M1PartitionedModelMemoryKvPoolV1,
    tables: M1FullStepKvWorkspaceTablesV1,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
    selected: Vec<ActiveDeviceKvCache>,
}

impl M1ServingQueuedFirstPublicationV1 {
    /// Joins already-reserved first-generation KV tables and active caches to
    /// two independently constructed copies of the exact workspace plans.
    /// One plan set is consumed by recipe derivation and one by image creation.
    pub const fn new(
        binding: M1ServingQueuedGenerationBindingV1,
        memory: M1PartitionedModelMemoryKvPoolV1,
        tables: M1FullStepKvWorkspaceTablesV1,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
        selected: Vec<ActiveDeviceKvCache>,
    ) -> Self {
        Self {
            binding,
            memory,
            tables,
            preparation_plans,
            recipe_plans,
            selected,
        }
    }

    #[must_use = "the exact queued binding remains attached to first-publication inputs"]
    pub const fn binding(&self) -> &M1ServingQueuedGenerationBindingV1 {
        &self.binding
    }
}

/// Same-shape continuation owners queued before the predecessor is detached.
#[must_use = "same-shape inputs own linear KV page leases and workspace plans"]
#[derive(Debug)]
pub struct M1ServingQueuedSameShapeRearmV1 {
    binding: M1ServingQueuedGenerationBindingV1,
    kv_inputs: M1LongLivedQueueRearmKvInputsV1,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1ServingQueuedSameShapeRearmV1 {
    pub const fn new(
        binding: M1ServingQueuedGenerationBindingV1,
        kv_inputs: M1LongLivedQueueRearmKvInputsV1,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            binding,
            kv_inputs,
            preparation_plans,
            recipe_plans,
        }
    }

    #[must_use = "the exact queued binding remains attached to same-shape inputs"]
    pub const fn binding(&self) -> &M1ServingQueuedGenerationBindingV1 {
        &self.binding
    }
}

/// Exact paired-prefill to S1/K4 successor owners queued before detachment.
#[must_use = "rollover inputs own linear KV page leases and workspace plans"]
#[derive(Debug)]
pub struct M1ServingQueuedS1K4RolloverV1 {
    binding: M1ServingQueuedGenerationBindingV1,
    kv_inputs: M1S1K4QueueRolloverKvInputsV1,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1ServingQueuedS1K4RolloverV1 {
    pub const fn new(
        binding: M1ServingQueuedGenerationBindingV1,
        kv_inputs: M1S1K4QueueRolloverKvInputsV1,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            binding,
            kv_inputs,
            preparation_plans,
            recipe_plans,
        }
    }

    #[must_use = "the exact queued binding remains attached to rollover inputs"]
    pub const fn binding(&self) -> &M1ServingQueuedGenerationBindingV1 {
        &self.binding
    }
}

/// One move-only physical generation in exact serving order.
#[must_use = "queued generation inputs must be consumed in serving order"]
#[derive(Debug)]
pub enum M1ServingQueuedGenerationInputV1 {
    FirstPublication(M1ServingQueuedFirstPublicationV1),
    SameShapeRearm(M1ServingQueuedSameShapeRearmV1),
    S1K4Rollover(M1ServingQueuedS1K4RolloverV1),
}

impl M1ServingQueuedGenerationInputV1 {
    #[must_use]
    pub const fn phase(&self) -> M1ServingQueuedGenerationPhaseV1 {
        match self {
            Self::FirstPublication(_) => M1ServingQueuedGenerationPhaseV1::FirstPublication,
            Self::SameShapeRearm(_) => M1ServingQueuedGenerationPhaseV1::SameShapeRearm,
            Self::S1K4Rollover(_) => M1ServingQueuedGenerationPhaseV1::S1K4Rollover,
        }
    }

    #[must_use = "the exact queued binding remains attached to physical inputs"]
    pub const fn binding(&self) -> &M1ServingQueuedGenerationBindingV1 {
        match self {
            Self::FirstPublication(input) => input.binding(),
            Self::SameShapeRearm(input) => input.binding(),
            Self::S1K4Rollover(input) => input.binding(),
        }
    }
}

/// Fallible queue growth failure retaining the rejected generation input.
#[must_use = "enqueue failure retains the unqueued physical input"]
#[derive(Debug)]
pub struct M1ServingPhysicalInputEnqueueFailureV1 {
    source: TryReserveError,
    input: M1ServingQueuedGenerationInputV1,
}

impl M1ServingPhysicalInputEnqueueFailureV1 {
    #[must_use]
    pub const fn source(&self) -> &TryReserveError {
        &self.source
    }

    #[must_use = "the rejected generation input remains linear"]
    pub fn into_input(self) -> M1ServingQueuedGenerationInputV1 {
        self.input
    }
}

/// Stable preparation phase for a terminal provider rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalInputPreparationPhaseV1 {
    QueueSelection,
    BatchBinding,
    PhysicalInputPreflight,
    RecipeDerivation,
    FirstWorkspacePreparation,
    FirstWorkspaceAllocation,
    S1K4OutputReservation,
    CompletionOutputAllocation,
    SemanticEvidenceAllocation,
    SameShapeKvReservation,
    SameShapeWorkspacePreparation,
    S1K4KvReservation,
    S1K4WorkspacePreparation,
}

/// Stable reason accompanying exact retained failure custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalInputPreparationErrorV1 {
    QueueEmpty {
        expected: M1ServingQueuedGenerationPhaseV1,
    },
    QueuePhaseMismatch {
        expected: M1ServingQueuedGenerationPhaseV1,
        actual: M1ServingQueuedGenerationPhaseV1,
    },
    BatchBindingMismatch,
    WorkspaceKindMismatch,
    SelectedRosterMismatch,
    DeviceMismatch,
    UnsupportedPlanShape,
    LowerRejected,
}

#[derive(Debug)]
struct OpaqueM1ServingPhysicalInputCustodyV1(Box<dyn fmt::Debug>);

/// Terminal provider failure retaining all owners consumed after scheduling.
#[must_use = "provider failure custody must remain retained for teardown"]
#[derive(Debug)]
pub struct M1ServingPhysicalInputPreparationFailureV1 {
    phase: M1ServingPhysicalInputPreparationPhaseV1,
    error: M1ServingPhysicalInputPreparationErrorV1,
    retained: OpaqueM1ServingPhysicalInputCustodyV1,
}

impl M1ServingPhysicalInputPreparationFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1ServingPhysicalInputPreparationPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn error(&self) -> M1ServingPhysicalInputPreparationErrorV1 {
        self.error
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }

    /// Provider preparation starts only after exact scheduling, so every
    /// rejection is conservatively terminal for the in-flight adapter.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }
}

fn preparation_failure(
    phase: M1ServingPhysicalInputPreparationPhaseV1,
    error: M1ServingPhysicalInputPreparationErrorV1,
    retained: impl fmt::Debug + 'static,
) -> M1ServingPhysicalInputPreparationFailureV1 {
    M1ServingPhysicalInputPreparationFailureV1 {
        phase,
        error,
        retained: OpaqueM1ServingPhysicalInputCustodyV1(Box::new(retained)),
    }
}

/// Concrete ordered physical-input provider for production M1 serving.
#[must_use = "queued physical inputs and linear owners must remain retained"]
#[derive(Debug, Default)]
pub struct M1QueuedServingPhysicalInputProviderV1 {
    pending: VecDeque<M1ServingQueuedGenerationInputV1>,
}

impl M1QueuedServingPhysicalInputProviderV1 {
    #[must_use = "the provider owns every queued linear physical input"]
    pub const fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    /// Installs an already-ordered finite generation program.
    #[must_use = "queued physical inputs remain linear"]
    pub fn from_ordered_inputs(inputs: Vec<M1ServingQueuedGenerationInputV1>) -> Self {
        Self {
            pending: VecDeque::from(inputs),
        }
    }

    /// Appends one physical generation without losing it on host allocation failure.
    ///
    /// # Errors
    ///
    /// Returns the rejected input and allocation diagnostic unchanged.
    pub fn try_enqueue(
        &mut self,
        input: M1ServingQueuedGenerationInputV1,
    ) -> Result<(), M1ServingPhysicalInputEnqueueFailureV1> {
        if let Err(source) = self.pending.try_reserve(1) {
            return Err(M1ServingPhysicalInputEnqueueFailureV1 { source, input });
        }
        self.pending.push_back(input);
        Ok(())
    }

    #[must_use]
    pub fn pending_generation_count(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn next_generation_phase(&self) -> Option<M1ServingQueuedGenerationPhaseV1> {
        self.pending
            .front()
            .map(M1ServingQueuedGenerationInputV1::phase)
    }

    /// Recovers every generation that has not been consumed by preparation.
    #[must_use = "pending physical generations remain linear"]
    pub fn into_pending_inputs(self) -> VecDeque<M1ServingQueuedGenerationInputV1> {
        self.pending
    }
}

fn workspace_kind(plan: M1ServingPlanV1) -> Option<M1FullStepWorkspaceInputKind> {
    match plan.shape() {
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            Some(M1FullStepWorkspaceInputKind::PairedPrefill)
        }
        M1PhysicalFixedBatchShapeV1::TargetOnly => Some(M1FullStepWorkspaceInputKind::TargetOnly),
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            Some(M1FullStepWorkspaceInputKind::SpeculativeRound)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => None,
    }
}

fn dispatch_intent(plan: M1ServingPlanV1) -> Option<M1StepDispatchIntent> {
    match plan.shape() {
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            Some(M1StepDispatchIntent::PairedPrefill(plan.target()))
        }
        M1PhysicalFixedBatchShapeV1::TargetOnly => {
            Some(M1StepDispatchIntent::TargetOnly(plan.target()))
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            Some(M1StepDispatchIntent::SpeculativeRound(plan.target()))
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => None,
    }
}

fn semantic_evidence(plan: M1ServingPlanV1) -> Option<M1ServingPreparedSemanticEvidenceV1> {
    match plan.shape() {
        M1PhysicalFixedBatchShapeV1::PairedPrefill | M1PhysicalFixedBatchShapeV1::TargetOnly => {
            Some(M1ServingPreparedSemanticEvidenceV1::Direct)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            Some(M1ServingPreparedSemanticEvidenceV1::SpeculativeK4)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => None,
    }
}

fn scheduled_roster_matches(
    scheduled: &M1ScheduledDispatchV1,
    requests: &[RequestId],
    epoch: CompletionEpoch,
) -> bool {
    scheduled.epoch() == epoch
        && scheduled.member_count() == requests.len()
        && requests
            .iter()
            .copied()
            .enumerate()
            .all(|(index, request)| scheduled.member(index) == Some(request))
}

fn exact_batch_binding_matches(
    binding: &M1ServingQueuedGenerationBindingV1,
    batch: &M1ServingBatchPlanV1,
    scheduled: &M1ScheduledDispatchV1,
) -> bool {
    binding.matches(batch.plan(), batch.requests(), batch.epoch())
        && scheduled_roster_matches(scheduled, batch.requests(), batch.epoch())
}

fn first_physical_preflight(
    input: &M1ServingQueuedFirstPublicationV1,
    batch: &M1ServingBatchPlanV1,
) -> Result<(), M1ServingPhysicalInputPreparationErrorV1> {
    let Some(expected_kind) = workspace_kind(batch.plan()) else {
        return Err(M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape);
    };
    if input.tables.kind() != expected_kind
        || input.preparation_plans.kind() != expected_kind
        || input.recipe_plans.kind() != expected_kind
    {
        return Err(M1ServingPhysicalInputPreparationErrorV1::WorkspaceKindMismatch);
    }
    if input.selected.len() != batch.requests().len()
        || input
            .selected
            .iter()
            .zip(batch.requests().iter().copied())
            .any(|(cache, request)| cache.projection().request != request)
    {
        return Err(M1ServingPhysicalInputPreparationErrorV1::SelectedRosterMismatch);
    }
    let device = input.memory.device();
    if input
        .selected
        .iter()
        .any(|cache| cache.projection().device != device)
    {
        return Err(M1ServingPhysicalInputPreparationErrorV1::DeviceMismatch);
    }
    Ok(())
}

fn continuation_physical_preflight(
    preparation_plans: &M1FullStepWorkspacePlans,
    recipe_plans: &M1FullStepWorkspacePlans,
    batch: &M1ServingBatchPlanV1,
) -> Result<(), M1ServingPhysicalInputPreparationErrorV1> {
    let Some(expected_kind) = workspace_kind(batch.plan()) else {
        return Err(M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape);
    };
    if preparation_plans.kind() == expected_kind && recipe_plans.kind() == expected_kind {
        Ok(())
    } else {
        Err(M1ServingPhysicalInputPreparationErrorV1::WorkspaceKindMismatch)
    }
}

impl<const C: usize> M1ServingPhysicalInputProviderV1<C>
    for M1QueuedServingPhysicalInputProviderV1
{
    type Failure = M1ServingPhysicalInputPreparationFailureV1;

    fn prepare_first_publication(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        _engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledDispatchV1,
    ) -> Result<M1ServingPreparedFirstPublicationV1, Self::Failure> {
        let expected = M1ServingQueuedGenerationPhaseV1::FirstPublication;
        let Some(front) = self.pending.front() else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueueEmpty { expected },
                scheduled,
            ));
        };
        if front.phase() != expected {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueuePhaseMismatch {
                    expected,
                    actual: front.phase(),
                },
                scheduled,
            ));
        }
        if !exact_batch_binding_matches(front.binding(), batch, &scheduled) {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::BatchBinding,
                M1ServingPhysicalInputPreparationErrorV1::BatchBindingMismatch,
                scheduled,
            ));
        }
        let M1ServingQueuedGenerationInputV1::FirstPublication(front) = front else {
            unreachable!("phase checked above")
        };
        if let Err(error) = first_physical_preflight(front, batch) {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::PhysicalInputPreflight,
                error,
                scheduled,
            ));
        }

        let Some(M1ServingQueuedGenerationInputV1::FirstPublication(input)) =
            self.pending.pop_front()
        else {
            unreachable!("front variant checked before dequeue")
        };
        let M1ServingQueuedFirstPublicationV1 {
            binding,
            memory,
            tables,
            preparation_plans,
            recipe_plans,
            selected,
        } = input;
        let Some(intent) = dispatch_intent(binding.plan()) else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::RecipeDerivation,
                M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape,
                (
                    binding,
                    memory,
                    tables,
                    preparation_plans,
                    recipe_plans,
                    selected,
                    scheduled,
                ),
            ));
        };
        let recipe = match runner.derive_step_recipe(intent, recipe_plans) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::RecipeDerivation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (
                        binding,
                        memory,
                        tables,
                        preparation_plans,
                        selected,
                        scheduled,
                        failure,
                    ),
                ));
            }
        };
        let prepared =
            match runner.prepare_scheduled_workspaces(scheduled, preparation_plans, tables) {
                Ok(prepared) => prepared,
                Err(failure) => {
                    return Err(preparation_failure(
                        M1ServingPhysicalInputPreparationPhaseV1::FirstWorkspacePreparation,
                        M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                        (binding, memory, selected, recipe, failure),
                    ));
                }
            };
        let mut allocated = match runner.allocate_scheduled_workspaces(memory, prepared) {
            Ok(allocated) => allocated,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::FirstWorkspaceAllocation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, selected, recipe, failure),
                ));
            }
        };
        if binding.plan().shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill {
            if let Err(failure) = allocated.reserve_s1_k4_rollover_output() {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::S1K4OutputReservation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, allocated, selected, recipe, failure),
                ));
            }
        }
        let completion_output = match allocated.allocate_completion_output(binding.plan().target())
        {
            Ok(completion_output) => completion_output,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::CompletionOutputAllocation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, allocated, selected, recipe, failure),
                ));
            }
        };
        let Some(semantic_evidence) = semantic_evidence(binding.plan()) else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::SemanticEvidenceAllocation,
                M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape,
                (binding, allocated, selected, recipe, completion_output),
            ));
        };
        let completion_output = match semantic_evidence {
            M1ServingPreparedSemanticEvidenceV1::Direct => {
                match allocated.enable_direct_diagnostic_choices_capture(completion_output) {
                    Ok(completion_output) => completion_output,
                    Err(failure) => {
                        return Err(preparation_failure(
                            M1ServingPhysicalInputPreparationPhaseV1::SemanticEvidenceAllocation,
                            M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                            (binding, allocated, selected, recipe, failure),
                        ));
                    }
                }
            }
            M1ServingPreparedSemanticEvidenceV1::SpeculativeK4 => {
                match allocated.enable_speculative_k4_diagnostic_choices_capture(completion_output)
                {
                    Ok(completion_output) => completion_output,
                    Err(failure) => {
                        return Err(preparation_failure(
                            M1ServingPhysicalInputPreparationPhaseV1::SemanticEvidenceAllocation,
                            M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                            (binding, allocated, selected, recipe, failure),
                        ));
                    }
                }
            }
        };
        Ok(M1ServingPreparedFirstPublicationV1::new(
            allocated,
            recipe,
            completion_output,
            selected,
            semantic_evidence,
        ))
    }

    fn prepare_same_shape_rearm(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledLongLivedQueueRearmV1,
    ) -> Result<M1ServingPreparedSameShapeRearmV1, Self::Failure> {
        let expected = M1ServingQueuedGenerationPhaseV1::SameShapeRearm;
        let Some(front) = self.pending.front() else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueueEmpty { expected },
                scheduled,
            ));
        };
        if front.phase() != expected {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueuePhaseMismatch {
                    expected,
                    actual: front.phase(),
                },
                scheduled,
            ));
        }
        if !exact_batch_binding_matches(front.binding(), batch, scheduled.scheduled_dispatch()) {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::BatchBinding,
                M1ServingPhysicalInputPreparationErrorV1::BatchBindingMismatch,
                scheduled,
            ));
        }
        let M1ServingQueuedGenerationInputV1::SameShapeRearm(front) = front else {
            unreachable!("phase checked above")
        };
        if let Err(error) =
            continuation_physical_preflight(&front.preparation_plans, &front.recipe_plans, batch)
        {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::PhysicalInputPreflight,
                error,
                scheduled,
            ));
        }

        let Some(M1ServingQueuedGenerationInputV1::SameShapeRearm(input)) =
            self.pending.pop_front()
        else {
            unreachable!("front variant checked before dequeue")
        };
        let M1ServingQueuedSameShapeRearmV1 {
            binding,
            kv_inputs,
            preparation_plans,
            recipe_plans,
        } = input;
        let Some(intent) = dispatch_intent(binding.plan()) else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::RecipeDerivation,
                M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape,
                (
                    binding,
                    kv_inputs,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                ),
            ));
        };
        let recipe = match runner.derive_step_recipe(intent, recipe_plans) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::RecipeDerivation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, kv_inputs, preparation_plans, scheduled, failure),
                ));
            }
        };
        let reserved = match reserve_m1_long_lived_queue_rearm_kv_v1(engine, scheduled, kv_inputs) {
            Ok(reserved) => reserved,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::SameShapeKvReservation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, preparation_plans, recipe, failure),
                ));
            }
        };
        let prepared = match prepare_m1_long_lived_queue_rearm_v1(
            engine,
            reserved,
            runner.logical_runner(),
            preparation_plans,
        ) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::SameShapeWorkspacePreparation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, recipe, failure),
                ));
            }
        };
        let Some(semantic_evidence) = semantic_evidence(binding.plan()) else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::PhysicalInputPreflight,
                M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape,
                (binding, prepared, recipe),
            ));
        };
        Ok(M1ServingPreparedSameShapeRearmV1::new(
            prepared,
            recipe,
            semantic_evidence,
        ))
    }

    fn prepare_s1_k4_rollover(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledS1K4QueueRolloverV1,
    ) -> Result<M1ServingPreparedS1K4RolloverV1, Self::Failure> {
        let expected = M1ServingQueuedGenerationPhaseV1::S1K4Rollover;
        let Some(front) = self.pending.front() else {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueueEmpty { expected },
                scheduled,
            ));
        };
        if front.phase() != expected {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::QueueSelection,
                M1ServingPhysicalInputPreparationErrorV1::QueuePhaseMismatch {
                    expected,
                    actual: front.phase(),
                },
                scheduled,
            ));
        }
        if !exact_batch_binding_matches(front.binding(), batch, scheduled.scheduled_dispatch()) {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::BatchBinding,
                M1ServingPhysicalInputPreparationErrorV1::BatchBindingMismatch,
                scheduled,
            ));
        }
        let M1ServingQueuedGenerationInputV1::S1K4Rollover(front) = front else {
            unreachable!("phase checked above")
        };
        if batch.plan().shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::PhysicalInputPreflight,
                M1ServingPhysicalInputPreparationErrorV1::UnsupportedPlanShape,
                scheduled,
            ));
        }
        if let Err(error) =
            continuation_physical_preflight(&front.preparation_plans, &front.recipe_plans, batch)
        {
            return Err(preparation_failure(
                M1ServingPhysicalInputPreparationPhaseV1::PhysicalInputPreflight,
                error,
                scheduled,
            ));
        }

        let Some(M1ServingQueuedGenerationInputV1::S1K4Rollover(input)) = self.pending.pop_front()
        else {
            unreachable!("front variant checked before dequeue")
        };
        let M1ServingQueuedS1K4RolloverV1 {
            binding,
            kv_inputs,
            preparation_plans,
            recipe_plans,
        } = input;
        let recipe = match runner.derive_step_recipe(
            M1StepDispatchIntent::SpeculativeRound(binding.plan().target()),
            recipe_plans,
        ) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::RecipeDerivation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, kv_inputs, preparation_plans, scheduled, failure),
                ));
            }
        };
        let reserved = match reserve_m1_s1_k4_queue_rollover_kv_v1(engine, scheduled, kv_inputs) {
            Ok(reserved) => reserved,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::S1K4KvReservation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, preparation_plans, recipe, failure),
                ));
            }
        };
        let prepared = match prepare_m1_s1_k4_queue_rollover_v1(
            engine,
            reserved,
            runner.logical_runner(),
            preparation_plans,
        ) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(preparation_failure(
                    M1ServingPhysicalInputPreparationPhaseV1::S1K4WorkspacePreparation,
                    M1ServingPhysicalInputPreparationErrorV1::LowerRejected,
                    (binding, recipe, failure),
                ));
            }
        };
        Ok(M1ServingPreparedS1K4RolloverV1::new(
            prepared,
            recipe,
            M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
        ))
    }
}

#[cfg(test)]
mod tests {
    use ferric_spec::{
        completion::CompletionEpoch, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
        Qwen3PlanSelection, RequestId,
    };

    use super::{
        dispatch_intent, semantic_evidence, workspace_kind, M1QueuedServingPhysicalInputProviderV1,
        M1ServingQueuedGenerationBindingV1,
    };
    use crate::{
        M1FullStepWorkspaceInputKind, M1PhysicalFixedBatchShapeV1, M1ServingPlanV1,
        M1ServingPreparedSemanticEvidenceV1, M1StepDispatchIntent,
    };

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn serving_plan(
        target_mode: Qwen3ExecutionMode,
        target_bucket: Qwen3PlanBucket,
        draft_mode: Qwen3ExecutionMode,
        draft_bucket: Qwen3PlanBucket,
    ) -> M1ServingPlanV1 {
        M1ServingPlanV1::new(
            selection(Qwen3ModelRole::Target8B, target_mode, target_bucket),
            selection(Qwen3ModelRole::Draft06B, draft_mode, draft_bucket),
        )
        .expect("test plan must be valid")
    }

    #[test]
    fn exact_binding_rejects_plan_epoch_roster_and_order_drift() {
        let selected_plan = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let requests = [RequestId::new(7, 1), RequestId::new(9, 2)];
        let epoch = CompletionEpoch::new(11);
        let binding =
            M1ServingQueuedGenerationBindingV1::new(selected_plan, Box::new(requests), epoch);

        assert!(binding.matches(selected_plan, &requests, epoch));
        assert!(!binding.matches(selected_plan, &[requests[1], requests[0]], epoch));
        assert!(!binding.matches(selected_plan, &requests, CompletionEpoch::new(12)));
        let other = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        assert!(!binding.matches(other, &requests, epoch));
    }

    #[test]
    fn admitted_shapes_select_exact_recipe_workspace_and_evidence_contracts() {
        let paired = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let direct = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );

        assert_eq!(paired.shape(), M1PhysicalFixedBatchShapeV1::PairedPrefill);
        assert_eq!(
            workspace_kind(paired),
            Some(M1FullStepWorkspaceInputKind::PairedPrefill)
        );
        assert_eq!(
            dispatch_intent(paired),
            Some(M1StepDispatchIntent::PairedPrefill(paired.target()))
        );
        assert!(matches!(
            semantic_evidence(paired),
            Some(M1ServingPreparedSemanticEvidenceV1::Direct)
        ));

        assert_eq!(
            workspace_kind(direct),
            Some(M1FullStepWorkspaceInputKind::TargetOnly)
        );
        assert_eq!(
            dispatch_intent(direct),
            Some(M1StepDispatchIntent::TargetOnly(direct.target()))
        );
        assert!(matches!(
            semantic_evidence(direct),
            Some(M1ServingPreparedSemanticEvidenceV1::Direct)
        ));

        assert_eq!(
            workspace_kind(speculative),
            Some(M1FullStepWorkspaceInputKind::SpeculativeRound)
        );
        assert_eq!(
            dispatch_intent(speculative),
            Some(M1StepDispatchIntent::SpeculativeRound(speculative.target()))
        );
        assert!(matches!(
            semantic_evidence(speculative),
            Some(M1ServingPreparedSemanticEvidenceV1::SpeculativeK4)
        ));

        let unsupported = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert_eq!(workspace_kind(unsupported), None);
        assert_eq!(dispatch_intent(unsupported), None);
        assert!(semantic_evidence(unsupported).is_none());
    }

    #[test]
    fn empty_provider_reports_no_pending_generation() {
        let provider = M1QueuedServingPhysicalInputProviderV1::new();

        assert_eq!(provider.pending_generation_count(), 0);
        assert_eq!(provider.next_generation_phase(), None);
        assert!(provider.into_pending_inputs().is_empty());
    }
}
