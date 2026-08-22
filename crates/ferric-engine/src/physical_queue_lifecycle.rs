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
    QuarantinedServiceQueueV1, ServiceAllocationSessionV1, ServiceCompletedQueueSessionV1,
    ServicePublishedQueueSessionV1, ServiceQueueCreateFailureV1, ServiceQueueErrorV1,
    ServiceQueueOperationFailureV1, ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1,
    ServiceQueueSessionV1, ServiceQueueUnboundSessionV1, ServiceRecycledQueueSessionV1,
};
use ferric_spec::completion::CompletionEpoch;

use crate::completed_readback_join::{check_m1_completed_output_v1, CompletedReadbackMetadataV1};
use crate::{
    CompletionWireExpectation, CompletionWireSemanticExpectation, ExactCompletion,
    M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1, M1FullStepKvReservationCustodyV1,
    M1PhysicalFixedBatchCaseV1, M1PhysicalFixedBatchCustodyV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalFixedBatchV1, M1PrepublicationBatchV1, M1PrepublicationStepCustodyV1,
    M1ScheduledDispatchV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
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
        matches!(self, Self::Recycled | Self::ReadbackJoined)
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
    custody: M1PhysicalFixedBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl<Q> M1PhysicalQueuePhaseCaseV1<Q> {
    const fn new(
        lower: Q,
        custody: M1PhysicalFixedBatchCustodyV1,
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
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
        M1PhysicalFixedBatchCustodyV1,
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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

/// Post-readback generic queue custody with no remaining scheduler dispatch authority.
#[must_use = "post-readback queue custody must be detached, released, or retained"]
pub struct M1PhysicalReadbackQueueCaseV1<const N: usize> {
    lower: ServiceRecycledQueueSessionV1<N>,
    custody: M1PhysicalFixedBatchCustodyV1,
}

impl<const N: usize> M1PhysicalReadbackQueueCaseV1<N> {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the post-readback queue"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    fn into_parts(
        self,
    ) -> (
        ServiceRecycledQueueSessionV1<N>,
        M1PhysicalFixedBatchCustodyV1,
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }
}

/// Post-readback detached generic queue and exact Ferric custody.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalReadbackDetachedQueueCaseV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalFixedBatchCustodyV1,
}

impl M1PhysicalReadbackDetachedQueueCaseV1 {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "both detached owners must remain retained"]
    pub fn into_parts(self) -> (ServiceQueueUnboundSessionV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower, self.custody)
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "both detached owners must remain retained"]
    pub fn into_parts(self) -> (ServiceQueueUnboundSessionV1, M1PhysicalFixedBatchCustodyV1) {
        let case = match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case,
        };
        case.into_parts()
    }
}

/// Terminal post-readback transition failure with available generic quarantine.
#[must_use = "terminal failure retains generic quarantine and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalReadbackQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueOperationFailureV1,
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
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

    /// Consumes the failure into opaque generic quarantine and Ferric custody.
    #[must_use = "both terminal owners must remain retained"]
    pub fn into_parts(self) -> (QuarantinedServiceQueueV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower.into_quarantined(), *self.custody)
    }
}

/// Terminal post-readback release failure retaining every available owner.
#[must_use = "terminal release failure retains lower and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalReadbackQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueReleaseFailureV1,
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
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

    /// Consumes the failure into the lower failure and Ferric custody.
    #[must_use = "both terminal owners must remain retained"]
    pub fn into_parts(self) -> (ServiceQueueReleaseFailureV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower, *self.custody)
    }
}

/// One-shot completed-readback or semantic-join diagnostic.
#[derive(Debug)]
pub enum M1CompletedReadbackJoinErrorV1 {
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
    /// Copied records failed scheduler, padding, wire, or semantic validation.
    Output(M1CompletedOutputCheckErrorV1),
}

impl fmt::Display for M1CompletedReadbackJoinErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completed readback join rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletedReadbackJoinErrorV1 {}

/// Retry-safe one-shot join failure retaining unchanged recycled queue custody.
///
/// No [`ExactCompletion`] exists on this path.
#[must_use = "join failure retains the recycled queue for retry or teardown"]
#[derive(Debug)]
pub struct M1CompletedReadbackJoinFailureV1 {
    error: M1CompletedReadbackJoinErrorV1,
    queue: Box<M1PhysicalRecycledQueueSessionV1>,
}

