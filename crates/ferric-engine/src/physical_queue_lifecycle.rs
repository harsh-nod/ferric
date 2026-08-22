//! Linear Ferric custody around one complete M1 fixed-batch queue generation.
//!
//! The generic service host owns KFD queue state and enforces publication,
//! completion, recycle, readback, detach, and release. This module keeps the
//! corresponding M1 recipe, scheduler-bound target plans, pending KV
//! reservations, and allocation custody beside every generic phase. The
//! completed-readback join checks the exact K7 range against those retained
//! plans before minting completion authority.

use core::fmt;

use fe2o3_service_host::{
    ServiceCompletedQueueSessionV1, ServiceCompletedReadbackV1, ServicePublishedQueueSessionV1,
    ServiceQueueCreateFailureV1, ServiceQueueErrorV1, ServiceQueueOperationFailureV1,
    ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1, ServiceQueueSessionV1,
    ServiceQueueUnboundSessionV1, ServiceRecycledQueueSessionV1,
};
use ferric_spec::completion::CompletionEpoch;

use crate::completed_readback_join::check_m1_completed_output_v1;
use crate::observed_completion::observe_m1_completed_output_v1;
use crate::qualification_logits::observe_m1_qualification_logits_v1;
use crate::{
    CompletionWireExpectation, CompletionWireSemanticExpectation, ExactCompletion,
    Gfx942DeviceBinding, M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1,
    M1FullStepKvReservationCustodyV1, M1ObservedCompletionImageErrorV1,
    M1ObservedCompletionImageV1, M1ObservedQualificationLogitsV1, M1PhysicalFixedBatchCaseV1,
    M1PhysicalFixedBatchCustodyV1, M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchV1,
    M1PhysicalQueueBatchCustodyV1, M1PrepublicationBatchV1, M1PrepublicationStepCustodyV1,
    M1QualificationLogitsErrorV1, M1ScheduledDispatchV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Observable Ferric phase for one M1 queue generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalQueuePhaseV1 {
    /// A complete batch is attached but not published.
    Prepared,
    /// The complete batch was published exactly once.
    Published,
    /// Every completion signal was observed, before signal recycle.
    Completed,
    /// Every completion signal was recycled; readback, detach, or release is allowed.
    Recycled,
    /// Exact K7 bytes were copied once and structurally observed without semantic authority.
    Observed,
    /// Exact completed bytes were checked and completion authority was minted once.
    ReadbackJoined,
    /// Exact data custody was detached while the native queue remains live.
    Detached,
    /// A consuming lower-layer failure denied retry and retained opaque quarantine where possible.
    Quarantined,
}

impl M1PhysicalQueuePhaseV1 {
    /// Whether this phase grants exactly one batch submission transition.
    #[must_use]
    pub const fn can_submit(self) -> bool {
        matches!(self, Self::Prepared)
    }

    /// Whether this phase grants a bounded completion wait.
    #[must_use]
    pub const fn can_wait(self) -> bool {
        matches!(self, Self::Published)
    }

    /// Whether this phase grants exact signal recycle.
    #[must_use]
    pub const fn can_recycle(self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Whether this phase grants completed-readback join or a terminal transition.
    #[must_use]
    pub const fn can_read_detach_or_release(self) -> bool {
        matches!(self, Self::Recycled)
    }

    /// Whether this phase grants detach or release without another readback join.
    #[must_use]
    pub const fn can_detach_or_release(self) -> bool {
        matches!(self, Self::Recycled | Self::Observed | Self::ReadbackJoined)
    }
}

/// Pure classification of a queue-creation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalQueueCreateFailureClassV1 {
    /// Validation rejected unchanged allocation and fixed-batch inputs.
    Rejected,
    /// Native queue creation may have consumed inputs and retry is denied.
    Terminal,
}

impl M1PhysicalQueueCreateFailureClassV1 {
    /// Whether unchanged construction inputs can be recovered and retried.
    #[must_use]
    pub const fn can_recover_inputs(self) -> bool {
        matches!(self, Self::Rejected)
    }

    /// Whether the result denies retry because lower-layer effects may be ambiguous.
    #[must_use]
    pub const fn denies_retry(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

/// Opaque pairing of one exact generic queue phase with retained Ferric custody.
///
/// There is no public constructor, so callers cannot associate custody with a
/// different generic queue. The carrier intentionally does not implement
/// `Clone`.
#[must_use = "generic queue and Ferric custody must remain paired"]
pub struct M1PhysicalQueuePhaseCaseV1<Q> {
    lower: Q,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl<Q> M1PhysicalQueuePhaseCaseV1<Q> {
    pub(crate) const fn from_queue_rearm(
        lower: Q,
        custody: M1PhysicalQueueBatchCustodyV1,
        step: M1PrepublicationStepCustodyV1,
    ) -> Self {
        Self {
            lower,
            custody,
            step,
        }
    }

    const fn new(
        lower: Q,
        custody: M1PhysicalQueueBatchCustodyV1,
        step: M1PrepublicationStepCustodyV1,
    ) -> Self {
        Self {
            lower,
            custody,
            step,
        }
    }

    /// Returns retained Ferric recipe and allocation custody without exposing generic authority.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Checked physical-device receipt retained beside the generic queue phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device()
    }

    /// Returns the immutable logical epoch bound before queue creation.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Returns the exact linear scheduler dispatch retained by this queue phase.
    #[must_use = "scheduler dispatch authority remains paired with the physical queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    fn into_parts(
        self,
    ) -> (
        Q,
        M1PhysicalQueueBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.lower, self.custody, self.step)
    }
}

impl<Q: fmt::Debug> fmt::Debug for M1PhysicalQueuePhaseCaseV1<Q> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1PhysicalQueuePhaseCaseV1")
            .field("lower", &self.lower)
            .field("custody", &self.custody)
            .field("step", &self.step)
            .finish()
    }
}

/// Closed prepared M1 queue owner.
#[must_use = "a prepared M1 queue must be submitted or explicitly retained"]
#[derive(Debug)]
pub enum M1PhysicalQueueSessionV1 {
    /// One complete target-only publication.
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// One complete paired-prefill publication.
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// One complete K4 speculative publication, for either S1 or S8.
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// One complete K8 speculative publication.
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// One complete K16 speculative publication.
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
}

impl M1PhysicalQueueSessionV1 {
    /// Returns the exact closed M1 publication shape.
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

    /// Returns the compile-time packet cardinality of the retained queue phase.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Returns the prepared phase represented by this closed owner.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Prepared
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::TargetOnly(case) => case.queue_epoch(),
            Self::PairedPrefill(case) => case.queue_epoch(),
            Self::SpeculativeK4(case) => case.queue_epoch(),
            Self::SpeculativeK8(case) => case.queue_epoch(),
            Self::SpeculativeK16(case) => case.queue_epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained by this queue phase.
    #[must_use = "scheduler dispatch authority remains paired with the physical queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }
}

/// Closed published M1 queue owner.
#[must_use = "a published M1 queue must complete or remain retained"]
#[derive(Debug)]
pub enum M1PhysicalPublishedQueueSessionV1 {
    /// Published target-only queue.
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServicePublishedQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Published paired-prefill queue.
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServicePublishedQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Published K4 speculative queue.
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServicePublishedQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Published K8 speculative queue.
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServicePublishedQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Published K16 speculative queue.
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServicePublishedQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
}

impl M1PhysicalPublishedQueueSessionV1 {
    /// Returns the exact closed M1 publication shape.
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

    /// Returns the compile-time packet cardinality of the retained queue phase.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Returns the published phase represented by this closed owner.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Published
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::TargetOnly(case) => case.queue_epoch(),
            Self::PairedPrefill(case) => case.queue_epoch(),
            Self::SpeculativeK4(case) => case.queue_epoch(),
            Self::SpeculativeK8(case) => case.queue_epoch(),
            Self::SpeculativeK16(case) => case.queue_epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained by this queue phase.
    #[must_use = "scheduler dispatch authority remains paired with the physical queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }
}

/// Closed completed M1 queue owner before signal recycle.
#[must_use = "a completed M1 queue must recycle every exact signal"]
#[derive(Debug)]
pub enum M1PhysicalCompletedQueueSessionV1 {
    /// Completed target-only queue.
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceCompletedQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Completed paired-prefill queue.
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceCompletedQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Completed K4 speculative queue.
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceCompletedQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Completed K8 speculative queue.
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceCompletedQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Completed K16 speculative queue.
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceCompletedQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
}

impl M1PhysicalCompletedQueueSessionV1 {
    /// Returns the exact closed M1 publication shape.
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

    /// Returns the compile-time packet cardinality of the retained queue phase.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Returns the completed phase represented by this closed owner.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Completed
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::TargetOnly(case) => case.queue_epoch(),
            Self::PairedPrefill(case) => case.queue_epoch(),
            Self::SpeculativeK4(case) => case.queue_epoch(),
            Self::SpeculativeK8(case) => case.queue_epoch(),
            Self::SpeculativeK16(case) => case.queue_epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained by this queue phase.
    #[must_use = "scheduler dispatch authority remains paired with the physical queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }
}

/// Closed recycled M1 queue owner before the one-shot readback join.
#[must_use = "a recycled M1 queue must be joined, detached, released, or explicitly retained"]
#[derive(Debug)]
pub enum M1PhysicalRecycledQueueSessionV1 {
    /// Recycled target-only queue.
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Recycled paired-prefill queue.
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Recycled K4 speculative queue.
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Recycled K8 speculative queue.
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Recycled K16 speculative queue.
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
}

impl M1PhysicalRecycledQueueSessionV1 {
    /// Returns the exact closed M1 publication shape.
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

    /// Returns the compile-time packet cardinality of the retained queue phase.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Returns the recycled phase represented by this closed owner.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Recycled
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::TargetOnly(case) => case.queue_epoch(),
            Self::PairedPrefill(case) => case.queue_epoch(),
            Self::SpeculativeK4(case) => case.queue_epoch(),
            Self::SpeculativeK8(case) => case.queue_epoch(),
            Self::SpeculativeK16(case) => case.queue_epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained by this queue phase.
    #[must_use = "scheduler dispatch authority remains paired with the physical queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }
}

