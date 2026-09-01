//! Exact fixed-cardinality packet batches for complete M1 inference steps.
//!
//! This is the Ferric-specific lowering boundary from checked M1 program,
//! kernarg, workspace, model-memory, host-completion, and explicit-buffer
//! ownership into generic fe2o3 fixed packet descriptions. It constructs no
//! queue, publishes no AQL packet, launches no work, observes no completion,
//! and proves no refinement or hardware behavior.

use core::fmt;

use fe2o3_service_host::{ServiceFixedBatchV1, ServiceFixedDispatchPacketV1};
use ferric_spec::{Identity, Qwen3PlanBucket, Qwen3PlanSelection};
#[allow(unused_imports)]
use vstd::prelude::*;

use crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1;
use crate::{
    derive_m1_step_dispatch_plan, m1_completion_output_shape_v1,
    AddresslessM1FullStepWorkspaceComposition, AddresslessM1PhysicalBufferRecipeV1,
    AddresslessM1PhysicalDispatchRecipeV1, AddresslessM1PhysicalKernargRecipeV1,
    BoundM1CompletionOutputV1, BoundM1PhysicalBufferBindingsV1, ContentBoundM1ProgramCatalogV1,
    DeclaredOperationKernelPlan, Gfx942DeviceBinding, M1AuthenticatedWorkerV3ProgramSetV1,
    M1BoundPhysicalBufferRowV1, M1CompletionOutputShapeV1, M1FullStepWorkspaceSubleaseOwners,
    M1PartitionedModelMemoryKvPoolV1, M1PartitionedModelMemoryKvQueueCustodyV1,
    M1PhysicalBufferRecipeRowV1, M1PhysicalKernargImageV1, M1PhysicalProgramV1,
    M1StepDispatchCompositionError, M1StepDispatchIntent, M1_PHYSICAL_PROGRAM_COUNT_V1,
};

/// Exact packet count of every target-only M1 step.
pub const M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1: usize = 545;
/// Exact packet count of every paired draft/target prefill M1 step.
pub const M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1: usize = 969;
/// Exact packet count of both four-token speculative M1 shapes.
pub const M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1: usize = 2_242;
/// Exact packet count of the eight-token speculative M1 shape.
pub const M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1: usize = 3_938;
/// Exact packet count of the sixteen-token speculative M1 shape.
pub const M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1: usize = 7_330;

/// Closed M1 publication shape selected before generic packet construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalFixedBatchShapeV1 {
    /// One complete target graph.
    TargetOnly,
    /// Draft prefill followed by target prefill.
    PairedPrefill,
    /// Four draft decode graphs and one target verification graph.
    SpeculativeK4,
    /// Eight draft decode graphs and one target verification graph.
    SpeculativeK8,
    /// Sixteen draft decode graphs and one target verification graph.
    SpeculativeK16,
}

impl M1PhysicalFixedBatchShapeV1 {
    /// Exact compile-time packet count of this shape.
    #[must_use]
    pub const fn packet_count(self) -> usize {
        match self {
            Self::TargetOnly => M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
            Self::PairedPrefill => M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
            Self::SpeculativeK4 => M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
            Self::SpeculativeK8 => M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
            Self::SpeculativeK16 => M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
        }
    }
}

verus! {

/// Verifier view of the target-only fixed-batch packet cardinality.
pub open spec fn m1_target_only_fixed_batch_packet_count_spec() -> nat {
    545
}

/// Exposes the exact reviewed target-only fixed-batch cardinality.
pub proof fn m1_target_only_fixed_batch_shape()
    ensures m1_target_only_fixed_batch_packet_count_spec() == 545,
{
}

} // verus!

/// Ferric ownership retained beside one generic fixed batch.
///
/// These values retain the exact recipes and allocation sublease owners from
/// which packet-local range descriptors were copied. This owner intentionally
/// does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::M1PhysicalFixedBatchCustodyV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1PhysicalFixedBatchCustodyV1>();
/// ```
#[must_use = "fixed-batch allocation and recipe custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalFixedBatchCustodyV1 {
    catalog_id: Identity,
    selection: Qwen3PlanSelection,
    physical_recipe: AddresslessM1PhysicalDispatchRecipeV1,
    workspace_composition: AddresslessM1FullStepWorkspaceComposition,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

impl M1PhysicalFixedBatchCustodyV1 {
    /// Checked physical-device receipt retained by the allocation owner.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.partitioned_memory.device()
    }

    /// Domain-separated identity of the exact selected program catalog.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Exact target selection determining the complete publication shape.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Retained exact physical dispatch recipe.
    #[must_use]
    pub const fn physical_recipe(&self) -> &AddresslessM1PhysicalDispatchRecipeV1 {
        &self.physical_recipe
    }

    /// Retained exact dispatch/workspace association.
    #[must_use = "the exact workspace composition remains retained"]
    pub const fn workspace_composition(&self) -> &AddresslessM1FullStepWorkspaceComposition {
        &self.workspace_composition
    }

    /// Retained exact finite workspace-owner shape.
    #[must_use = "the exact workspace allocation custody remains retained"]
    pub const fn workspace_owners(&self) -> &M1FullStepWorkspaceSubleaseOwners {
        &self.workspace_owners
    }

    /// Retained closed model-memory, partition, ledger, and allocation custody.
    #[must_use = "partitioned memory custody remains retained by the fixed batch"]
    pub const fn partitioned_memory(&self) -> &M1PartitionedModelMemoryKvPoolV1 {
        &self.partitioned_memory
    }

    /// Retained coherent host-download completion-output allocation custody.
    #[must_use = "the exact completion-output allocation custody remains retained"]
    pub const fn completion_output(&self) -> &BoundM1CompletionOutputV1 {
        &self.completion_output
    }

    /// Retained semantic buffer-source rows in publication order.
    #[must_use]
    pub fn source_rows(&self) -> &[M1PhysicalBufferRecipeRowV1] {
        &self.source_rows
    }

    /// Retained owner-checked generic buffer rows in publication order.
    #[must_use]
    pub fn bound_rows(&self) -> &[M1BoundPhysicalBufferRowV1] {
        &self.bound_rows
    }

    pub(crate) fn into_queue_creation_parts(
        self,
    ) -> (
        fe2o3_service_host::ServiceAllocationSessionV1,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        let (allocations, partitioned_memory) = self.partitioned_memory.into_queue_creation_parts();
        (
            allocations,
            M1PhysicalQueueBatchCustodyV1 {
                catalog_id: self.catalog_id,
                selection: self.selection,
                physical_recipe: self.physical_recipe,
                workspace_composition: self.workspace_composition,
                workspace_owners: self.workspace_owners,
                partitioned_memory,
                completion_output: self.completion_output,
                source_rows: self.source_rows,
                bound_rows: self.bound_rows,
            },
        )
    }

    pub(crate) fn from_rejected_queue_creation(
        allocations: fe2o3_service_host::ServiceAllocationSessionV1,
        custody: M1PhysicalQueueBatchCustodyV1,
    ) -> Self {
        Self {
            catalog_id: custody.catalog_id,
            selection: custody.selection,
            physical_recipe: custody.physical_recipe,
            workspace_composition: custody.workspace_composition,
            workspace_owners: custody.workspace_owners,
            partitioned_memory: M1PartitionedModelMemoryKvPoolV1::from_rejected_queue_creation(
                allocations,
                custody.partitioned_memory,
            ),
            completion_output: custody.completion_output,
            source_rows: custody.source_rows,
            bound_rows: custody.bound_rows,
        }
    }
}

/// Ferric batch custody retained after the allocation session enters a queue ledger.
///
/// This post-split owner cannot allocate, resolve ranges, or mint leases. It
/// retains the only partition/model/page-ledger witnesses beside every queue
/// phase and intentionally cannot be converted back without the exact rejected
/// generic allocation session inside crate-private queue creation code.
#[must_use = "post-split Ferric custody must remain paired with generic queue custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueBatchCustodyV1 {
    catalog_id: Identity,
    selection: Qwen3PlanSelection,
    physical_recipe: AddresslessM1PhysicalDispatchRecipeV1,
    workspace_composition: AddresslessM1FullStepWorkspaceComposition,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvQueueCustodyV1,
    completion_output: BoundM1CompletionOutputV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

