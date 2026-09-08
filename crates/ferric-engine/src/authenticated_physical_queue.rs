//! Authenticated Ferric custody around one complete M1 queue generation.
//!
//! Packet arrays and service-program indices remain private. Every queue phase
//! retains the exact generated operation plan, authenticated program witness,
//! allocation/KV custody, and scheduler authority that produced the batch.

use core::num::NonZeroU32;
use fe2o3_host::{
    AuthenticatedServiceCompletedQueueSessionV1, AuthenticatedServicePublishedQueueSessionV1,
    AuthenticatedServiceQueueCreateFailureV1, AuthenticatedServiceQueueOperationFailureV1,
    AuthenticatedServiceQueuePollWithProgressV1, AuthenticatedServiceQueueSessionV1,
    AuthenticatedServiceQueueSubmitFailureV1, AuthenticatedServiceQueueUnboundSessionV1,
    AuthenticatedServiceRecycledQueueSessionV1, AuthenticatedWorkerV3ProgramMaterializationErrorV1,
};
use fe2o3_kfd::{ComputeAqlQueueObservationV1, Gfx942TimeoutExecutionObservationV1};
use fe2o3_service_host::ServiceQueueErrorV1;

use ferric_spec::completion::CompletionEpoch;
use ferric_spec::Identity;

use crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1;
use crate::physical_fixed_batch::{
    M1AuthenticatedPhysicalPacketBatchCaseV1, M1AuthenticatedPhysicalPacketBatchV1,
};
use crate::physical_queue_lifecycle::{
    wait_with_completion_progress_deadline_policy, wait_with_completion_progress_policy,
    CompletionProgressPollV1, CompletionProgressWaitFailureV1,
};
use crate::{
    DeclaredOperationKernelPlan, Engine, Gfx942DeviceBinding, M1AuthenticatedPhysicalRunnerV1,
    M1AuthenticatedPrepublicationBatchV1, M1CompletionProgressObservationV1,
    M1CompletionProgressWaitDiagnosticV1, M1PhysicalFixedBatchCustodyV1,
    M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1, M1PrepublicationStepCustodyV1,
    M1ScheduledDispatchV1, M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
    M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Nonzero wall-clock budget for one production authenticated queue wait.
///
/// The field is private so production entry points cannot silently request the
/// immediate zero-deadline terminalizer reserved for queue-custody closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1QueueWaitTimeoutV1(NonZeroU32);

impl M1QueueWaitTimeoutV1 {
    /// Constructs a production queue wait budget in milliseconds.
    #[must_use]
    pub const fn new(milliseconds: u32) -> Option<Self> {
        match NonZeroU32::new(milliseconds) {
            Some(milliseconds) => Some(Self(milliseconds)),
            None => None,
        }
    }

    /// Returns the exact millisecond budget passed to the bounded wait policy.
    #[must_use]
    pub const fn milliseconds(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug)]
enum M1AuthenticatedPhysicalQueuePhaseV1<const N: usize> {
    Prepared(AuthenticatedServiceQueueSessionV1<N>),
    Published(AuthenticatedServicePublishedQueueSessionV1<N>),
    Completed(AuthenticatedServiceCompletedQueueSessionV1<N>),
    Recycled(AuthenticatedServiceRecycledQueueSessionV1<N>),
    Transitioning,
}

/// One persistent authenticated phase allocation paired with all Ferric authority.
///
/// Successful queue transitions replace only the private lower typestate. The
/// allocation itself remains stable for the complete resident queue lifetime.
#[must_use = "authenticated lower queue and Ferric custody must remain paired"]
pub struct M1AuthenticatedPhysicalQueuePhaseOwnerV1<const N: usize> {
    lower: M1AuthenticatedPhysicalQueuePhaseV1<N>,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl<const N: usize> M1AuthenticatedPhysicalQueuePhaseOwnerV1<N> {
    const fn new(
        lower: AuthenticatedServiceQueueSessionV1<N>,
        witness: M1AuthenticatedProgramCatalogWitnessV1,
        operations: DeclaredOperationKernelPlan,
        custody: M1PhysicalQueueBatchCustodyV1,
        step: M1PrepublicationStepCustodyV1,
    ) -> Self {
        Self {
            lower: M1AuthenticatedPhysicalQueuePhaseV1::Prepared(lower),
            witness,
            operations,
            custody,
            step,
        }
    }

    pub(crate) const fn from_queue_rearm(
        lower: AuthenticatedServiceQueueSessionV1<N>,
        witness: M1AuthenticatedProgramCatalogWitnessV1,
        operations: DeclaredOperationKernelPlan,
        custody: M1PhysicalQueueBatchCustodyV1,
        step: M1PrepublicationStepCustodyV1,
    ) -> Self {
        Self::new(lower, witness, operations, custody, step)
    }

    /// Checked physical-device receipt retained beside the lower queue.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device()
    }

    /// Exact authenticated program-catalog identity.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.witness.catalog_id()
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.operations.runner_declaration_id()
    }

    /// Exact structural kernel-catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.operations.kernel_catalog_id()
    }

    /// Immutable scheduler-issued queue epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Exact scheduler dispatch retained by this phase.
    #[must_use = "scheduler authority remains retained by the queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric queue custody remains retained"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    pub(crate) const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }

    pub(crate) fn recycled_observation_parts(
        &mut self,
    ) -> (
        &mut AuthenticatedServiceRecycledQueueSessionV1<N>,
        &M1PhysicalQueueBatchCustodyV1,
        &M1PrepublicationStepCustodyV1,
    ) {
        let M1AuthenticatedPhysicalQueuePhaseV1::Recycled(lower) = &mut self.lower else {
            unreachable!("recycled owner carries the recycled lower phase")
        };
        (lower, &self.custody, &self.step)
    }

    fn take_prepared(&mut self) -> AuthenticatedServiceQueueSessionV1<N> {
        let M1AuthenticatedPhysicalQueuePhaseV1::Prepared(lower) = core::mem::replace(
            &mut self.lower,
            M1AuthenticatedPhysicalQueuePhaseV1::Transitioning,
        ) else {
            unreachable!("prepared owner carries the prepared lower phase")
        };
        lower
    }

    fn take_published(&mut self) -> AuthenticatedServicePublishedQueueSessionV1<N> {
        let M1AuthenticatedPhysicalQueuePhaseV1::Published(lower) = core::mem::replace(
            &mut self.lower,
            M1AuthenticatedPhysicalQueuePhaseV1::Transitioning,
        ) else {
            unreachable!("published owner carries the published lower phase")
        };
        lower
    }

    fn take_completed(&mut self) -> AuthenticatedServiceCompletedQueueSessionV1<N> {
        let M1AuthenticatedPhysicalQueuePhaseV1::Completed(lower) = core::mem::replace(
            &mut self.lower,
            M1AuthenticatedPhysicalQueuePhaseV1::Transitioning,
        ) else {
            unreachable!("completed owner carries the completed lower phase")
        };
        lower
    }

    fn take_recycled(&mut self) -> AuthenticatedServiceRecycledQueueSessionV1<N> {
        let M1AuthenticatedPhysicalQueuePhaseV1::Recycled(lower) = core::mem::replace(
            &mut self.lower,
            M1AuthenticatedPhysicalQueuePhaseV1::Transitioning,
        ) else {
            unreachable!("recycled owner carries the recycled lower phase")
        };
        lower
    }

    fn set_prepared(&mut self, lower: AuthenticatedServiceQueueSessionV1<N>) {
        self.lower = M1AuthenticatedPhysicalQueuePhaseV1::Prepared(lower);
    }

    fn set_published(&mut self, lower: AuthenticatedServicePublishedQueueSessionV1<N>) {
        self.lower = M1AuthenticatedPhysicalQueuePhaseV1::Published(lower);
    }

    fn set_completed(&mut self, lower: AuthenticatedServiceCompletedQueueSessionV1<N>) {
        self.lower = M1AuthenticatedPhysicalQueuePhaseV1::Completed(lower);
    }

    fn set_recycled(&mut self, lower: AuthenticatedServiceRecycledQueueSessionV1<N>) {
        self.lower = M1AuthenticatedPhysicalQueuePhaseV1::Recycled(lower);
    }

    fn into_prepared_parts(
        self,
    ) -> (
        AuthenticatedServiceQueueSessionV1<N>,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        let M1AuthenticatedPhysicalQueuePhaseV1::Prepared(lower) = self.lower else {
            unreachable!("prepared owner carries the prepared lower phase")
        };
        (
            lower,
            self.witness,
            self.operations,
            self.custody,
            self.step,
        )
    }

    pub(crate) fn into_recycled_parts(
        self,
    ) -> (
        AuthenticatedServiceRecycledQueueSessionV1<N>,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        let M1AuthenticatedPhysicalQueuePhaseV1::Recycled(lower) = self.lower else {
            unreachable!("recycled owner carries the recycled lower phase")
        };
        (
            lower,
            self.witness,
            self.operations,
            self.custody,
            self.step,
        )
    }

    fn into_custody_parts(
        self,
    ) -> (
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.witness, self.operations, self.custody, self.step)
    }
}