/// One exact recycled queue generation paired with its single copied K7 image.
///
/// The carrier intentionally has no completed-read method and is not `Clone`.
#[must_use = "observed bytes and all queue, scheduler, and KV custody remain linear"]
pub struct M1ObservedCompletionCaseV1<const N: usize> {
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    image: M1ObservedCompletionImageV1,
}

impl<const N: usize> fmt::Debug for M1ObservedCompletionCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1ObservedCompletionCaseV1")
            .field("case", &self.case)
            .field("image", &self.image)
            .finish()
    }
}

/// Move-only inert observation after one exact completed K7 readback.
///
/// This value retains the generic queue, physical allocation custody, exact
/// scheduler roster, target plans, pending KV reservations, dispatch
/// generation, selection, and copied bytes. It creates no [`ExactCompletion`]
/// and grants no numerical, inference, refinement, or performance authority.
///
/// ```compile_fail
/// use ferric_engine::M1ObservedCompletionOutputV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ObservedCompletionOutputV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1ObservedCompletionOutputV1;
/// fn read_twice(observed: M1ObservedCompletionOutputV1) {
///     let _ = observed.observe_completion();
/// }
/// ```
#[must_use = "observed completion custody must be checked, destroyed, or retained"]
#[derive(Debug)]
pub enum M1ObservedCompletionOutputV1 {
    /// Observed target-only queue.
    TargetOnly(Box<M1ObservedCompletionCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// Observed paired-prefill queue.
    PairedPrefill(Box<M1ObservedCompletionCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>),
    /// Observed K4 speculative queue.
    SpeculativeK4(Box<M1ObservedCompletionCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>),
    /// Observed K8 speculative queue.
    SpeculativeK8(Box<M1ObservedCompletionCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>),
    /// Observed K16 speculative queue.
    SpeculativeK16(Box<M1ObservedCompletionCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>),
}

fn join_observed_output_case<const N: usize>(
    case: Box<M1ObservedCompletionCaseV1<N>>,
    observed: fn(Box<M1ObservedCompletionCaseV1<N>>) -> M1ObservedCompletionOutputV1,
    readback: fn(Box<M1PhysicalReadbackQueueCaseV1<N>>) -> M1PhysicalReadbackQueueSessionV1,
    expectations: &[CompletionWireSemanticExpectation<'_>],
) -> Result<M1PhysicalCompletedReadbackV1, M1CompletedReadbackJoinFailureV1> {
    match check_observed_case(case, expectations) {
        Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
            queue: readback(case),
            checked,
            completion,
            kv,
        }),
        Err((source, case)) => Err(M1CompletedReadbackJoinFailureV1 {
            error: M1CompletedReadbackJoinErrorV1 { source },
            observed: Box::new(observed(case)),
        }),
    }
}

fn release_observed_case<const N: usize>(
    case: M1ObservedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
    let M1ObservedCompletionCaseV1 { case, image: _ } = case;
    release_case(case, shape)
}

fn release_rejected_case<const N: usize>(
    case: M1RejectedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
    let M1RejectedCompletionCaseV1 { case, readback: _ } = case;
    release_case(case, shape)
}

impl M1ObservedCompletionOutputV1 {
    /// Returns the exact former M1 publication shape.
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

    /// Returns the observed phase represented by this closed owner.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Observed
    }

    /// Borrows the inert copied image and decoded records.
    #[must_use = "the observed image remains paired with physical custody"]
    pub const fn image(&self) -> &M1ObservedCompletionImageV1 {
        match self {
            Self::TargetOnly(case) => &case.image,
            Self::PairedPrefill(case) => &case.image,
            Self::SpeculativeK4(case) => &case.image,
            Self::SpeculativeK8(case) => &case.image,
            Self::SpeculativeK16(case) => &case.image,
        }
    }

    /// Returns the exact scheduler dispatch retained beside the copied image.
    #[must_use = "scheduler authority remains paired with the observation"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.case.step.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.case.step.scheduled_dispatch(),
        }
    }

    /// Returns the checked physical-device receipt retained through observation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.case.custody.device(),
            Self::PairedPrefill(case) => case.case.custody.device(),
            Self::SpeculativeK4(case) => case.case.custody.device(),
            Self::SpeculativeK8(case) => case.case.custody.device(),
            Self::SpeculativeK16(case) => case.case.custody.device(),
        }
    }

    /// Returns the exact selected-program catalog identity.
    #[must_use]
    pub const fn catalog_id(&self) -> ferric_spec::Identity {
        match self {
            Self::TargetOnly(case) => case.case.custody.catalog_id(),
            Self::PairedPrefill(case) => case.case.custody.catalog_id(),
            Self::SpeculativeK4(case) => case.case.custody.catalog_id(),
            Self::SpeculativeK8(case) => case.case.custody.catalog_id(),
            Self::SpeculativeK16(case) => case.case.custody.catalog_id(),
        }
    }

    /// Returns pending KV reservation custody retained through observation.
    #[must_use = "pending KV custody remains paired with the observation"]
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.case.step.kv_reservations(),
            Self::PairedPrefill(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK4(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK8(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK16(case) => case.case.step.kv_reservations(),
        }
    }
}

/// One exact recycled queue generation paired with a rejected copied K7 image.
///
/// The copied bytes are retained so structural rejection cannot reopen the
/// lower completed-read operation.
#[must_use = "rejected copied bytes and all Ferric custody remain linear"]
pub struct M1RejectedCompletionCaseV1<const N: usize> {
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    readback: ServiceCompletedReadbackV1,
}

impl<const N: usize> fmt::Debug for M1RejectedCompletionCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1RejectedCompletionCaseV1")
            .field("case", &self.case)
            .field("readback", &self.readback)
            .finish()
    }
}

/// Move-only custody after one copied K7 image failed structural observation.
///
/// This owner exposes the rejected raw copy for diagnosis and can tear down the
/// queue, but has no completed-read or semantic-completion transition.
///
/// ```compile_fail
/// use ferric_engine::M1RejectedCompletionOutputV1;
/// fn reread(rejected: M1RejectedCompletionOutputV1) {
///     let _ = rejected.observe_completion();
/// }
/// ```
#[must_use = "rejected observation custody must be destroyed or retained"]
#[derive(Debug)]
pub enum M1RejectedCompletionOutputV1 {
    /// Rejected target-only queue observation.
    TargetOnly(Box<M1RejectedCompletionCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// Rejected paired-prefill queue observation.
    PairedPrefill(Box<M1RejectedCompletionCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>),
    /// Rejected K4 speculative queue observation.
    SpeculativeK4(Box<M1RejectedCompletionCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>),
    /// Rejected K8 speculative queue observation.
    SpeculativeK8(Box<M1RejectedCompletionCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>),
    /// Rejected K16 speculative queue observation.
    SpeculativeK16(Box<M1RejectedCompletionCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>),
}

impl M1RejectedCompletionOutputV1 {
    /// Returns the exact former M1 publication shape.
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

    /// Returns the exact target selection retained beside the rejected copy.
    #[must_use]
    pub const fn selection(&self) -> ferric_spec::Qwen3PlanSelection {
        match self {
            Self::TargetOnly(case) => case.case.custody.selection(),
            Self::PairedPrefill(case) => case.case.custody.selection(),
            Self::SpeculativeK4(case) => case.case.custody.selection(),
            Self::SpeculativeK8(case) => case.case.custody.selection(),
            Self::SpeculativeK16(case) => case.case.custody.selection(),
        }
    }

    /// Returns the checked physical-device receipt retained through rejection.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.case.custody.device(),
            Self::PairedPrefill(case) => case.case.custody.device(),
            Self::SpeculativeK4(case) => case.case.custody.device(),
            Self::SpeculativeK8(case) => case.case.custody.device(),
            Self::SpeculativeK16(case) => case.case.custody.device(),
        }
    }

    /// Returns the observed phase represented by this rejected copied image.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Observed
    }

    /// Returns the exact generation that authorized the rejected byte copy.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        match self {
            Self::TargetOnly(case) => case.readback.dispatch_generation(),
            Self::PairedPrefill(case) => case.readback.dispatch_generation(),
            Self::SpeculativeK4(case) => case.readback.dispatch_generation(),
            Self::SpeculativeK8(case) => case.readback.dispatch_generation(),
            Self::SpeculativeK16(case) => case.readback.dispatch_generation(),
        }
    }

    /// Returns the addressless data ordinal bound to the rejected byte copy.
    #[must_use]
    pub const fn data_index(&self) -> usize {
        match self {
            Self::TargetOnly(case) => case.readback.data_index(),
            Self::PairedPrefill(case) => case.readback.data_index(),
            Self::SpeculativeK4(case) => case.readback.data_index(),
            Self::SpeculativeK8(case) => case.readback.data_index(),
            Self::SpeculativeK16(case) => case.readback.data_index(),
        }
    }

    /// Returns the copied offset reported for the rejected image.
    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        match self {
            Self::TargetOnly(case) => case.readback.offset_bytes(),
            Self::PairedPrefill(case) => case.readback.offset_bytes(),
            Self::SpeculativeK4(case) => case.readback.offset_bytes(),
            Self::SpeculativeK8(case) => case.readback.offset_bytes(),
            Self::SpeculativeK16(case) => case.readback.offset_bytes(),
        }
    }

    /// Returns the exact rejected copied bytes without granting semantic authority.
    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        match self {
            Self::TargetOnly(case) => case.readback.bytes(),
            Self::PairedPrefill(case) => case.readback.bytes(),
            Self::SpeculativeK4(case) => case.readback.bytes(),
            Self::SpeculativeK8(case) => case.readback.bytes(),
            Self::SpeculativeK16(case) => case.readback.bytes(),
        }
    }

    /// Returns the exact scheduler dispatch retained beside the rejected copy.
    #[must_use = "scheduler authority remains paired with the rejected observation"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.case.step.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.case.step.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.case.step.scheduled_dispatch(),
        }
    }

    /// Returns pending KV reservation custody retained through rejection.
    #[must_use = "pending KV custody remains paired with the rejected observation"]
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.case.step.kv_reservations(),
            Self::PairedPrefill(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK4(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK8(case) => case.case.step.kv_reservations(),
            Self::SpeculativeK16(case) => case.case.step.kv_reservations(),
        }
    }
}