#[derive(Debug)]
pub(crate) struct M1PhysicalQueueBatchRearmPartsV1 {
    pub(crate) catalog_id: Identity,
    pub(crate) selection: Qwen3PlanSelection,
    pub(crate) physical_recipe: AddresslessM1PhysicalDispatchRecipeV1,
    pub(crate) workspace_composition: AddresslessM1FullStepWorkspaceComposition,
    pub(crate) workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    pub(crate) partitioned_memory: M1PartitionedModelMemoryKvQueueCustodyV1,
    pub(crate) completion_output: BoundM1CompletionOutputV1,
    pub(crate) source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    pub(crate) bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

impl M1PhysicalQueueBatchCustodyV1 {
    /// Checked physical-device receipt retained through every queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.partitioned_memory.device()
    }

    /// Exact target selection retained by the former fixed batch.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Exact selected-program catalog identity.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Opaque partition/model/page-ledger custody retained beside the queue.
    #[must_use = "partition custody remains paired with the generic queue"]
    pub const fn partitioned_memory(&self) -> &M1PartitionedModelMemoryKvQueueCustodyV1 {
        &self.partitioned_memory
    }

    pub(crate) const fn partitioned_memory_mut(
        &mut self,
    ) -> &mut M1PartitionedModelMemoryKvQueueCustodyV1 {
        &mut self.partitioned_memory
    }

    /// Exact coherent completion-output binding retained for readback.
    #[must_use = "completion-output custody remains paired with the queue"]
    pub const fn completion_output(&self) -> &BoundM1CompletionOutputV1 {
        &self.completion_output
    }

    /// Retained exact physical dispatch recipe.
    #[must_use]
    pub const fn physical_recipe(&self) -> &AddresslessM1PhysicalDispatchRecipeV1 {
        &self.physical_recipe
    }

    /// Retained exact workspace composition.
    #[must_use = "workspace composition remains retained by queue custody"]
    pub const fn workspace_composition(&self) -> &AddresslessM1FullStepWorkspaceComposition {
        &self.workspace_composition
    }

    /// Retained workspace sublease witnesses.
    #[must_use = "workspace sublease witnesses remain retained by queue custody"]
    pub const fn workspace_owners(&self) -> &M1FullStepWorkspaceSubleaseOwners {
        &self.workspace_owners
    }

    /// Retained semantic buffer rows.
    #[must_use]
    pub fn source_rows(&self) -> &[M1PhysicalBufferRecipeRowV1] {
        &self.source_rows
    }

    /// Retained owner-checked generic buffer rows.
    #[must_use]
    pub fn bound_rows(&self) -> &[M1BoundPhysicalBufferRowV1] {
        &self.bound_rows
    }

    pub(crate) fn retained_intent_shape(&self) -> Option<M1PhysicalFixedBatchShapeV1> {
        m1_physical_fixed_batch_shape_for_intent_v1(
            self.workspace_composition.dispatch_plan().intent(),
        )
    }

    pub(crate) fn into_rearm_parts(self) -> M1PhysicalQueueBatchRearmPartsV1 {
        M1PhysicalQueueBatchRearmPartsV1 {
            catalog_id: self.catalog_id,
            selection: self.selection,
            physical_recipe: self.physical_recipe,
            workspace_composition: self.workspace_composition,
            workspace_owners: self.workspace_owners,
            partitioned_memory: self.partitioned_memory,
            completion_output: self.completion_output,
            source_rows: self.source_rows,
            bound_rows: self.bound_rows,
        }
    }

    pub(crate) fn from_rearm_parts(parts: M1PhysicalQueueBatchRearmPartsV1) -> Self {
        Self {
            catalog_id: parts.catalog_id,
            selection: parts.selection,
            physical_recipe: parts.physical_recipe,
            workspace_composition: parts.workspace_composition,
            workspace_owners: parts.workspace_owners,
            partitioned_memory: parts.partitioned_memory,
            completion_output: parts.completion_output,
            source_rows: parts.source_rows,
            bound_rows: parts.bound_rows,
        }
    }
}

/// One exact const-cardinality generic batch and its Ferric custody.
///
/// Construction remains descriptive and grants no queue or publication
/// authority. This owner intentionally does not implement `Clone`.
#[must_use = "the fixed batch and its Ferric custody must be consumed together"]
#[derive(Debug)]
pub struct M1PhysicalFixedBatchCaseV1<'a, const N: usize> {
    batch: ServiceFixedBatchV1<'a, N>,
    custody: M1PhysicalFixedBatchCustodyV1,
}

impl<'a, const N: usize> M1PhysicalFixedBatchCaseV1<'a, N> {
    pub(crate) fn from_parts(
        batch: ServiceFixedBatchV1<'a, N>,
        custody: M1PhysicalFixedBatchCustodyV1,
    ) -> Self {
        Self { batch, custody }
    }

    /// Exact generic fixed batch, by borrow.
    #[must_use = "the exact generic batch remains retained by the Ferric case"]
    pub const fn batch(&self) -> &ServiceFixedBatchV1<'a, N> {
        &self.batch
    }

    /// Exact retained Ferric recipe and allocation custody, by borrow.
    #[must_use = "the exact Ferric custody remains retained beside the generic batch"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    /// Exact retained coherent completion-output custody, by borrow.
    #[must_use = "the completion-output owner remains retained beside the generic batch"]
    pub const fn completion_output(&self) -> &BoundM1CompletionOutputV1 {
        self.custody.completion_output()
    }

    /// Consumes the wrapper into the generic batch and all Ferric custody.
    #[must_use = "the generic batch and all Ferric custody remain live"]
    pub(crate) fn into_parts(self) -> (ServiceFixedBatchV1<'a, N>, M1PhysicalFixedBatchCustodyV1) {
        (self.batch, self.custody)
    }

    /// Const-generic packet cardinality retained by fe2o3.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.batch.packet_count()
    }

    /// Exact selected program count retained by fe2o3.
    #[must_use]
    pub fn program_count(&self) -> usize {
        self.batch.program_count()
    }

    /// A descriptive fixed batch alone grants no queue publication authority.
    #[must_use]
    pub const fn grants_queue_authority(&self) -> bool {
        false
    }
}