impl<const N: usize> core::fmt::Debug for M1AuthenticatedPhysicalQueuePhaseOwnerV1<N> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalQueuePhaseCaseV1")
            .field("lower", &self.lower)
            .field("witness", &self.witness)
            .field("operations", &self.operations)
            .field("custody", &self.custody)
            .field("step", &self.step)
            .finish()
    }
}

/// Stable heap slot for one authenticated queue owner.
///
/// Empty slots carry no queue authority and are allocated only by trusted host
/// preparation before a resident timing interval begins.
pub struct M1AuthenticatedPhysicalQueuePhaseSlotV1<const N: usize> {
    owner: Option<M1AuthenticatedPhysicalQueuePhaseOwnerV1<N>>,
}

impl<const N: usize> M1AuthenticatedPhysicalQueuePhaseSlotV1<N> {
    pub(crate) const fn empty() -> Self {
        Self { owner: None }
    }

    pub(crate) const fn occupied(owner: M1AuthenticatedPhysicalQueuePhaseOwnerV1<N>) -> Self {
        Self { owner: Some(owner) }
    }

    pub(crate) fn install_prepared(
        &mut self,
        lower: AuthenticatedServiceQueueSessionV1<N>,
        witness: M1AuthenticatedProgramCatalogWitnessV1,
        operations: DeclaredOperationKernelPlan,
        custody: M1PhysicalQueueBatchCustodyV1,
        step: M1PrepublicationStepCustodyV1,
    ) {
        assert!(
            self.owner.is_none(),
            "phase storage is installed exactly once"
        );
        self.owner = Some(M1AuthenticatedPhysicalQueuePhaseOwnerV1::from_queue_rearm(
            lower, witness, operations, custody, step,
        ));
    }

    pub(crate) fn install_prepared_from_owner(
        &mut self,
        owner: M1AuthenticatedPhysicalQueuePhaseOwnerV1<N>,
    ) {
        assert!(
            self.owner.is_none(),
            "phase storage is installed exactly once"
        );
        self.owner = Some(owner);
    }

    const fn owner(&self) -> &M1AuthenticatedPhysicalQueuePhaseOwnerV1<N> {
        match &self.owner {
            Some(owner) => owner,
            None => panic!("live phase slot is occupied"),
        }
    }

    fn owner_mut(&mut self) -> &mut M1AuthenticatedPhysicalQueuePhaseOwnerV1<N> {
        self.owner.as_mut().expect("live phase slot is occupied")
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.owner().device()
    }

    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.owner().program_catalog_id()
    }

    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.owner().runner_declaration_id()
    }

    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.owner().kernel_catalog_id()
    }

    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.owner().queue_epoch()
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.owner().scheduled_dispatch()
    }

    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        self.owner().custody()
    }

    pub(crate) const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        self.owner().step()
    }

    pub(crate) fn recycled_observation_parts(
        &mut self,
    ) -> (
        &mut AuthenticatedServiceRecycledQueueSessionV1<N>,
        &M1PhysicalQueueBatchCustodyV1,
        &M1PrepublicationStepCustodyV1,
    ) {
        self.owner_mut().recycled_observation_parts()
    }

    fn into_prepared_parts(
        mut self,
    ) -> (
        AuthenticatedServiceQueueSessionV1<N>,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        self.owner
            .take()
            .expect("prepared phase slot is occupied")
            .into_prepared_parts()
    }

    pub(crate) fn into_recycled_parts(
        mut self,
    ) -> (
        AuthenticatedServiceRecycledQueueSessionV1<N>,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        self.owner
            .take()
            .expect("recycled phase slot is occupied")
            .into_recycled_parts()
    }

    fn into_custody_parts(
        mut self,
    ) -> (
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        self.owner
            .take()
            .expect("transitioning phase slot is occupied")
            .into_custody_parts()
    }
}

impl<const N: usize> core::ops::Deref for M1AuthenticatedPhysicalQueuePhaseSlotV1<N> {
    type Target = M1AuthenticatedPhysicalQueuePhaseOwnerV1<N>;

    fn deref(&self) -> &Self::Target {
        self.owner.as_ref().expect("live phase slot is occupied")
    }
}

impl<const N: usize> core::ops::DerefMut for M1AuthenticatedPhysicalQueuePhaseSlotV1<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.owner.as_mut().expect("live phase slot is occupied")
    }
}

impl<const N: usize> core::fmt::Debug for M1AuthenticatedPhysicalQueuePhaseSlotV1<N> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalQueuePhaseSlotV1")
            .field("owner", &self.owner)
            .finish()
    }
}