impl M1CompletedReadbackJoinFailureV1 {
    /// Returns the exact failure without discarding retry-capable queue custody.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    /// Returns the unchanged recycled queue by borrow.
    #[must_use = "recycled queue custody remains retained by this failure"]
    pub const fn queue(&self) -> &M1PhysicalRecycledQueueSessionV1 {
        &self.queue
    }

    /// Recovers the exact failure and unchanged recycled queue.
    #[must_use = "retry-capable recycled queue custody must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedReadbackJoinErrorV1,
        M1PhysicalRecycledQueueSessionV1,
    ) {
        (self.error, *self.queue)
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

/// Queue creation rejection or terminal failure with exact Ferric custody.
#[must_use = "pure rejection retains exact inputs; terminal failure retains Ferric custody"]
pub enum M1PhysicalQueueCreateFailureV1<'a> {
    /// Pre-transfer rejection with unchanged allocation and fixed-batch inputs.
    Rejected {
        /// Exact generic rejection.
        error: ServiceQueueErrorV1,
        /// Unchanged generic allocation session.
        allocations: Box<ServiceAllocationSessionV1>,
        /// Exact reconstructed opaque prepublication batch.
        batch: Box<M1PrepublicationBatchV1<'a>>,
    },
    /// KFD may have consumed the generic inputs; only Ferric custody remains recoverable.
    Terminal {
        /// Exact generic terminal error.
        error: ServiceQueueErrorV1,
        /// Original fixed-batch shape.
        shape: M1PhysicalFixedBatchShapeV1,
        /// Scheduler, plan, and KV authority retained without retry authority.
        step: Box<M1PrepublicationStepCustodyV1>,
        /// Ferric custody retained without retry authority.
        custody: Box<M1PhysicalFixedBatchCustodyV1>,
    },
}

impl fmt::Debug for M1PhysicalQueueCreateFailureV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected { error, batch, .. } => formatter
                .debug_struct("Rejected")
                .field("error", error)
                .field("shape", &batch.shape())
                .field("step", &batch.step())
                .finish_non_exhaustive(),
            Self::Terminal {
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
    /// Classifies recoverable pure rejection versus terminal consumption.
    #[must_use]
    pub const fn class(&self) -> M1PhysicalQueueCreateFailureClassV1 {
        match self {
            Self::Rejected { .. } => M1PhysicalQueueCreateFailureClassV1::Rejected,
            Self::Terminal { .. } => M1PhysicalQueueCreateFailureClassV1::Terminal,
        }
    }

    /// Returns the exact generic error without consuming retained ownership.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        match self {
            Self::Rejected { error, .. } | Self::Terminal { error, .. } => error,
        }
    }

    /// Returns the exact rejected or terminal M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        match self {
            Self::Rejected { batch, .. } => batch.shape(),
            Self::Terminal { shape, .. } => *shape,
        }
    }

    /// Returns the exact logical epoch supplied for queue construction.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        match self {
            Self::Rejected { batch, .. } => batch.step().scheduled_dispatch().epoch(),
            Self::Terminal { step, .. } => step.scheduled_dispatch().epoch(),
        }
    }

    /// Returns the exact scheduler dispatch retained by this failure.
    #[must_use = "scheduler dispatch authority remains retained by the failure"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::Rejected { batch, .. } => batch.step().scheduled_dispatch(),
            Self::Terminal { step, .. } => step.scheduled_dispatch(),
        }
    }

    /// Recovers unchanged inputs only after pure pre-transfer rejection.
    #[must_use = "pure rejection recovery returns both unchanged construction inputs"]
    pub fn into_rejected_inputs(
        self,
    ) -> Option<(ServiceAllocationSessionV1, M1PrepublicationBatchV1<'a>)> {
        match self {
            Self::Rejected {
                allocations, batch, ..
            } => Some((*allocations, *batch)),
            Self::Terminal { .. } => None,
        }
    }

    /// Recovers Ferric custody and scheduler dispatch after terminal creation failure.
    ///
    /// The returned owners grant no retry authority; the generic allocation and
    /// batch owners are unavailable after this failure class.
    #[must_use = "terminal Ferric and scheduler custody must remain retained"]
    pub fn into_terminal_parts(
        self,
    ) -> Option<(M1PhysicalFixedBatchCustodyV1, M1PrepublicationStepCustodyV1)> {
        match self {
            Self::Rejected { .. } => None,
            Self::Terminal { custody, step, .. } => Some((*custody, *step)),
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
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
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

    /// Consumes the failure into opaque generic quarantine and Ferric custody.
    ///
    /// Neither returned value grants retry authority.
    #[must_use = "both terminal owners must remain retained"]
    pub fn into_quarantined_parts(
        self,
    ) -> (
        QuarantinedServiceQueueV1,
        M1PhysicalFixedBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.lower.into_quarantined(), *self.custody, *self.step)
    }
}