/// Post-readback generic queue custody with no remaining scheduler dispatch authority.
#[must_use = "post-readback queue custody must be detached, released, or retained"]
pub struct M1PhysicalReadbackQueueCaseV1<const N: usize> {
    lower: ServiceRecycledQueueSessionV1<N>,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl<const N: usize> M1PhysicalReadbackQueueCaseV1<N> {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the post-readback queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    const fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1 {
        &mut self.custody
    }

    fn into_parts(
        self,
    ) -> (
        ServiceRecycledQueueSessionV1<N>,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        (self.lower, self.custody)
    }
}

impl<const N: usize> fmt::Debug for M1PhysicalReadbackQueueCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1PhysicalReadbackQueueCaseV1")
            .field("lower", &self.lower)
            .field("custody", &self.custody)
            .finish()
    }
}

/// Closed M1 queue owner after the one-shot completed-readback join.
///
/// A fresh scheduler roster cannot reuse request-specific batch custody.
///
/// ```compile_fail
/// use ferric_engine::{M1PhysicalReadbackQueueSessionV1, M1ScheduledDispatchV1};
/// fn raw_reuse(queue: M1PhysicalReadbackQueueSessionV1, scheduled: M1ScheduledDispatchV1) {
///     let _ = queue.reuse(scheduled);
/// }
/// ```
#[must_use = "post-readback queue custody must be detached, released, or retained"]
#[derive(Debug)]
pub enum M1PhysicalReadbackQueueSessionV1 {
    /// Post-readback target-only queue.
    TargetOnly(Box<M1PhysicalReadbackQueueCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    /// Post-readback paired-prefill queue.
    PairedPrefill(Box<M1PhysicalReadbackQueueCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>),
    /// Post-readback K4 speculative queue.
    SpeculativeK4(Box<M1PhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>),
    /// Post-readback K8 speculative queue.
    SpeculativeK8(Box<M1PhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>),
    /// Post-readback K16 speculative queue.
    SpeculativeK16(Box<M1PhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>),
}

impl M1PhysicalReadbackQueueSessionV1 {
    /// Returns the exact former M1 publication shape.
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

    /// Returns the one-shot readback-joined phase.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::ReadbackJoined
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the post-readback queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    pub(crate) const fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody_mut(),
            Self::PairedPrefill(case) => case.custody_mut(),
            Self::SpeculativeK4(case) => case.custody_mut(),
            Self::SpeculativeK8(case) => case.custody_mut(),
            Self::SpeculativeK16(case) => case.custody_mut(),
        }
    }
}

/// Post-readback detached generic queue and exact Ferric custody.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalReadbackDetachedQueueCaseV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl M1PhysicalReadbackDetachedQueueCaseV1 {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Returns the completed generic dispatch generation retained by detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        self.lower.detached_dispatch_generation()
    }
}

/// Closed former M1 shape after readback join and exact detachment.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub enum M1PhysicalReadbackDetachedQueueSessionV1 {
    /// Detached target-only queue.
    TargetOnly(Box<M1PhysicalReadbackDetachedQueueCaseV1>),
    /// Detached paired-prefill queue.
    PairedPrefill(Box<M1PhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K4 speculative queue.
    SpeculativeK4(Box<M1PhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K8 speculative queue.
    SpeculativeK8(Box<M1PhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K16 speculative queue.
    SpeculativeK16(Box<M1PhysicalReadbackDetachedQueueCaseV1>),
}

impl M1PhysicalReadbackDetachedQueueSessionV1 {
    /// Returns the exact former M1 publication shape.
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

    /// Returns the detached phase classification.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Detached
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the completed generic dispatch generation retained by detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.detached_dispatch_generation(),
        }
    }

    pub(crate) fn into_rearm_parts(
        self,
    ) -> (
        M1PhysicalFixedBatchShapeV1,
        ServiceQueueUnboundSessionV1,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        match self {
            Self::TargetOnly(case) => (
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                case.lower,
                case.custody,
            ),
            Self::PairedPrefill(case) => (
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                case.lower,
                case.custody,
            ),
            Self::SpeculativeK4(case) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                case.lower,
                case.custody,
            ),
            Self::SpeculativeK8(case) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                case.lower,
                case.custody,
            ),
            Self::SpeculativeK16(case) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                case.lower,
                case.custody,
            ),
        }
    }
}

/// Terminal post-readback transition failure with available generic quarantine.
#[must_use = "terminal failure retains generic quarantine and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalReadbackQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueOperationFailureV1,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

impl M1PhysicalReadbackQueueOperationFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the exact generic operation error.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        self.lower.error()
    }

    /// Returns the exact Ferric custody retained beside generic quarantine.
    #[must_use = "Ferric custody remains retained by terminal failure"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

/// Terminal post-readback release failure retaining every available owner.
#[must_use = "terminal release failure retains lower and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalReadbackQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueReleaseFailureV1,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

impl M1PhysicalReadbackQueueReleaseFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the lower terminal release failure by borrow.
    #[must_use = "the lower failure retains available generic custody"]
    pub const fn lower(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.lower
    }

    /// Returns the exact Ferric custody retained beside release failure.
    #[must_use = "Ferric custody remains retained by terminal failure"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

/// One-shot completed-copy or structural-observation diagnostic.
#[derive(Debug)]
pub enum M1CompletionObservationErrorV1 {
    /// The generic generation-bound completed copy failed.
    Queue(ServiceQueueErrorV1),
    /// The returned allocation offset differed from the retained K7 range.
    OffsetDrift {
        /// Retained K7 output offset.
        expected: u64,
        /// Generic readback offset.
        actual: u64,
    },
    /// The returned byte count differed from the retained K7 range.
    ExtentDrift {
        /// Retained K7 output extent.
        expected: u64,
        /// Generic readback byte count.
        actual: u64,
    },
    /// Copied records failed bounded structural decoding or inactive padding checks.
    Image(M1ObservedCompletionImageErrorV1),
}

impl fmt::Display for M1CompletionObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completion observation rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletionObservationErrorV1 {}

/// Linear custody retained by an observation failure.
#[must_use = "failure custody must be retried, destroyed, or retained"]
#[derive(Debug)]
pub enum M1CompletionObservationFailureCustodyV1 {
    /// No completed copy succeeded, so the exact recycled queue remains retryable.
    Recycled(Box<M1PhysicalRecycledQueueSessionV1>),
    /// A completed copy succeeded and is closed against another read.
    Rejected(Box<M1RejectedCompletionOutputV1>),
}

/// Observation failure retaining exact pre-copy or post-copy custody.
///
/// Only [`M1CompletionObservationFailureCustodyV1::Recycled`] permits retry.
/// Coordinate, extent, and image failures retain the first copied bytes in a
/// [`M1RejectedCompletionOutputV1`] with no completed-read transition.
#[must_use = "observation failure retains linear queue and byte custody"]
#[derive(Debug)]
pub struct M1CompletionObservationFailureV1 {
    error: M1CompletionObservationErrorV1,
    custody: M1CompletionObservationFailureCustodyV1,
}

impl M1CompletionObservationFailureV1 {
    /// Returns the exact observation failure.
    #[must_use]
    pub const fn error(&self) -> &M1CompletionObservationErrorV1 {
        &self.error
    }

    /// Returns the exact retained pre-copy or post-copy custody.
    #[must_use = "linear failure custody remains retained"]
    pub const fn custody(&self) -> &M1CompletionObservationFailureCustodyV1 {
        &self.custody
    }

    /// Recovers the exact failure and its linear custody.
    #[must_use = "linear failure custody must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletionObservationErrorV1,
        M1CompletionObservationFailureCustodyV1,
    ) {
        (self.error, self.custody)
    }
}

/// Semantic-join diagnostic after an exact structural observation exists.
#[derive(Debug)]
pub struct M1CompletedReadbackJoinErrorV1 {
    source: M1CompletedOutputCheckErrorV1,
}

impl M1CompletedReadbackJoinErrorV1 {
    /// Returns the exact scheduler, plan, wire, or token-semantic rejection.
    #[must_use]
    pub const fn source(&self) -> &M1CompletedOutputCheckErrorV1 {
        &self.source
    }
}

impl fmt::Display for M1CompletedReadbackJoinErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 completed readback join rejected: {:?}",
            self.source
        )
    }
}

impl std::error::Error for M1CompletedReadbackJoinErrorV1 {}

/// Semantic rejection retaining the same captured observation for correction.
///
/// No [`ExactCompletion`] exists on this path, and retry never recopies the
/// completed host range.
#[must_use = "join failure retains the observed completion for retry or teardown"]
#[derive(Debug)]
pub struct M1CompletedReadbackJoinFailureV1 {
    error: M1CompletedReadbackJoinErrorV1,
    observed: Box<M1ObservedCompletionOutputV1>,
}

impl M1CompletedReadbackJoinFailureV1 {
    /// Returns the exact semantic failure.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    /// Returns the unchanged captured observation by borrow.
    #[must_use = "observed queue and byte custody remain retained by this failure"]
    pub const fn observed(&self) -> &M1ObservedCompletionOutputV1 {
        &self.observed
    }

    /// Recovers the exact failure and unchanged captured observation.
    #[must_use = "captured observation custody must remain retained"]
    pub fn into_parts(self) -> (M1CompletedReadbackJoinErrorV1, M1ObservedCompletionOutputV1) {
        (self.error, *self.observed)
    }
}