#[must_use = "prepared authenticated queue custody must be submitted or retained"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalQueueSessionV1 {
    /// One complete target-only queue generation.
    TargetOnly(Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// One complete paired-prefill queue generation.
    PairedPrefill(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K4 speculative queue generation.
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K8 speculative queue generation.
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K16 speculative queue generation.
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalQueueSessionV1 {
    /// Exact closed M1 publication shape.
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

    /// Exact compile-time packet cardinality.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Exact scheduler authority retained by this queue phase.
    #[must_use = "scheduler authority remains retained by the queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Checked physical-device receipt retained by this queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.device(),
            Self::PairedPrefill(case) => case.device(),
            Self::SpeculativeK4(case) => case.device(),
            Self::SpeculativeK8(case) => case.device(),
            Self::SpeculativeK16(case) => case.device(),
        }
    }

    /// Consumes an unpublished queue without exposing it to a higher-level
    /// failure API.
    pub(crate) fn close_unpublished(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
        match self {
            Self::TargetOnly(case) => close_unpublished_case(case),
            Self::PairedPrefill(case) => close_unpublished_case(case),
            Self::SpeculativeK4(case) => close_unpublished_case(case),
            Self::SpeculativeK8(case) => close_unpublished_case(case),
            Self::SpeculativeK16(case) => close_unpublished_case(case),
        }
    }
}

#[derive(Debug)]
pub(crate) enum M1AuthenticatedPhysicalQueueClosureV1 {
    Released(Box<dyn core::fmt::Debug>),
    Quarantined(Box<dyn core::fmt::Debug>),
}

pub(crate) fn retain_in_queue_closure(
    closure: M1AuthenticatedPhysicalQueueClosureV1,
    retained: impl core::fmt::Debug + 'static,
) -> M1AuthenticatedPhysicalQueueClosureV1 {
    match closure {
        M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
            M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new((released, retained)))
        }
        M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new((quarantined, retained)))
        }
    }
}

trait M1AuthenticatedUnsubmittedQueueCloseEffectV1: core::fmt::Debug {
    fn close(self) -> M1AuthenticatedPhysicalQueueClosureV1;
}

impl M1AuthenticatedUnsubmittedQueueCloseEffectV1 for M1AuthenticatedPhysicalQueueSessionV1 {
    fn close(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
        self.close_unpublished()
    }
}

fn close_without_authority_core<const C: usize, Q, L>(
    engine: &mut Engine<C>,
    queue: Q,
    logical: L,
) -> M1AuthenticatedPhysicalQueueClosureV1
where
    Q: M1AuthenticatedUnsubmittedQueueCloseEffectV1,
    L: core::fmt::Debug + 'static,
{
    engine.quarantine_m1_queue_rearm_failure();
    match queue.close() {
        M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
            M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new((released, logical)))
        }
        M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new((quarantined, logical)))
        }
    }
}

fn quarantine_without_authority_core<const C: usize>(
    engine: &mut Engine<C>,
    quarantined: Box<dyn core::fmt::Debug>,
) -> M1AuthenticatedPhysicalQueueClosureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined)
}

fn close_unpublished_case<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
) -> M1AuthenticatedPhysicalQueueClosureV1 {
    let (lower, witness, operations, custody, step) = (*case).into_prepared_parts();
    match lower.destroy_and_release() {
        Ok(released) => M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new((
            released, witness, operations, custody, step,
        ))),
        Err(quarantined) => M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new((
            quarantined,
            witness,
            operations,
            custody,
            step,
        ))),
    }
}

#[must_use = "published authenticated queue custody must be completed"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalPublishedQueueSessionV1 {
    /// One complete target-only queue generation.
    TargetOnly(Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// One complete paired-prefill queue generation.
    PairedPrefill(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K4 speculative queue generation.
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K8 speculative queue generation.
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K16 speculative queue generation.
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalPublishedQueueSessionV1 {
    /// Attempts terminalization of an in-flight queue without an unbounded wait.
    pub(crate) fn close_in_flight(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
        match self.wait_for(0) {
            Ok(completed) => completed.close_completed(),
            Err(quarantined) => M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined),
        }
    }

    /// Exact closed M1 publication shape.
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

    /// Exact compile-time packet cardinality.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Exact scheduler authority retained by this queue phase.
    #[must_use = "scheduler authority remains retained by the queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Checked physical-device receipt retained by this queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.device(),
            Self::PairedPrefill(case) => case.device(),
            Self::SpeculativeK4(case) => case.device(),
            Self::SpeculativeK8(case) => case.device(),
            Self::SpeculativeK16(case) => case.device(),
        }
    }
}

#[must_use = "completed authenticated queue custody must be recycled"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalCompletedQueueSessionV1 {
    /// One complete target-only queue generation.
    TargetOnly(Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// One complete paired-prefill queue generation.
    PairedPrefill(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K4 speculative queue generation.
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K8 speculative queue generation.
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K16 speculative queue generation.
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalCompletedQueueSessionV1 {
    /// Recycles completion signals and destroys the resulting quiescent queue.
    pub(crate) fn close_completed(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
        match self.recycle() {
            Ok(recycled) => recycled.close_recycled(),
            Err(quarantined) => M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined),
        }
    }

    /// Exact closed M1 publication shape.
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

    /// Exact compile-time packet cardinality.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Exact scheduler authority retained by this queue phase.
    #[must_use = "scheduler authority remains retained by the queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Checked physical-device receipt retained by this queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.device(),
            Self::PairedPrefill(case) => case.device(),
            Self::SpeculativeK4(case) => case.device(),
            Self::SpeculativeK8(case) => case.device(),
            Self::SpeculativeK16(case) => case.device(),
        }
    }
}

#[must_use = "recycled authenticated queue custody must be reused, detached, or released"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalRecycledQueueSessionV1 {
    /// One complete target-only queue generation.
    TargetOnly(Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// One complete paired-prefill queue generation.
    PairedPrefill(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K4 speculative queue generation.
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K8 speculative queue generation.
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    /// One complete K16 speculative queue generation.
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalRecycledQueueSessionV1 {
    /// Destroys the exact quiescent queue and retains all Ferric custody.
    pub(crate) fn close_recycled(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
        match self {
            Self::TargetOnly(case) => close_recycled_case(case),
            Self::PairedPrefill(case) => close_recycled_case(case),
            Self::SpeculativeK4(case) => close_recycled_case(case),
            Self::SpeculativeK8(case) => close_recycled_case(case),
            Self::SpeculativeK16(case) => close_recycled_case(case),
        }
    }

    /// Exact closed M1 publication shape.
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

    /// Exact compile-time packet cardinality.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Exact scheduler authority retained by this queue phase.
    #[must_use = "scheduler authority remains retained by the queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Checked physical-device receipt retained by this queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.device(),
            Self::PairedPrefill(case) => case.device(),
            Self::SpeculativeK4(case) => case.device(),
            Self::SpeculativeK8(case) => case.device(),
            Self::SpeculativeK16(case) => case.device(),
        }
    }
}