/// Closed family of every admitted complete M1 fixed batch cardinality.
///
/// The two K4 selections share one const-generic packet type while their exact
/// S1/S8 selection remains retained in [`M1PhysicalFixedBatchCustodyV1`].
#[must_use = "a complete fixed batch must remain retained until queue construction"]
#[derive(Debug)]
pub enum M1PhysicalFixedBatchV1<'a> {
    /// One complete target-only publication.
    TargetOnly(Box<M1PhysicalFixedBatchCaseV1<'a, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// One complete paired-prefill publication.
    PairedPrefill(Box<M1PhysicalFixedBatchCaseV1<'a, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>),
    /// One complete K4 speculative publication, for either S1 or S8.
    SpeculativeK4(Box<M1PhysicalFixedBatchCaseV1<'a, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>),
    /// One complete K8 speculative publication.
    SpeculativeK8(Box<M1PhysicalFixedBatchCaseV1<'a, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>),
    /// One complete K16 speculative publication.
    SpeculativeK16(Box<M1PhysicalFixedBatchCaseV1<'a, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>),
}

/// One exact authenticated packet array and its Ferric allocation custody.
///
/// Packet construction is crate-private and the packet array has no public
/// accessor, so callers cannot substitute service program indices after the
/// authenticated Worker V3 mapping has been applied.
#[must_use = "authenticated packet and allocation custody must remain joined"]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedPhysicalPacketBatchCaseV1<const N: usize> {
    packets: [ServiceFixedDispatchPacketV1; N],
    custody: M1PhysicalFixedBatchCustodyV1,
}

impl<const N: usize> M1AuthenticatedPhysicalPacketBatchCaseV1<N> {
    pub(crate) const fn from_parts(
        packets: [ServiceFixedDispatchPacketV1; N],
        custody: M1PhysicalFixedBatchCustodyV1,
    ) -> Self {
        Self { packets, custody }
    }

    pub(crate) const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        [ServiceFixedDispatchPacketV1; N],
        M1PhysicalFixedBatchCustodyV1,
    ) {
        (self.packets, self.custody)
    }
}

/// Closed family of authenticated packet cardinalities for every complete M1 step.
#[must_use = "authenticated packet custody must enter the Ferric queue boundary"]
#[derive(Debug)]
pub(crate) enum M1AuthenticatedPhysicalPacketBatchV1 {
    TargetOnly(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>,
    ),
    PairedPrefill(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalPacketBatchV1 {
    pub(crate) const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        match self {
            Self::TargetOnly(_) => M1PhysicalFixedBatchShapeV1::TargetOnly,
            Self::PairedPrefill(_) => M1PhysicalFixedBatchShapeV1::PairedPrefill,
            Self::SpeculativeK4(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            Self::SpeculativeK8(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            Self::SpeculativeK16(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK16,
        }
    }

    pub(crate) const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }
}

/// One freshly lowered authenticated packet array and retained queue custody.
///
/// Only the authenticated program-catalog witness can select service program
/// indices. The fresh images and owner-checked ranges remain inaccessible
/// outside the crate-private detached-queue rebind boundary.
#[must_use = "authenticated rebind packets and queue custody must remain joined"]
#[derive(Debug)]
#[expect(dead_code, reason = "staged for authenticated detached rebind")]
pub(crate) struct M1AuthenticatedQueuePacketBatchCaseV1<const N: usize> {
    packets: [ServiceFixedDispatchPacketV1; N],
    custody: M1PhysicalQueueBatchCustodyV1,
}

#[expect(dead_code, reason = "staged for authenticated detached rebind")]
impl<const N: usize> M1AuthenticatedQueuePacketBatchCaseV1<N> {
    pub(crate) fn into_parts(
        self,
    ) -> (
        [ServiceFixedDispatchPacketV1; N],
        M1PhysicalQueueBatchCustodyV1,
    ) {
        (self.packets, self.custody)
    }
}

/// Closed authenticated packet-array family accepted by detached queue rebind.
#[must_use = "authenticated rebind packet custody must enter the detached queue boundary"]
#[derive(Debug)]
#[expect(dead_code, reason = "staged for authenticated detached rebind")]
pub(crate) enum M1AuthenticatedQueuePacketBatchV1 {
    TargetOnly(Box<M1AuthenticatedQueuePacketBatchCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    PairedPrefill(
        Box<M1AuthenticatedQueuePacketBatchCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK4(
        Box<M1AuthenticatedQueuePacketBatchCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK8(
        Box<M1AuthenticatedQueuePacketBatchCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK16(
        Box<M1AuthenticatedQueuePacketBatchCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

#[expect(dead_code, reason = "staged for authenticated detached rebind")]
impl M1AuthenticatedQueuePacketBatchV1 {
    pub(crate) const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        match self {
            Self::TargetOnly(_) => M1PhysicalFixedBatchShapeV1::TargetOnly,
            Self::PairedPrefill(_) => M1PhysicalFixedBatchShapeV1::PairedPrefill,
            Self::SpeculativeK4(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            Self::SpeculativeK8(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            Self::SpeculativeK16(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK16,
        }
    }
}

impl M1PhysicalFixedBatchV1<'_> {
    /// Checked physical-device receipt retained by this complete fixed batch.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody().device()
    }

    /// Exact closed shape of this batch.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        match self {
            Self::TargetOnly(_) => M1PhysicalFixedBatchShapeV1::TargetOnly,
            Self::PairedPrefill(_) => M1PhysicalFixedBatchShapeV1::PairedPrefill,
            Self::SpeculativeK4(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            Self::SpeculativeK8(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            Self::SpeculativeK16(_) => M1PhysicalFixedBatchShapeV1::SpeculativeK16,
        }
    }

    /// Exact packet count of this batch.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Exact retained Ferric recipe and allocation custody, by borrow.
    #[must_use = "the exact Ferric custody remains retained beside the generic batch"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => &case.custody,
            Self::PairedPrefill(case) => &case.custody,
            Self::SpeculativeK4(case) => &case.custody,
            Self::SpeculativeK8(case) => &case.custody,
            Self::SpeculativeK16(case) => &case.custody,
        }
    }

    /// Exact retained coherent completion-output custody, by borrow.
    #[must_use = "the completion-output owner remains retained beside the generic batch"]
    pub const fn completion_output(&self) -> &BoundM1CompletionOutputV1 {
        self.custody().completion_output()
    }

    /// Exact selected target plan.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.custody().selection
    }

    /// Exact selected-program catalog identity.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.custody().catalog_id
    }

    /// Exact program count retained by the generic batch.
    #[must_use]
    pub fn program_count(&self) -> usize {
        match self {
            Self::TargetOnly(case) => case.program_count(),
            Self::PairedPrefill(case) => case.program_count(),
            Self::SpeculativeK4(case) => case.program_count(),
            Self::SpeculativeK8(case) => case.program_count(),
            Self::SpeculativeK16(case) => case.program_count(),
        }
    }

    /// This stage constructs no queue and grants no publication authority.
    #[must_use]
    pub const fn grants_queue_authority(&self) -> bool {
        false
    }

    /// Construction reports no hardware execution or completion.
    #[must_use]
    pub const fn proves_hardware_execution_or_completion(&self) -> bool {
        false
    }
}

/// Row collection named by a fixed-batch build diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalFixedBatchRowSetV1 {
    /// Physical geometry/program recipe rows.
    PhysicalRecipe,
    /// Complete zero-pointer kernarg images.
    KernargImages,
    /// Semantic explicit-buffer source rows.
    SourceBuffers,
    /// Owner-checked generic service-buffer rows.
    BoundBuffers,
}

/// Fail-closed physical fixed-batch build diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalFixedBatchBuildErrorV1 {
    /// A bounded host allocation failed before consuming any source owner.
    HostAllocation,
    /// Selected program roster cardinality drifted.
    ProgramCount { expected: usize, actual: usize },
    /// Detached queue custody names another authenticated program catalog.
    ProgramCatalogIdentity,
    /// Authenticated program families differ from the generated operation plan.
    ProgramFamilyArtifacts,
    /// The retained step intent cannot be derived from the authenticated operation plan.
    OperationPlan(M1StepDispatchCompositionError),
    /// The exact retained dispatch plan differs from fresh authenticated derivation.
    OperationPlanDrift,
    /// The retained physical recipe names another generated runner declaration.
    RunnerDeclarationIdentity,
    /// The retained physical recipe names another structural kernel catalog.
    KernelCatalogIdentity,
    /// Physical and workspace compositions no longer name the same step.
    CompositionIdentity,
    /// Fresh rebind physical structure differs from retained queue custody.
    RetainedPhysicalRecipe,
    /// Fresh rebind workspace structure differs from retained queue custody.
    RetainedWorkspaceComposition,
    /// Fresh rebind semantic buffer sources differ from retained queue custody.
    RetainedSourceBuffers,
    /// Fresh rebind target selection differs from retained queue custody.
    RetainedSelection,
    /// A retained intent is outside the closed M1 fixed-batch family.
    UnsupportedIntent(M1StepDispatchIntent),
    /// The retained dispatch count differs from its closed shape.
    ShapePacketCount {
        shape: M1PhysicalFixedBatchShapeV1,
        expected: usize,
        actual: usize,
    },
    /// One publication-order row collection has the wrong cardinality.
    RowCount {
        rows: M1PhysicalFixedBatchRowSetV1,
        expected: usize,
        actual: usize,
    },
    /// Global dispatch indices no longer agree at one position.
    DispatchIndex { expected: u32 },
    /// Exact plan selection no longer agrees at one position.
    Selection { dispatch_index: u32 },
    /// Canonical profile identity no longer agrees at one position.
    ProfileIdentity { dispatch_index: u32 },
    /// Selected physical program no longer agrees at one position.
    Program { dispatch_index: u32 },
    /// A complete kernarg allocation has the wrong byte count.
    KernargLength {
        dispatch_index: u32,
        expected: usize,
        actual: usize,
    },
    /// Semantic and owner-checked explicit-buffer rosters differ.
    BufferCount {
        dispatch_index: u32,
        expected: usize,
        actual: usize,
    },
    /// One copied generic buffer no longer names the inspected argument ordinal.
    BufferArgument {
        dispatch_index: u32,
        buffer_index: usize,
        expected: usize,
        actual: usize,
    },
    /// The retained completion-output owner names another target selection or shape.
    CompletionOutputShape {
        /// Exact target selection required by the batch.
        expected_selection: Qwen3PlanSelection,
        /// Rejected selection retained by the completion owner.
        actual_selection: Qwen3PlanSelection,
        /// Exact canonical sequence count required by the batch.
        expected_sequences: u32,
        /// Rejected retained sequence count.
        actual_sequences: u32,
        /// Exact canonical byte extent required by the batch.
        expected_extent: u64,
        /// Rejected retained byte extent.
        actual_extent: u64,
    },
    /// A checked count or index could not be represented on this host.
    ArithmeticOverflow,
}

impl fmt::Display for M1PhysicalFixedBatchBuildErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 physical fixed-batch construction rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1PhysicalFixedBatchBuildErrorV1 {}

/// Linear rejection retaining the exact unchanged build inputs.
#[must_use = "a rejected program catalog and physical binding remain recoverable"]
#[derive(Debug)]
pub struct M1PhysicalFixedBatchBuildFailureV1<'a> {
    error: M1PhysicalFixedBatchBuildErrorV1,
    catalog: Box<ContentBoundM1ProgramCatalogV1<'a>>,
    bindings: Box<BoundM1PhysicalBufferBindingsV1>,
}

impl<'a> M1PhysicalFixedBatchBuildFailureV1<'a> {
    /// Exact rejection diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1PhysicalFixedBatchBuildErrorV1 {
        self.error
    }

    /// Recovers the exact unchanged program catalog and physical bindings.
    #[must_use = "both exact unchanged linear inputs remain live"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalFixedBatchBuildErrorV1,
        ContentBoundM1ProgramCatalogV1<'a>,
        BoundM1PhysicalBufferBindingsV1,
    ) {
        (self.error, *self.catalog, *self.bindings)
    }
}

/// Authenticated packet-lowering rejection retaining the exact unchanged bindings.
#[must_use = "authenticated packet lowering failure retains physical bindings"]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
    error: M1PhysicalFixedBatchBuildErrorV1,
    bindings: Box<BoundM1PhysicalBufferBindingsV1>,
}

impl M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
    pub(crate) fn into_parts(
        self,
    ) -> (
        M1PhysicalFixedBatchBuildErrorV1,
        BoundM1PhysicalBufferBindingsV1,
    ) {
        (self.error, *self.bindings)
    }
}