/// Detached generic queue custody paired with the exact former M1 batch custody.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalDetachedQueueCaseV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalFixedBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl M1PhysicalDetachedQueueCaseV1 {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "the live generic queue and Ferric custody must both remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        ServiceQueueUnboundSessionV1,
        M1PhysicalFixedBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.lower, self.custody, self.step)
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
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
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

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "the live generic queue and Ferric custody must both remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        ServiceQueueUnboundSessionV1,
        M1PhysicalFixedBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        let case = match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case,
        };
        case.into_parts()
    }
}

/// Terminal queue-release failure retaining the lower failure and Ferric custody.
#[must_use = "terminal release failure retains all available lower and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    step: Box<M1PrepublicationStepCustodyV1>,
    lower: ServiceQueueReleaseFailureV1,
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
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

    /// Consumes the terminal failure into the exact lower failure and Ferric custody.
    ///
    /// The lower failure retains any generic quarantine made available by the
    /// generic service host. Neither part grants retry authority.
    #[must_use = "all terminal release custody must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        ServiceQueueReleaseFailureV1,
        M1PhysicalFixedBatchCustodyV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.lower, *self.custody, *self.step)
    }
}

enum CreateCaseResultV1<'a, const N: usize> {
    Ready(Box<M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>>),
    Rejected {
        error: ServiceQueueErrorV1,
        allocations: Box<ServiceAllocationSessionV1>,
        batch: Box<M1PhysicalFixedBatchCaseV1<'a, N>>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
    Terminal {
        error: ServiceQueueErrorV1,
        custody: Box<M1PhysicalFixedBatchCustodyV1>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
}

fn create_case<const N: usize>(
    allocations: ServiceAllocationSessionV1,
    ring_bytes: u32,
    step: M1PrepublicationStepCustodyV1,
    case: M1PhysicalFixedBatchCaseV1<'_, N>,
) -> CreateCaseResultV1<'_, N> {
    let (batch, custody) = case.into_parts();
    match ServiceQueueSessionV1::create(allocations, ring_bytes, batch) {
        Ok(lower) => CreateCaseResultV1::Ready(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(ServiceQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch,
        }) => CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PhysicalFixedBatchCaseV1::from_parts(*batch, custody)),
            step: Box::new(step),
        },
        Err(ServiceQueueCreateFailureV1::Terminal { error }) => CreateCaseResultV1::Terminal {
            error,
            custody: Box::new(custody),
            step: Box::new(step),
        },
    }
}

fn finish_target_only_create(
    result: CreateCaseResultV1<'_, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::TargetOnly(case)),
        CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PrepublicationBatchV1 {
                batch: M1PhysicalFixedBatchV1::TargetOnly(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Terminal {
            error,
            shape: M1PhysicalFixedBatchShapeV1::TargetOnly,
            step,
            custody,
        }),
    }
}

fn finish_paired_prefill_create(
    result: CreateCaseResultV1<'_, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::PairedPrefill(case)),
        CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PrepublicationBatchV1 {
                batch: M1PhysicalFixedBatchV1::PairedPrefill(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Terminal {
            error,
            shape: M1PhysicalFixedBatchShapeV1::PairedPrefill,
            step,
            custody,
        }),
    }
}

fn finish_speculative_k4_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK4(case)),
        CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PrepublicationBatchV1 {
                batch: M1PhysicalFixedBatchV1::SpeculativeK4(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Terminal {
            error,
            shape: M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            step,
            custody,
        }),
    }
}

fn finish_speculative_k8_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK8(case)),
        CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PrepublicationBatchV1 {
                batch: M1PhysicalFixedBatchV1::SpeculativeK8(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Terminal {
            error,
            shape: M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            step,
            custody,
        }),
    }
}