fn close_recycled_case<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
) -> M1AuthenticatedPhysicalQueueClosureV1 {
    let (lower, witness, operations, custody, step) = (*case).into_recycled_parts();
    match lower.destroy_and_release() {
        Ok(released) => M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new((
            released, witness, operations, custody, step,
        ))),
        Err(quarantined) => M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new((
            quarantined,
            witness,
            operations,
            custody,
            step,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        close_without_authority_core, M1AuthenticatedPhysicalCompletedQueueSessionV1,
        M1AuthenticatedPhysicalPublishedQueueSessionV1, M1AuthenticatedPhysicalQueueClosureV1,
        M1AuthenticatedPhysicalQueueOperationFailureV1,
        M1AuthenticatedUnsubmittedQueueCloseEffectV1, M1QueueWaitTimeoutV1,
    };
    use crate::authenticated_test_runtime::{ModelPreparedQueueV1, ModelQueueV1};
    use crate::Engine;
    use fe2o3_kfd::Gfx942TimeoutExecutionObservationV1;

    #[derive(Debug)]
    struct ModelUnsubmittedQueueV1 {
        prepared: ModelPreparedQueueV1,
        clean: bool,
    }

    impl M1AuthenticatedUnsubmittedQueueCloseEffectV1 for ModelUnsubmittedQueueV1 {
        fn close(self) -> M1AuthenticatedPhysicalQueueClosureV1 {
            self.prepared.destroy(self.clean);
            if self.clean {
                M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new("destroyed"))
            } else {
                M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new("quarantined"))
            }
        }
    }

    #[test]
    fn currentness_closure_executes_one_unpublished_destroy_and_faults_engine() {
        for clean in [true, false] {
            let queue = ModelQueueV1::new([], []);
            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let closure = close_without_authority_core(
                &mut engine,
                ModelUnsubmittedQueueV1 {
                    prepared: ModelPreparedQueueV1::new(queue.clone()),
                    clean,
                },
                "currentness failure",
            );
            assert!(engine.is_faulted());
            assert_eq!(queue.snapshot().submits, 0);
            assert_eq!(queue.snapshot().destroys, 1);
            assert_eq!(
                matches!(closure, M1AuthenticatedPhysicalQueueClosureV1::Released(_)),
                clean
            );
        }
    }

    #[test]
    fn bounded_wait_surface_consumes_publication_and_borrows_timeout_observation() {
        type WaitFor = fn(
            M1AuthenticatedPhysicalPublishedQueueSessionV1,
            u32,
        ) -> Result<
            M1AuthenticatedPhysicalCompletedQueueSessionV1,
            Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
        >;
        type TimeoutObservation = for<'a> fn(
            &'a M1AuthenticatedPhysicalQueueOperationFailureV1,
        )
            -> Option<&'a Gfx942TimeoutExecutionObservationV1>;

        let _: WaitFor = M1AuthenticatedPhysicalPublishedQueueSessionV1::wait_for;
        let _: TimeoutObservation =
            M1AuthenticatedPhysicalQueueOperationFailureV1::timeout_observation;
    }

    #[test]
    fn production_queue_wait_timeout_rejects_zero_and_preserves_nonzero_milliseconds() {
        assert_eq!(M1QueueWaitTimeoutV1::new(0), None);
        let timeout = M1QueueWaitTimeoutV1::new(17).expect("nonzero timeout is admitted");
        assert_eq!(timeout.milliseconds(), 17);
    }
}

/// Live detached queue retaining authenticated program history and prior-step custody.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPhysicalDetachedQueueSessionV1;
/// fn split(detached: M1AuthenticatedPhysicalDetachedQueueSessionV1) {
///     let _ = detached.into_parts();
/// }
/// ```
#[must_use = "the live detached authenticated queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedPhysicalDetachedQueueSessionV1 {
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    former_shape: M1PhysicalFixedBatchShapeV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
    prior_step: M1PrepublicationStepCustodyV1,
}

impl M1AuthenticatedPhysicalDetachedQueueSessionV1 {
    /// Exact closed shape of the completed generation that was detached.
    #[must_use]
    pub const fn former_shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.former_shape
    }

    /// Checked physical-device receipt retained beside the live queue.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device()
    }

    /// Exact authenticated program-catalog identity retained as history.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.witness.catalog_id()
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.operations.runner_declaration_id()
    }

    /// Exact structural kernel-catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.operations.kernel_catalog_id()
    }

    /// Immutable scheduler-issued queue epoch retained from the prior step.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.prior_step.scheduled_dispatch().epoch()
    }

    /// Redacted observation of the still-live native queue.
    #[must_use]
    pub const fn observation(&self) -> ComputeAqlQueueObservationV1 {
        self.lower.observation()
    }

    /// Completed lower dispatch generation that authorized detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        self.lower.detached_dispatch_generation()
    }

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric queue custody remains retained"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

/// Pure reason why authenticated queue creation returned unchanged inputs.
#[must_use]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalQueueCreateDiagnosticV1 {
    /// Retained identities did not remain exactly joined.
    Identity,
    /// Authenticated program materialization rejected before KFD mutation.
    Program(Box<AuthenticatedWorkerV3ProgramMaterializationErrorV1>),
    /// Structural queue validation rejected before KFD mutation.
    Queue(Box<ServiceQueueErrorV1>),
}

/// Terminal queue-creation custody after native allocation transfer began.
#[must_use = "terminal queue creation custody denies retry and must remain classified"]
#[derive(Debug)]
pub struct M1AuthenticatedPhysicalQueueCreateTerminalV1 {
    error: Box<ServiceQueueErrorV1>,
    runner: Box<M1AuthenticatedPhysicalRunnerV1>,
    shape: M1PhysicalFixedBatchShapeV1,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
    step: Box<M1PrepublicationStepCustodyV1>,
}

impl M1AuthenticatedPhysicalQueueCreateTerminalV1 {
    /// Exact lower terminal error.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        &self.error
    }

    /// Exact closed batch shape whose native creation became terminal.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Authenticated runner custody retained after terminal creation.
    #[must_use = "authenticated runner custody remains retained"]
    pub const fn runner(&self) -> &M1AuthenticatedPhysicalRunnerV1 {
        &self.runner
    }

    /// Post-split Ferric allocation and model-memory custody.
    #[must_use = "post-split Ferric custody remains retained"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Scheduler and KV authority retained after terminal creation.
    #[must_use = "scheduler and KV authority remain retained"]
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }
}

/// Authenticated queue-creation rejection or terminal custody.
#[must_use = "queue creation failure retains all available authenticated custody"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalQueueCreateFailureV1 {
    /// Pure rejection with the exact unchanged prepublication owner.
    Rejected {
        /// Stable rejection diagnostic.
        diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1,
        /// Exact unchanged retry owner.
        prepublication: Box<M1AuthenticatedPrepublicationBatchV1>,
    },
    /// Native queue creation began and retry is forbidden.
    Terminal(Box<M1AuthenticatedPhysicalQueueCreateTerminalV1>),
}

/// Authenticated queue-creation failure after the paired scheduler Engine was faulted.
#[must_use = "Engine-quarantined authenticated creation custody must remain retained"]
#[derive(Debug)]
pub struct M1EngineQuarantinedAuthenticatedPhysicalQueueCreateFailureV1 {
    failure: Box<M1AuthenticatedPhysicalQueueCreateFailureV1>,
}

impl M1EngineQuarantinedAuthenticatedPhysicalQueueCreateFailureV1 {
    /// Exact rejected or terminal authenticated creation owner.
    #[must_use = "authenticated creation failure custody remains retained"]
    pub const fn failure(&self) -> &M1AuthenticatedPhysicalQueueCreateFailureV1 {
        &self.failure
    }
}