/// Successful one-shot join of queue custody, checked K7 records, and quiescence.
///
/// The embedded post-readback queue type has no completed-readback join method,
/// so this owner cannot mint a second completion for the same scheduler batch.
///
/// ```compile_fail
/// use ferric_engine::M1PhysicalCompletedReadbackV1;
/// fn consume_twice(joined: M1PhysicalCompletedReadbackV1) {
///     let _first = joined.into_parts();
///     let _second = joined.into_parts();
/// }
/// ```
#[must_use = "joined queue, checked records, and exact completion must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalCompletedReadbackV1 {
    queue: M1PhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
}

/// Copied compact K7 bytes and final live BF16 logits rows for qualification.
#[must_use = "qualification evidence must be reported or retained"]
#[derive(Debug)]
pub struct M1QualificationCompletionEvidenceV1 {
    compact_raw_bytes: Box<[u8]>,
    compact_raw_sha256: [u8; 32],
    logits: M1ObservedQualificationLogitsV1,
}

impl M1QualificationCompletionEvidenceV1 {
    /// Exact copied compact K7 image, including inactive canonical padding.
    #[must_use]
    pub fn compact_raw_bytes(&self) -> &[u8] {
        &self.compact_raw_bytes
    }

    /// SHA-256 of the exact compact K7 image.
    #[must_use]
    pub const fn compact_raw_sha256(&self) -> &[u8; 32] {
        &self.compact_raw_sha256
    }

    /// Exact final live BF16 rows in scheduler order.
    #[must_use = "the captured logits rows remain retained by this evidence"]
    pub const fn logits(&self) -> &M1ObservedQualificationLogitsV1 {
        &self.logits
    }
}

/// Move-only target-only observation retaining compact and final-logits evidence.
///
/// ```compile_fail
/// use ferric_engine::{CompletionWireSemanticExpectation, M1ObservedQualificationOutputV1};
/// fn consume_twice(
///     observed: M1ObservedQualificationOutputV1,
///     expectations: &[CompletionWireSemanticExpectation<'_>],
/// ) {
///     let _first = observed.check_completion(expectations);
///     let _second = observed.destroy_and_release();
/// }
/// ```
#[must_use = "qualification observation must be checked, destroyed, or retained"]
#[derive(Debug)]
pub struct M1ObservedQualificationOutputV1 {
    completion: M1ObservedCompletionOutputV1,
    evidence: M1QualificationCompletionEvidenceV1,
}

impl M1ObservedQualificationOutputV1 {
    /// Structurally observed compact K7 image.
    #[must_use = "the compact image remains retained by this observation"]
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        self.completion.image()
    }

    /// Copied compact and final-logits qualification evidence.
    #[must_use = "qualification evidence remains retained by this observation"]
    pub const fn evidence(&self) -> &M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    /// Consumes the compact observation through the existing semantic join.
    ///
    /// Failure retains the same copied evidence and cannot reopen either read.
    ///
    /// # Errors
    ///
    /// Returns the existing compact semantic rejection while retaining the
    /// same already-copied qualification evidence in closed custody.
    pub fn check_completion(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1QualifiedPhysicalCompletedReadbackV1, M1QualificationCompletedReadbackJoinFailureV1>
    {
        let Self {
            completion,
            evidence,
        } = self;
        match completion.check_completion(expectations) {
            Ok(completed) => Ok(M1QualifiedPhysicalCompletedReadbackV1 {
                completed,
                evidence,
            }),
            Err(failure) => {
                let (error, completion) = failure.into_parts();
                Err(M1QualificationCompletedReadbackJoinFailureV1 {
                    error,
                    observed: Box::new(Self {
                        completion,
                        evidence,
                    }),
                })
            }
        }
    }

    /// Tears down the queue without granting semantic completion authority.
    ///
    /// # Errors
    ///
    /// Returns the existing terminal release failure with all available queue
    /// and Ferric custody retained.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        self.completion.destroy_and_release()
    }
}

/// Successful semantic join retaining qualification evidence beside completion.
#[must_use = "completed readback and qualification evidence must remain retained"]
#[derive(Debug)]
pub struct M1QualifiedPhysicalCompletedReadbackV1 {
    completed: M1PhysicalCompletedReadbackV1,
    evidence: M1QualificationCompletionEvidenceV1,
}

impl M1QualifiedPhysicalCompletedReadbackV1 {
    /// Existing exact completed-readback custody.
    #[must_use = "completed readback custody remains retained by this join"]
    pub const fn completed(&self) -> &M1PhysicalCompletedReadbackV1 {
        &self.completed
    }

    /// Exact copied qualification evidence.
    #[must_use = "qualification evidence remains retained by this join"]
    pub const fn evidence(&self) -> &M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    /// Separates completed authority and inert qualification evidence once.
    #[must_use = "both completion custody and qualification evidence remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalCompletedReadbackV1,
        M1QualificationCompletionEvidenceV1,
    ) {
        (self.completed, self.evidence)
    }
}

/// Qualification semantic failure retaining the same already-copied evidence.
#[must_use = "semantic rejection retains one-shot qualification custody"]
#[derive(Debug)]
pub struct M1QualificationCompletedReadbackJoinFailureV1 {
    error: M1CompletedReadbackJoinErrorV1,
    observed: Box<M1ObservedQualificationOutputV1>,
}

impl M1QualificationCompletedReadbackJoinFailureV1 {
    /// Existing exact compact semantic rejection.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    /// Same one-shot observation; no generic completed-read transition exists.
    #[must_use = "the rejected observation retains all copied evidence"]
    pub const fn observed(&self) -> &M1ObservedQualificationOutputV1 {
        &self.observed
    }

    /// Recovers the error and unchanged qualification observation once.
    #[must_use = "the captured evidence remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedReadbackJoinErrorV1,
        M1ObservedQualificationOutputV1,
    ) {
        (self.error, *self.observed)
    }
}

/// Qualification observation failure before or after completed copies.
#[derive(Debug)]
pub enum M1QualificationObservationErrorV1 {
    /// Only target-only physical batches admit this evidence path.
    NotTargetOnly,
    /// Qualification capture was not explicitly attached before publication.
    CaptureNotEnabled,
    /// Existing compact K7 observation rejected.
    Compact(M1CompletionObservationErrorV1),
    /// Host allocation for an immutable compact evidence copy failed.
    HostAllocation,
    /// One final logits row failed its generation-bound generic copy.
    Queue {
        lane: usize,
        source: ServiceQueueErrorV1,
    },
    /// Final-row shape or completed coordinates rejected.
    Logits(M1QualificationLogitsErrorV1),
}

impl fmt::Display for M1QualificationObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 qualification observation rejected: {self:?}")
    }
}

impl std::error::Error for M1QualificationObservationErrorV1 {}

/// Linear custody retained after a qualification observation failure.
///
/// ```compile_fail
/// use ferric_engine::M1QualificationObservationFailureCustodyV1;
/// fn destroy(custody: M1QualificationObservationFailureCustodyV1) {
///     match custody {
///         M1QualificationObservationFailureCustodyV1::Recycled(queue) => {
///             let _ = queue.destroy_and_release();
///         }
///         M1QualificationObservationFailureCustodyV1::CompactRejected(output) => {
///             let _ = output.destroy_and_release();
///         }
///         M1QualificationObservationFailureCustodyV1::Observed { completion, .. } => {
///             let _ = completion.destroy_and_release();
///         }
///     }
/// }
/// fn destroy_twice(custody: M1QualificationObservationFailureCustodyV1) {
///     destroy(custody);
///     destroy(custody);
/// }
/// ```
#[must_use = "qualification failure custody must be torn down or retained"]
#[derive(Debug)]
pub enum M1QualificationObservationFailureCustodyV1 {
    /// No completed copy succeeded; the exact recycled queue remains retryable.
    Recycled(Box<M1PhysicalRecycledQueueSessionV1>),
    /// Compact K7 copied but failed structural observation; no read is reopened.
    CompactRejected(Box<M1RejectedCompletionOutputV1>),
    /// Compact K7 observed; zero or more final logits rows were also copied.
    Observed {
        completion: Box<M1ObservedCompletionOutputV1>,
        partial_logits: Box<[ServiceCompletedReadbackV1]>,
    },
}

/// Fail-closed qualification observation with exact phase-local custody.
#[must_use = "qualification failure retains linear queue and copied-byte custody"]
#[derive(Debug)]
pub struct M1QualificationObservationFailureV1 {
    error: M1QualificationObservationErrorV1,
    custody: M1QualificationObservationFailureCustodyV1,
}

impl M1QualificationObservationFailureV1 {
    /// Exact pre-copy, compact-copy, or logits-copy rejection.
    #[must_use]
    pub const fn error(&self) -> &M1QualificationObservationErrorV1 {
        &self.error
    }

    /// Exact retained custody; only `Recycled` permits another observation.
    #[must_use = "linear observation failure custody remains retained"]
    pub const fn custody(&self) -> &M1QualificationObservationFailureCustodyV1 {
        &self.custody
    }

    /// Recovers the diagnostic and phase-local custody once.
    #[must_use = "failure custody remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1QualificationObservationErrorV1,
        M1QualificationObservationFailureCustodyV1,
    ) {
        (self.error, self.custody)
    }
}

impl M1PhysicalCompletedReadbackV1 {
    /// Returns post-readback queue custody without exposing another join transition.
    #[must_use = "post-readback queue custody remains retained by the join"]
    pub const fn queue(&self) -> &M1PhysicalReadbackQueueSessionV1 {
        &self.queue
    }

    /// Returns all checked live records in exact scheduler-member order.
    #[must_use = "checked completion records remain retained by the join"]
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    /// Returns the single exact completion epoch without duplicating authority.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    pub(crate) const fn completion_authority(&self) -> &ExactCompletion {
        &self.completion
    }

    /// Returns pending KV reservations retained through exact readback.
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }

    /// Separates post-readback queue custody, checked records, and completion authority once.
    #[must_use = "all three linear outputs must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalReadbackQueueSessionV1,
        M1CheckedCompletionOutputV1,
        ExactCompletion,
        M1FullStepKvReservationCustodyV1,
    ) {
        (self.queue, self.checked, self.completion, self.kv)
    }
}