/// Authenticated rebind lowering rejection retaining every exact linear input.
#[must_use = "authenticated rebind lowering failure retains fresh recipe and queue custody"]
#[derive(Debug)]
#[expect(dead_code, reason = "staged for authenticated detached rebind")]
pub(crate) struct M1AuthenticatedQueuePacketBatchBuildFailureV1 {
    error: M1PhysicalFixedBatchBuildErrorV1,
    recipe: Box<AddresslessM1PhysicalBufferRecipeV1>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

#[expect(dead_code, reason = "staged for authenticated detached rebind")]
impl M1AuthenticatedQueuePacketBatchBuildFailureV1 {
    pub(crate) const fn error(&self) -> M1PhysicalFixedBatchBuildErrorV1 {
        self.error
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        M1PhysicalFixedBatchBuildErrorV1,
        AddresslessM1PhysicalBufferRecipeV1,
        Box<[M1BoundPhysicalBufferRowV1]>,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        (self.error, *self.recipe, self.bound_rows, *self.custody)
    }
}

#[derive(Clone, Copy)]
struct PacketRowMetadataV1 {
    expected_dispatch_index: u32,
    physical_dispatch_index: u32,
    image_dispatch_index: u32,
    source_dispatch_index: u32,
    bound_dispatch_index: u32,
    physical_selection: Qwen3PlanSelection,
    image_selection: Qwen3PlanSelection,
    source_selection: Qwen3PlanSelection,
    physical_profile_id: Identity,
    image_profile_id: Identity,
    source_profile_id: Identity,
    bound_profile_id: Identity,
    physical_program: M1PhysicalProgramV1,
    image_program: M1PhysicalProgramV1,
    source_program: M1PhysicalProgramV1,
    bound_program: M1PhysicalProgramV1,
    expected_kernarg_bytes: usize,
    actual_kernarg_bytes: usize,
    source_buffer_count: usize,
    bound_buffer_count: usize,
}

/// Constructs one exact generic fixed batch while retaining all Ferric owners.
///
/// Every fallible correspondence check runs before either move-only input is
/// dismantled. Rejection therefore returns the exact unchanged catalog and
/// bindings. Success consumes the zero-pointer images, copies only the already
/// owner-checked addressless range descriptors into packets, and retains the
/// originating buffer rows and allocation owners beside the generic batch.
/// No queue is constructed or published by this operation.
///
/// # Errors
///
/// Returns [`M1PhysicalFixedBatchBuildFailureV1`] for program, composition,
/// shape, order, profile, kernarg-size, or explicit-buffer roster drift.
pub fn build_m1_physical_fixed_batch_v1(
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    bindings: BoundM1PhysicalBufferBindingsV1,
) -> Result<M1PhysicalFixedBatchV1<'_>, M1PhysicalFixedBatchBuildFailureV1<'_>> {
    let shape = match validate_inputs(&catalog, &bindings) {
        Ok(shape) => shape,
        Err(error) => {
            return Err(M1PhysicalFixedBatchBuildFailureV1 {
                error,
                catalog: Box::new(catalog),
                bindings: Box::new(bindings),
            });
        }
    };

    let catalog_id = catalog.catalog_id();
    let (recipe, workspace_owners, partitioned_memory, completion_output, bound_rows) =
        bindings.into_parts();
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    let selection = workspace_composition
        .dispatch_plan()
        .intent()
        .target_selection();
    let parts = LoweringPartsV1 {
        catalog_id,
        selection,
        catalog,
        physical_recipe,
        images,
        workspace_composition,
        workspace_owners,
        partitioned_memory,
        completion_output,
        source_rows,
        bound_rows,
    };

    let lowered = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => {
            lower_boxed_fixed_batch(parts).map(M1PhysicalFixedBatchV1::TargetOnly)
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            lower_boxed_fixed_batch(parts).map(M1PhysicalFixedBatchV1::PairedPrefill)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            lower_boxed_fixed_batch(parts).map(M1PhysicalFixedBatchV1::SpeculativeK4)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => {
            lower_boxed_fixed_batch(parts).map(M1PhysicalFixedBatchV1::SpeculativeK8)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            lower_boxed_fixed_batch(parts).map(M1PhysicalFixedBatchV1::SpeculativeK16)
        }
    };
    match lowered {
        Ok(batch) => Ok(batch),
        Err(failure) => {
            let (error, catalog, bindings) = (*failure).into_original_inputs();
            Err(M1PhysicalFixedBatchBuildFailureV1 {
                error,
                catalog: Box::new(catalog),
                bindings: Box::new(bindings),
            })
        }
    }
}

/// Lowers owner-checked bindings with the exact authenticated Worker V3 program map.
///
/// The expected declaration and structural catalog identities come from the
/// same authenticated runner that retained `programs`. Every service program
/// index is selected privately from that set after all validation succeeds.
pub(crate) fn build_m1_authenticated_physical_packet_batch_v1(
    programs: &M1AuthenticatedWorkerV3ProgramSetV1,
    operations: &DeclaredOperationKernelPlan,
    bindings: BoundM1PhysicalBufferBindingsV1,
) -> Result<M1AuthenticatedPhysicalPacketBatchV1, M1AuthenticatedPhysicalPacketBatchBuildFailureV1>
{
    let shape = match validate_bound_inputs(programs.program_count(), &bindings) {
        Ok(shape) => shape,
        Err(error) => {
            return Err(M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
                error,
                bindings: Box::new(bindings),
            });
        }
    };
    let dispatch_plan = bindings.workspace_bindings().composition().dispatch_plan();
    if programs.family_artifacts() != operations.families() {
        return Err(M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::ProgramFamilyArtifacts,
            bindings: Box::new(bindings),
        });
    }
    if let Err(error) = validate_authenticated_operation_plan_v1(operations, dispatch_plan) {
        return Err(M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
            error,
            bindings: Box::new(bindings),
        });
    }

    let catalog_id = programs.catalog_id();
    let (recipe, workspace_owners, partitioned_memory, completion_output, bound_rows) =
        bindings.into_parts();
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    let selection = workspace_composition
        .dispatch_plan()
        .intent()
        .target_selection();
    let parts = AuthenticatedLoweringPartsV1 {
        catalog_id,
        selection,
        physical_recipe,
        images,
        workspace_composition,
        workspace_owners,
        partitioned_memory,
        completion_output,
        source_rows,
        bound_rows,
    };

    let lowered = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => {
            lower_authenticated_boxed_packet_batch(parts, programs)
                .map(M1AuthenticatedPhysicalPacketBatchV1::TargetOnly)
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            lower_authenticated_boxed_packet_batch(parts, programs)
                .map(M1AuthenticatedPhysicalPacketBatchV1::PairedPrefill)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            lower_authenticated_boxed_packet_batch(parts, programs)
                .map(M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK4)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => {
            lower_authenticated_boxed_packet_batch(parts, programs)
                .map(M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK8)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            lower_authenticated_boxed_packet_batch(parts, programs)
                .map(M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK16)
        }
    };
    match lowered {
        Ok(batch) => Ok(batch),
        Err(failure) => {
            let (error, bindings) = (*failure).into_original_inputs();
            Err(M1AuthenticatedPhysicalPacketBatchBuildFailureV1 {
                error,
                bindings: Box::new(bindings),
            })
        }
    }
}