impl M1AuthenticatedPhysicalQueueCreateFailureV1 {
    /// Permanently faults the paired scheduler and retains this exact failure.
    #[must_use = "Engine-quarantined authenticated creation custody must remain retained"]
    pub fn quarantine_engine<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1EngineQuarantinedAuthenticatedPhysicalQueueCreateFailureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        M1EngineQuarantinedAuthenticatedPhysicalQueueCreateFailureV1 {
            failure: Box::new(self),
        }
    }
}

enum CreateCaseResultV1<const N: usize> {
    Ready(Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>),
    Rejected {
        diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1,
        runner: Box<M1AuthenticatedPhysicalRunnerV1>,
        batch: Box<M1AuthenticatedPhysicalPacketBatchCaseV1<N>>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
    Terminal(M1AuthenticatedPhysicalQueueCreateTerminalV1),
}

struct CreateCaseInputV1<const N: usize>(Box<M1AuthenticatedPhysicalPacketBatchCaseV1<N>>);

#[inline(never)]
fn create_case<const N: usize>(
    ring_bytes: u32,
    runner: M1AuthenticatedPhysicalRunnerV1,
    batch: CreateCaseInputV1<N>,
    step: M1PrepublicationStepCustodyV1,
    shape: M1PhysicalFixedBatchShapeV1,
) -> CreateCaseResultV1<N> {
    let (programs, operations) = runner.into_parts();
    let (programs, witness) = programs.into_queue_parts();
    let (packets, custody) = (*batch.0).into_parts();
    let exact_identity = witness.catalog_id() == custody.catalog_id()
        && witness.family_artifacts() == operations.families()
        && packets
            .iter()
            .zip(custody.physical_recipe().rows())
            .all(|(packet, row)| {
                packet.program_index() == witness.service_program_index(row.program())
            });
    if !exact_identity {
        return CreateCaseResultV1::Rejected {
            diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1::Identity,
            runner: Box::new(M1AuthenticatedPhysicalRunnerV1::from_parts(
                crate::M1AuthenticatedWorkerV3ProgramSetV1::from_queue_parts(programs, witness),
                operations,
            )),
            batch: Box::new(M1AuthenticatedPhysicalPacketBatchCaseV1::from_parts(
                packets, custody,
            )),
            step: Box::new(step),
        };
    }
    let (allocations, queue_custody) = custody.into_queue_creation_parts();
    match AuthenticatedServiceQueueSessionV1::create(programs, allocations, ring_bytes, packets) {
        Ok(lower) => {
            CreateCaseResultV1::Ready(Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::occupied(
                M1AuthenticatedPhysicalQueuePhaseOwnerV1::new(
                    lower,
                    witness,
                    operations,
                    queue_custody,
                    step,
                ),
            )))
        }
        Err(AuthenticatedServiceQueueCreateFailureV1::Program {
            error,
            programs,
            allocations,
            packets,
        }) => CreateCaseResultV1::Rejected {
            diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1::Program(error),
            runner: Box::new(M1AuthenticatedPhysicalRunnerV1::from_parts(
                crate::M1AuthenticatedWorkerV3ProgramSetV1::from_queue_parts(programs, witness),
                operations,
            )),
            batch: Box::new(M1AuthenticatedPhysicalPacketBatchCaseV1::from_parts(
                *packets,
                M1PhysicalFixedBatchCustodyV1::from_rejected_queue_creation(
                    *allocations,
                    queue_custody,
                ),
            )),
            step: Box::new(step),
        },
        Err(AuthenticatedServiceQueueCreateFailureV1::QueueRejected {
            error,
            programs,
            allocations,
            packets,
        }) => CreateCaseResultV1::Rejected {
            diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1::Queue(error),
            runner: Box::new(M1AuthenticatedPhysicalRunnerV1::from_parts(
                crate::M1AuthenticatedWorkerV3ProgramSetV1::from_queue_parts(programs, witness),
                operations,
            )),
            batch: Box::new(M1AuthenticatedPhysicalPacketBatchCaseV1::from_parts(
                *packets,
                M1PhysicalFixedBatchCustodyV1::from_rejected_queue_creation(
                    *allocations,
                    queue_custody,
                ),
            )),
            step: Box::new(step),
        },
        Err(AuthenticatedServiceQueueCreateFailureV1::QueueTerminal { error, programs }) => {
            CreateCaseResultV1::Terminal(M1AuthenticatedPhysicalQueueCreateTerminalV1 {
                error,
                runner: Box::new(M1AuthenticatedPhysicalRunnerV1::from_parts(
                    crate::M1AuthenticatedWorkerV3ProgramSetV1::from_queue_parts(programs, witness),
                    operations,
                )),
                shape,
                custody: Box::new(queue_custody),
                step: Box::new(step),
            })
        }
    }
}