enum M1PhysicalQueueCreateFailureStateV1<'a> {
    Rejected {
        error: ServiceQueueErrorV1,
        batch: Box<M1PrepublicationBatchV1<'a>>,
    },
    Terminal {
        error: ServiceQueueErrorV1,
        shape: M1PhysicalFixedBatchShapeV1,
        step: Box<M1PrepublicationStepCustodyV1>,
        custody: Box<M1PhysicalQueueBatchCustodyV1>,
    },
}

/// Opaque queue-creation rejection or terminal failure with exact Ferric custody.
///
/// Pure rejection can recover only the exact recombined prepublication input
/// through [`Self::into_rejected_input`]. Terminal model, partition, ledger,
/// scheduler, and batch custody cannot be pattern-matched apart.
///
/// ```compile_fail
/// use ferric_engine::M1PhysicalQueueCreateFailureV1;
/// fn split(failure: M1PhysicalQueueCreateFailureV1<'_>) {
///     let M1PhysicalQueueCreateFailureV1::Terminal { custody, .. } = failure;
/// }
/// ```
#[must_use = "pure rejection retains exact inputs; terminal failure retains Ferric custody"]
pub struct M1PhysicalQueueCreateFailureV1<'a> {
    state: M1PhysicalQueueCreateFailureStateV1<'a>,
}

impl fmt::Debug for M1PhysicalQueueCreateFailureV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { error, batch, .. } => formatter
                .debug_struct("Rejected")
                .field("error", error)
                .field("shape", &batch.shape())
                .field("step", &batch.step())
                .finish_non_exhaustive(),
            M1PhysicalQueueCreateFailureStateV1::Terminal {
                error, shape, step, ..
            } => formatter
                .debug_struct("Terminal")
                .field("error", error)
                .field("shape", shape)
                .field("step", step)
                .finish_non_exhaustive(),
        }
    }
}

impl<'a> M1PhysicalQueueCreateFailureV1<'a> {
    fn rejected(error: ServiceQueueErrorV1, batch: M1PrepublicationBatchV1<'a>) -> Self {
        Self {
            state: M1PhysicalQueueCreateFailureStateV1::Rejected {
                error,
                batch: Box::new(batch),
            },
        }
    }

    fn terminal(
        error: ServiceQueueErrorV1,
        shape: M1PhysicalFixedBatchShapeV1,
        step: Box<M1PrepublicationStepCustodyV1>,
        custody: Box<M1PhysicalQueueBatchCustodyV1>,
    ) -> Self {
        Self {
            state: M1PhysicalQueueCreateFailureStateV1::Terminal {
                error,
                shape,
                step,
                custody,
            },
        }
    }

    /// Classifies recoverable pure rejection versus terminal consumption.
    #[must_use]
    pub const fn class(&self) -> M1PhysicalQueueCreateFailureClassV1 {
        match self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { .. } => {
                M1PhysicalQueueCreateFailureClassV1::Rejected
            }
            M1PhysicalQueueCreateFailureStateV1::Terminal { .. } => {
                M1PhysicalQueueCreateFailureClassV1::Terminal
            }
        }
    }

    /// Returns the exact generic error without consuming retained ownership.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { error, .. }
            | M1PhysicalQueueCreateFailureStateV1::Terminal { error, .. } => error,
        }
    }

    /// Returns the exact rejected or terminal M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { batch, .. } => batch.shape(),
            M1PhysicalQueueCreateFailureStateV1::Terminal { shape, .. } => *shape,
        }
    }

    /// Returns the exact logical epoch supplied for queue construction.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { batch, .. } => {
                batch.step().scheduled_dispatch().epoch()
            }
            M1PhysicalQueueCreateFailureStateV1::Terminal { step, .. } => {
                step.scheduled_dispatch().epoch()
            }
        }
    }

    /// Returns the exact scheduler dispatch retained by this failure.
    #[must_use = "scheduler dispatch authority remains retained by the failure"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { batch, .. } => {
                batch.step().scheduled_dispatch()
            }
            M1PhysicalQueueCreateFailureStateV1::Terminal { step, .. } => step.scheduled_dispatch(),
        }
    }

    /// Returns terminal Ferric custody without exposing either raw owner.
    #[must_use = "terminal Ferric custody remains inert and retained"]
    pub const fn terminal_custody(&self) -> Option<&M1PhysicalQueueBatchCustodyV1> {
        match &self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { .. } => None,
            M1PhysicalQueueCreateFailureStateV1::Terminal { custody, .. } => Some(custody),
        }
    }

    /// Recovers the exact recombined prepublication input after pure rejection.
    #[must_use = "pure rejection recovery returns the exact opaque queue input"]
    pub fn into_rejected_input(self) -> Option<M1PrepublicationBatchV1<'a>> {
        match self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { batch, .. } => Some(*batch),
            M1PhysicalQueueCreateFailureStateV1::Terminal { .. } => None,
        }
    }
}

/// Terminal consuming transition failure with generic quarantine and Ferric custody.
#[must_use = "terminal failure retains generic quarantine and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    step: Box<M1PrepublicationStepCustodyV1>,
    lower: ServiceQueueOperationFailureV1,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

impl M1PhysicalQueueOperationFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the logical epoch retained from the failed queue generation.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Returns the exact scheduler dispatch retained after terminal queue failure.
    #[must_use = "scheduler dispatch authority remains retained by quarantine"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Returns the terminal phase classification.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Quarantined
    }

    /// Returns the exact generic operation error without discarding custody.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        self.lower.error()
    }

    /// Returns the exact Ferric custody retained beside generic quarantine.
    #[must_use = "Ferric custody remains retained by terminal failure"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

/// Detached generic queue custody paired with the exact former M1 batch custody.
///
/// ```compile_fail
/// use ferric_engine::M1PhysicalDetachedQueueSessionV1;
/// fn split(detached: M1PhysicalDetachedQueueSessionV1) {
///     let _ = detached.into_parts();
/// }
/// ```
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalDetachedQueueCaseV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl M1PhysicalDetachedQueueCaseV1 {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Returns the logical epoch retained from the detached generation.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Returns the exact scheduler dispatch retained from the detached generation.
    #[must_use = "scheduler dispatch authority remains retained by the detached queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Returns the completed generic dispatch generation retained by detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        self.lower.detached_dispatch_generation()
    }
}

/// Closed former M1 shape retained after exact queue detachment.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub enum M1PhysicalDetachedQueueSessionV1 {
    /// Detached target-only queue.
    TargetOnly(Box<M1PhysicalDetachedQueueCaseV1>),
    /// Detached paired-prefill queue.
    PairedPrefill(Box<M1PhysicalDetachedQueueCaseV1>),
    /// Detached K4 speculative queue.
    SpeculativeK4(Box<M1PhysicalDetachedQueueCaseV1>),
    /// Detached K8 speculative queue.
    SpeculativeK8(Box<M1PhysicalDetachedQueueCaseV1>),
    /// Detached K16 speculative queue.
    SpeculativeK16(Box<M1PhysicalDetachedQueueCaseV1>),
}

impl M1PhysicalDetachedQueueSessionV1 {
    /// Returns the exact former M1 publication shape.
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

    /// Returns the detached phase classification.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::Detached
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the logical epoch retained from the detached generation.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.queue_epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained from the detached generation.
    #[must_use = "scheduler dispatch authority remains retained by the detached queue"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Returns the completed generic dispatch generation retained by detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.detached_dispatch_generation(),
        }
    }
}

/// Terminal queue-release failure retaining the lower failure and Ferric custody.
#[must_use = "terminal release failure retains all available lower and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    step: Box<M1PrepublicationStepCustodyV1>,
    lower: ServiceQueueReleaseFailureV1,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

impl M1PhysicalQueueReleaseFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the logical epoch retained from the failed queue generation.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Returns the exact scheduler dispatch retained after terminal release failure.
    #[must_use = "scheduler dispatch authority remains retained by the release failure"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Returns the lower release failure without discarding retained ownership.
    #[must_use = "the lower terminal failure retains available generic custody"]
    pub const fn lower(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.lower
    }

    /// Returns the exact Ferric custody retained beside release failure.
    #[must_use = "Ferric custody remains retained by terminal failure"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

enum CreateCaseResultV1<'a, const N: usize> {
    Ready(Box<M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>>),
    Rejected {
        error: ServiceQueueErrorV1,
        batch: Box<M1PhysicalFixedBatchCaseV1<'a, N>>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
    Terminal {
        error: ServiceQueueErrorV1,
        custody: Box<M1PhysicalQueueBatchCustodyV1>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
}

fn create_case<const N: usize>(
    ring_bytes: u32,
    step: M1PrepublicationStepCustodyV1,
    case: M1PhysicalFixedBatchCaseV1<'_, N>,
) -> CreateCaseResultV1<'_, N> {
    let (batch, custody) = case.into_parts();
    let (allocations, queue_custody) = custody.into_queue_creation_parts();
    match ServiceQueueSessionV1::create(allocations, ring_bytes, batch) {
        Ok(lower) => CreateCaseResultV1::Ready(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower,
            queue_custody,
            step,
        ))),
        Err(ServiceQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch,
        }) => CreateCaseResultV1::Rejected {
            error,
            batch: Box::new(M1PhysicalFixedBatchCaseV1::from_parts(
                *batch,
                M1PhysicalFixedBatchCustodyV1::from_rejected_queue_creation(
                    *allocations,
                    queue_custody,
                ),
            )),
            step: Box::new(step),
        },
        Err(ServiceQueueCreateFailureV1::Terminal { error }) => CreateCaseResultV1::Terminal {
            error,
            custody: Box::new(queue_custody),
            step: Box::new(step),
        },
    }
}

fn finish_target_only_create(
    result: CreateCaseResultV1<'_, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::TargetOnly(case)),
        CreateCaseResultV1::Rejected { error, batch, step } => {
            Err(M1PhysicalQueueCreateFailureV1::rejected(
                error,
                M1PrepublicationBatchV1 {
                    batch: M1PhysicalFixedBatchV1::TargetOnly(batch),
                    step: *step,
                },
            ))
        }
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::terminal(
            error,
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            step,
            custody,
        )),
    }
}