/// Lowers a fresh same-structure recipe for one detached authenticated queue.
///
/// Service program indices are resolved only through `witness`. The fresh
/// recipe may carry new zero-pointer kernarg scalar bytes and `bound_rows` may
/// carry freshly replaced allocation ranges, but its physical recipe,
/// workspace composition, and semantic source rows must remain exactly equal
/// to retained queue custody.
///
/// # Errors
///
/// Returns every exact linear input unchanged on authenticated identity,
/// operation-plan, retained-structure, packet-row, cardinality, completion
/// shape, or host-allocation rejection.
#[expect(dead_code, reason = "staged for authenticated detached rebind")]
pub(crate) fn build_m1_authenticated_queue_packet_batch_v1(
    witness: &M1AuthenticatedProgramCatalogWitnessV1,
    operations: &DeclaredOperationKernelPlan,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
    custody: M1PhysicalQueueBatchCustodyV1,
) -> Result<M1AuthenticatedQueuePacketBatchV1, M1AuthenticatedQueuePacketBatchBuildFailureV1> {
    let shape = match validate_authenticated_queue_packet_inputs(
        witness,
        operations,
        &recipe,
        &bound_rows,
        &custody,
    ) {
        Ok(shape) => shape,
        Err(error) => {
            return Err(M1AuthenticatedQueuePacketBatchBuildFailureV1 {
                error,
                recipe: Box::new(recipe),
                bound_rows,
                custody: Box::new(custody),
            });
        }
    };

    match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => {
            lower_authenticated_queue_packet_case(witness, recipe, bound_rows, custody)
                .map(M1AuthenticatedQueuePacketBatchV1::TargetOnly)
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            lower_authenticated_queue_packet_case(witness, recipe, bound_rows, custody)
                .map(M1AuthenticatedQueuePacketBatchV1::PairedPrefill)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            lower_authenticated_queue_packet_case(witness, recipe, bound_rows, custody)
                .map(M1AuthenticatedQueuePacketBatchV1::SpeculativeK4)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => {
            lower_authenticated_queue_packet_case(witness, recipe, bound_rows, custody)
                .map(M1AuthenticatedQueuePacketBatchV1::SpeculativeK8)
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            lower_authenticated_queue_packet_case(witness, recipe, bound_rows, custody)
                .map(M1AuthenticatedQueuePacketBatchV1::SpeculativeK16)
        }
    }
}

fn validate_authenticated_queue_packet_inputs(
    witness: &M1AuthenticatedProgramCatalogWitnessV1,
    operations: &DeclaredOperationKernelPlan,
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    bound_rows: &[M1BoundPhysicalBufferRowV1],
    custody: &M1PhysicalQueueBatchCustodyV1,
) -> Result<M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchBuildErrorV1> {
    if witness.catalog_id() != custody.catalog_id() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::ProgramCatalogIdentity);
    }
    if witness.family_artifacts() != operations.families() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::ProgramFamilyArtifacts);
    }

    let physical_recipe = recipe.kernarg_recipe().source_recipe();
    let workspace_composition = recipe.workspace_composition();
    let shape = validate_packet_inputs(PacketValidationInputsV1 {
        program_count: M1_PHYSICAL_PROGRAM_COUNT_V1,
        physical_recipe,
        images: recipe.kernarg_recipe().images(),
        workspace_composition,
        source_rows: recipe.rows(),
        bound_rows,
        completion_output_shape: custody.completion_output().shape(),
    })?;
    validate_authenticated_operation_plan_v1(operations, workspace_composition.dispatch_plan())?;

    let selection = workspace_composition
        .dispatch_plan()
        .intent()
        .target_selection();
    if selection != custody.selection() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RetainedSelection);
    }
    if physical_recipe != custody.physical_recipe() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RetainedPhysicalRecipe);
    }
    if workspace_composition != custody.workspace_composition() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RetainedWorkspaceComposition);
    }
    if recipe.rows() != custody.source_rows() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RetainedSourceBuffers);
    }
    Ok(shape)
}

pub(crate) fn validate_authenticated_operation_plan_v1(
    operations: &DeclaredOperationKernelPlan,
    dispatch_plan: &crate::AddresslessM1StepDispatchPlan,
) -> Result<(), M1PhysicalFixedBatchBuildErrorV1> {
    if dispatch_plan.runner_declaration_id() != operations.runner_declaration_id() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RunnerDeclarationIdentity);
    }
    if dispatch_plan.kernel_catalog_id() != operations.kernel_catalog_id() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::KernelCatalogIdentity);
    }
    let expected_dispatch_plan = derive_m1_step_dispatch_plan(operations, dispatch_plan.intent())
        .map_err(M1PhysicalFixedBatchBuildErrorV1::OperationPlan)?;
    if &expected_dispatch_plan != dispatch_plan {
        return Err(M1PhysicalFixedBatchBuildErrorV1::OperationPlanDrift);
    }
    Ok(())
}

fn validate_inputs(
    catalog: &ContentBoundM1ProgramCatalogV1<'_>,
    bindings: &BoundM1PhysicalBufferBindingsV1,
) -> Result<M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchBuildErrorV1> {
    validate_bound_inputs(catalog.program_count(), bindings)
}

fn validate_bound_inputs(
    program_count: usize,
    bindings: &BoundM1PhysicalBufferBindingsV1,
) -> Result<M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchBuildErrorV1> {
    validate_packet_inputs(PacketValidationInputsV1 {
        program_count,
        physical_recipe: bindings.kernarg_recipe().source_recipe(),
        images: bindings.kernarg_recipe().images(),
        workspace_composition: bindings.workspace_bindings().composition(),
        source_rows: bindings.source_rows(),
        bound_rows: bindings.rows(),
        completion_output_shape: bindings.completion_output().shape(),
    })
}

struct PacketValidationInputsV1<'a> {
    program_count: usize,
    physical_recipe: &'a AddresslessM1PhysicalDispatchRecipeV1,
    images: &'a [M1PhysicalKernargImageV1],
    workspace_composition: &'a AddresslessM1FullStepWorkspaceComposition,
    source_rows: &'a [M1PhysicalBufferRecipeRowV1],
    bound_rows: &'a [M1BoundPhysicalBufferRowV1],
    completion_output_shape: M1CompletionOutputShapeV1,
}

fn validate_packet_inputs(
    inputs: PacketValidationInputsV1<'_>,
) -> Result<M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchBuildErrorV1> {
    let PacketValidationInputsV1 {
        program_count,
        physical_recipe,
        images,
        workspace_composition,
        source_rows,
        bound_rows,
        completion_output_shape,
    } = inputs;
    if program_count != M1_PHYSICAL_PROGRAM_COUNT_V1 {
        return Err(M1PhysicalFixedBatchBuildErrorV1::ProgramCount {
            expected: M1_PHYSICAL_PROGRAM_COUNT_V1,
            actual: program_count,
        });
    }

    if physical_recipe.composition_id() != workspace_composition.dispatch_plan().composition_id() {
        return Err(M1PhysicalFixedBatchBuildErrorV1::CompositionIdentity);
    }
    let count = usize::try_from(physical_recipe.dispatch_count())
        .map_err(|_| M1PhysicalFixedBatchBuildErrorV1::ArithmeticOverflow)?;
    let shape = classify_shape(workspace_composition.dispatch_plan().intent(), count)?;
    let selection = workspace_composition
        .dispatch_plan()
        .intent()
        .target_selection();
    validate_completion_output_shape(selection, completion_output_shape)?;

    validate_row_count(
        M1PhysicalFixedBatchRowSetV1::PhysicalRecipe,
        count,
        physical_recipe.rows().len(),
    )?;
    validate_row_count(
        M1PhysicalFixedBatchRowSetV1::KernargImages,
        count,
        images.len(),
    )?;
    validate_row_count(
        M1PhysicalFixedBatchRowSetV1::SourceBuffers,
        count,
        source_rows.len(),
    )?;
    validate_row_count(
        M1PhysicalFixedBatchRowSetV1::BoundBuffers,
        count,
        bound_rows.len(),
    )?;

    for (position, (((physical, image), source), bound)) in physical_recipe
        .rows()
        .iter()
        .zip(images)
        .zip(source_rows)
        .zip(bound_rows)
        .enumerate()
    {
        let expected_dispatch_index = u32::try_from(position)
            .map_err(|_| M1PhysicalFixedBatchBuildErrorV1::ArithmeticOverflow)?;
        let expected_kernarg_bytes = usize::try_from(physical.kernarg_bytes())
            .map_err(|_| M1PhysicalFixedBatchBuildErrorV1::ArithmeticOverflow)?;
        validate_packet_row(PacketRowMetadataV1 {
            expected_dispatch_index,
            physical_dispatch_index: physical.dispatch_index(),
            image_dispatch_index: image.dispatch_index(),
            source_dispatch_index: source.dispatch_index(),
            bound_dispatch_index: bound.dispatch_index(),
            physical_selection: physical.selection(),
            image_selection: image.selection(),
            source_selection: source.selection(),
            physical_profile_id: physical.profile_id(),
            image_profile_id: image.profile_id(),
            source_profile_id: source.profile_id(),
            bound_profile_id: bound.profile_id(),
            physical_program: physical.program(),
            image_program: image.program(),
            source_program: source.program(),
            bound_program: bound.program(),
            expected_kernarg_bytes,
            actual_kernarg_bytes: image.bytes().len(),
            source_buffer_count: source.buffers().len(),
            bound_buffer_count: bound.buffers().len(),
        })?;
        for (buffer_index, (source_buffer, bound_buffer)) in
            source.buffers().iter().zip(bound.buffers()).enumerate()
        {
            let expected = source_buffer.explicit_argument_index();
            let actual = bound_buffer.explicit_argument_index();
            if actual != expected {
                return Err(M1PhysicalFixedBatchBuildErrorV1::BufferArgument {
                    dispatch_index: expected_dispatch_index,
                    buffer_index,
                    expected,
                    actual,
                });
            }
        }
    }
    Ok(shape)
}