fn finish_create<const N: usize>(
    result: CreateCaseResultV1<N>,
    queue_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
    batch_variant: fn(
        Box<M1AuthenticatedPhysicalPacketBatchCaseV1<N>>,
    ) -> M1AuthenticatedPhysicalPacketBatchV1,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalQueueCreateFailureV1> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(queue_variant(case)),
        CreateCaseResultV1::Rejected {
            diagnostic,
            runner,
            batch,
            step,
        } => Err(M1AuthenticatedPhysicalQueueCreateFailureV1::Rejected {
            diagnostic,
            prepublication: Box::new(M1AuthenticatedPrepublicationBatchV1 {
                runner: *runner,
                batch: batch_variant(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal(terminal) => Err(
            M1AuthenticatedPhysicalQueueCreateFailureV1::Terminal(Box::new(terminal)),
        ),
    }
}

impl M1AuthenticatedPhysicalQueueSessionV1 {
    /// Creates a queue only from the opaque authenticated prepublication owner.
    ///
    /// # Errors
    ///
    /// Returns exact unchanged prepublication custody on pure rejection or
    /// terminal post-split custody after native queue creation begins.
    pub fn create(
        ring_bytes: u32,
        prepublication: M1AuthenticatedPrepublicationBatchV1,
    ) -> Result<Self, M1AuthenticatedPhysicalQueueCreateFailureV1> {
        let exact_identity = prepublication.program_catalog_id()
            == prepublication.batch.custody().catalog_id()
            && prepublication.runner_declaration_id()
                == prepublication
                    .batch
                    .custody()
                    .workspace_composition()
                    .dispatch_plan()
                    .runner_declaration_id()
            && prepublication.kernel_catalog_id()
                == prepublication
                    .batch
                    .custody()
                    .workspace_composition()
                    .dispatch_plan()
                    .kernel_catalog_id();
        if !exact_identity {
            return Err(M1AuthenticatedPhysicalQueueCreateFailureV1::Rejected {
                diagnostic: M1AuthenticatedPhysicalQueueCreateDiagnosticV1::Identity,
                prepublication: Box::new(prepublication),
            });
        }
        let M1AuthenticatedPrepublicationBatchV1 {
            runner,
            batch,
            step,
        } = prepublication;
        match batch {
            M1AuthenticatedPhysicalPacketBatchV1::TargetOnly(batch) => finish_create(
                create_case(
                    ring_bytes,
                    runner,
                    CreateCaseInputV1(batch),
                    step,
                    M1PhysicalFixedBatchShapeV1::TargetOnly,
                ),
                M1AuthenticatedPhysicalQueueSessionV1::TargetOnly,
                M1AuthenticatedPhysicalPacketBatchV1::TargetOnly,
            ),
            M1AuthenticatedPhysicalPacketBatchV1::PairedPrefill(batch) => finish_create(
                create_case(
                    ring_bytes,
                    runner,
                    CreateCaseInputV1(batch),
                    step,
                    M1PhysicalFixedBatchShapeV1::PairedPrefill,
                ),
                M1AuthenticatedPhysicalQueueSessionV1::PairedPrefill,
                M1AuthenticatedPhysicalPacketBatchV1::PairedPrefill,
            ),
            M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK4(batch) => finish_create(
                create_case(
                    ring_bytes,
                    runner,
                    CreateCaseInputV1(batch),
                    step,
                    M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                ),
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4,
                M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK4,
            ),
            M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK8(batch) => finish_create(
                create_case(
                    ring_bytes,
                    runner,
                    CreateCaseInputV1(batch),
                    step,
                    M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                ),
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8,
                M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK8,
            ),
            M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK16(batch) => finish_create(
                create_case(
                    ring_bytes,
                    runner,
                    CreateCaseInputV1(batch),
                    step,
                    M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                ),
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16,
                M1AuthenticatedPhysicalPacketBatchV1::SpeculativeK16,
            ),
        }
    }
}

/// Effectful lower failure retaining every authenticated and Ferric owner.
#[must_use = "effectful queue failure retains quarantine custody"]
pub struct M1AuthenticatedPhysicalQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: AuthenticatedServiceQueueOperationFailureV1,
    witness: Box<M1AuthenticatedProgramCatalogWitnessV1>,
    operations: Box<DeclaredOperationKernelPlan>,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
    step: Box<M1PrepublicationStepCustodyV1>,
    completion_progress_wait: Option<Box<M1CompletionProgressWaitDiagnosticV1>>,
}

/// Authenticated queue operation failure after the paired scheduler Engine was faulted.
#[must_use = "Engine-quarantined authenticated operation custody must remain retained"]
#[derive(Debug)]
pub struct M1EngineQuarantinedAuthenticatedPhysicalQueueOperationFailureV1 {
    failure: Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
}

impl M1EngineQuarantinedAuthenticatedPhysicalQueueOperationFailureV1 {
    /// Exact terminal authenticated queue operation failure.
    #[must_use = "authenticated operation failure custody remains retained"]
    pub const fn failure(&self) -> &M1AuthenticatedPhysicalQueueOperationFailureV1 {
        &self.failure
    }
}

impl M1AuthenticatedPhysicalQueueOperationFailureV1 {
    /// Permanently faults the paired scheduler and retains this exact terminal failure.
    #[must_use = "Engine-quarantined authenticated operation custody must remain retained"]
    pub fn quarantine_engine<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1EngineQuarantinedAuthenticatedPhysicalQueueOperationFailureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        M1EngineQuarantinedAuthenticatedPhysicalQueueOperationFailureV1 {
            failure: Box::new(self),
        }
    }

    /// Exact lower queue error.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        self.lower.error()
    }

    /// Exact closed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Optional Ferric liveness-policy diagnostic.
    #[must_use]
    pub fn completion_progress_wait(&self) -> Option<&M1CompletionProgressWaitDiagnosticV1> {
        self.completion_progress_wait.as_deref()
    }

    /// Addressless terminal state captured by an upstream deadline timeout.
    #[must_use]
    pub fn timeout_observation(&self) -> Option<&Gfx942TimeoutExecutionObservationV1> {
        self.lower.timeout_observation()
    }

    /// Exact Ferric allocation and model-memory custody.
    #[must_use = "Ferric queue custody remains retained"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Exact scheduler and KV authority.
    #[must_use = "scheduler authority remains retained"]
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }
}

impl core::fmt::Debug for M1AuthenticatedPhysicalQueueOperationFailureV1 {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalQueueOperationFailureV1")
            .field("shape", &self.shape)
            .field("lower", &self.lower)
            .field("witness", &self.witness)
            .field("operations", &self.operations)
            .field("custody", &self.custody)
            .field("step", &self.step)
            .field("completion_progress_wait", &self.completion_progress_wait)
            .finish()
    }
}

fn operation_failure(
    shape: M1PhysicalFixedBatchShapeV1,
    lower: AuthenticatedServiceQueueOperationFailureV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    completion_progress_wait: Option<M1CompletionProgressWaitDiagnosticV1>,
) -> Box<M1AuthenticatedPhysicalQueueOperationFailureV1> {
    Box::new(M1AuthenticatedPhysicalQueueOperationFailureV1 {
        shape,
        lower,
        witness: Box::new(witness),
        operations: Box::new(operations),
        custody: Box::new(custody),
        step: Box::new(step),
        completion_progress_wait: completion_progress_wait.map(Box::new),
    })
}

/// Publication rejection retaining either retryable or quarantined custody.
#[must_use = "publication failure retains authenticated queue custody"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalQueueSubmitFailureV1 {
    /// Currentness rejected before publication and the prepared owner is unchanged.
    Currentness {
        /// Exact currentness/materialization error.
        error: Box<AuthenticatedWorkerV3ProgramMaterializationErrorV1>,
        /// Exact unchanged prepared owner.
        retained: Box<M1AuthenticatedPhysicalQueueSessionV1>,
    },
    /// The lower publication transition became quarantined.
    Queue(Box<M1AuthenticatedPhysicalQueueOperationFailureV1>),
}

/// Authenticated submit failure after the paired scheduler Engine was faulted.
#[must_use = "Engine-quarantined authenticated submit custody must remain retained"]
#[derive(Debug)]
pub struct M1EngineQuarantinedAuthenticatedPhysicalQueueSubmitFailureV1 {
    failure: Box<M1AuthenticatedPhysicalQueueSubmitFailureV1>,
}

impl M1EngineQuarantinedAuthenticatedPhysicalQueueSubmitFailureV1 {
    /// Exact currentness or terminal lower submit failure.
    #[must_use = "authenticated submit failure custody remains retained"]
    pub const fn failure(&self) -> &M1AuthenticatedPhysicalQueueSubmitFailureV1 {
        &self.failure
    }
}

impl M1AuthenticatedPhysicalQueueSubmitFailureV1 {
    /// Permanently faults the paired scheduler and retains this exact failure.
    #[must_use = "Engine-quarantined authenticated submit custody must remain retained"]
    pub fn quarantine_engine<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1EngineQuarantinedAuthenticatedPhysicalQueueSubmitFailureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        M1EngineQuarantinedAuthenticatedPhysicalQueueSubmitFailureV1 {
            failure: Box::new(self),
        }
    }

    /// Terminal high-level closure that never returns the retryable unpublished
    /// queue retained by a currentness rejection.
    pub(crate) fn close_without_authority<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1AuthenticatedPhysicalQueueClosureV1 {
        match self {
            Self::Currentness { error, retained } => {
                close_without_authority_core(engine, *retained, error)
            }
            Self::Queue(quarantined) => quarantine_without_authority_core(engine, quarantined),
        }
    }
}