fn finish_paired_prefill_create(
    result: CreateCaseResultV1<'_, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::PairedPrefill(case)),
        CreateCaseResultV1::Rejected { error, batch, step } => {
            Err(M1PhysicalQueueCreateFailureV1::rejected(
                error,
                M1PrepublicationBatchV1 {
                    batch: M1PhysicalFixedBatchV1::PairedPrefill(batch),
                    step: *step,
                },
            ))
        }
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::terminal(
            error,
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            step,
            custody,
        )),
    }
}

fn finish_speculative_k4_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK4(case)),
        CreateCaseResultV1::Rejected { error, batch, step } => {
            Err(M1PhysicalQueueCreateFailureV1::rejected(
                error,
                M1PrepublicationBatchV1 {
                    batch: M1PhysicalFixedBatchV1::SpeculativeK4(batch),
                    step: *step,
                },
            ))
        }
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::terminal(
            error,
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            step,
            custody,
        )),
    }
}

fn finish_speculative_k8_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK8(case)),
        CreateCaseResultV1::Rejected { error, batch, step } => {
            Err(M1PhysicalQueueCreateFailureV1::rejected(
                error,
                M1PrepublicationBatchV1 {
                    batch: M1PhysicalFixedBatchV1::SpeculativeK8(batch),
                    step: *step,
                },
            ))
        }
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::terminal(
            error,
            M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            step,
            custody,
        )),
    }
}

fn finish_speculative_k16_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK16(case)),
        CreateCaseResultV1::Rejected { error, batch, step } => {
            Err(M1PhysicalQueueCreateFailureV1::rejected(
                error,
                M1PrepublicationBatchV1 {
                    batch: M1PhysicalFixedBatchV1::SpeculativeK16(batch),
                    step: *step,
                },
            ))
        }
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::terminal(
            error,
            M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            step,
            custody,
        )),
    }
}

fn submit_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1PhysicalQueuePhaseCaseV1<ServicePublishedQueueSessionV1<N>>>,
    M1PhysicalQueueOperationFailureV1,
> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.submit() {
        Ok(lower) => Ok(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(lower) => Err(operation_failure(shape, step, lower, custody)),
    }
}

fn wait_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServicePublishedQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
    polls: u32,
) -> Result<
    Box<M1PhysicalQueuePhaseCaseV1<ServiceCompletedQueueSessionV1<N>>>,
    M1PhysicalQueueOperationFailureV1,
> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.wait(polls) {
        Ok(lower) => Ok(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(lower) => Err(operation_failure(shape, step, lower, custody)),
    }
}

fn recycle_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceCompletedQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    M1PhysicalQueueOperationFailureV1,
> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.recycle() {
        Ok(lower) => Ok(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(lower) => Err(operation_failure(shape, step, lower, custody)),
    }
}

fn detach_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<Box<M1PhysicalDetachedQueueCaseV1>, M1PhysicalQueueOperationFailureV1> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.detach() {
        Ok(lower) => Ok(Box::new(M1PhysicalDetachedQueueCaseV1 {
            lower,
            custody,
            step,
        })),
        Err(lower) => Err(operation_failure(shape, step, lower, custody)),
    }
}

fn release_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.destroy_and_release() {
        Ok(observation) => Ok(observation),
        Err(lower) => Err(M1PhysicalQueueReleaseFailureV1 {
            shape,
            step: Box::new(step),
            lower,
            custody: Box::new(custody),
        }),
    }
}