fn validate_completion_output_shape(
    selection: Qwen3PlanSelection,
    actual: M1CompletionOutputShapeV1,
) -> Result<(), M1PhysicalFixedBatchBuildErrorV1> {
    let expected = m1_completion_output_shape_v1(selection).map_err(|_| {
        M1PhysicalFixedBatchBuildErrorV1::CompletionOutputShape {
            expected_selection: selection,
            actual_selection: actual.selection(),
            expected_sequences: 0,
            actual_sequences: actual.sequences(),
            expected_extent: 0,
            actual_extent: actual.extent_bytes(),
        }
    })?;
    if actual != expected {
        return Err(M1PhysicalFixedBatchBuildErrorV1::CompletionOutputShape {
            expected_selection: selection,
            actual_selection: actual.selection(),
            expected_sequences: expected.sequences(),
            actual_sequences: actual.sequences(),
            expected_extent: expected.extent_bytes(),
            actual_extent: actual.extent_bytes(),
        });
    }
    Ok(())
}

fn classify_shape(
    intent: M1StepDispatchIntent,
    actual: usize,
) -> Result<M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchBuildErrorV1> {
    let shape = m1_physical_fixed_batch_shape_for_intent_v1(intent)
        .ok_or(M1PhysicalFixedBatchBuildErrorV1::UnsupportedIntent(intent))?;
    let expected = shape.packet_count();
    if actual != expected {
        return Err(M1PhysicalFixedBatchBuildErrorV1::ShapePacketCount {
            shape,
            expected,
            actual,
        });
    }
    Ok(shape)
}

pub(crate) fn m1_physical_fixed_batch_shape_for_intent_v1(
    intent: M1StepDispatchIntent,
) -> Option<M1PhysicalFixedBatchShapeV1> {
    let shape = match intent {
        M1StepDispatchIntent::TargetOnly(_) => M1PhysicalFixedBatchShapeV1::TargetOnly,
        M1StepDispatchIntent::PairedPrefill(_) => M1PhysicalFixedBatchShapeV1::PairedPrefill,
        M1StepDispatchIntent::SpeculativeRound(selection) => match selection.bucket {
            Qwen3PlanBucket::SpeculativeS1K4C8192 | Qwen3PlanBucket::SpeculativeS8K4C8192 => {
                M1PhysicalFixedBatchShapeV1::SpeculativeK4
            }
            Qwen3PlanBucket::SpeculativeS1K8C8192 => M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            Qwen3PlanBucket::SpeculativeS1K16C8192 => M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            _ => return None,
        },
    };
    Some(shape)
}

fn validate_row_count(
    rows: M1PhysicalFixedBatchRowSetV1,
    expected: usize,
    actual: usize,
) -> Result<(), M1PhysicalFixedBatchBuildErrorV1> {
    if actual != expected {
        return Err(M1PhysicalFixedBatchBuildErrorV1::RowCount {
            rows,
            expected,
            actual,
        });
    }
    Ok(())
}

fn validate_packet_row(row: PacketRowMetadataV1) -> Result<(), M1PhysicalFixedBatchBuildErrorV1> {
    let dispatch_index = row.expected_dispatch_index;
    if row.physical_dispatch_index != dispatch_index
        || row.image_dispatch_index != dispatch_index
        || row.source_dispatch_index != dispatch_index
        || row.bound_dispatch_index != dispatch_index
    {
        return Err(M1PhysicalFixedBatchBuildErrorV1::DispatchIndex {
            expected: dispatch_index,
        });
    }
    if row.image_selection != row.physical_selection
        || row.source_selection != row.physical_selection
    {
        return Err(M1PhysicalFixedBatchBuildErrorV1::Selection { dispatch_index });
    }
    if row.image_profile_id != row.physical_profile_id
        || row.source_profile_id != row.physical_profile_id
        || row.bound_profile_id != row.physical_profile_id
    {
        return Err(M1PhysicalFixedBatchBuildErrorV1::ProfileIdentity { dispatch_index });
    }
    if row.image_program != row.physical_program
        || row.source_program != row.physical_program
        || row.bound_program != row.physical_program
    {
        return Err(M1PhysicalFixedBatchBuildErrorV1::Program { dispatch_index });
    }
    if row.actual_kernarg_bytes != row.expected_kernarg_bytes {
        return Err(M1PhysicalFixedBatchBuildErrorV1::KernargLength {
            dispatch_index,
            expected: row.expected_kernarg_bytes,
            actual: row.actual_kernarg_bytes,
        });
    }
    if row.bound_buffer_count != row.source_buffer_count {
        return Err(M1PhysicalFixedBatchBuildErrorV1::BufferCount {
            dispatch_index,
            expected: row.source_buffer_count,
            actual: row.bound_buffer_count,
        });
    }
    Ok(())
}

struct LoweringPartsV1<'a> {
    catalog_id: Identity,
    selection: Qwen3PlanSelection,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    physical_recipe: AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[M1PhysicalKernargImageV1]>,
    workspace_composition: AddresslessM1FullStepWorkspaceComposition,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

impl<'a> LoweringPartsV1<'a> {
    fn into_original_inputs(
        self,
    ) -> (
        ContentBoundM1ProgramCatalogV1<'a>,
        BoundM1PhysicalBufferBindingsV1,
    ) {
        let kernargs =
            AddresslessM1PhysicalKernargRecipeV1::from_parts(self.physical_recipe, self.images);
        let recipe = AddresslessM1PhysicalBufferRecipeV1::from_parts(
            kernargs,
            self.workspace_composition,
            self.source_rows,
        );
        let bindings = BoundM1PhysicalBufferBindingsV1::from_parts(
            recipe,
            self.workspace_owners,
            self.partitioned_memory,
            self.completion_output,
            self.bound_rows,
        );
        (self.catalog, bindings)
    }
}

struct LoweringFailureV1<'a> {
    error: M1PhysicalFixedBatchBuildErrorV1,
    parts: LoweringPartsV1<'a>,
}

impl<'a> LoweringFailureV1<'a> {
    fn into_original_inputs(
        self,
    ) -> (
        M1PhysicalFixedBatchBuildErrorV1,
        ContentBoundM1ProgramCatalogV1<'a>,
        BoundM1PhysicalBufferBindingsV1,
    ) {
        let (catalog, bindings) = self.parts.into_original_inputs();
        (self.error, catalog, bindings)
    }
}