enum SubmitCaseFailureV1<const N: usize> {
    Currentness {
        error: Box<AuthenticatedWorkerV3ProgramMaterializationErrorV1>,
        retained: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    },
    Queue(Box<M1AuthenticatedPhysicalQueueOperationFailureV1>),
}

fn submit_case<const N: usize>(
    mut case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>, SubmitCaseFailureV1<N>> {
    let lower = case.take_prepared();
    match lower.submit() {
        Ok(lower) => {
            case.set_published(lower);
            Ok(case)
        }
        Err(AuthenticatedServiceQueueSubmitFailureV1::Currentness { error, retained }) => {
            case.set_prepared(*retained);
            Err(SubmitCaseFailureV1::Currentness {
                error,
                retained: case,
            })
        }
        Err(AuthenticatedServiceQueueSubmitFailureV1::Queue(lower)) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(SubmitCaseFailureV1::Queue(operation_failure(
                shape, *lower, witness, operations, custody, step, None,
            )))
        }
    }
}

fn submit_variant<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
    published_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalPublishedQueueSessionV1,
    retained_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
) -> Result<
    M1AuthenticatedPhysicalPublishedQueueSessionV1,
    M1AuthenticatedPhysicalQueueSubmitFailureV1,
> {
    match submit_case(case, shape) {
        Ok(case) => Ok(published_variant(case)),
        Err(SubmitCaseFailureV1::Currentness { error, retained }) => {
            Err(M1AuthenticatedPhysicalQueueSubmitFailureV1::Currentness {
                error,
                retained: Box::new(retained_variant(retained)),
            })
        }
        Err(SubmitCaseFailureV1::Queue(failure)) => {
            Err(M1AuthenticatedPhysicalQueueSubmitFailureV1::Queue(failure))
        }
    }
}

impl M1AuthenticatedPhysicalQueueSessionV1 {
    /// Revalidates current publication and submits the exact batch once.
    ///
    /// # Errors
    ///
    /// Returns unchanged prepared custody on a pre-publication currentness
    /// rejection or terminal quarantine custody after lower queue mutation.
    pub fn submit(
        self,
    ) -> Result<
        M1AuthenticatedPhysicalPublishedQueueSessionV1,
        M1AuthenticatedPhysicalQueueSubmitFailureV1,
    > {
        match self {
            Self::TargetOnly(case) => submit_variant(
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                M1AuthenticatedPhysicalPublishedQueueSessionV1::TargetOnly,
                Self::TargetOnly,
            ),
            Self::PairedPrefill(case) => submit_variant(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                M1AuthenticatedPhysicalPublishedQueueSessionV1::PairedPrefill,
                Self::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => submit_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                M1AuthenticatedPhysicalPublishedQueueSessionV1::SpeculativeK4,
                Self::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => submit_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                M1AuthenticatedPhysicalPublishedQueueSessionV1::SpeculativeK8,
                Self::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => submit_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                M1AuthenticatedPhysicalPublishedQueueSessionV1::SpeculativeK16,
                Self::SpeculativeK16,
            ),
        }
    }
}

fn wait_case<const N: usize>(
    mut case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    let lower = case.take_published();
    let completed = wait_with_completion_progress_policy::<N, _, _, _>(
        lower,
        M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
        |published| {
            published.poll_with_progress().map(|outcome| match outcome {
                AuthenticatedServiceQueuePollWithProgressV1::Pending { session, progress } => {
                    CompletionProgressPollV1::Pending {
                        session,
                        progress: M1CompletionProgressObservationV1::from_service(progress),
                    }
                }
                AuthenticatedServiceQueuePollWithProgressV1::Ready { session, progress } => {
                    CompletionProgressPollV1::Ready {
                        session,
                        progress: M1CompletionProgressObservationV1::from_service(progress),
                    }
                }
            })
        },
        || {
            std::thread::sleep(std::time::Duration::from_micros(
                M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1,
            ));
        },
        |published| match published.wait(0) {
            Ok(_) => unreachable!("a zero-scan lower wait cannot complete a published batch"),
            Err(lower) => lower,
        },
    );
    match completed {
        Ok(lower) => {
            case.set_completed(lower);
            Ok(case)
        }
        Err(CompletionProgressWaitFailureV1::Lower(lower)) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(operation_failure(
                shape, lower, witness, operations, custody, step, None,
            ))
        }
        Err(CompletionProgressWaitFailureV1::Policy { lower, diagnostic }) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(operation_failure(
                shape,
                lower,
                witness,
                operations,
                custody,
                step,
                Some(diagnostic),
            ))
        }
    }
}

fn wait_variant<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
    completed_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalCompletedQueueSessionV1,
) -> Result<
    M1AuthenticatedPhysicalCompletedQueueSessionV1,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    wait_case(case, shape).map(completed_variant)
}

fn wait_for_case<const N: usize>(
    mut case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
    timeout_ms: u32,
) -> Result<
    Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    let lower = case.take_published();
    let completed = wait_with_completion_progress_deadline_policy::<N, _, _, _>(
        lower,
        M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
        timeout_ms,
        |published| {
            published.poll_with_progress().map(|outcome| match outcome {
                AuthenticatedServiceQueuePollWithProgressV1::Pending { session, progress } => {
                    CompletionProgressPollV1::Pending {
                        session,
                        progress: M1CompletionProgressObservationV1::from_service(progress),
                    }
                }
                AuthenticatedServiceQueuePollWithProgressV1::Ready { session, progress } => {
                    CompletionProgressPollV1::Ready {
                        session,
                        progress: M1CompletionProgressObservationV1::from_service(progress),
                    }
                }
            })
        },
        || {
            std::thread::sleep(std::time::Duration::from_micros(
                M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1,
            ));
        },
        |published| published.wait_for(0),
    );
    match completed {
        Ok(lower) => {
            case.set_completed(lower);
            Ok(case)
        }
        Err(CompletionProgressWaitFailureV1::Lower(lower)) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(operation_failure(
                shape, lower, witness, operations, custody, step, None,
            ))
        }
        Err(CompletionProgressWaitFailureV1::Policy { lower, diagnostic }) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(operation_failure(
                shape,
                lower,
                witness,
                operations,
                custody,
                step,
                Some(diagnostic),
            ))
        }
    }
}

fn wait_for_variant<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
    timeout_ms: u32,
    completed_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalCompletedQueueSessionV1,
) -> Result<
    M1AuthenticatedPhysicalCompletedQueueSessionV1,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    wait_for_case(case, shape, timeout_ms).map(completed_variant)
}