fn finish_speculative_k16_create(
    result: CreateCaseResultV1<'_, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
) -> Result<M1PhysicalQueueSessionV1, M1PhysicalQueueCreateFailureV1<'_>> {
    match result {
        CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::SpeculativeK16(case)),
        CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PrepublicationBatchV1 {
                batch: M1PhysicalFixedBatchV1::SpeculativeK16(batch),
                step: *step,
            }),
        }),
        CreateCaseResultV1::Terminal {
            error,
            custody,
            step,
        } => Err(M1PhysicalQueueCreateFailureV1::Terminal {
            error,
            shape: M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            step,
            custody,
        }),
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
    /// Consumes one allocation session and the opaque prepublication batch into a queue.
    ///
    /// Raw scheduler and fixed-batch inputs cannot enter this boundary.
    ///
    /// ```compile_fail
    /// use fe2o3_service_host::ServiceAllocationSessionV1;
    /// use ferric_engine::{M1PhysicalFixedBatchV1, M1PhysicalQueueSessionV1, M1ScheduledDispatchV1};
    /// fn raw_create(
    ///     allocations: ServiceAllocationSessionV1,
    ///     scheduled: M1ScheduledDispatchV1,
    ///     batch: M1PhysicalFixedBatchV1<'_>,
    /// ) {
    ///     let _ = M1PhysicalQueueSessionV1::create(allocations, 4096, scheduled, batch);
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
        allocations: ServiceAllocationSessionV1,
        ring_bytes: u32,
        prepublication: M1PrepublicationBatchV1<'_>,
    ) -> Result<Self, M1PhysicalQueueCreateFailureV1<'_>> {
        let M1PrepublicationBatchV1 { batch, step } = prepublication;
        match batch {
            M1PhysicalFixedBatchV1::TargetOnly(case) => {
                finish_target_only_create(create_case(allocations, ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::PairedPrefill(case) => {
                finish_paired_prefill_create(create_case(allocations, ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK4(case) => {
                finish_speculative_k4_create(create_case(allocations, ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK8(case) => {
                finish_speculative_k8_create(create_case(allocations, ring_bytes, step, *case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK16(case) => {
                finish_speculative_k16_create(create_case(allocations, ring_bytes, step, *case))
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

type ReadAndCheckCaseResultV1<const N: usize> = Result<
    (
        Box<M1PhysicalReadbackQueueCaseV1<N>>,
        M1CheckedCompletionOutputV1,
        ExactCompletion,
        M1FullStepKvReservationCustodyV1,
    ),
    (
        M1CompletedReadbackJoinErrorV1,
        Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    ),
>;

fn read_and_check_case<const N: usize>(
    mut case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> ReadAndCheckCaseResultV1<N> {
    let output = case.custody.completion_output();
    let output_shape = output.shape();
    let range = output.retained_host_dispatch_range();
    let request = case.lower.completed_read_request(range);
    let readback = match case.lower.read_completed(request) {
        Ok(readback) => readback,
        Err(error) => return Err((M1CompletedReadbackJoinErrorV1::Queue(error), case)),
    };
    if readback.offset_bytes() != range.offset_bytes() {
        return Err((
            M1CompletedReadbackJoinErrorV1::OffsetDrift {
                expected: range.offset_bytes(),
                actual: readback.offset_bytes(),
            },
            case,
        ));
    }
    let extent_bytes = u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX);
    if extent_bytes != range.extent_bytes() {
        return Err((
            M1CompletedReadbackJoinErrorV1::ExtentDrift {
                expected: range.extent_bytes(),
                actual: extent_bytes,
            },
            case,
        ));
    }
    let metadata = CompletedReadbackMetadataV1 {
        dispatch_generation: readback.dispatch_generation(),
        data_index: readback.data_index(),
        offset_bytes: readback.offset_bytes(),
    };
    let scheduled = case.step.scheduled_dispatch();
    if semantics.len() != scheduled.member_count() {
        return Err((
            M1CompletedReadbackJoinErrorV1::Output(
                M1CompletedOutputCheckErrorV1::ExpectationCount {
                    expected: scheduled.member_count(),
                    actual: semantics.len(),
                },
            ),
            case,
        ));
    }
    let mut expectations = Vec::new();
    if expectations.try_reserve_exact(semantics.len()).is_err() {
        return Err((
            M1CompletedReadbackJoinErrorV1::Output(M1CompletedOutputCheckErrorV1::Output(
                crate::M1CompletionOutputErrorV1::ExtentOverflow,
            )),
            case,
        ));
    }
    for (lane, semantic) in semantics.iter().copied().enumerate() {
        let Some(plan) = case.step.target_plans()[lane].as_ref() else {
            return Err((
                M1CompletedReadbackJoinErrorV1::Output(
                    M1CompletedOutputCheckErrorV1::ExpectationCount {
                        expected: scheduled.member_count(),
                        actual: lane,
                    },
                ),
                case,
            ));
        };
        expectations.push(CompletionWireExpectation::new(plan, semantic));
    }
    let checked = match check_m1_completed_output_v1(
        output_shape,
        case.custody.selection(),
        scheduled,
        metadata,
        readback.bytes(),
        &expectations,
    ) {
        Ok(checked) => checked,
        Err(error) => {
            return Err((M1CompletedReadbackJoinErrorV1::Output(error), case));
        }
    };
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

impl M1PhysicalRecycledQueueSessionV1 {
    /// Reads and semantically joins the exact K7 output exactly once.
    ///
    /// The generic request is minted from the exact retained host range. A
    /// successful generic read therefore validates the hidden dispatch-data
    /// ordinal through the queue ledger and uses that ordinal for the KFD copy;
    /// Ferric additionally compares the public offset and byte length. Every
    /// scheduled live record is then checked in exact request order, and all
    /// remaining capacity records must be canonical zero before this transition
    /// consumes scheduler authority to mint one [`ExactCompletion`].
    ///
    /// ```compile_fail
    /// use ferric_engine::{CompletionWireSemanticExpectation, M1PhysicalRecycledQueueSessionV1};
    /// fn join_twice(
    ///     queue: M1PhysicalRecycledQueueSessionV1,
    ///     expectations: &[CompletionWireSemanticExpectation<'_>],
    /// ) {
    ///     let _first = queue.read_and_check_completion(expectations);
    ///     let _second = queue.read_and_check_completion(expectations);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the unchanged recycled queue for generic read, coordinate,
    /// scheduler-roster, padding, wire, or semantic rejection. No completion
    /// authority is created on failure.
    pub fn read_and_check_completion(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1PhysicalCompletedReadbackV1, M1CompletedReadbackJoinFailureV1> {
        match self {
            Self::TargetOnly(case) => match read_and_check_case(case, expectations) {
                Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::TargetOnly(case),
                    checked,
                    completion,
                    kv,
                }),
                Err((error, case)) => Err(M1CompletedReadbackJoinFailureV1 {
                    error,
                    queue: Box::new(Self::TargetOnly(case)),
                }),
            },
            Self::PairedPrefill(case) => match read_and_check_case(case, expectations) {
                Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::PairedPrefill(case),
                    checked,
                    completion,
                    kv,
                }),
                Err((error, case)) => Err(M1CompletedReadbackJoinFailureV1 {
                    error,
                    queue: Box::new(Self::PairedPrefill(case)),
                }),
            },
            Self::SpeculativeK4(case) => match read_and_check_case(case, expectations) {
                Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::SpeculativeK4(case),
                    checked,
                    completion,
                    kv,
                }),
                Err((error, case)) => Err(M1CompletedReadbackJoinFailureV1 {
                    error,
                    queue: Box::new(Self::SpeculativeK4(case)),
                }),
            },
            Self::SpeculativeK8(case) => match read_and_check_case(case, expectations) {
                Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::SpeculativeK8(case),
                    checked,
                    completion,
                    kv,
                }),
                Err((error, case)) => Err(M1CompletedReadbackJoinFailureV1 {
                    error,
                    queue: Box::new(Self::SpeculativeK8(case)),
                }),
            },
            Self::SpeculativeK16(case) => match read_and_check_case(case, expectations) {
                Ok((case, checked, completion, kv)) => Ok(M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::SpeculativeK16(case),
                    checked,
                    completion,
                    kv,
                }),
                Err((error, case)) => Err(M1CompletedReadbackJoinFailureV1 {
                    error,
                    queue: Box::new(Self::SpeculativeK16(case)),
                }),
            },
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

fn operation_failure(
    shape: M1PhysicalFixedBatchShapeV1,
    step: M1PrepublicationStepCustodyV1,
    lower: ServiceQueueOperationFailureV1,
    custody: M1PhysicalFixedBatchCustodyV1,
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