struct AuthenticatedLoweringPartsV1 {
    catalog_id: Identity,
    selection: Qwen3PlanSelection,
    physical_recipe: AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[M1PhysicalKernargImageV1]>,
    workspace_composition: AddresslessM1FullStepWorkspaceComposition,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

impl AuthenticatedLoweringPartsV1 {
    fn into_original_bindings(self) -> BoundM1PhysicalBufferBindingsV1 {
        let kernargs =
            AddresslessM1PhysicalKernargRecipeV1::from_parts(self.physical_recipe, self.images);
        let recipe = AddresslessM1PhysicalBufferRecipeV1::from_parts(
            kernargs,
            self.workspace_composition,
            self.source_rows,
        );
        BoundM1PhysicalBufferBindingsV1::from_parts(
            recipe,
            self.workspace_owners,
            self.partitioned_memory,
            self.completion_output,
            self.bound_rows,
        )
    }
}

struct AuthenticatedLoweringFailureV1 {
    error: M1PhysicalFixedBatchBuildErrorV1,
    parts: AuthenticatedLoweringPartsV1,
}

impl AuthenticatedLoweringFailureV1 {
    fn into_original_inputs(
        self,
    ) -> (
        M1PhysicalFixedBatchBuildErrorV1,
        BoundM1PhysicalBufferBindingsV1,
    ) {
        (self.error, self.parts.into_original_bindings())
    }
}

struct PacketLoweringInputV1 {
    physical: crate::M1PhysicalDispatchRecipeRowV1,
    image: M1PhysicalKernargImageV1,
    buffers: Box<[fe2o3_service_host::ServiceFixedDispatchBufferV1]>,
}

trait AuthenticatedProgramIndexResolverV1 {
    fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize;
}

impl AuthenticatedProgramIndexResolverV1 for M1AuthenticatedWorkerV3ProgramSetV1 {
    fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        M1AuthenticatedWorkerV3ProgramSetV1::service_program_index(self, program)
    }
}

impl AuthenticatedProgramIndexResolverV1 for M1AuthenticatedProgramCatalogWitnessV1 {
    fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        M1AuthenticatedProgramCatalogWitnessV1::service_program_index(self, program)
    }
}

struct AuthenticatedPacketArrayLoweringFailureV1 {
    error: M1PhysicalFixedBatchBuildErrorV1,
    images: Box<[M1PhysicalKernargImageV1]>,
}

// Keep each const-cardinality array construction out of the shape dispatcher.
#[inline(never)]
fn lower_boxed_fixed_batch<const N: usize>(
    parts: LoweringPartsV1<'_>,
) -> Result<Box<M1PhysicalFixedBatchCaseV1<'_, N>>, Box<LoweringFailureV1<'_>>> {
    lower_fixed_batch(parts).map(Box::new)
}

// Keep each authenticated const-cardinality array construction out of the shape dispatcher.
#[inline(never)]
fn lower_authenticated_boxed_packet_batch<
    const N: usize,
    R: AuthenticatedProgramIndexResolverV1,
>(
    parts: AuthenticatedLoweringPartsV1,
    programs: &R,
) -> Result<Box<M1AuthenticatedPhysicalPacketBatchCaseV1<N>>, Box<AuthenticatedLoweringFailureV1>> {
    lower_authenticated_packet_batch(parts, programs).map(Box::new)
}

fn lower_authenticated_packet_batch<const N: usize, R: AuthenticatedProgramIndexResolverV1>(
    mut parts: AuthenticatedLoweringPartsV1,
    programs: &R,
) -> Result<M1AuthenticatedPhysicalPacketBatchCaseV1<N>, Box<AuthenticatedLoweringFailureV1>> {
    let images = core::mem::take(&mut parts.images);
    let packets = match lower_authenticated_packet_array(
        programs,
        &parts.physical_recipe,
        images,
        &parts.bound_rows,
    ) {
        Ok(packets) => packets,
        Err(failure) => {
            let AuthenticatedPacketArrayLoweringFailureV1 { error, images } = *failure;
            parts.images = images;
            return Err(Box::new(AuthenticatedLoweringFailureV1 { error, parts }));
        }
    };
    let custody = M1PhysicalFixedBatchCustodyV1 {
        catalog_id: parts.catalog_id,
        selection: parts.selection,
        physical_recipe: parts.physical_recipe,
        workspace_composition: parts.workspace_composition,
        workspace_owners: parts.workspace_owners,
        partitioned_memory: parts.partitioned_memory,
        completion_output: parts.completion_output,
        source_rows: parts.source_rows,
        bound_rows: parts.bound_rows,
    };
    Ok(M1AuthenticatedPhysicalPacketBatchCaseV1 { packets, custody })
}

#[inline(never)]
fn lower_authenticated_queue_packet_case<const N: usize>(
    witness: &M1AuthenticatedProgramCatalogWitnessV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
    custody: M1PhysicalQueueBatchCustodyV1,
) -> Result<
    Box<M1AuthenticatedQueuePacketBatchCaseV1<N>>,
    M1AuthenticatedQueuePacketBatchBuildFailureV1,
> {
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    let packets =
        match lower_authenticated_packet_array(witness, &physical_recipe, images, &bound_rows) {
            Ok(packets) => packets,
            Err(failure) => {
                let AuthenticatedPacketArrayLoweringFailureV1 { error, images } = *failure;
                let kernargs =
                    AddresslessM1PhysicalKernargRecipeV1::from_parts(physical_recipe, images);
                let recipe = AddresslessM1PhysicalBufferRecipeV1::from_parts(
                    kernargs,
                    workspace_composition,
                    source_rows,
                );
                return Err(M1AuthenticatedQueuePacketBatchBuildFailureV1 {
                    error,
                    recipe: Box::new(recipe),
                    bound_rows,
                    custody: Box::new(custody),
                });
            }
        };

    let mut parts = custody.into_rearm_parts();
    parts.physical_recipe = physical_recipe;
    parts.workspace_composition = workspace_composition;
    parts.source_rows = source_rows;
    parts.bound_rows = bound_rows;
    Ok(Box::new(M1AuthenticatedQueuePacketBatchCaseV1 {
        packets,
        custody: M1PhysicalQueueBatchCustodyV1::from_rearm_parts(parts),
    }))
}

fn lower_authenticated_packet_array<const N: usize, R: AuthenticatedProgramIndexResolverV1>(
    programs: &R,
    physical_recipe: &AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[M1PhysicalKernargImageV1]>,
    bound_rows: &[M1BoundPhysicalBufferRowV1],
) -> Result<[ServiceFixedDispatchPacketV1; N], Box<AuthenticatedPacketArrayLoweringFailureV1>> {
    if physical_recipe.rows().len() != N {
        return Err(Box::new(AuthenticatedPacketArrayLoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::PhysicalRecipe,
                expected: N,
                actual: physical_recipe.rows().len(),
            },
            images,
        }));
    }
    if images.len() != N {
        return Err(Box::new(AuthenticatedPacketArrayLoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::KernargImages,
                expected: N,
                actual: images.len(),
            },
            images,
        }));
    }
    if bound_rows.len() != N {
        return Err(Box::new(AuthenticatedPacketArrayLoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::BoundBuffers,
                expected: N,
                actual: bound_rows.len(),
            },
            images,
        }));
    }
    let mut inputs = Vec::new();
    if inputs.try_reserve_exact(N).is_err() {
        return Err(Box::new(AuthenticatedPacketArrayLoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::HostAllocation,
            images,
        }));
    }
    for ((image, physical), bound) in images
        .into_vec()
        .into_iter()
        .zip(physical_recipe.rows().iter().copied())
        .zip(bound_rows)
    {
        inputs.push(PacketLoweringInputV1 {
            physical,
            image,
            buffers: bound.buffers().to_vec().into_boxed_slice(),
        });
    }
    let inputs: [PacketLoweringInputV1; N] = match inputs.try_into() {
        Ok(inputs) => inputs,
        Err(inputs) => {
            let images = inputs
                .into_iter()
                .map(|input| input.image)
                .collect::<Vec<_>>()
                .into_boxed_slice();
            return Err(Box::new(AuthenticatedPacketArrayLoweringFailureV1 {
                error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                    rows: M1PhysicalFixedBatchRowSetV1::KernargImages,
                    expected: N,
                    actual: images.len(),
                },
                images,
            }));
        }
    };
    Ok(inputs.map(|input| {
        let PacketLoweringInputV1 {
            physical,
            image,
            buffers,
        } = input;
        let service_program_index = programs.service_program_index(physical.program());
        ServiceFixedDispatchPacketV1::new(
            service_program_index,
            physical.geometry(),
            physical.dynamic_group_segment_bytes(),
            image.into_bytes(),
            buffers,
        )
    }))
}