impl M1PhysicalQueueSessionV1 {
    /// Privately transfers the prepublication owner's allocation session into a queue.
    ///
    /// Raw scheduler and fixed-batch inputs cannot enter this boundary.
    ///
    /// ```compile_fail
    /// use fe2o3_service_host::ServiceAllocationSessionV1;
    /// use ferric_engine::{M1PhysicalQueueSessionV1, M1PrepublicationBatchV1};
    /// fn raw_create(
    ///     allocations: ServiceAllocationSessionV1,
    ///     batch: M1PrepublicationBatchV1<'_>,
    /// ) {
    ///     let _ = M1PhysicalQueueSessionV1::create(allocations, 4096, batch);
    /// }
    /// ```
    ///
    /// Pure validation rejection reconstructs the exact input fixed-batch
    /// variant. Terminal creation failure retains Ferric custody but grants no
    /// retry because the generic inputs may have been consumed.
    ///
    /// # Errors
    ///
    /// Returns [`M1PhysicalQueueCreateFailureV1`] with the generic error and all
    /// ownership that the generic service-host layer can honestly return.
    pub fn create(
        ring_bytes: u32,
        prepublication: M1PrepublicationBatchV1<'_>,
    ) -> Result<Self, M1PhysicalQueueCreateFailureV1<'_>> {
        let M1PrepublicationBatchV1 { batch, step } = prepublication;
        match batch {
            M1PhysicalFixedBatchV1::TargetOnly(case) => {
                finish_target_only_create(create_case(ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::PairedPrefill(case) => {
                finish_paired_prefill_create(create_case(ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK4(case) => {
                finish_speculative_k4_create(create_case(ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK8(case) => {
                finish_speculative_k8_create(create_case(ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK16(case) => {
                finish_speculative_k16_create(create_case(ring_bytes, step, *case))
            }
        }
    }

    /// Publishes the complete M1 fixed batch exactly once.
    ///
    /// This consumes prepared custody, so the same generation cannot be
    /// submitted twice.
    ///
    /// ```compile_fail
    /// use ferric_engine::M1PhysicalQueueSessionV1;
    /// fn submit_twice(queue: M1PhysicalQueueSessionV1) {
    ///     let _published = queue.submit();
    ///     let _published_again = queue.submit();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn submit(
        self,
    ) -> Result<M1PhysicalPublishedQueueSessionV1, M1PhysicalQueueOperationFailureV1> {
        match self {
            Self::TargetOnly(case) => submit_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                .map(M1PhysicalPublishedQueueSessionV1::TargetOnly),
            Self::PairedPrefill(case) => {
                submit_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1PhysicalPublishedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                submit_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1PhysicalPublishedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                submit_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1PhysicalPublishedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                submit_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
                    .map(M1PhysicalPublishedQueueSessionV1::SpeculativeK16)
            }
        }
    }
}

impl M1PhysicalPublishedQueueSessionV1 {
    /// Waits for every exact completion signal using a bounded poll count.
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn wait(
        self,
        polls: u32,
    ) -> Result<M1PhysicalCompletedQueueSessionV1, M1PhysicalQueueOperationFailureV1> {
        match self {
            Self::TargetOnly(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly, polls)
                    .map(M1PhysicalCompletedQueueSessionV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill, polls)
                    .map(M1PhysicalCompletedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4, polls)
                    .map(M1PhysicalCompletedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8, polls)
                    .map(M1PhysicalCompletedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16, polls)
                    .map(M1PhysicalCompletedQueueSessionV1::SpeculativeK16)
            }
        }
    }
}

impl M1PhysicalCompletedQueueSessionV1 {
    /// Recycles every exact completion signal into quiescent queue custody.
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn recycle(
        self,
    ) -> Result<M1PhysicalRecycledQueueSessionV1, M1PhysicalQueueOperationFailureV1> {
        match self {
            Self::TargetOnly(case) => recycle_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                .map(M1PhysicalRecycledQueueSessionV1::TargetOnly),
            Self::PairedPrefill(case) => {
                recycle_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1PhysicalRecycledQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                recycle_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1PhysicalRecycledQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                recycle_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1PhysicalRecycledQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                recycle_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
                    .map(M1PhysicalRecycledQueueSessionV1::SpeculativeK16)
            }
        }
    }
}

fn detach_readback_case<const N: usize>(
    case: Box<M1PhysicalReadbackQueueCaseV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<Box<M1PhysicalReadbackDetachedQueueCaseV1>, M1PhysicalReadbackQueueOperationFailureV1> {
    let (lower, custody) = (*case).into_parts();
    match lower.detach() {
        Ok(lower) => Ok(Box::new(M1PhysicalReadbackDetachedQueueCaseV1 {
            lower,
            custody,
        })),
        Err(lower) => Err(M1PhysicalReadbackQueueOperationFailureV1 {
            shape,
            lower,
            custody: Box::new(custody),
        }),
    }
}

fn release_readback_case<const N: usize>(
    case: Box<M1PhysicalReadbackQueueCaseV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalReadbackQueueReleaseFailureV1> {
    let (lower, custody) = (*case).into_parts();
    match lower.destroy_and_release() {
        Ok(observation) => Ok(observation),
        Err(lower) => Err(M1PhysicalReadbackQueueReleaseFailureV1 {
            shape,
            lower,
            custody: Box::new(custody),
        }),
    }
}

impl M1PhysicalReadbackQueueSessionV1 {
    /// Detaches exact data custody after the completed-readback join.
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn detach(
        self,
    ) -> Result<M1PhysicalReadbackDetachedQueueSessionV1, M1PhysicalReadbackQueueOperationFailureV1>
    {
        match self {
            Self::TargetOnly(case) => {
                detach_readback_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                    .map(M1PhysicalReadbackDetachedQueueSessionV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                detach_readback_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1PhysicalReadbackDetachedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                detach_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1PhysicalReadbackDetachedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                detach_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1PhysicalReadbackDetachedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                detach_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
                    .map(M1PhysicalReadbackDetachedQueueSessionV1::SpeculativeK16)
            }
        }
    }

    /// Destroys the post-readback queue and releases exact allocation storage.
    ///
    /// # Errors
    ///
    /// Returns a terminal failure retaining the lower release failure and
    /// Ferric custody. No retry is claimed.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalReadbackQueueReleaseFailureV1> {
        match self {
            Self::TargetOnly(case) => {
                release_readback_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                release_readback_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

enum ObserveCaseFailureV1<const N: usize> {
    BeforeCopy {
        error: M1CompletionObservationErrorV1,
        case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    },
    AfterCopy {
        error: M1CompletionObservationErrorV1,
        case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
        readback: ServiceCompletedReadbackV1,
    },
}

type ObserveCaseResultV1<const N: usize> =
    Result<Box<M1ObservedCompletionCaseV1<N>>, Box<ObserveCaseFailureV1<N>>>;

fn observe_case<const N: usize>(
    mut case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
) -> ObserveCaseResultV1<N> {
    let output = case.custody.completion_output();
    let output_shape = output.shape();
    let range = output.retained_host_dispatch_range();
    let request = case.lower.completed_read_request(range);
    let readback = match case.lower.read_completed(request) {
        Ok(readback) => readback,
        Err(error) => {
            return Err(Box::new(ObserveCaseFailureV1::BeforeCopy {
                error: M1CompletionObservationErrorV1::Queue(error),
                case,
            }))
        }
    };
    if readback.offset_bytes() != range.offset_bytes() {
        return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
            error: M1CompletionObservationErrorV1::OffsetDrift {
                expected: range.offset_bytes(),
                actual: readback.offset_bytes(),
            },
            case,
            readback,
        }));
    }
    let extent_bytes = u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX);
    if extent_bytes != range.extent_bytes() {
        return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
            error: M1CompletionObservationErrorV1::ExtentDrift {
                expected: range.extent_bytes(),
                actual: extent_bytes,
            },
            case,
            readback,
        }));
    }
    let scheduled = case.step.scheduled_dispatch();
    let image = match observe_m1_completed_output_v1(
        output_shape,
        case.custody.selection(),
        scheduled,
        readback,
    ) {
        Ok(image) => image,
        Err((error, readback)) => {
            return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
                error: M1CompletionObservationErrorV1::Image(error),
                case,
                readback,
            }))
        }
    };
    Ok(Box::new(M1ObservedCompletionCaseV1 { case, image }))
}

fn retain_observation_failure<const N: usize>(
    failure: ObserveCaseFailureV1<N>,
    recycled: fn(
        Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    ) -> M1PhysicalRecycledQueueSessionV1,
    rejected: fn(Box<M1RejectedCompletionCaseV1<N>>) -> M1RejectedCompletionOutputV1,
) -> M1CompletionObservationFailureV1 {
    match failure {
        ObserveCaseFailureV1::BeforeCopy { error, case } => M1CompletionObservationFailureV1 {
            error,
            custody: M1CompletionObservationFailureCustodyV1::Recycled(Box::new(recycled(case))),
        },
        ObserveCaseFailureV1::AfterCopy {
            error,
            case,
            readback,
        } => M1CompletionObservationFailureV1 {
            error,
            custody: M1CompletionObservationFailureCustodyV1::Rejected(Box::new(rejected(
                Box::new(M1RejectedCompletionCaseV1 { case, readback }),
            ))),
        },
    }
}

type CheckObservedCaseResultV1<const N: usize> = Result<
    (
        Box<M1PhysicalReadbackQueueCaseV1<N>>,
        M1CheckedCompletionOutputV1,
        ExactCompletion,
        M1FullStepKvReservationCustodyV1,
    ),
    (
        M1CompletedOutputCheckErrorV1,
        Box<M1ObservedCompletionCaseV1<N>>,
    ),
>;

fn check_observed_case<const N: usize>(
    case: Box<M1ObservedCompletionCaseV1<N>>,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> CheckObservedCaseResultV1<N> {
    let scheduled = case.case.step.scheduled_dispatch();
    if semantics.len() != scheduled.member_count() {
        return Err((
            M1CompletedOutputCheckErrorV1::ExpectationCount {
                expected: scheduled.member_count(),
                actual: semantics.len(),
            },
            case,
        ));
    }
    let mut expectations = Vec::new();
    if expectations.try_reserve_exact(semantics.len()).is_err() {
        return Err((
            M1CompletedOutputCheckErrorV1::Output(crate::M1CompletionOutputErrorV1::ExtentOverflow),
            case,
        ));
    }
    for (lane, semantic) in semantics.iter().copied().enumerate() {
        let Some(plan) = case.case.step.target_plans()[lane].as_ref() else {
            return Err((
                M1CompletedOutputCheckErrorV1::ExpectationCount {
                    expected: scheduled.member_count(),
                    actual: lane,
                },
                case,
            ));
        };
        expectations.push(CompletionWireExpectation::new(plan, semantic));
    }
    let checked = match check_m1_completed_output_v1(
        &case.image,
        case.case.custody.selection(),
        scheduled,
        &expectations,
    ) {
        Ok(checked) => checked,
        Err(error) => return Err((error, case)),
    };
    let M1ObservedCompletionCaseV1 { case, image: _ } = *case;
    let (lower, custody, step) = (*case).into_parts();
    let (scheduled, _target_plans, kv) = step.into_parts();
    let completion = ExactCompletion::from_completed_m1_queue_readback(scheduled);
    Ok((
        Box::new(M1PhysicalReadbackQueueCaseV1 { lower, custody }),
        checked,
        completion,
        kv,
    ))
}

fn qualification_preflight(
    recycled: &M1PhysicalRecycledQueueSessionV1,
) -> Result<(), M1QualificationObservationErrorV1> {
    let M1PhysicalRecycledQueueSessionV1::TargetOnly(case) = recycled else {
        return Err(M1QualificationObservationErrorV1::NotTargetOnly);
    };
    let Some(logits) = case.custody.completion_output().qualification_logits() else {
        return Err(M1QualificationObservationErrorV1::CaptureNotEnabled);
    };
    let active = case.step.target_active_lengths();
    if active.len() != case.step.scheduled_dispatch().member_count() {
        return Err(M1QualificationObservationErrorV1::Logits(
            M1QualificationLogitsErrorV1::LiveLaneCount {
                capacity: case.step.scheduled_dispatch().member_count(),
                actual: active.len(),
            },
        ));
    }
    for (lane, length) in active.enumerate() {
        logits
            .shape()
            .final_row_relative_offset(lane, length)
            .map_err(M1QualificationObservationErrorV1::Logits)?;
    }
    Ok(())
}

fn qualification_preflight_failure(
    error: M1QualificationObservationErrorV1,
    recycled: M1PhysicalRecycledQueueSessionV1,
) -> M1QualificationObservationFailureV1 {
    M1QualificationObservationFailureV1 {
        error,
        custody: M1QualificationObservationFailureCustodyV1::Recycled(Box::new(recycled)),
    }
}

fn qualification_compact_failure(
    failure: M1CompletionObservationFailureV1,
) -> M1QualificationObservationFailureV1 {
    let (error, custody) = failure.into_parts();
    let custody = match custody {
        M1CompletionObservationFailureCustodyV1::Recycled(recycled) => {
            M1QualificationObservationFailureCustodyV1::Recycled(recycled)
        }
        M1CompletionObservationFailureCustodyV1::Rejected(rejected) => {
            M1QualificationObservationFailureCustodyV1::CompactRejected(rejected)
        }
    };
    M1QualificationObservationFailureV1 {
        error: M1QualificationObservationErrorV1::Compact(error),
        custody,
    }
}

fn qualification_observed_failure(
    error: M1QualificationObservationErrorV1,
    completion: M1ObservedCompletionOutputV1,
    partial_logits: Vec<ServiceCompletedReadbackV1>,
) -> M1QualificationObservationFailureV1 {
    M1QualificationObservationFailureV1 {
        error,
        custody: M1QualificationObservationFailureCustodyV1::Observed {
            completion: Box::new(completion),
            partial_logits: partial_logits.into_boxed_slice(),
        },
    }
}

fn finish_qualification_observation(
    mut completion: M1ObservedCompletionOutputV1,
) -> Result<M1ObservedQualificationOutputV1, Box<M1QualificationObservationFailureV1>> {
    let M1ObservedCompletionOutputV1::TargetOnly(case) = &mut completion else {
        return Err(Box::new(qualification_observed_failure(
            M1QualificationObservationErrorV1::NotTargetOnly,
            completion,
            Vec::new(),
        )));
    };
    let Some(logits) = case.case.custody.completion_output().qualification_logits() else {
        return Err(Box::new(qualification_observed_failure(
            M1QualificationObservationErrorV1::CaptureNotEnabled,
            completion,
            Vec::new(),
        )));
    };
    let shape = logits.shape();
    let full_range = logits.retained_host_dispatch_range();
    let active_lengths = case.case.step.target_active_lengths().collect::<Vec<_>>();
    let dispatch_generation = case.image.dispatch_generation();

    let mut compact_raw = Vec::new();
    if compact_raw
        .try_reserve_exact(case.image.raw_bytes().len())
        .is_err()
    {
        return Err(Box::new(qualification_observed_failure(
            M1QualificationObservationErrorV1::HostAllocation,
            completion,
            Vec::new(),
        )));
    }
    compact_raw.extend_from_slice(case.image.raw_bytes());
    let compact_raw_sha256 = *case.image.raw_sha256();

    let mut readbacks = Vec::new();
    if readbacks.try_reserve_exact(active_lengths.len()).is_err() {
        return Err(Box::new(qualification_observed_failure(
            M1QualificationObservationErrorV1::HostAllocation,
            completion,
            readbacks,
        )));
    }
    for (lane, active_length) in active_lengths.iter().copied().enumerate() {
        let relative = match shape.final_row_relative_offset(lane, active_length) {
            Ok(relative) => relative,
            Err(error) => {
                return Err(Box::new(qualification_observed_failure(
                    M1QualificationObservationErrorV1::Logits(error),
                    completion,
                    readbacks,
                )))
            }
        };
        let row_range = match full_range.checked_subrange(
            relative,
            shape.row_bytes(),
            crate::M1_QUALIFICATION_LOGITS_ALIGNMENT_V1,
        ) {
            Ok(range) => range,
            Err(error) => {
                return Err(Box::new(qualification_observed_failure(
                    M1QualificationObservationErrorV1::Logits(
                        M1QualificationLogitsErrorV1::Allocation(error),
                    ),
                    completion,
                    readbacks,
                )))
            }
        };
        let request = case.case.lower.completed_read_request(row_range);
        match case.case.lower.read_completed(request) {
            Ok(readback) => readbacks.push(readback),
            Err(source) => {
                return Err(Box::new(qualification_observed_failure(
                    M1QualificationObservationErrorV1::Queue { lane, source },
                    completion,
                    readbacks,
                )))
            }
        }
    }
    let logits = match observe_m1_qualification_logits_v1(
        shape,
        full_range,
        dispatch_generation,
        &active_lengths,
        readbacks,
    ) {
        Ok(logits) => logits,
        Err((error, readbacks)) => {
            return Err(Box::new(qualification_observed_failure(
                M1QualificationObservationErrorV1::Logits(error),
                completion,
                readbacks,
            )))
        }
    };
    Ok(M1ObservedQualificationOutputV1 {
        completion,
        evidence: M1QualificationCompletionEvidenceV1 {
            compact_raw_bytes: compact_raw.into_boxed_slice(),
            compact_raw_sha256,
            logits,
        },
    })
}

impl M1PhysicalRecycledQueueSessionV1 {
    /// Copies target-only compact K7 output and each final live BF16 logits row.
    ///
    /// Qualification capture must have been explicitly enabled before physical
    /// binding. All shape checks run before the first compact copy. Once any
    /// copy succeeds, failures retain closed observed custody with no method
    /// that can issue either completed read again.
    ///
    /// ```compile_fail
    /// use ferric_engine::M1PhysicalRecycledQueueSessionV1;
    /// fn observe_twice(queue: M1PhysicalRecycledQueueSessionV1) {
    ///     let _first = queue.observe_qualification_completion();
    ///     let _second = queue.observe_qualification_completion();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Rejects non-target or uncaptured queues before copying. Compact or
    /// logits copy, allocation, and shape failures retain phase-local linear
    /// custody and never reopen a copy that may already have succeeded.
    pub fn observe_qualification_completion(
        self,
    ) -> Result<M1ObservedQualificationOutputV1, Box<M1QualificationObservationFailureV1>> {
        if let Err(error) = qualification_preflight(&self) {
            return Err(Box::new(qualification_preflight_failure(error, self)));
        }
        let observed = self
            .observe_completion()
            .map_err(|failure| Box::new(qualification_compact_failure(failure)))?;
        finish_qualification_observation(observed)
    }

    /// Copies and structurally observes the exact K7 output exactly once.
    ///
    /// The generic request is minted from the exact retained host range. A
    /// successful generic read therefore validates the hidden dispatch-data
    /// ordinal through the queue ledger and uses that ordinal for the KFD copy;
    /// Ferric additionally compares the public offset and byte length, decodes
    /// the bounded live records, and requires canonical zero inactive rows.
    /// Scheduler authority and pending KV custody remain retained, and this
    /// transition does not mint [`ExactCompletion`].
    ///
    /// ```compile_fail
    /// use ferric_engine::M1PhysicalRecycledQueueSessionV1;
    /// fn observe_twice(queue: M1PhysicalRecycledQueueSessionV1) {
    ///     let _first = queue.observe_completion();
    ///     let _second = queue.observe_completion();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// A generic read failure returns retryable recycled custody because no copy
    /// succeeded. Every rejection after a successful copy returns closed
    /// rejected-observation custody, so the first copy cannot be repeated. No
    /// completion authority is created on either path.
    pub fn observe_completion(
        self,
    ) -> Result<M1ObservedCompletionOutputV1, M1CompletionObservationFailureV1> {
        match self {
            Self::TargetOnly(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::TargetOnly)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::TargetOnly,
                        M1RejectedCompletionOutputV1::TargetOnly,
                    )
                }),
            Self::PairedPrefill(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::PairedPrefill)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::PairedPrefill,
                        M1RejectedCompletionOutputV1::PairedPrefill,
                    )
                }),
            Self::SpeculativeK4(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK4)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK4,
                        M1RejectedCompletionOutputV1::SpeculativeK4,
                    )
                }),
            Self::SpeculativeK8(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK8)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK8,
                        M1RejectedCompletionOutputV1::SpeculativeK8,
                    )
                }),
            Self::SpeculativeK16(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK16)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK16,
                        M1RejectedCompletionOutputV1::SpeculativeK16,
                    )
                }),
        }
    }

    /// Detaches exact data custody after completion and recycle.
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn detach(
        self,
    ) -> Result<M1PhysicalDetachedQueueSessionV1, M1PhysicalQueueOperationFailureV1> {
        match self {
            Self::TargetOnly(case) => detach_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                .map(M1PhysicalDetachedQueueSessionV1::TargetOnly),
            Self::PairedPrefill(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1PhysicalDetachedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1PhysicalDetachedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1PhysicalDetachedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                detach_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
                    .map(M1PhysicalDetachedQueueSessionV1::SpeculativeK16)
            }
        }
    }

    /// Destroys the native queue, restores exact allocation custody, and releases storage.
    ///
    /// # Errors
    ///
    /// Returns a terminal failure retaining the entire generic release failure
    /// and Ferric custody. No retry is claimed.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        match self {
            Self::TargetOnly(case) => release_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly),
            Self::PairedPrefill(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

impl M1ObservedCompletionOutputV1 {
    /// Consumes the captured bytes through the existing roster and semantic join.
    ///
    /// Success mints the only [`ExactCompletion`] for this scheduler batch.
    /// Failure returns the unchanged observed owner, so corrected semantic
    /// expectations can be retried without another generic completed read.
    ///
    /// # Errors
    ///
    /// Returns retained observation custody for expectation-count, roster,
    /// selection, epoch, wire-identity, or token-semantic rejection.
    pub fn check_completion(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1PhysicalCompletedReadbackV1, M1CompletedReadbackJoinFailureV1> {
        match self {
            Self::TargetOnly(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::TargetOnly,
                M1PhysicalReadbackQueueSessionV1::TargetOnly,
                expectations,
            ),
            Self::PairedPrefill(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::PairedPrefill,
                M1PhysicalReadbackQueueSessionV1::PairedPrefill,
                expectations,
            ),
            Self::SpeculativeK4(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK4,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK4,
                expectations,
            ),
            Self::SpeculativeK8(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK8,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK8,
                expectations,
            ),
            Self::SpeculativeK16(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK16,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK16,
                expectations,
            ),
        }
    }

    /// Destroys the observed queue and releases its exact allocation storage.
    ///
    /// This teardown consumes the inert observation without checking semantics
    /// or minting completion authority.
    ///
    /// # Errors
    ///
    /// Returns terminal release failure retaining all available queue, batch,
    /// scheduler, and KV custody.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        match self {
            Self::TargetOnly(case) => {
                release_observed_case(*case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                release_observed_case(*case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

impl M1RejectedCompletionOutputV1 {
    /// Destroys the queue after a structurally rejected one-shot byte copy.
    ///
    /// This teardown consumes the rejected raw copy without reopening readback
    /// or minting completion authority.
    ///
    /// # Errors
    ///
    /// Returns terminal release failure retaining all available queue, batch,
    /// scheduler, and KV custody.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        match self {
            Self::TargetOnly(case) => {
                release_rejected_case(*case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                release_rejected_case(*case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

fn operation_failure(
    shape: M1PhysicalFixedBatchShapeV1,
    step: M1PrepublicationStepCustodyV1,
    lower: ServiceQueueOperationFailureV1,
    custody: M1PhysicalQueueBatchCustodyV1,
) -> M1PhysicalQueueOperationFailureV1 {
    M1PhysicalQueueOperationFailureV1 {
        shape,
        step: Box::new(step),
        lower,
        custody: Box::new(custody),
    }
}

#[cfg(test)]
mod tests {
    use super::{M1PhysicalQueueCreateFailureClassV1, M1PhysicalQueuePhaseV1};

    #[test]
    fn state_capabilities_are_disjoint_and_fail_closed() {
        let phases = [
            M1PhysicalQueuePhaseV1::Prepared,
            M1PhysicalQueuePhaseV1::Published,
            M1PhysicalQueuePhaseV1::Completed,
            M1PhysicalQueuePhaseV1::Recycled,
            M1PhysicalQueuePhaseV1::Observed,
            M1PhysicalQueuePhaseV1::ReadbackJoined,
            M1PhysicalQueuePhaseV1::Detached,
            M1PhysicalQueuePhaseV1::Quarantined,
        ];
        for phase in phases {
            let grants = usize::from(phase.can_submit())
                + usize::from(phase.can_wait())
                + usize::from(phase.can_recycle())
                + usize::from(phase.can_read_detach_or_release());
            assert!(grants <= 1);
        }
        assert!(M1PhysicalQueuePhaseV1::Prepared.can_submit());
        assert!(M1PhysicalQueuePhaseV1::Published.can_wait());
        assert!(M1PhysicalQueuePhaseV1::Completed.can_recycle());
        assert!(M1PhysicalQueuePhaseV1::Recycled.can_read_detach_or_release());
        assert!(M1PhysicalQueuePhaseV1::Observed.can_detach_or_release());
        assert!(!M1PhysicalQueuePhaseV1::Observed.can_read_detach_or_release());
        assert!(M1PhysicalQueuePhaseV1::ReadbackJoined.can_detach_or_release());
        assert!(!M1PhysicalQueuePhaseV1::ReadbackJoined.can_read_detach_or_release());
        assert_eq!(0, grants_for(M1PhysicalQueuePhaseV1::Detached));
        assert_eq!(0, grants_for(M1PhysicalQueuePhaseV1::Quarantined));
    }

    #[test]
    fn only_pure_creation_rejection_allows_input_recovery() {
        assert!(M1PhysicalQueueCreateFailureClassV1::Rejected.can_recover_inputs());
        assert!(!M1PhysicalQueueCreateFailureClassV1::Rejected.denies_retry());
        assert!(!M1PhysicalQueueCreateFailureClassV1::Terminal.can_recover_inputs());
        assert!(M1PhysicalQueueCreateFailureClassV1::Terminal.denies_retry());
    }

    fn grants_for(phase: M1PhysicalQueuePhaseV1) -> usize {
        usize::from(phase.can_submit())
            + usize::from(phase.can_wait())
            + usize::from(phase.can_recycle())
            + usize::from(phase.can_read_detach_or_release())
    }
}