impl M1AuthenticatedPhysicalPublishedQueueSessionV1 {
    /// Waits under Ferric's bounded monotonic completion-progress policy.
    ///
    /// # Errors
    ///
    /// Returns terminal lower quarantine custody, with an additional Ferric
    /// progress-policy diagnostic when liveness validation terminated the wait.
    pub fn wait(
        self,
    ) -> Result<
        M1AuthenticatedPhysicalCompletedQueueSessionV1,
        Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => wait_variant(
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => wait_variant(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => wait_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => wait_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => wait_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK16,
            ),
        }
    }

    /// Waits under both Ferric's completion-progress policy and a monotonic
    /// relative millisecond deadline.
    ///
    /// This path preserves the exact authenticated program, queue, allocation,
    /// model-memory, and scheduler owners. A timeout or any other lower failure
    /// returns terminal quarantine custody and may carry a redacted timeout
    /// observation through [`M1AuthenticatedPhysicalQueueOperationFailureV1`].
    ///
    /// # Errors
    ///
    /// Returns terminal lower quarantine custody after either bounded policy
    /// fails. Terminalization consumes the exact lower owner through fe2o3
    /// KFD's zero-duration `wait_for` path and retains its timeout observation.
    pub fn wait_for(
        self,
        timeout_ms: u32,
    ) -> Result<
        M1AuthenticatedPhysicalCompletedQueueSessionV1,
        Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => wait_for_variant(
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                timeout_ms,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => wait_for_variant(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                timeout_ms,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => wait_for_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                timeout_ms,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => wait_for_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                timeout_ms,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => wait_for_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                timeout_ms,
                M1AuthenticatedPhysicalCompletedQueueSessionV1::SpeculativeK16,
            ),
        }
    }
}

fn recycle_case<const N: usize>(
    mut case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    let lower = case.take_completed();
    match lower.recycle() {
        Ok(lower) => {
            case.set_recycled(lower);
            Ok(case)
        }
        Err(lower) => {
            let (witness, operations, custody, step) = (*case).into_custody_parts();
            Err(operation_failure(
                shape, lower, witness, operations, custody, step, None,
            ))
        }
    }
}

fn recycle_variant<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
    recycled_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalRecycledQueueSessionV1,
) -> Result<
    M1AuthenticatedPhysicalRecycledQueueSessionV1,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    recycle_case(case, shape).map(recycled_variant)
}

impl M1AuthenticatedPhysicalCompletedQueueSessionV1 {
    /// Recycles every exact completion signal while retaining all authority.
    ///
    /// # Errors
    ///
    /// Returns terminal lower quarantine paired with every Ferric owner.
    pub fn recycle(
        self,
    ) -> Result<
        M1AuthenticatedPhysicalRecycledQueueSessionV1,
        Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => recycle_variant(
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                M1AuthenticatedPhysicalRecycledQueueSessionV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => recycle_variant(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                M1AuthenticatedPhysicalRecycledQueueSessionV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => recycle_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => recycle_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => recycle_variant(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK16,
            ),
        }
    }
}

/// Currentness rejection retaining the exact recycled queue owner.
#[must_use = "reuse rejection retains exact recycled custody"]
#[derive(Debug)]
pub struct M1AuthenticatedPhysicalQueueReuseFailureV1 {
    error: Box<AuthenticatedWorkerV3ProgramMaterializationErrorV1>,
    retained: Box<M1AuthenticatedPhysicalRecycledQueueSessionV1>,
}

impl M1AuthenticatedPhysicalQueueReuseFailureV1 {
    /// Exact currentness/materialization error.
    #[must_use]
    pub const fn error(&self) -> &AuthenticatedWorkerV3ProgramMaterializationErrorV1 {
        &self.error
    }

    /// Recovers the exact unchanged recycled owner.
    #[must_use = "recycled queue custody remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        AuthenticatedWorkerV3ProgramMaterializationErrorV1,
        M1AuthenticatedPhysicalRecycledQueueSessionV1,
    ) {
        (*self.error, *self.retained)
    }
}

struct ReuseCaseFailureV1<const N: usize> {
    error: AuthenticatedWorkerV3ProgramMaterializationErrorV1,
    retained: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
}

fn reuse_case<const N: usize>(
    mut case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
) -> Result<Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>, ReuseCaseFailureV1<N>> {
    let lower = case.take_recycled();
    match lower.reuse() {
        Ok(lower) => {
            case.set_prepared(lower);
            Ok(case)
        }
        Err(failure) => {
            let (error, lower) = failure.into_parts();
            case.set_recycled(lower);
            Err(ReuseCaseFailureV1 {
                error,
                retained: case,
            })
        }
    }
}

fn reuse_variant<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    prepared_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
    retained_variant: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalRecycledQueueSessionV1,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalQueueReuseFailureV1> {
    match reuse_case(case) {
        Ok(case) => Ok(prepared_variant(case)),
        Err(ReuseCaseFailureV1 { error, retained }) => {
            Err(M1AuthenticatedPhysicalQueueReuseFailureV1 {
                error: Box::new(error),
                retained: Box::new(retained_variant(retained)),
            })
        }
    }
}

impl M1AuthenticatedPhysicalRecycledQueueSessionV1 {
    /// Revalidates current publication and makes the exact attached batch publishable again.
    ///
    /// # Errors
    ///
    /// Returns currentness/materialization rejection with the exact unchanged
    /// recycled owner.
    pub fn reuse(
        self,
    ) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalQueueReuseFailureV1>
    {
        match self {
            Self::TargetOnly(case) => reuse_variant(
                case,
                M1AuthenticatedPhysicalQueueSessionV1::TargetOnly,
                Self::TargetOnly,
            ),
            Self::PairedPrefill(case) => reuse_variant(
                case,
                M1AuthenticatedPhysicalQueueSessionV1::PairedPrefill,
                Self::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => reuse_variant(
                case,
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4,
                Self::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => reuse_variant(
                case,
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8,
                Self::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => reuse_variant(
                case,
                M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16,
                Self::SpeculativeK16,
            ),
        }
    }
}

fn detach_case<const N: usize>(
    case: Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    former_shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    M1AuthenticatedPhysicalDetachedQueueSessionV1,
    Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
> {
    let (lower, witness, operations, custody, prior_step) = (*case).into_recycled_parts();
    match lower.detach() {
        Ok(lower) => Ok(M1AuthenticatedPhysicalDetachedQueueSessionV1 {
            lower,
            former_shape,
            witness,
            operations,
            custody,
            prior_step,
        }),
        Err(lower) => Err(operation_failure(
            former_shape,
            lower,
            witness,
            operations,
            custody,
            prior_step,
            None,
        )),
    }
}

impl M1AuthenticatedPhysicalRecycledQueueSessionV1 {
    /// Detaches the completed batch while preserving program history and prior-step authority.
    ///
    /// # Errors
    ///
    /// Returns terminal lower quarantine paired with every authenticated and
    /// Ferric owner when native detachment becomes ambiguous.
    pub fn detach(
        self,
    ) -> Result<
        M1AuthenticatedPhysicalDetachedQueueSessionV1,
        Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => detach_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly),
            Self::PairedPrefill(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}