fn lower_fixed_batch<const N: usize>(
    mut parts: LoweringPartsV1<'_>,
) -> Result<M1PhysicalFixedBatchCaseV1<'_, N>, Box<LoweringFailureV1<'_>>> {
    if parts.physical_recipe.rows().len() != N {
        return Err(Box::new(LoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::PhysicalRecipe,
                expected: N,
                actual: parts.physical_recipe.rows().len(),
            },
            parts,
        }));
    }
    if parts.images.len() != N {
        return Err(Box::new(LoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::KernargImages,
                expected: N,
                actual: parts.images.len(),
            },
            parts,
        }));
    }
    if parts.bound_rows.len() != N {
        return Err(Box::new(LoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::BoundBuffers,
                expected: N,
                actual: parts.bound_rows.len(),
            },
            parts,
        }));
    }
    let mut inputs = Vec::new();
    if inputs.try_reserve_exact(N).is_err() {
        return Err(Box::new(LoweringFailureV1 {
            error: M1PhysicalFixedBatchBuildErrorV1::HostAllocation,
            parts,
        }));
    }
    let images = core::mem::take(&mut parts.images).into_vec();
    for ((image, physical), bound) in images
        .into_iter()
        .zip(parts.physical_recipe.rows().iter().copied())
        .zip(parts.bound_rows.iter())
    {
        inputs.push(PacketLoweringInputV1 {
            physical,
            image,
            buffers: bound.buffers().to_vec().into_boxed_slice(),
        });
    }
    let inputs: [PacketLoweringInputV1; N] = match inputs.try_into() {
        Ok(inputs) => inputs,
        Err(inputs) => {
            parts.images = inputs
                .into_iter()
                .map(|input| input.image)
                .collect::<Vec<_>>()
                .into_boxed_slice();
            return Err(Box::new(LoweringFailureV1 {
                error: M1PhysicalFixedBatchBuildErrorV1::RowCount {
                    rows: M1PhysicalFixedBatchRowSetV1::KernargImages,
                    expected: N,
                    actual: parts.images.len(),
                },
                parts,
            }));
        }
    };
    let packets = inputs.map(|input| {
        let PacketLoweringInputV1 {
            physical,
            image,
            buffers,
        } = input;
        ServiceFixedDispatchPacketV1::new(
            physical.program_index(),
            physical.geometry(),
            physical.dynamic_group_segment_bytes(),
            image.into_bytes(),
            buffers,
        )
    });
    let batch = ServiceFixedBatchV1::new(parts.catalog.into_programs(), packets);
    let custody = M1PhysicalFixedBatchCustodyV1 {
        catalog_id: parts.catalog_id,
        selection: parts.selection,
        physical_recipe: parts.physical_recipe,
        workspace_composition: parts.workspace_composition,
        workspace_owners: parts.workspace_owners,
        partitioned_memory: parts.partitioned_memory,
        completion_output: parts.completion_output,
        source_rows: parts.source_rows,
        bound_rows: parts.bound_rows,
    };
    Ok(M1PhysicalFixedBatchCaseV1 { batch, custody })
}

#[cfg(test)]
mod tests {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

    use super::{
        classify_shape, m1_physical_fixed_batch_shape_for_intent_v1,
        validate_completion_output_shape, validate_packet_row, validate_row_count,
        M1PhysicalFixedBatchBuildErrorV1, M1PhysicalFixedBatchRowSetV1,
        M1PhysicalFixedBatchShapeV1, PacketRowMetadataV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
        M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
        M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
    };
    use crate::{m1_completion_output_shape_v1, M1PhysicalProgramV1, M1StepDispatchIntent};
    use ferric_spec::Identity;

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    #[test]
    fn closed_shapes_have_the_reviewed_exact_cardinalities() {
        let cases = [
            (
                M1StepDispatchIntent::TargetOnly(target(
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS1C8192,
                )),
                M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
            ),
            (
                M1StepDispatchIntent::PairedPrefill(target(
                    Qwen3ExecutionMode::Prefill,
                    Qwen3PlanBucket::PrefillS8T128,
                )),
                M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
            ),
            (
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K4C8192,
                )),
                M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS8K4C8192,
                )),
                M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K8C8192,
                )),
                M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            (
                M1StepDispatchIntent::SpeculativeRound(target(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K16C8192,
                )),
                M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
        ];
        for (intent, count, expected) in cases {
            assert_eq!(classify_shape(intent, count), Ok(expected));
            assert_eq!(expected.packet_count(), count);
        }
    }

    #[test]
    fn retained_intent_distinguishes_target_only_and_paired_prefill() {
        let selection = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        assert_eq!(
            m1_physical_fixed_batch_shape_for_intent_v1(M1StepDispatchIntent::TargetOnly(
                selection
            )),
            Some(M1PhysicalFixedBatchShapeV1::TargetOnly)
        );
        assert_eq!(
            m1_physical_fixed_batch_shape_for_intent_v1(M1StepDispatchIntent::PairedPrefill(
                selection
            )),
            Some(M1PhysicalFixedBatchShapeV1::PairedPrefill)
        );
    }

    #[test]
    fn shape_classifier_rejects_cardinality_and_bucket_drift() {
        let intent = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ));
        assert_eq!(
            classify_shape(intent, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1 - 1),
            Err(M1PhysicalFixedBatchBuildErrorV1::ShapePacketCount {
                shape: M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                expected: M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
                actual: M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1 - 1,
            })
        );
        let unsupported = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::DecodeS1C8192,
        ));
        assert_eq!(
            classify_shape(unsupported, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1),
            Err(M1PhysicalFixedBatchBuildErrorV1::UnsupportedIntent(
                unsupported
            ))
        );
    }

    #[test]
    fn fixed_batch_rejects_completion_owner_selection_drift_at_equal_extent() {
        let expected = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        let stale = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        let actual = m1_completion_output_shape_v1(stale).unwrap();
        assert_eq!(actual.extent_bytes(), 960);
        assert_eq!(
            validate_completion_output_shape(expected, actual),
            Err(M1PhysicalFixedBatchBuildErrorV1::CompletionOutputShape {
                expected_selection: expected,
                actual_selection: stale,
                expected_sequences: 8,
                actual_sequences: 8,
                expected_extent: 960,
                actual_extent: 960,
            })
        );
    }

    fn exact_row() -> PacketRowMetadataV1 {
        let selection = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let profile_id = Identity::new([7; 32]);
        PacketRowMetadataV1 {
            expected_dispatch_index: 3,
            physical_dispatch_index: 3,
            image_dispatch_index: 3,
            source_dispatch_index: 3,
            bound_dispatch_index: 3,
            physical_selection: selection,
            image_selection: selection,
            source_selection: selection,
            physical_profile_id: profile_id,
            image_profile_id: profile_id,
            source_profile_id: profile_id,
            bound_profile_id: profile_id,
            physical_program: M1PhysicalProgramV1::RmsNorm,
            image_program: M1PhysicalProgramV1::RmsNorm,
            source_program: M1PhysicalProgramV1::RmsNorm,
            bound_program: M1PhysicalProgramV1::RmsNorm,
            expected_kernarg_bytes: 312,
            actual_kernarg_bytes: 312,
            source_buffer_count: 5,
            bound_buffer_count: 5,
        }
    }

    #[test]
    fn packet_row_join_rejects_each_cross_owner_drift_class() {
        assert_eq!(validate_packet_row(exact_row()), Ok(()));

        let mut row = exact_row();
        row.bound_dispatch_index = 4;
        assert_eq!(
            validate_packet_row(row),
            Err(M1PhysicalFixedBatchBuildErrorV1::DispatchIndex { expected: 3 })
        );

        let mut row = exact_row();
        row.image_profile_id = Identity::new([8; 32]);
        assert_eq!(
            validate_packet_row(row),
            Err(M1PhysicalFixedBatchBuildErrorV1::ProfileIdentity { dispatch_index: 3 })
        );

        let mut row = exact_row();
        row.bound_program = M1PhysicalProgramV1::Rope;
        assert_eq!(
            validate_packet_row(row),
            Err(M1PhysicalFixedBatchBuildErrorV1::Program { dispatch_index: 3 })
        );

        let mut row = exact_row();
        row.actual_kernarg_bytes = 311;
        assert_eq!(
            validate_packet_row(row),
            Err(M1PhysicalFixedBatchBuildErrorV1::KernargLength {
                dispatch_index: 3,
                expected: 312,
                actual: 311,
            })
        );

        let mut row = exact_row();
        row.bound_buffer_count = 4;
        assert_eq!(
            validate_packet_row(row),
            Err(M1PhysicalFixedBatchBuildErrorV1::BufferCount {
                dispatch_index: 3,
                expected: 5,
                actual: 4,
            })
        );
    }

    #[test]
    fn row_count_diagnostic_names_the_drifting_owner() {
        assert_eq!(
            validate_row_count(M1PhysicalFixedBatchRowSetV1::KernargImages, 545, 544),
            Err(M1PhysicalFixedBatchBuildErrorV1::RowCount {
                rows: M1PhysicalFixedBatchRowSetV1::KernargImages,
                expected: 545,
                actual: 544,
            })
        );
    }
}
