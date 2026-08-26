//! Linear Ferric custody around one complete M1 fixed-batch queue generation.
//!
//! The generic service host owns KFD queue state and enforces publication,
//! completion, recycle, readback, detach, and release. This module keeps the
//! corresponding M1 recipe, scheduler-bound target plans, pending KV
//! reservations, and allocation custody beside every generic phase. The
//! completed-readback join checks the exact K7 range against those retained
//! plans before minting completion authority.

use core::fmt;

#[cfg(feature = "qualification-fault-injection")]
use fe2o3_kfd::Gfx942CompletionRecycleObservationV1;
use fe2o3_service_host::{
    ServiceCompletedQueueSessionV1, ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1,
    ServicePublishedQueueSessionV1, ServiceQueueCreateFailureV1, ServiceQueueErrorV1,
    ServiceQueueOperationFailureV1, ServiceQueuePollWithProgressV1, ServiceQueueProgressV1,
    ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1, ServiceQueueSessionV1,
    ServiceQueueUnboundSessionV1, ServiceRecycledQueueSessionV1,
};
#[cfg(feature = "qualification-fault-injection")]
use fe2o3_service_host::{
    ServiceQualificationFaultedQueueSessionV1, ServiceQualificationQueueFaultPointV1,
};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{Qwen3ExecutionMode, M1_MAX_ACTIVE_SEQUENCES};

use crate::completed_readback_join::{
    check_m1_completed_output_v1, check_m1_qualification_completed_output_v1,
};
use crate::direct_diagnostic_choices::observe_m1_direct_diagnostic_choices_v1;
use crate::observed_completion::{
    observe_m1_completed_output_v1, observe_m1_guarded_completed_output_v1,
};
use crate::qualification_logits::{
    observe_m1_qualification_logits_v1, M1QualificationFinalRowChoicesV1,
};
use crate::speculative_diagnostic_choices::observe_m1_speculative_diagnostic_choices_v1;
use crate::{
    preflight_m1_completion_canary_v1, validate_m1_completion_canary_readback_v1,
    CompletionWireExpectation, CompletionWireSemanticExpectation, Engine, ExactCompletion,
    Gfx942DeviceBinding, M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1,
    M1CompletionCanaryErrorV1, M1DirectDiagnosticChoicesErrorV1, M1FullStepKvReservationCustodyV1,
    M1ObservedCompletionImageErrorV1, M1ObservedCompletionImageV1,
    M1ObservedDirectDiagnosticChoicesV1, M1ObservedQualificationLogitsV1,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1PhysicalDispatchRecipeRowV1,
    M1PhysicalFixedBatchCaseV1, M1PhysicalFixedBatchCustodyV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalFixedBatchV1, M1PhysicalQueueBatchCustodyV1, M1PrepublicationBatchV1,
    M1PrepublicationStepCustodyV1, M1QualificationLogitsErrorV1, M1ScheduledDispatchV1,
    M1SpeculativeDiagnosticChoicesErrorV1, M1ValidatedQualificationContextStepV1,
    M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
    M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Stable identity of Ferric's M1 completion-progress liveness policy.
pub const M1_COMPLETION_PROGRESS_WAIT_POLICY_ID_V2: &str = "ferric-m1-completion-progress-wait-v2";

/// Maximum consecutive completion scans that may show no liveness progress.
pub const M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1: u32 = 8_192;

/// Minimum pause between two consecutive pending completion scans.
pub const M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1: u64 = 10_000;

/// Addressless counts retained from one sequential completion-signal scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1CompletionProgressObservationV1 {
    packet_count: u16,
    completed_count: u16,
    pending_count: u16,
    first_pending_batch_index: Option<u16>,
}

impl M1CompletionProgressObservationV1 {
    fn from_service(progress: ServiceQueueProgressV1) -> Self {
        Self {
            packet_count: progress.packet_count(),
            completed_count: progress.completed_count(),
            pending_count: progress.pending_count(),
            first_pending_batch_index: progress.first_pending_batch_index(),
        }
    }

    /// Returns the fixed-batch packet count reported by the scan.
    #[must_use]
    pub const fn packet_count(self) -> u16 {
        self.packet_count
    }

    /// Returns the number of signals observed completed in the scan.
    #[must_use]
    pub const fn completed_count(self) -> u16 {
        self.completed_count
    }

    /// Returns the number of signals observed pending in the scan.
    #[must_use]
    pub const fn pending_count(self) -> u16 {
        self.pending_count
    }

    /// Returns the earliest batch-local index observed pending in the scan.
    #[must_use]
    pub const fn first_pending_batch_index(self) -> Option<u16> {
        self.first_pending_batch_index
    }
}

/// Ferric reason for terminalizing an otherwise live lower queue generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1CompletionProgressWaitTerminalReasonV1 {
    /// The compile-time packet count cannot be represented by the lower progress ABI.
    PacketCountNotRepresentable,
    /// A scan reported a packet count other than the retained fixed-batch shape.
    PacketCountMismatch,
    /// Completed and pending counts did not sum to the exact packet count.
    CountSumMismatch,
    /// A pending result reported no pending signal or omitted its first pending index.
    PendingObservationInvalid,
    /// The first pending index was outside the exact fixed batch.
    FirstPendingIndexOutOfBounds,
    /// A ready result did not report the canonical all-completed observation.
    ReadyObservationInvalid,
    /// Completed-count liveness regressed below its prior high-water mark.
    CompletedCountRegressed,
    /// The bounded number of consecutive scans without progress was exhausted.
    ConsecutiveScansWithoutProgress,
    /// The checked whole-policy scan bound was exhausted.
    TotalScanBoundReached,
    /// The whole-policy scan bound could not be represented.
    TotalScanBoundOverflow,
}

/// Bounded addressless evidence retained when Ferric terminalizes a wait policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1CompletionProgressWaitDiagnosticV1 {
    reason: M1CompletionProgressWaitTerminalReasonV1,
    scans_performed: u32,
    consecutive_scans_without_progress: u32,
    total_scan_bound: Option<u32>,
    completed_count_high_water: u16,
    last_observation: Option<M1CompletionProgressObservationV1>,
}

impl M1CompletionProgressWaitDiagnosticV1 {
    /// Returns the stable policy identity governing this diagnostic.
    #[must_use]
    pub const fn policy_id(&self) -> &'static str {
        M1_COMPLETION_PROGRESS_WAIT_POLICY_ID_V2
    }

    /// Returns the exact terminal policy reason.
    #[must_use]
    pub const fn reason(&self) -> M1CompletionProgressWaitTerminalReasonV1 {
        self.reason
    }

    /// Returns the number of consuming progress scans performed.
    #[must_use]
    pub const fn scans_performed(&self) -> u32 {
        self.scans_performed
    }

    /// Returns the final count of consecutive scans without high-water progress.
    #[must_use]
    pub const fn consecutive_scans_without_progress(&self) -> u32 {
        self.consecutive_scans_without_progress
    }

    /// Returns the checked whole-policy scan bound, when representable.
    #[must_use]
    pub const fn total_scan_bound(&self) -> Option<u32> {
        self.total_scan_bound
    }

    /// Returns the monotonic completed-count liveness high-water mark.
    #[must_use]
    pub const fn completed_count_high_water(&self) -> u16 {
        self.completed_count_high_water
    }

    /// Returns the exact last scan, including a malformed or regressing scan.
    #[must_use]
    pub const fn last_observation(&self) -> Option<M1CompletionProgressObservationV1> {
        self.last_observation
    }
}

/// Derives the checked whole-policy scan bound for one closed M1 shape.
#[must_use]
pub fn m1_completion_progress_total_scan_bound_v1(
    shape: M1PhysicalFixedBatchShapeV1,
) -> Option<u32> {
    checked_completion_progress_total_scan_bound(
        shape.packet_count(),
        M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
    )
}

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
    /// A deliberate qualification transition consumed recycled custody; only release remains.
    #[cfg(feature = "qualification-fault-injection")]
    QualificationFaulted,
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

    /// Whether this qualification-only phase grants returning teardown and no other transition.
    #[cfg(feature = "qualification-fault-injection")]
    #[must_use]
    pub const fn can_release_only(self) -> bool {
        matches!(self, Self::QualificationFaulted)
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

/// Ferric classification of a rejected deliberate queue-transition fault.
#[cfg(feature = "qualification-fault-injection")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1QualificationQueueTransitionFaultInjectionRejectionReasonV1 {
    /// The Engine was already terminal before the qualification transition.
    EngineAlreadyFaulted,
    /// A completed-read attempt preceded the requested fault point.
    CompletedReadAlreadyAttempted,
}

/// Pure fault-injection rejection retaining the unchanged recycled M1 queue.
#[cfg(feature = "qualification-fault-injection")]
#[must_use = "rejection retains exact recycled queue custody"]
#[derive(Debug)]
pub struct M1QualificationQueueTransitionFaultInjectionRejectionV1 {
    reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1,
    queue: Box<M1PhysicalRecycledQueueSessionV1>,
}

#[cfg(feature = "qualification-fault-injection")]
impl M1QualificationQueueTransitionFaultInjectionRejectionV1 {
    /// Returns why the deliberate transition was rejected without faulting the Engine.
    #[must_use]
    pub const fn reason(&self) -> M1QualificationQueueTransitionFaultInjectionRejectionReasonV1 {
        self.reason
    }

    /// Recovers the exact unchanged recycled M1 queue.
    #[must_use = "recycled queue custody must remain retained"]
    pub fn into_queue(self) -> M1PhysicalRecycledQueueSessionV1 {
        *self.queue
    }
}

/// Terminal M1 queue after a deliberate post-recycle qualification transition.
///
/// The lower service owner and Ferric scheduler/KV custody remain paired. This
/// type exposes returning teardown but no readback, reuse, detach, or submit
/// transition. It is not evidence of a native KFD or device fault.
///
/// ```compile_fail
/// use ferric_engine::M1QualificationQueueTransitionFaultSessionV1;
/// fn read(queue: M1QualificationQueueTransitionFaultSessionV1) {
///     let _ = queue.observe_completion();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1QualificationQueueTransitionFaultSessionV1;
/// fn detach(queue: M1QualificationQueueTransitionFaultSessionV1) {
///     let _ = queue.detach();
/// }
/// ```
#[cfg(feature = "qualification-fault-injection")]
#[must_use = "the qualification-faulted queue must be destroyed or retained"]
#[derive(Debug)]
pub enum M1QualificationQueueTransitionFaultSessionV1 {
    /// Faulted target-only queue.
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQualificationFaultedQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Faulted paired-prefill queue.
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQualificationFaultedQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Faulted K4 speculative queue.
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQualificationFaultedQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Faulted K8 speculative queue.
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQualificationFaultedQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    /// Faulted K16 speculative queue.
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceQualificationFaultedQueueSessionV1<
                    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
                >,
            >,
        >,
    ),
}

#[cfg(feature = "qualification-fault-injection")]
impl M1QualificationQueueTransitionFaultSessionV1 {
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

    /// Returns the compile-time packet cardinality of the retained queue.
    #[must_use]
    pub const fn packet_count(&self) -> usize {
        self.shape().packet_count()
    }

    /// Returns the terminal qualification-only queue phase.
    #[must_use]
    pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
        M1PhysicalQueuePhaseV1::QualificationFaulted
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

    /// Returns the lower dispatch generation at the deliberate transition.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        match self {
            Self::TargetOnly(case) => case.lower.dispatch_generation(),
            Self::PairedPrefill(case) => case.lower.dispatch_generation(),
            Self::SpeculativeK4(case) => case.lower.dispatch_generation(),
            Self::SpeculativeK8(case) => case.lower.dispatch_generation(),
            Self::SpeculativeK16(case) => case.lower.dispatch_generation(),
        }
    }

    /// Returns the deliberate lower service transition point.
    #[must_use]
    pub const fn fault_point(&self) -> ServiceQualificationQueueFaultPointV1 {
        match self {
            Self::TargetOnly(case) => case.lower.point(),
            Self::PairedPrefill(case) => case.lower.point(),
            Self::SpeculativeK4(case) => case.lower.point(),
            Self::SpeculativeK8(case) => case.lower.point(),
            Self::SpeculativeK16(case) => case.lower.point(),
        }
    }

    /// Returns the exact lower recycle observation preceding the transition.
    #[must_use]
    pub const fn recycle_observation(&self) -> Gfx942CompletionRecycleObservationV1 {
        match self {
            Self::TargetOnly(case) => case.lower.recycle_observation(),
            Self::PairedPrefill(case) => case.lower.recycle_observation(),
            Self::SpeculativeK4(case) => case.lower.recycle_observation(),
            Self::SpeculativeK8(case) => case.lower.recycle_observation(),
            Self::SpeculativeK16(case) => case.lower.recycle_observation(),
        }
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains paired with the terminal queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => case.custody(),
            Self::PairedPrefill(case) => case.custody(),
            Self::SpeculativeK4(case) => case.custody(),
            Self::SpeculativeK8(case) => case.custody(),
            Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Returns the exact scheduler dispatch retained beside the terminal queue.
    #[must_use = "scheduler dispatch authority remains paired with physical custody"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.scheduled_dispatch(),
        }
    }

    /// Destroys the native queue through the qualification state's only transition.
    ///
    /// # Errors
    ///
    /// Returns terminal lower release failure with all available Ferric custody.
    pub fn destroy_and_release(
        self,
    ) -> Result<
        M1QualificationQueueTransitionFaultTeardownSuccessV1,
        Box<M1QualificationQueueTransitionFaultTeardownFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => {
                release_qualification_fault_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                release_qualification_fault_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_qualification_fault_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_qualification_fault_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_qualification_fault_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

/// Successful returning teardown of a deliberately faulted qualification queue.
#[cfg(feature = "qualification-fault-injection")]
#[must_use]
#[derive(Debug)]
pub struct M1QualificationQueueTransitionFaultTeardownSuccessV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    queue_epoch: CompletionEpoch,
    dispatch_generation: u64,
    fault_point: ServiceQualificationQueueFaultPointV1,
    release: ServiceQueueReleaseObservationV1,
}

#[cfg(feature = "qualification-fault-injection")]
impl M1QualificationQueueTransitionFaultTeardownSuccessV1 {
    /// Returns the exact former M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.queue_epoch
    }

    /// Returns the lower dispatch generation at injection.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Returns the deliberate lower service transition point.
    #[must_use]
    pub const fn fault_point(&self) -> ServiceQualificationQueueFaultPointV1 {
        self.fault_point
    }

    /// Returns the exact native queue and allocation release observation.
    #[must_use]
    pub const fn release(&self) -> &ServiceQueueReleaseObservationV1 {
        &self.release
    }
}

/// Terminal release failure retaining lower and Ferric qualification custody.
#[cfg(feature = "qualification-fault-injection")]
#[must_use = "release failure retains all available lower and Ferric custody"]
#[derive(Debug)]
pub struct M1QualificationQueueTransitionFaultTeardownFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    queue_epoch: CompletionEpoch,
    dispatch_generation: u64,
    fault_point: ServiceQualificationQueueFaultPointV1,
    lower: ServiceQueueReleaseFailureV1,
    step: Box<M1PrepublicationStepCustodyV1>,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
}

#[cfg(feature = "qualification-fault-injection")]
impl M1QualificationQueueTransitionFaultTeardownFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Returns the immutable scheduler-issued logical epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.queue_epoch
    }

    /// Returns the lower dispatch generation at injection.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Returns the deliberate lower service transition point.
    #[must_use]
    pub const fn fault_point(&self) -> ServiceQualificationQueueFaultPointV1 {
        self.fault_point
    }

    /// Returns the lower terminal release failure by borrow.
    #[must_use = "lower failure retains available generic custody"]
    pub const fn lower(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.lower
    }

    /// Returns the exact scheduler dispatch retained after release failure.
    #[must_use = "scheduler dispatch authority remains retained"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "Ferric custody remains retained"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }
}

/// One exact recycled queue generation paired with its completed output copy.
///
/// Ordinary backing owns the copied K7 image. Guarded backing privately owns
/// the enclosing snapshot while exposing only its K7 interior. The carrier
/// intentionally has no completed-read method and is not `Clone`.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1CompletionEvidenceJoinAuthorityV1 {
    Generic,
    DirectDiagnostic,
    SpeculativeDiagnostic,
}

fn join_observed_output_case<const N: usize>(
    case: Box<M1ObservedCompletionCaseV1<N>>,
    observed: fn(Box<M1ObservedCompletionCaseV1<N>>) -> M1ObservedCompletionOutputV1,
    readback: fn(Box<M1PhysicalReadbackQueueCaseV1<N>>) -> M1PhysicalReadbackQueueSessionV1,
    authority: M1CompletionEvidenceJoinAuthorityV1,
    expectations: &[CompletionWireSemanticExpectation<'_>],
) -> Result<M1PhysicalCompletedReadbackV1, M1CompletedReadbackJoinFailureV1> {
    match check_observed_case(case, authority, expectations) {
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

fn release_observed_case_retaining_image<const N: usize>(
    case: M1ObservedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    (
        ServiceQueueReleaseObservationV1,
        M1ObservedCompletionImageV1,
    ),
    Box<(M1PhysicalQueueReleaseFailureV1, M1ObservedCompletionImageV1)>,
> {
    let M1ObservedCompletionCaseV1 { case, image } = case;
    match release_case(case, shape) {
        Ok(released) => Ok((released, image)),
        Err(source) => Err(Box::new((source, image))),
    }
}

fn release_rejected_case_retaining_readback<const N: usize>(
    case: M1RejectedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    (ServiceQueueReleaseObservationV1, ServiceCompletedReadbackV1),
    Box<(M1PhysicalQueueReleaseFailureV1, ServiceCompletedReadbackV1)>,
> {
    let M1RejectedCompletionCaseV1 { case, readback } = case;
    match release_case(case, shape) {
        Ok(released) => Ok((released, readback)),
        Err(source) => Err(Box::new((source, readback))),
    }
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

    pub(crate) const fn qualification_logits_enabled(&self) -> bool {
        match self {
            Self::TargetOnly(case) => case
                .case
                .custody
                .completion_output()
                .qualification_logits()
                .is_some(),
            Self::PairedPrefill(case) => case
                .case
                .custody
                .completion_output()
                .qualification_logits()
                .is_some(),
            Self::SpeculativeK4(case) => case
                .case
                .custody
                .completion_output()
                .qualification_logits()
                .is_some(),
            Self::SpeculativeK8(case) => case
                .case
                .custody
                .completion_output()
                .qualification_logits()
                .is_some(),
            Self::SpeculativeK16(case) => case
                .case
                .custody
                .completion_output()
                .qualification_logits()
                .is_some(),
        }
    }

    pub(crate) const fn direct_diagnostic_choices_enabled(&self) -> bool {
        match self {
            Self::TargetOnly(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
            Self::PairedPrefill(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK4(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK8(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK16(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
        }
    }

    pub(crate) const fn speculative_diagnostic_choices_enabled(&self) -> bool {
        match self {
            Self::TargetOnly(case) => case
                .case
                .custody
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
            Self::PairedPrefill(case) => case
                .case
                .custody
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK4(case) => case
                .case
                .custody
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK8(case) => case
                .case
                .custody
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
            Self::SpeculativeK16(case) => case
                .case
                .custody
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
        }
    }

    /// Copies the independently retained final active-row target choice for
    /// every live direct lane.
    ///
    /// The attachment must have been enabled before physical binding. Target
    /// active lengths come from the queue-retained pending KV reservations, not
    /// from compact K7 output. This admits `TargetOnly` and `PairedPrefill` only.
    ///
    /// # Errors
    ///
    /// Rejects another queue shape, absent attachment, active-row drift,
    /// completed-copy failure, generation/offset/extent drift, or an
    /// out-of-vocabulary device choice. Once any scalar copy succeeds, this
    /// failure exposes no observation retry.
    pub fn observe_direct_diagnostic_choices(
        mut self,
    ) -> Result<M1ObservedDirectDiagnosticOutputV1, Box<M1DirectDiagnosticObservationFailureV1>>
    {
        let mut active_lengths = [0_u32; M1_MAX_ACTIVE_SEQUENCES as usize];
        let mut ranges = [None; M1_MAX_ACTIVE_SEQUENCES as usize];
        let prepared = match &self {
            Self::TargetOnly(case) => {
                prepare_direct_diagnostic_ranges(case, &mut active_lengths, &mut ranges)
            }
            Self::PairedPrefill(case) => {
                prepare_direct_diagnostic_ranges(case, &mut active_lengths, &mut ranges)
            }
            _ => {
                return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                    error: M1DirectDiagnosticObservationErrorV1::NotDirectShape,
                    completion: Box::new(self),
                    partial_choices: Box::new([]),
                }))
            }
        };
        let (live, generation) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                    error,
                    completion: Box::new(self),
                    partial_choices: Box::new([]),
                }))
            }
        };
        let mut readbacks = Vec::new();
        if readbacks.try_reserve_exact(live).is_err() {
            return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                error: M1DirectDiagnosticObservationErrorV1::HostAllocation,
                completion: Box::new(self),
                partial_choices: Box::new([]),
            }));
        }
        for (lane, range) in ranges.iter().copied().take(live).enumerate() {
            let Some(range) = range else {
                return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                    error: M1DirectDiagnosticObservationErrorV1::PreparedRangeMissing { lane },
                    completion: Box::new(self),
                    partial_choices: readbacks.into_boxed_slice(),
                }));
            };
            let result = match &mut self {
                Self::TargetOnly(case) => {
                    let request = case.case.lower.completed_read_request(range);
                    case.case.lower.read_completed(request)
                }
                Self::PairedPrefill(case) => {
                    let request = case.case.lower.completed_read_request(range);
                    case.case.lower.read_completed(request)
                }
                _ => unreachable!("direct preflight retains a direct shape"),
            };
            match result {
                Ok(readback) => readbacks.push(readback),
                Err(source) => {
                    return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                        error: M1DirectDiagnosticObservationErrorV1::Queue { lane, source },
                        completion: Box::new(self),
                        partial_choices: readbacks.into_boxed_slice(),
                    }))
                }
            }
        }
        let owner = match &self {
            Self::TargetOnly(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices(),
            Self::PairedPrefill(case) => case
                .case
                .custody
                .completion_output()
                .direct_diagnostic_choices(),
            _ => None,
        };
        let Some(owner) = owner else {
            return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                error: M1DirectDiagnosticObservationErrorV1::CaptureNotEnabled,
                completion: Box::new(self),
                partial_choices: readbacks.into_boxed_slice(),
            }));
        };
        let choices = match observe_m1_direct_diagnostic_choices_v1(
            owner,
            generation,
            &active_lengths[..live],
            readbacks,
        ) {
            Ok(choices) => choices,
            Err((error, partial_choices)) => {
                return Err(Box::new(M1DirectDiagnosticObservationFailureV1 {
                    error: M1DirectDiagnosticObservationErrorV1::Choices(error),
                    completion: Box::new(self),
                    partial_choices: partial_choices.into_boxed_slice(),
                }))
            }
        };
        Ok(M1ObservedDirectDiagnosticOutputV1 {
            completion: self,
            choices,
        })
    }

    /// Copies the exact four draft and five target choice scalars after the
    /// same S1/K4 queue generation has already produced its compact image.
    ///
    /// The diagnostic attachment must have been enabled before physical
    /// binding. This transition is unavailable to target-only qualification
    /// and every speculative shape other than S1/K4.
    ///
    /// # Errors
    ///
    /// Rejects another queue shape, absent attachment, completed-copy failure,
    /// generation/offset/extent drift, or an out-of-vocabulary device choice.
    /// Once a choice copy succeeds, failure custody exposes no observation
    /// retry.
    pub fn observe_speculative_k4_diagnostic_choices(
        mut self,
    ) -> Result<
        M1ObservedSpeculativeDiagnosticOutputV1,
        Box<M1SpeculativeDiagnosticObservationFailureV1>,
    > {
        let (draft_ranges, target_range, generation) = match &self {
            Self::SpeculativeK4(case) => {
                let Some(choices) = case
                    .case
                    .custody
                    .completion_output()
                    .speculative_diagnostic_choices()
                else {
                    return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                        custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                            error: M1SpeculativeDiagnosticObservationErrorV1::CaptureNotEnabled,
                            completion: Box::new(self),
                            partial_choices: Box::new([]),
                        },
                    }));
                };
                let draft_ranges = match choices.retained_draft_read_ranges() {
                    Ok(ranges) => ranges,
                    Err(error) => {
                        return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                            custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                                error: M1SpeculativeDiagnosticObservationErrorV1::Choices(error),
                                completion: Box::new(self),
                                partial_choices: Box::new([]),
                            },
                        }));
                    }
                };
                (
                    draft_ranges,
                    choices.retained_target_range(),
                    case.image.dispatch_generation(),
                )
            }
            _ => {
                return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                    custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                        error: M1SpeculativeDiagnosticObservationErrorV1::NotSpeculativeK4,
                        completion: Box::new(self),
                        partial_choices: Box::new([]),
                    },
                }))
            }
        };
        let (backend, draft, target) = match read_m1_diagnostic_choice_ranges_v1(
            M1ProductionDiagnosticChoiceReadBackendV1 { completion: self },
            draft_ranges,
            target_range,
        ) {
            Ok(copies) => copies,
            Err(failure) => {
                return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                    custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Read(failure),
                }));
            }
        };
        self = backend.completion;
        let choices_owner = match &self {
            Self::SpeculativeK4(case) => {
                let Some(owner) = case
                    .case
                    .custody
                    .completion_output()
                    .speculative_diagnostic_choices()
                else {
                    return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                        custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                            error: M1SpeculativeDiagnosticObservationErrorV1::CaptureNotEnabled,
                            completion: Box::new(self),
                            partial_choices: retain_all_m1_diagnostic_choice_copies(draft, target),
                        },
                    }));
                };
                owner
            }
            _ => {
                return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                    custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                        error: M1SpeculativeDiagnosticObservationErrorV1::NotSpeculativeK4,
                        completion: Box::new(self),
                        partial_choices: retain_all_m1_diagnostic_choice_copies(draft, target),
                    },
                }))
            }
        };
        let choices = match observe_m1_speculative_diagnostic_choices_v1(
            choices_owner,
            generation,
            draft,
            target,
        ) {
            Ok(choices) => choices,
            Err(failure) => {
                let (error, draft, target) = *failure;
                return Err(Box::new(M1SpeculativeDiagnosticObservationFailureV1 {
                    custody: M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                        error: M1SpeculativeDiagnosticObservationErrorV1::Choices(error),
                        completion: Box::new(self),
                        partial_choices: retain_all_m1_diagnostic_choice_copies(draft, target),
                    },
                }));
            }
        };
        Ok(M1ObservedSpeculativeDiagnosticOutputV1 {
            completion: self,
            choices,
        })
    }
}

fn prepare_direct_diagnostic_ranges<const N: usize>(
    case: &M1ObservedCompletionCaseV1<N>,
    active_lengths: &mut [u32; M1_MAX_ACTIVE_SEQUENCES as usize],
    ranges: &mut [Option<ServiceHostDispatchRangeV1>; M1_MAX_ACTIVE_SEQUENCES as usize],
) -> Result<(usize, u64), M1DirectDiagnosticObservationErrorV1> {
    let Some(owner) = case
        .case
        .custody
        .completion_output()
        .direct_diagnostic_choices()
    else {
        return Err(M1DirectDiagnosticObservationErrorV1::CaptureNotEnabled);
    };
    let active = case.case.step.target_active_lengths();
    let live = active.len();
    let expected = case.case.step.scheduled_dispatch().member_count();
    if live != expected || live > active_lengths.len() {
        return Err(M1DirectDiagnosticObservationErrorV1::LiveLaneCount {
            capacity: active_lengths.len(),
            expected,
            actual: live,
        });
    }
    for (lane, active_length) in active.enumerate() {
        active_lengths[lane] = active_length;
        ranges[lane] = Some(
            owner
                .retained_final_choice_range(lane, active_length)
                .map_err(M1DirectDiagnosticObservationErrorV1::Choices)?,
        );
    }
    Ok((live, case.image.dispatch_generation()))
}

/// One exact recycled queue generation paired with a rejected completed copy.
///
/// Ordinary backing retains the exact K7 image. Guarded backing retains the
/// complete enclosing snapshot, including both adjacent guards, so structural
/// rejection cannot reopen the lower completed-read operation.
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

/// Move-only custody after one completed copy failed structural observation.
///
/// This owner retains the rejected raw copy for diagnosis and can tear down the
/// queue, but has no completed-read or semantic-completion transition. The raw
/// copy may be the enclosing guarded snapshot rather than only compact bytes.
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

#[derive(Debug)]
enum M1CompletionSnapshotReadFailedInnerV1 {
    TargetOnly(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    PairedPrefill(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    SpeculativeK4(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    SpeculativeK8(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    SpeculativeK16(
        Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
}

/// Opaque terminal custody after the enclosing snapshot read was attempted.
///
/// The generic queue may have been poisoned by a memory error. This owner does
/// not expose recycled custody or any retry/read transition; it supports only
/// fail-closed queue teardown.
///
/// ```compile_fail
/// use ferric_engine::M1CompletionSnapshotReadFailedOutputV1;
/// fn retry(failed: M1CompletionSnapshotReadFailedOutputV1) {
///     let _ = failed.observe_completion();
/// }
/// ```
#[must_use = "failed snapshot-read custody must be destroyed or retained"]
#[derive(Debug)]
pub struct M1CompletionSnapshotReadFailedOutputV1 {
    inner: M1CompletionSnapshotReadFailedInnerV1,
}

impl M1CompletionSnapshotReadFailedOutputV1 {
    fn target_only(
        case: Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ) -> Self {
        Self {
            inner: M1CompletionSnapshotReadFailedInnerV1::TargetOnly(case),
        }
    }

    fn paired_prefill(
        case: Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ) -> Self {
        Self {
            inner: M1CompletionSnapshotReadFailedInnerV1::PairedPrefill(case),
        }
    }

    fn speculative_k4(
        case: Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ) -> Self {
        Self {
            inner: M1CompletionSnapshotReadFailedInnerV1::SpeculativeK4(case),
        }
    }

    fn speculative_k8(
        case: Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ) -> Self {
        Self {
            inner: M1CompletionSnapshotReadFailedInnerV1::SpeculativeK8(case),
        }
    }

    fn speculative_k16(
        case: Box<
            M1PhysicalQueuePhaseCaseV1<
                ServiceRecycledQueueSessionV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ) -> Self {
        Self {
            inner: M1CompletionSnapshotReadFailedInnerV1::SpeculativeK16(case),
        }
    }

    /// Destroys the queue without reopening the attempted snapshot read.
    ///
    /// # Errors
    ///
    /// Returns terminal release failure retaining all available custody.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        match self.inner {
            M1CompletionSnapshotReadFailedInnerV1::TargetOnly(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            M1CompletionSnapshotReadFailedInnerV1::PairedPrefill(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            M1CompletionSnapshotReadFailedInnerV1::SpeculativeK4(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            M1CompletionSnapshotReadFailedInnerV1::SpeculativeK8(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            M1CompletionSnapshotReadFailedInnerV1::SpeculativeK16(case) => {
                release_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
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
    /// Ferric guarded geometry or adjacent bytes rejected.
    Canary(M1CompletionCanaryErrorV1),
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
    /// The enclosing snapshot read was attempted and can never be retried.
    SnapshotReadFailed(Box<M1CompletionSnapshotReadFailedOutputV1>),
    /// A completed copy succeeded and is closed against another read.
    Rejected(Box<M1RejectedCompletionOutputV1>),
}

/// Observation failure retaining exact pre-copy or post-copy custody.
///
/// Only [`M1CompletionObservationFailureCustodyV1::Recycled`] permits retry.
/// Coordinate, extent, guard, and image failures retain the first completed
/// copy in a [`M1RejectedCompletionOutputV1`] with no completed-read transition.
/// On the guarded path that owned copy is the complete enclosing snapshot.
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

    pub(crate) const fn from_parts(
        error: M1CompletionObservationErrorV1,
        custody: M1CompletionObservationFailureCustodyV1,
    ) -> Self {
        Self { error, custody }
    }

    /// Retries only a failure that occurred before the compact copy.
    ///
    /// # Errors
    ///
    /// Returns renewed failure custody. Post-copy rejections remain closed and
    /// are returned unchanged without attempting another device read.
    pub fn retry(
        self,
    ) -> Result<M1ObservedCompletionOutputV1, Box<M1CompletionObservationFailureV1>> {
        let Self { error, custody } = self;
        match custody {
            M1CompletionObservationFailureCustodyV1::Recycled(queue) => {
                queue.observe_completion().map_err(Box::new)
            }
            custody @ (M1CompletionObservationFailureCustodyV1::Rejected(_)
            | M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(_)) => {
                Err(Box::new(Self { error, custody }))
            }
        }
    }

    /// Destroys the failed queue after faulting the logical Engine and retains
    /// any completed bytes already copied before the observation rejection,
    /// including the whole enclosing snapshot on the guarded path.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to the observation
    /// diagnostic and the same copied evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1CompletionEvidenceTeardownSuccessV1, Box<M1CompletionEvidenceTeardownFailureV1>>
    {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { error, custody } = self;
        match custody {
            M1CompletionObservationFailureCustodyV1::Recycled(queue) => {
                finish_completion_evidence_teardown(
                    M1CompletionEvidenceTeardownDiagnosticV1::Observation(error),
                    M1CompletionEvidenceTeardownEvidenceV1::None,
                    queue.destroy_and_release(),
                )
            }
            M1CompletionObservationFailureCustodyV1::Rejected(output) => {
                match output.destroy_and_release_retaining_readback() {
                    Ok((queue_release, readback)) => Ok(M1CompletionEvidenceTeardownSuccessV1 {
                        diagnostic: M1CompletionEvidenceTeardownDiagnosticV1::Observation(error),
                        evidence: M1CompletionEvidenceTeardownEvidenceV1::Rejected(readback),
                        queue_release,
                    }),
                    Err(source) => {
                        let (source, readback) = *source;
                        Err(Box::new(M1CompletionEvidenceTeardownFailureV1 {
                            diagnostic: M1CompletionEvidenceTeardownDiagnosticV1::Observation(
                                error,
                            ),
                            evidence: M1CompletionEvidenceTeardownEvidenceV1::Rejected(readback),
                            source,
                        }))
                    }
                }
            }
            M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(output) => {
                finish_completion_evidence_teardown(
                    M1CompletionEvidenceTeardownDiagnosticV1::Observation(error),
                    M1CompletionEvidenceTeardownEvidenceV1::None,
                    output.destroy_and_release(),
                )
            }
        }
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

    /// Rechecks the unchanged copied compact image against corrected semantic
    /// expectations without issuing another device read.
    ///
    /// # Errors
    ///
    /// Returns the unchanged captured observation when semantic validation
    /// still rejects.
    pub fn retry(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1PhysicalCompletedReadbackV1, Self> {
        self.retry_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::Generic,
            expectations,
        )
    }

    fn retry_with_evidence_authority(
        self,
        authority: M1CompletionEvidenceJoinAuthorityV1,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1PhysicalCompletedReadbackV1, Self> {
        (*self.observed).check_completion_with_evidence_authority(authority, expectations)
    }

    /// Destroys the semantically rejected queue after faulting the logical
    /// Engine and retains the exact compact image used by the failed join.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to the semantic diagnostic
    /// and unchanged compact image.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1CompletionEvidenceTeardownSuccessV1, Box<M1CompletionEvidenceTeardownFailureV1>>
    {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { error, observed } = self;
        match observed.destroy_and_release_retaining_image() {
            Ok((queue_release, image)) => Ok(M1CompletionEvidenceTeardownSuccessV1 {
                diagnostic: M1CompletionEvidenceTeardownDiagnosticV1::Semantic(error),
                evidence: M1CompletionEvidenceTeardownEvidenceV1::Observed(image),
                queue_release,
            }),
            Err(source) => {
                let (source, image) = *source;
                Err(Box::new(M1CompletionEvidenceTeardownFailureV1 {
                    diagnostic: M1CompletionEvidenceTeardownDiagnosticV1::Semantic(error),
                    evidence: M1CompletionEvidenceTeardownEvidenceV1::Observed(image),
                    source,
                }))
            }
        }
    }
}

/// Exact diagnostic retained by a generic completion evidence teardown.
#[derive(Debug)]
pub enum M1CompletionEvidenceTeardownDiagnosticV1 {
    /// Structural observation rejection.
    Observation(M1CompletionObservationErrorV1),
    /// Semantic completion-join rejection.
    Semantic(M1CompletedReadbackJoinErrorV1),
}

/// Copied evidence retained by a generic completion failure teardown.
#[derive(Debug)]
pub enum M1CompletionEvidenceTeardownEvidenceV1 {
    /// No owned completed bytes are available; a snapshot read may be terminal.
    None,
    /// One completed copy was structurally rejected; guarded backing is whole.
    Rejected(ServiceCompletedReadbackV1),
    /// Structurally decoded output used by a rejected semantic join.
    Observed(M1ObservedCompletionImageV1),
}

/// Clean generic queue teardown retaining its failure diagnostic and evidence.
#[must_use = "completion failure diagnostic and copied evidence remain retained"]
#[derive(Debug)]
pub struct M1CompletionEvidenceTeardownSuccessV1 {
    diagnostic: M1CompletionEvidenceTeardownDiagnosticV1,
    evidence: M1CompletionEvidenceTeardownEvidenceV1,
    queue_release: ServiceQueueReleaseObservationV1,
}

impl M1CompletionEvidenceTeardownSuccessV1 {
    #[must_use]
    pub const fn diagnostic(&self) -> &M1CompletionEvidenceTeardownDiagnosticV1 {
        &self.diagnostic
    }

    #[must_use]
    pub const fn evidence(&self) -> &M1CompletionEvidenceTeardownEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

/// Terminal lower queue-release quarantine retaining generic completion evidence.
#[must_use = "completion evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1CompletionEvidenceTeardownFailureV1 {
    diagnostic: M1CompletionEvidenceTeardownDiagnosticV1,
    evidence: M1CompletionEvidenceTeardownEvidenceV1,
    source: M1PhysicalQueueReleaseFailureV1,
}

impl M1CompletionEvidenceTeardownFailureV1 {
    #[must_use]
    pub const fn diagnostic(&self) -> &M1CompletionEvidenceTeardownDiagnosticV1 {
        &self.diagnostic
    }

    #[must_use]
    pub const fn evidence(&self) -> &M1CompletionEvidenceTeardownEvidenceV1 {
        &self.evidence
    }

    pub const fn source(&self) -> &M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }
}

fn finish_completion_evidence_teardown(
    diagnostic: M1CompletionEvidenceTeardownDiagnosticV1,
    evidence: M1CompletionEvidenceTeardownEvidenceV1,
    release: Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1>,
) -> Result<M1CompletionEvidenceTeardownSuccessV1, Box<M1CompletionEvidenceTeardownFailureV1>> {
    match release {
        Ok(queue_release) => Ok(M1CompletionEvidenceTeardownSuccessV1 {
            diagnostic,
            evidence,
            queue_release,
        }),
        Err(source) => Err(Box::new(M1CompletionEvidenceTeardownFailureV1 {
            diagnostic,
            evidence,
            source,
        })),
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

/// Post-compact observation rejection for direct target-choice readback.
#[derive(Debug)]
pub enum M1DirectDiagnosticObservationErrorV1 {
    /// The observed physical queue was not `TargetOnly` or `PairedPrefill`.
    NotDirectShape,
    /// Direct choice allocation was not attached before publication.
    CaptureNotEnabled,
    /// Retained live-lane and scheduler cardinality drifted.
    LiveLaneCount {
        capacity: usize,
        expected: usize,
        actual: usize,
    },
    /// Internal preflight did not retain a range for one live lane.
    PreparedRangeMissing { lane: usize },
    /// A bounded host evidence vector could not be reserved.
    HostAllocation,
    /// One generation-bound completed scalar copy failed.
    Queue {
        lane: usize,
        source: ServiceQueueErrorV1,
    },
    /// Choice shape, coordinates, or token value rejected.
    Choices(M1DirectDiagnosticChoicesErrorV1),
}

impl fmt::Display for M1DirectDiagnosticObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 direct diagnostic observation rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1DirectDiagnosticObservationErrorV1 {}

/// Failure retaining compact custody and every successfully copied scalar.
#[must_use = "direct diagnostic observation failure retains queue and evidence custody"]
#[derive(Debug)]
pub struct M1DirectDiagnosticObservationFailureV1 {
    error: M1DirectDiagnosticObservationErrorV1,
    completion: Box<M1ObservedCompletionOutputV1>,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
}

impl M1DirectDiagnosticObservationFailureV1 {
    /// Exact pre-copy, copy, or validation rejection.
    #[must_use]
    pub const fn error(&self) -> &M1DirectDiagnosticObservationErrorV1 {
        &self.error
    }

    /// Number of scalar ranges copied before fail-closed retention.
    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    /// Destroys the observed queue without minting completion authority.
    ///
    /// # Errors
    ///
    /// Returns the existing terminal queue-release quarantine.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        self.completion.destroy_and_release()
    }

    /// Faults the Engine and destroys the queue while retaining compact and
    /// partially copied direct-choice evidence.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to unchanged evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1DirectDiagnosticObservationTeardownSuccessV1,
        Box<M1DirectDiagnosticObservationTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            error,
            completion,
            partial_choices,
        } = self;
        match completion.destroy_and_release_retaining_image() {
            Ok((queue_release, compact)) => Ok(M1DirectDiagnosticObservationTeardownSuccessV1 {
                error,
                compact,
                partial_choices,
                queue_release,
            }),
            Err(source) => {
                let (source, compact) = *source;
                Err(Box::new(M1DirectDiagnosticObservationTeardownFailureV1 {
                    error,
                    compact,
                    partial_choices,
                    source,
                }))
            }
        }
    }
}

/// Clean teardown retaining a failed direct-choice observation.
#[must_use = "direct diagnostic observation evidence remains retained"]
#[derive(Debug)]
pub struct M1DirectDiagnosticObservationTeardownSuccessV1 {
    error: M1DirectDiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    queue_release: ServiceQueueReleaseObservationV1,
}

impl M1DirectDiagnosticObservationTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &M1DirectDiagnosticObservationErrorV1 {
        &self.error
    }

    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

/// Terminal release quarantine retaining a failed direct-choice observation.
#[must_use = "direct diagnostic observation evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1DirectDiagnosticObservationTeardownFailureV1 {
    error: M1DirectDiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    source: M1PhysicalQueueReleaseFailureV1,
}

impl M1DirectDiagnosticObservationTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1DirectDiagnosticObservationErrorV1 {
        &self.error
    }

    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    pub const fn source(&self) -> &M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }
}

/// Post-compact observation rejection for diagnostic S1/K4 choice readback.
#[derive(Debug)]
pub enum M1SpeculativeDiagnosticObservationErrorV1 {
    /// The observed physical queue was not the exact K4 shape.
    NotSpeculativeK4,
    /// Diagnostic choice allocations were not attached before publication.
    CaptureNotEnabled,
    /// One completed generic copy failed.
    Queue {
        /// One exact `draft-N` scalar or `target` full-choice range.
        range: &'static str,
        /// Existing generation-bound queue diagnostic.
        source: ServiceQueueErrorV1,
    },
    /// Copied coordinates, extent, or token values rejected.
    Choices(M1SpeculativeDiagnosticChoicesErrorV1),
    /// Internal typed copy custody did not contain four draft scalars then target.
    PartialCopyCount { actual: usize },
}

#[derive(Debug)]
struct M1DiagnosticChoiceCopyCustodyV1<T> {
    copies: Vec<T>,
}

impl<T> M1DiagnosticChoiceCopyCustodyV1<T> {
    fn new() -> Self {
        Self {
            copies: Vec::with_capacity(5),
        }
    }

    fn retain(&mut self, copy: T) {
        self.copies.push(copy);
    }

    fn into_partial(self) -> Box<[T]> {
        self.copies.into_boxed_slice()
    }

    fn into_complete(self) -> Result<([T; 4], T), Box<[T]>> {
        let complete: Result<[T; 5], Vec<T>> = self.copies.try_into();
        match complete {
            Ok([draft_0, draft_1, draft_2, draft_3, target]) => {
                Ok(([draft_0, draft_1, draft_2, draft_3], target))
            }
            Err(copies) => Err(copies.into_boxed_slice()),
        }
    }
}

fn retain_all_m1_diagnostic_choice_copies<T>(draft: [T; 4], target: T) -> Box<[T]> {
    draft.into_iter().chain(core::iter::once(target)).collect()
}

trait M1DiagnosticChoiceReadBackendV1: fmt::Debug + Sized {
    type Range: Copy + fmt::Debug;
    type Readback: fmt::Debug;
    type Error: fmt::Debug;
    type TeardownSuccess: fmt::Debug;
    type TeardownFailure: fmt::Debug;

    fn read_completed(
        &mut self,
        range_name: &'static str,
        range: Self::Range,
    ) -> Result<Self::Readback, Self::Error>;

    fn destroy_or_quarantine(self) -> Result<Self::TeardownSuccess, Self::TeardownFailure>;
}

type M1DiagnosticChoiceReadSuccessV1<B> = (
    B,
    [<B as M1DiagnosticChoiceReadBackendV1>::Readback; 4],
    <B as M1DiagnosticChoiceReadBackendV1>::Readback,
);

type M1DiagnosticChoiceReadResultV1<B> =
    Result<M1DiagnosticChoiceReadSuccessV1<B>, M1DiagnosticChoiceReadFailureV1<B>>;

#[derive(Debug)]
struct M1DiagnosticChoiceReadFailureV1<B: M1DiagnosticChoiceReadBackendV1> {
    error: B::Error,
    partial: M1DiagnosticChoiceCopyCustodyV1<B::Readback>,
    backend: B,
}

#[derive(Debug)]
struct M1DiagnosticChoiceReadTeardownSuccessV1<B: M1DiagnosticChoiceReadBackendV1> {
    error: B::Error,
    partial: Box<[B::Readback]>,
    teardown: B::TeardownSuccess,
}

#[derive(Debug)]
struct M1DiagnosticChoiceReadTeardownFailureV1<B: M1DiagnosticChoiceReadBackendV1> {
    error: B::Error,
    partial: Box<[B::Readback]>,
    teardown: B::TeardownFailure,
}

impl<B: M1DiagnosticChoiceReadBackendV1> M1DiagnosticChoiceReadFailureV1<B> {
    fn error(&self) -> &B::Error {
        &self.error
    }

    fn copied_choice_ranges(&self) -> usize {
        self.partial.copies.len()
    }

    fn into_parts(self) -> (B::Error, B, Box<[B::Readback]>) {
        (self.error, self.backend, self.partial.into_partial())
    }

    fn destroy_or_quarantine<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1DiagnosticChoiceReadTeardownSuccessV1<B>,
        M1DiagnosticChoiceReadTeardownFailureV1<B>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            error,
            partial,
            backend,
        } = self;
        let partial = partial.into_partial();
        match backend.destroy_or_quarantine() {
            Ok(teardown) => Ok(M1DiagnosticChoiceReadTeardownSuccessV1 {
                error,
                partial,
                teardown,
            }),
            Err(teardown) => Err(M1DiagnosticChoiceReadTeardownFailureV1 {
                error,
                partial,
                teardown,
            }),
        }
    }
}

fn read_m1_diagnostic_choice_ranges_v1<B: M1DiagnosticChoiceReadBackendV1>(
    mut backend: B,
    draft_ranges: [B::Range; 4],
    target_range: B::Range,
) -> M1DiagnosticChoiceReadResultV1<B> {
    let mut partial = M1DiagnosticChoiceCopyCustodyV1::new();
    for (range_name, range) in ["draft-0", "draft-1", "draft-2", "draft-3"]
        .into_iter()
        .zip(draft_ranges)
    {
        match backend.read_completed(range_name, range) {
            Ok(draft) => partial.retain(draft),
            Err(error) => {
                return Err(M1DiagnosticChoiceReadFailureV1 {
                    error,
                    partial,
                    backend,
                });
            }
        }
    }
    let target = match backend.read_completed("target", target_range) {
        Ok(target) => target,
        Err(error) => {
            return Err(M1DiagnosticChoiceReadFailureV1 {
                error,
                partial,
                backend,
            })
        }
    };
    partial.retain(target);
    match partial.into_complete() {
        Ok((draft, target)) => Ok((backend, draft, target)),
        Err(_) => unreachable!("the helper retained exactly five successful choice copies"),
    }
}

#[derive(Debug)]
struct M1ProductionDiagnosticChoiceReadBackendV1 {
    completion: M1ObservedCompletionOutputV1,
}

impl M1DiagnosticChoiceReadBackendV1 for M1ProductionDiagnosticChoiceReadBackendV1 {
    type Range = ServiceHostDispatchRangeV1;
    type Readback = ServiceCompletedReadbackV1;
    type Error = M1SpeculativeDiagnosticObservationErrorV1;
    type TeardownSuccess = (
        ServiceQueueReleaseObservationV1,
        M1ObservedCompletionImageV1,
    );
    type TeardownFailure = Box<(M1PhysicalQueueReleaseFailureV1, M1ObservedCompletionImageV1)>;

    fn read_completed(
        &mut self,
        range_name: &'static str,
        range: Self::Range,
    ) -> Result<Self::Readback, Self::Error> {
        let M1ObservedCompletionOutputV1::SpeculativeK4(case) = &mut self.completion else {
            unreachable!("production diagnostic backend was preflighted as K4")
        };
        let request = case.case.lower.completed_read_request(range);
        case.case.lower.read_completed(request).map_err(|source| {
            M1SpeculativeDiagnosticObservationErrorV1::Queue {
                range: range_name,
                source,
            }
        })
    }

    fn destroy_or_quarantine(self) -> Result<Self::TeardownSuccess, Self::TeardownFailure> {
        self.completion.destroy_and_release_retaining_image()
    }
}

impl fmt::Display for M1SpeculativeDiagnosticObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 speculative diagnostic observation rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1SpeculativeDiagnosticObservationErrorV1 {}

/// Failure retains the observed compact queue and every successfully copied
/// choice range. It exposes no observation retry, preventing a partial copy
/// from being silently re-opened.
#[derive(Debug)]
enum M1SpeculativeDiagnosticObservationFailureCustodyV1 {
    Direct {
        error: M1SpeculativeDiagnosticObservationErrorV1,
        completion: Box<M1ObservedCompletionOutputV1>,
        partial_choices: Box<[ServiceCompletedReadbackV1]>,
    },
    Read(M1DiagnosticChoiceReadFailureV1<M1ProductionDiagnosticChoiceReadBackendV1>),
}

#[must_use = "diagnostic observation failure retains queue and copied-byte custody"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticObservationFailureV1 {
    custody: M1SpeculativeDiagnosticObservationFailureCustodyV1,
}

impl M1SpeculativeDiagnosticObservationFailureV1 {
    /// Exact pre-copy, copy, or validation rejection.
    #[must_use]
    pub fn error(&self) -> &M1SpeculativeDiagnosticObservationErrorV1 {
        match &self.custody {
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct { error, .. } => error,
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Read(failure) => failure.error(),
        }
    }

    /// Number of choice ranges copied before fail-closed retention.
    #[must_use]
    pub fn copied_choice_ranges(&self) -> usize {
        match &self.custody {
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                partial_choices, ..
            } => partial_choices.len(),
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Read(failure) => {
                failure.copied_choice_ranges()
            }
        }
    }

    /// Destroys the observed queue without minting completion authority.
    ///
    /// # Errors
    ///
    /// Returns the existing terminal queue-release quarantine.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        match self.custody {
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct { completion, .. } => {
                completion.destroy_and_release()
            }
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Read(failure) => {
                let (_error, backend, _partial_choices) = failure.into_parts();
                backend.completion.destroy_and_release()
            }
        }
    }

    /// Faults the logical Engine, destroys the queue, and retains the compact
    /// image, diagnostic, and every choice range copied before failure.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to the same diagnostic and
    /// copied evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1SpeculativeDiagnosticObservationTeardownSuccessV1,
        Box<M1SpeculativeDiagnosticObservationTeardownFailureV1>,
    > {
        match self.custody {
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Direct {
                error,
                completion,
                partial_choices,
            } => {
                engine.quarantine_m1_queue_rearm_failure();
                finish_m1_speculative_diagnostic_observation_teardown(
                    error,
                    partial_choices,
                    completion.destroy_and_release_retaining_image(),
                )
            }
            M1SpeculativeDiagnosticObservationFailureCustodyV1::Read(failure) => {
                match failure.destroy_or_quarantine(engine) {
                    Ok(teardown) => finish_m1_speculative_diagnostic_observation_teardown(
                        teardown.error,
                        teardown.partial,
                        Ok(teardown.teardown),
                    ),
                    Err(teardown) => finish_m1_speculative_diagnostic_observation_teardown(
                        teardown.error,
                        teardown.partial,
                        Err(teardown.teardown),
                    ),
                }
            }
        }
    }
}

fn finish_m1_speculative_diagnostic_observation_teardown(
    error: M1SpeculativeDiagnosticObservationErrorV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    teardown: Result<
        (
            ServiceQueueReleaseObservationV1,
            M1ObservedCompletionImageV1,
        ),
        Box<(M1PhysicalQueueReleaseFailureV1, M1ObservedCompletionImageV1)>,
    >,
) -> Result<
    M1SpeculativeDiagnosticObservationTeardownSuccessV1,
    Box<M1SpeculativeDiagnosticObservationTeardownFailureV1>,
> {
    match teardown {
        Ok((queue_release, compact)) => Ok(M1SpeculativeDiagnosticObservationTeardownSuccessV1 {
            error,
            compact,
            partial_choices,
            queue_release,
        }),
        Err(source) => {
            let (source, compact) = *source;
            Err(Box::new(
                M1SpeculativeDiagnosticObservationTeardownFailureV1 {
                    error,
                    compact,
                    partial_choices,
                    source,
                },
            ))
        }
    }
}

/// Clean teardown retaining a failed S1/K4 diagnostic observation.
#[must_use = "diagnostic observation evidence remains retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticObservationTeardownSuccessV1 {
    error: M1SpeculativeDiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    queue_release: ServiceQueueReleaseObservationV1,
}

impl M1SpeculativeDiagnosticObservationTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &M1SpeculativeDiagnosticObservationErrorV1 {
        &self.error
    }

    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

/// Terminal release quarantine retaining failed S1/K4 diagnostic evidence.
#[must_use = "diagnostic observation evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticObservationTeardownFailureV1 {
    error: M1SpeculativeDiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    source: M1PhysicalQueueReleaseFailureV1,
}

impl M1SpeculativeDiagnosticObservationTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1SpeculativeDiagnosticObservationErrorV1 {
        &self.error
    }

    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    pub const fn source(&self) -> &M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }
}

fn direct_semantic_expectations(
    choices: &M1ObservedDirectDiagnosticChoicesV1,
) -> (
    [CompletionWireSemanticExpectation<'static>; M1_MAX_ACTIVE_SEQUENCES as usize],
    usize,
) {
    let mut expectations = [CompletionWireSemanticExpectation::DirectFinalRow { choice: 0 };
        M1_MAX_ACTIVE_SEQUENCES as usize];
    let count = choices.choices().len();
    debug_assert!(count <= expectations.len());
    for (expectation, choice) in expectations.iter_mut().zip(choices.choices()) {
        *expectation = CompletionWireSemanticExpectation::DirectFinalRow { choice: *choice };
    }
    (expectations, count)
}

/// Compact K7 observation paired with independent final-row target choices.
#[must_use = "direct diagnostic observation must be checked, destroyed, or retained"]
#[derive(Debug)]
pub struct M1ObservedDirectDiagnosticOutputV1 {
    completion: M1ObservedCompletionOutputV1,
    choices: M1ObservedDirectDiagnosticChoicesV1,
}

impl M1ObservedDirectDiagnosticOutputV1 {
    /// Structurally observed compact K7 image from the same queue generation.
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        self.completion.image()
    }

    /// Independently copied final active-row target choices.
    pub const fn choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    /// Checks direct semantics against captured K6 choices and joins scheduler,
    /// epoch, plan, completion, and KV custody.
    ///
    /// # Errors
    ///
    /// Returns unchanged copied evidence when the compact semantic join rejects.
    pub fn check_completion(
        self,
    ) -> Result<
        M1DirectDiagnosticCompletedReadbackV1,
        Box<M1DirectDiagnosticCompletedReadbackJoinFailureV1>,
    > {
        let (expectations, count) = direct_semantic_expectations(&self.choices);
        let Self {
            completion,
            choices,
        } = self;
        match completion.check_completion_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::DirectDiagnostic,
            &expectations[..count],
        ) {
            Ok(completed) => Ok(M1DirectDiagnosticCompletedReadbackV1 { completed, choices }),
            Err(failure) => Err(Box::new(M1DirectDiagnosticCompletedReadbackJoinFailureV1 {
                failure,
                choices,
            })),
        }
    }

    /// Tears down without granting semantic completion authority.
    ///
    /// # Errors
    ///
    /// Returns the existing terminal queue-release quarantine.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        self.completion.destroy_and_release()
    }
}

/// Positive direct join retaining exact independent choice evidence.
#[must_use = "completed readback and direct choice evidence remain retained"]
#[derive(Debug)]
pub struct M1DirectDiagnosticCompletedReadbackV1 {
    completed: M1PhysicalCompletedReadbackV1,
    choices: M1ObservedDirectDiagnosticChoicesV1,
}

impl M1DirectDiagnosticCompletedReadbackV1 {
    pub const fn completed(&self) -> &M1PhysicalCompletedReadbackV1 {
        &self.completed
    }

    pub const fn choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    /// Separates exact completion and inert direct evidence once.
    #[must_use = "completion and direct choice evidence remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalCompletedReadbackV1,
        M1ObservedDirectDiagnosticChoicesV1,
    ) {
        (self.completed, self.choices)
    }
}

/// Semantic rejection retaining the same copied direct-choice evidence.
#[must_use = "semantic rejection retains direct diagnostic custody"]
#[derive(Debug)]
pub struct M1DirectDiagnosticCompletedReadbackJoinFailureV1 {
    failure: M1CompletedReadbackJoinFailureV1,
    choices: M1ObservedDirectDiagnosticChoicesV1,
}

impl M1DirectDiagnosticCompletedReadbackJoinFailureV1 {
    /// Existing roster, epoch, plan, wire, or direct-token rejection.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        self.failure.error()
    }

    pub const fn choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    /// Rechecks unchanged copied bytes without another physical read.
    ///
    /// # Errors
    ///
    /// Returns unchanged failure custody when the join still rejects.
    pub fn retry(self) -> Result<M1DirectDiagnosticCompletedReadbackV1, Box<Self>> {
        let (expectations, count) = direct_semantic_expectations(&self.choices);
        let Self { failure, choices } = self;
        match failure.retry_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::DirectDiagnostic,
            &expectations[..count],
        ) {
            Ok(completed) => Ok(M1DirectDiagnosticCompletedReadbackV1 { completed, choices }),
            Err(failure) => Err(Box::new(Self { failure, choices })),
        }
    }

    /// Separates the generic semantic rejection and inert choices once.
    #[must_use = "semantic rejection and direct choices remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedReadbackJoinFailureV1,
        M1ObservedDirectDiagnosticChoicesV1,
    ) {
        (self.failure, self.choices)
    }

    /// Faults the Engine and tears down the rejected queue while retaining
    /// compact and direct-choice evidence.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to unchanged evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1DirectDiagnosticSemanticTeardownSuccessV1,
        Box<M1DirectDiagnosticSemanticTeardownFailureV1>,
    > {
        let Self { failure, choices } = self;
        match failure.destroy_queue_and_retain_evidence(engine) {
            Ok(teardown) => Ok(M1DirectDiagnosticSemanticTeardownSuccessV1 { choices, teardown }),
            Err(teardown) => Err(Box::new(M1DirectDiagnosticSemanticTeardownFailureV1 {
                choices,
                teardown,
            })),
        }
    }
}

/// Clean teardown retaining direct semantic rejection and choices.
#[must_use = "direct semantic diagnostic and choices remain retained"]
#[derive(Debug)]
pub struct M1DirectDiagnosticSemanticTeardownSuccessV1 {
    choices: M1ObservedDirectDiagnosticChoicesV1,
    teardown: M1CompletionEvidenceTeardownSuccessV1,
}

impl M1DirectDiagnosticSemanticTeardownSuccessV1 {
    pub const fn choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    pub const fn teardown(&self) -> &M1CompletionEvidenceTeardownSuccessV1 {
        &self.teardown
    }
}

/// Terminal release quarantine retaining direct semantic rejection and choices.
#[must_use = "direct semantic diagnostic, choices, and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1DirectDiagnosticSemanticTeardownFailureV1 {
    choices: M1ObservedDirectDiagnosticChoicesV1,
    teardown: Box<M1CompletionEvidenceTeardownFailureV1>,
}

impl M1DirectDiagnosticSemanticTeardownFailureV1 {
    pub const fn choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    pub const fn teardown(&self) -> &M1CompletionEvidenceTeardownFailureV1 {
        &self.teardown
    }
}

/// One compact K7 observation paired with five completed S1/K4 range copies.
#[must_use = "diagnostic observation must be checked, destroyed, or retained"]
#[derive(Debug)]
pub struct M1ObservedSpeculativeDiagnosticOutputV1 {
    completion: M1ObservedCompletionOutputV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1ObservedSpeculativeDiagnosticOutputV1 {
    /// Structurally observed compact K7 image from the same queue generation.
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        self.completion.image()
    }

    /// Exact four draft and five target choices.
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Checks maximal-prefix semantics against the captured choices and joins
    /// them to the existing scheduler/epoch/plan/completion custody.
    ///
    /// # Errors
    ///
    /// Returns unchanged copied evidence when compact wire, roster, epoch,
    /// plan, request, or greedy-token semantics reject.
    pub fn check_completion(
        self,
    ) -> Result<
        M1SpeculativeDiagnosticCompletedReadbackV1,
        Box<M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1>,
    > {
        let draft_tokens = *self.choices.draft_choices();
        let target_choices = *self.choices.target_choices();
        let Self {
            completion,
            choices,
        } = self;
        let semantic = CompletionWireSemanticExpectation::Speculative {
            draft_tokens: &draft_tokens,
            target_choices: &target_choices,
        };
        match completion.check_completion_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
            &[semantic],
        ) {
            Ok(completed) => Ok(M1SpeculativeDiagnosticCompletedReadbackV1 { completed, choices }),
            Err(failure) => Err(Box::new(
                M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1 { failure, choices },
            )),
        }
    }

    /// Tears down without granting semantic completion authority.
    ///
    /// # Errors
    ///
    /// Returns the existing terminal queue-release quarantine.
    pub fn destroy_and_release(
        self,
    ) -> Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1> {
        self.completion.destroy_and_release()
    }
}

/// Positive maximal-prefix join retaining the exact choice evidence.
#[must_use = "completed readback and diagnostic choice evidence remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticCompletedReadbackV1 {
    completed: M1PhysicalCompletedReadbackV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1SpeculativeDiagnosticCompletedReadbackV1 {
    /// Existing exact completion custody.
    pub const fn completed(&self) -> &M1PhysicalCompletedReadbackV1 {
        &self.completed
    }

    /// Exact copied draft and target choices.
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Target's next-token choice for the same pre-round context.
    #[must_use]
    pub const fn corresponding_target_only_token(&self) -> ferric_spec::TokenId {
        self.choices.target_choices()[0]
    }

    /// Whether the speculative round's first published token equals the
    /// corresponding target choice. Successful greedy validation makes this
    /// true; the explicit predicate is retained for capture reporting.
    #[must_use]
    pub fn target_token_matches(&self) -> bool {
        self.completed
            .checked()
            .records()
            .first()
            .is_some_and(|record| {
                record.record().emitted_token_count > 0
                    && record.record().emitted_tokens[0] == self.corresponding_target_only_token()
            })
    }

    /// Faults the logical Engine and tears down this positively joined queue
    /// while retaining checked completion, KV, and diagnostic choice custody.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to every unchanged owner.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1SpeculativeDiagnosticCompletedTeardownSuccessV1,
        Box<M1SpeculativeDiagnosticCompletedTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { completed, choices } = self;
        let (queue, checked, completion, kv) = completed.into_parts();
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(M1SpeculativeDiagnosticCompletedTeardownSuccessV1 {
                queue_release,
                checked,
                completion,
                kv,
                choices,
            }),
            Err(source) => Err(Box::new(
                M1SpeculativeDiagnosticCompletedTeardownFailureV1 {
                    source,
                    checked,
                    completion,
                    kv,
                    choices,
                },
            )),
        }
    }

    /// Separates exact completion and inert diagnostic evidence once.
    #[must_use = "completion and choice evidence remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalCompletedReadbackV1,
        M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        (self.completed, self.choices)
    }
}

/// Clean teardown retaining a positively joined S1/K4 diagnostic completion.
#[must_use = "checked completion, KV, and choice evidence remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticCompletedTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1SpeculativeDiagnosticCompletedTeardownSuccessV1 {
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    pub const fn kv(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }

    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }
}

/// Terminal release quarantine retaining a joined S1/K4 completion.
#[must_use = "joined completion evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticCompletedTeardownFailureV1 {
    source: M1PhysicalReadbackQueueReleaseFailureV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1SpeculativeDiagnosticCompletedTeardownFailureV1 {
    pub const fn source(&self) -> &M1PhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    pub const fn kv(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }

    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }
}

/// Semantic rejection retaining the same already-copied choice evidence.
#[must_use = "semantic rejection retains diagnostic completion custody"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1 {
    failure: M1CompletedReadbackJoinFailureV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1 {
    /// Existing roster, epoch, plan, wire, or greedy-semantic rejection.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        self.failure.error()
    }

    /// Exact choice evidence used by the rejected join.
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Rechecks unchanged copied bytes without issuing another device read.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure if the same semantic join still rejects.
    pub fn retry(self) -> Result<M1SpeculativeDiagnosticCompletedReadbackV1, Box<Self>> {
        let draft_tokens = *self.choices.draft_choices();
        let target_choices = *self.choices.target_choices();
        let Self { failure, choices } = self;
        let semantic = CompletionWireSemanticExpectation::Speculative {
            draft_tokens: &draft_tokens,
            target_choices: &target_choices,
        };
        match failure.retry_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
            &[semantic],
        ) {
            Ok(completed) => Ok(M1SpeculativeDiagnosticCompletedReadbackV1 { completed, choices }),
            Err(failure) => Err(Box::new(Self { failure, choices })),
        }
    }

    /// Faults the logical Engine and tears down a semantic rejection while
    /// retaining the exact draft/target choices beside generic compact evidence.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to the unchanged evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1SpeculativeDiagnosticSemanticTeardownSuccessV1,
        Box<M1SpeculativeDiagnosticSemanticTeardownFailureV1>,
    > {
        let Self { failure, choices } = self;
        match failure.destroy_queue_and_retain_evidence(engine) {
            Ok(teardown) => {
                Ok(M1SpeculativeDiagnosticSemanticTeardownSuccessV1 { choices, teardown })
            }
            Err(teardown) => Err(Box::new(M1SpeculativeDiagnosticSemanticTeardownFailureV1 {
                choices,
                teardown,
            })),
        }
    }
}

/// Clean teardown retaining semantic rejection and S1/K4 choices.
#[must_use = "semantic diagnostic and choice evidence remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticSemanticTeardownSuccessV1 {
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: M1CompletionEvidenceTeardownSuccessV1,
}

impl M1SpeculativeDiagnosticSemanticTeardownSuccessV1 {
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    pub const fn teardown(&self) -> &M1CompletionEvidenceTeardownSuccessV1 {
        &self.teardown
    }
}

/// Terminal release quarantine retaining semantic rejection and S1/K4 choices.
#[must_use = "semantic diagnostic, choices, and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticSemanticTeardownFailureV1 {
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: Box<M1CompletionEvidenceTeardownFailureV1>,
}

impl M1SpeculativeDiagnosticSemanticTeardownFailureV1 {
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    pub const fn teardown(&self) -> &M1CompletionEvidenceTeardownFailureV1 {
        &self.teardown
    }
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
/// use ferric_engine::{M1ObservedQualificationOutputV1, M1ValidatedQualificationContextStepV1};
/// fn consume_twice(
///     observed: M1ObservedQualificationOutputV1,
///     contexts: &[M1ValidatedQualificationContextStepV1],
/// ) {
///     let _first = observed.check_final_completion(contexts);
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

    /// Derives and joins one direct qualification prefill choice per live lane.
    ///
    /// This compatibility transition is restricted to target prefill graphs.
    /// Each choice comes from the same finite lowest-ID BF16 argmax validation
    /// used by terminal decode qualification; callers supply no token choices.
    ///
    /// # Errors
    ///
    /// Rejects a non-prefill selection, nonfinite row, or compact semantic
    /// mismatch while retaining the same one-shot qualification observation.
    pub fn check_prefill_completion(
        self,
    ) -> Result<M1QualifiedPhysicalCompletedReadbackV1, M1QualificationCompletedReadbackJoinFailureV1>
    {
        let actual = self.compact().selection();
        if actual.mode != Qwen3ExecutionMode::Prefill {
            return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 {
                    source: M1CompletedOutputCheckErrorV1::QualificationPrefillSelection { actual },
                },
                observed: Box::new(self),
            });
        }
        let final_rows = match self.evidence.logits.final_row_choices() {
            Ok(final_rows) => final_rows,
            Err(source) => {
                return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                    error: M1CompletedReadbackJoinErrorV1 {
                        source: M1CompletedOutputCheckErrorV1::QualificationFinalLogits(source),
                    },
                    observed: Box::new(self),
                })
            }
        };
        let Self {
            completion,
            evidence,
        } = self;
        let M1ObservedCompletionOutputV1::TargetOnly(case) = completion else {
            return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 {
                    source: M1CompletedOutputCheckErrorV1::QualificationFinalObservationShape,
                },
                observed: Box::new(Self {
                    completion,
                    evidence,
                }),
            });
        };
        match check_observed_qualification_prefill_case(case, &final_rows) {
            Ok((case, checked, completion, kv)) => Ok(M1QualifiedPhysicalCompletedReadbackV1 {
                completed: M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::TargetOnly(case),
                    checked,
                    completion,
                    kv,
                },
                evidence,
            }),
            Err((source, case)) => Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 { source },
                observed: Box::new(Self {
                    completion: M1ObservedCompletionOutputV1::TargetOnly(case),
                    evidence,
                }),
            }),
        }
    }

    /// Consumes terminal qualification evidence and derives each final choice.
    ///
    /// Every copied BF16 value must be finite. Rows are scanned in ascending
    /// token-ID order with strict greater-than replacement, so equal maxima
    /// select the lowest token ID. The derived choices, not caller-supplied
    /// tokens, are then compared with compact K7 under the exact context roster.
    ///
    /// # Errors
    ///
    /// Returns the unchanged one-shot observation when BF16 validation, exact
    /// context roster validation, or the compact semantic join rejects.
    pub fn check_final_completion(
        self,
        contexts: &[M1ValidatedQualificationContextStepV1],
    ) -> Result<M1QualifiedPhysicalCompletedReadbackV1, M1QualificationCompletedReadbackJoinFailureV1>
    {
        let expected = self.compact().records().len();
        if contexts.len() != expected {
            return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 {
                    source: M1CompletedOutputCheckErrorV1::ExpectationCount {
                        expected,
                        actual: contexts.len(),
                    },
                },
                observed: Box::new(self),
            });
        }
        let final_rows = match self.evidence.logits.final_row_choices() {
            Ok(final_rows) => final_rows,
            Err(source) => {
                return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                    error: M1CompletedReadbackJoinErrorV1 {
                        source: M1CompletedOutputCheckErrorV1::QualificationFinalLogits(source),
                    },
                    observed: Box::new(self),
                })
            }
        };
        let Self {
            completion,
            evidence,
        } = self;
        let M1ObservedCompletionOutputV1::TargetOnly(case) = completion else {
            return Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 {
                    source: M1CompletedOutputCheckErrorV1::QualificationFinalObservationShape,
                },
                observed: Box::new(Self {
                    completion,
                    evidence,
                }),
            });
        };
        match check_observed_qualification_final_case(case, contexts, &final_rows) {
            Ok((case, checked, completion, kv)) => Ok(M1QualifiedPhysicalCompletedReadbackV1 {
                completed: M1PhysicalCompletedReadbackV1 {
                    queue: M1PhysicalReadbackQueueSessionV1::TargetOnly(case),
                    checked,
                    completion,
                    kv,
                },
                evidence,
            }),
            Err((source, case)) => Err(M1QualificationCompletedReadbackJoinFailureV1 {
                error: M1CompletedReadbackJoinErrorV1 { source },
                observed: Box::new(Self {
                    completion: M1ObservedCompletionOutputV1::TargetOnly(case),
                    evidence,
                }),
            }),
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

    /// Destroys the queue while retaining the complete copied qualification
    /// evidence for terminal reporting.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to the copied evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1QualificationEvidenceTeardownSuccessV1,
        Box<M1QualificationEvidenceTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            completion,
            evidence,
        } = self;
        match completion.destroy_and_release() {
            Ok(queue_release) => Ok(M1QualificationEvidenceTeardownSuccessV1 {
                queue_release,
                evidence,
            }),
            Err(source) => Err(Box::new(M1QualificationEvidenceTeardownFailureV1 {
                source,
                evidence,
            })),
        }
    }

    pub(crate) fn into_teardown_parts(
        self,
    ) -> (
        M1ObservedCompletionOutputV1,
        M1QualificationCompletionEvidenceV1,
    ) {
        (self.completion, self.evidence)
    }
}

/// Clean queue teardown retaining complete qualification evidence.
#[must_use = "qualification evidence remains retained"]
#[derive(Debug)]
pub struct M1QualificationEvidenceTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    evidence: M1QualificationCompletionEvidenceV1,
}

impl M1QualificationEvidenceTeardownSuccessV1 {
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }
}

/// Terminal lower release quarantine retaining qualification evidence.
#[must_use = "qualification evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1QualificationEvidenceTeardownFailureV1 {
    source: M1PhysicalQueueReleaseFailureV1,
    evidence: M1QualificationCompletionEvidenceV1,
}

impl M1QualificationEvidenceTeardownFailureV1 {
    pub const fn source(&self) -> &M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &M1QualificationCompletionEvidenceV1 {
        &self.evidence
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

    /// Rechecks unchanged prefill evidence without reopening either compact or
    /// logits readback.
    ///
    /// # Errors
    ///
    /// Returns the unchanged semantic failure when the copied evidence still
    /// rejects.
    pub fn retry_prefill_completion(self) -> Result<M1QualifiedPhysicalCompletedReadbackV1, Self> {
        (*self.observed).check_prefill_completion()
    }

    /// Destroys the semantically rejected queue while retaining the diagnostic
    /// and complete copied qualification evidence.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to the semantic error and
    /// copied evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1QualificationSemanticTeardownSuccessV1,
        Box<M1QualificationSemanticTeardownFailureV1>,
    > {
        let Self { error, observed } = self;
        match observed.destroy_queue_and_retain_evidence(engine) {
            Ok(teardown) => Ok(M1QualificationSemanticTeardownSuccessV1 { error, teardown }),
            Err(teardown) => Err(Box::new(M1QualificationSemanticTeardownFailureV1 {
                error,
                teardown,
            })),
        }
    }
}

/// Clean queue teardown retaining semantic rejection and qualification evidence.
#[must_use = "semantic rejection and qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1QualificationSemanticTeardownSuccessV1 {
    error: M1CompletedReadbackJoinErrorV1,
    teardown: M1QualificationEvidenceTeardownSuccessV1,
}

impl M1QualificationSemanticTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    pub const fn teardown(&self) -> &M1QualificationEvidenceTeardownSuccessV1 {
        &self.teardown
    }
}

/// Terminal lower release quarantine retaining semantic rejection and evidence.
#[must_use = "semantic rejection and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1QualificationSemanticTeardownFailureV1 {
    error: M1CompletedReadbackJoinErrorV1,
    teardown: Box<M1QualificationEvidenceTeardownFailureV1>,
}

impl M1QualificationSemanticTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    pub const fn teardown(&self) -> &M1QualificationEvidenceTeardownFailureV1 {
        &self.teardown
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
///         M1QualificationObservationFailureCustodyV1::CompactSnapshotReadFailed(output) => {
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
    /// Guarded compact snapshot read was attempted and cannot be retried.
    CompactSnapshotReadFailed(Box<M1CompletionSnapshotReadFailedOutputV1>),
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

    pub(crate) const fn from_parts(
        error: M1QualificationObservationErrorV1,
        custody: M1QualificationObservationFailureCustodyV1,
    ) -> Self {
        Self { error, custody }
    }

    /// Retries only a qualification failure that occurred before any completed
    /// bytes were copied. Post-copy custody is returned unchanged.
    ///
    /// # Errors
    ///
    /// Returns renewed or unchanged failure custody when observation cannot
    /// complete.
    pub fn retry(
        self,
    ) -> Result<M1ObservedQualificationOutputV1, Box<M1QualificationObservationFailureV1>> {
        let Self { error, custody } = self;
        match custody {
            M1QualificationObservationFailureCustodyV1::Recycled(queue) => {
                queue.observe_qualification_completion()
            }
            custody => Err(Box::new(Self { error, custody })),
        }
    }

    /// Destroys the failed queue while retaining every copied compact/logits
    /// byte that existed at the point of failure.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to all partial evidence.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1QualificationObservationTeardownSuccessV1,
        Box<M1QualificationObservationTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { error, custody } = self;
        match custody {
            M1QualificationObservationFailureCustodyV1::Recycled(queue) => {
                finish_qualification_observation_teardown(
                    error,
                    M1QualificationObservationTeardownEvidenceV1::None,
                    queue.destroy_and_release(),
                )
            }
            M1QualificationObservationFailureCustodyV1::CompactRejected(output) => {
                match output.destroy_and_release_retaining_readback() {
                    Ok((queue_release, compact)) => {
                        Ok(M1QualificationObservationTeardownSuccessV1 {
                            error,
                            evidence: M1QualificationObservationTeardownEvidenceV1::RejectedCompact(
                                compact,
                            ),
                            queue_release,
                        })
                    }
                    Err(source) => {
                        let (source, compact) = *source;
                        Err(Box::new(M1QualificationObservationTeardownFailureV1 {
                            error,
                            evidence: M1QualificationObservationTeardownEvidenceV1::RejectedCompact(
                                compact,
                            ),
                            source,
                        }))
                    }
                }
            }
            M1QualificationObservationFailureCustodyV1::CompactSnapshotReadFailed(output) => {
                match output.destroy_and_release() {
                    Ok(queue_release) => Ok(M1QualificationObservationTeardownSuccessV1 {
                        error,
                        evidence: M1QualificationObservationTeardownEvidenceV1::None,
                        queue_release,
                    }),
                    Err(source) => Err(Box::new(M1QualificationObservationTeardownFailureV1 {
                        error,
                        evidence: M1QualificationObservationTeardownEvidenceV1::None,
                        source,
                    })),
                }
            }
            M1QualificationObservationFailureCustodyV1::Observed {
                completion,
                partial_logits,
            } => match completion.destroy_and_release_retaining_image() {
                Ok((queue_release, compact)) => Ok(M1QualificationObservationTeardownSuccessV1 {
                    error,
                    evidence: M1QualificationObservationTeardownEvidenceV1::Observed {
                        compact,
                        partial_logits,
                    },
                    queue_release,
                }),
                Err(source) => {
                    let (source, compact) = *source;
                    Err(Box::new(M1QualificationObservationTeardownFailureV1 {
                        error,
                        evidence: M1QualificationObservationTeardownEvidenceV1::Observed {
                            compact,
                            partial_logits,
                        },
                        source,
                    }))
                }
            },
        }
    }
}

/// Copied evidence retained from a failed qualification observation.
#[must_use = "failed qualification evidence remains retained"]
#[derive(Debug)]
pub enum M1QualificationObservationTeardownEvidenceV1 {
    None,
    RejectedCompact(ServiceCompletedReadbackV1),
    Observed {
        compact: M1ObservedCompletionImageV1,
        partial_logits: Box<[ServiceCompletedReadbackV1]>,
    },
}

impl M1QualificationObservationTeardownEvidenceV1 {
    #[must_use]
    pub const fn partial_logits_count(&self) -> usize {
        match self {
            Self::Observed { partial_logits, .. } => partial_logits.len(),
            Self::None | Self::RejectedCompact(_) => 0,
        }
    }
}

/// Clean teardown after qualification observation failure.
#[must_use = "observation diagnostic and copied evidence remain retained"]
#[derive(Debug)]
pub struct M1QualificationObservationTeardownSuccessV1 {
    error: M1QualificationObservationErrorV1,
    evidence: M1QualificationObservationTeardownEvidenceV1,
    queue_release: ServiceQueueReleaseObservationV1,
}

impl M1QualificationObservationTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &M1QualificationObservationErrorV1 {
        &self.error
    }

    pub const fn evidence(&self) -> &M1QualificationObservationTeardownEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

/// Terminal release quarantine after qualification observation failure.
#[must_use = "observation evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1QualificationObservationTeardownFailureV1 {
    error: M1QualificationObservationErrorV1,
    evidence: M1QualificationObservationTeardownEvidenceV1,
    source: M1PhysicalQueueReleaseFailureV1,
}

impl M1QualificationObservationTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1QualificationObservationErrorV1 {
        &self.error
    }

    pub const fn evidence(&self) -> &M1QualificationObservationTeardownEvidenceV1 {
        &self.evidence
    }

    pub const fn source(&self) -> &M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }
}

fn finish_qualification_observation_teardown(
    error: M1QualificationObservationErrorV1,
    evidence: M1QualificationObservationTeardownEvidenceV1,
    release: Result<ServiceQueueReleaseObservationV1, M1PhysicalQueueReleaseFailureV1>,
) -> Result<
    M1QualificationObservationTeardownSuccessV1,
    Box<M1QualificationObservationTeardownFailureV1>,
> {
    match release {
        Ok(queue_release) => Ok(M1QualificationObservationTeardownSuccessV1 {
            error,
            evidence,
            queue_release,
        }),
        Err(source) => Err(Box::new(M1QualificationObservationTeardownFailureV1 {
            error,
            evidence,
            source,
        })),
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

    /// Recovers exact unchanged construction input without consuming terminal
    /// queue-creation custody.
    ///
    /// # Errors
    ///
    /// Returns the unchanged terminal failure when retry is denied.
    pub fn into_rejected_input_or_self(self) -> Result<M1PrepublicationBatchV1<'a>, Self> {
        match self.state {
            M1PhysicalQueueCreateFailureStateV1::Rejected { batch, .. } => Ok(*batch),
            state @ M1PhysicalQueueCreateFailureStateV1::Terminal { .. } => Err(Self { state }),
        }
    }
}

/// Terminal consuming transition failure with generic quarantine and Ferric custody.
#[must_use = "terminal failure retains generic quarantine and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    step: Box<M1PrepublicationStepCustodyV1>,
    lower: Box<ServiceQueueOperationFailureV1>,
    custody: Box<M1PhysicalQueueBatchCustodyV1>,
    completion_progress_wait: Option<Box<M1CompletionProgressWaitDiagnosticV1>>,
}

/// Terminal physical queue operation failure after its scheduler Engine has
/// been permanently faulted.
#[must_use = "the Engine-quarantined queue operation failure remains terminal custody"]
#[derive(Debug)]
pub struct M1EngineQuarantinedPhysicalQueueOperationFailureV1 {
    failure: Box<M1PhysicalQueueOperationFailureV1>,
}

impl M1EngineQuarantinedPhysicalQueueOperationFailureV1 {
    #[must_use = "the exact physical queue failure remains retained"]
    pub const fn failure(&self) -> &M1PhysicalQueueOperationFailureV1 {
        &self.failure
    }
}

impl M1PhysicalQueueOperationFailureV1 {
    /// Faults the paired scheduler Engine and retains this exact terminal queue
    /// operation failure as the sole post-transition owner.
    #[must_use = "the Engine-quarantined queue operation failure remains terminal custody"]
    pub fn quarantine_engine<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1EngineQuarantinedPhysicalQueueOperationFailureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        M1EngineQuarantinedPhysicalQueueOperationFailureV1 {
            failure: Box::new(self),
        }
    }

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
    pub fn error(&self) -> &ServiceQueueErrorV1 {
        self.lower.error()
    }

    /// Returns the generic addressless execution snapshot captured before a
    /// terminal completion-timeout poison, when that was the lower failure.
    #[must_use]
    pub fn timeout_execution_observation(
        &self,
    ) -> Option<&fe2o3_kfd::Gfx942TimeoutExecutionObservationV1> {
        self.lower.timeout_observation()
    }

    /// Returns the exact Ferric custody retained beside generic quarantine.
    #[must_use = "Ferric custody remains retained by terminal failure"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Returns Ferric's bounded completion-progress diagnostic when its wait
    /// policy, rather than an ordinary lower fault, terminalized the queue.
    #[must_use]
    pub fn completion_progress_wait_diagnostic(
        &self,
    ) -> Option<&M1CompletionProgressWaitDiagnosticV1> {
        self.completion_progress_wait.as_deref()
    }

    /// Maps the retained scan's first observed-pending index back to the exact
    /// retained addressless physical recipe row.
    #[must_use]
    pub fn first_observed_pending_recipe_row(&self) -> Option<&M1PhysicalDispatchRecipeRowV1> {
        let index = usize::from(
            self.completion_progress_wait
                .as_deref()?
                .last_observation?
                .first_pending_batch_index?,
        );
        self.custody.physical_recipe().rows().get(index)
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

#[inline(never)]
#[allow(clippy::boxed_local)] // The box keeps this noinline shape boundary off its caller's stack.
fn create_case<const N: usize>(
    ring_bytes: u32,
    step: M1PrepublicationStepCustodyV1,
    case: Box<M1PhysicalFixedBatchCaseV1<'_, N>>,
) -> CreateCaseResultV1<'_, N> {
    let (batch, custody) = (*case).into_parts();
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

enum CompletionProgressPollV1<P, C> {
    Pending {
        session: P,
        progress: M1CompletionProgressObservationV1,
    },
    Ready {
        session: C,
        progress: M1CompletionProgressObservationV1,
    },
}

enum CompletionProgressWaitFailureV1<E> {
    Lower(E),
    Policy {
        lower: E,
        diagnostic: M1CompletionProgressWaitDiagnosticV1,
    },
}

fn checked_completion_progress_total_scan_bound(
    packet_count: usize,
    maximum_consecutive_stalled_scans: u32,
) -> Option<u32> {
    u32::try_from(packet_count)
        .ok()?
        .checked_add(1)?
        .checked_mul(maximum_consecutive_stalled_scans)
}

fn completion_progress_diagnostic(
    reason: M1CompletionProgressWaitTerminalReasonV1,
    scans_performed: u32,
    consecutive_scans_without_progress: u32,
    total_scan_bound: Option<u32>,
    completed_count_high_water: u16,
    last_observation: Option<M1CompletionProgressObservationV1>,
) -> M1CompletionProgressWaitDiagnosticV1 {
    M1CompletionProgressWaitDiagnosticV1 {
        reason,
        scans_performed,
        consecutive_scans_without_progress,
        total_scan_bound,
        completed_count_high_water,
        last_observation,
    }
}

fn validate_completion_progress_observation(
    progress: M1CompletionProgressObservationV1,
    expected_packet_count: u16,
    ready: bool,
    completed_count_high_water: u16,
) -> Result<(), M1CompletionProgressWaitTerminalReasonV1> {
    if progress.packet_count != expected_packet_count {
        return Err(M1CompletionProgressWaitTerminalReasonV1::PacketCountMismatch);
    }
    if progress.completed_count > expected_packet_count
        || progress.pending_count > expected_packet_count
        || progress.completed_count.checked_add(progress.pending_count)
            != Some(expected_packet_count)
    {
        return Err(M1CompletionProgressWaitTerminalReasonV1::CountSumMismatch);
    }
    if ready {
        if progress.completed_count != expected_packet_count
            || progress.pending_count != 0
            || progress.first_pending_batch_index.is_some()
        {
            return Err(M1CompletionProgressWaitTerminalReasonV1::ReadyObservationInvalid);
        }
    } else {
        if progress.pending_count == 0 || progress.first_pending_batch_index.is_none() {
            return Err(M1CompletionProgressWaitTerminalReasonV1::PendingObservationInvalid);
        }
        if progress.first_pending_batch_index >= Some(expected_packet_count) {
            return Err(M1CompletionProgressWaitTerminalReasonV1::FirstPendingIndexOutOfBounds);
        }
    }
    if progress.completed_count < completed_count_high_water {
        return Err(M1CompletionProgressWaitTerminalReasonV1::CompletedCountRegressed);
    }
    Ok(())
}

fn wait_with_completion_progress_policy<const N: usize, P, C, E>(
    pending: P,
    maximum_consecutive_stalled_scans: u32,
    mut poll: impl FnMut(P) -> Result<CompletionProgressPollV1<P, C>, E>,
    mut pace_pending_scan: impl FnMut(),
    terminalize: impl FnOnce(P) -> E,
) -> Result<C, CompletionProgressWaitFailureV1<E>> {
    let expected_packet_count = match u16::try_from(N) {
        Ok(packet_count) => packet_count,
        Err(_) => {
            let lower = terminalize(pending);
            return Err(CompletionProgressWaitFailureV1::Policy {
                lower,
                diagnostic: completion_progress_diagnostic(
                    M1CompletionProgressWaitTerminalReasonV1::PacketCountNotRepresentable,
                    0,
                    0,
                    None,
                    0,
                    None,
                ),
            });
        }
    };
    let total_scan_bound =
        match checked_completion_progress_total_scan_bound(N, maximum_consecutive_stalled_scans) {
            Some(bound) => bound,
            None => {
                let lower = terminalize(pending);
                return Err(CompletionProgressWaitFailureV1::Policy {
                    lower,
                    diagnostic: completion_progress_diagnostic(
                        M1CompletionProgressWaitTerminalReasonV1::TotalScanBoundOverflow,
                        0,
                        0,
                        None,
                        0,
                        None,
                    ),
                });
            }
        };
    if total_scan_bound == 0 {
        let lower = terminalize(pending);
        return Err(CompletionProgressWaitFailureV1::Policy {
            lower,
            diagnostic: completion_progress_diagnostic(
                M1CompletionProgressWaitTerminalReasonV1::TotalScanBoundReached,
                0,
                0,
                Some(total_scan_bound),
                0,
                None,
            ),
        });
    }

    let mut pending = pending;
    let mut scans_performed = 0_u32;
    let mut consecutive_scans_without_progress = 0_u32;
    let mut completed_count_high_water = 0_u16;
    loop {
        let outcome = poll(pending).map_err(CompletionProgressWaitFailureV1::Lower)?;
        scans_performed = scans_performed
            .checked_add(1)
            .expect("the checked total scan bound contains every policy scan");
        let (next, progress) = match outcome {
            CompletionProgressPollV1::Pending { session, progress } => (session, progress),
            CompletionProgressPollV1::Ready { session, progress } => {
                // The safe lower API constructs Ready and its canonical progress
                // together, after consuming published custody into completed
                // custody. No published owner remains to terminalize with wait(0),
                // so this lower-layer invariant is checked in debug builds and
                // Ready wins the liveness-threshold boundary in all builds.
                debug_assert!(validate_completion_progress_observation(
                    progress,
                    expected_packet_count,
                    true,
                    completed_count_high_water,
                )
                .is_ok());
                return Ok(session);
            }
        };
        if let Err(reason) = validate_completion_progress_observation(
            progress,
            expected_packet_count,
            false,
            completed_count_high_water,
        ) {
            let lower = terminalize(next);
            return Err(CompletionProgressWaitFailureV1::Policy {
                lower,
                diagnostic: completion_progress_diagnostic(
                    reason,
                    scans_performed,
                    consecutive_scans_without_progress,
                    Some(total_scan_bound),
                    completed_count_high_water,
                    Some(progress),
                ),
            });
        }
        pending = next;
        if progress.completed_count > completed_count_high_water {
            completed_count_high_water = progress.completed_count;
            consecutive_scans_without_progress = 0;
        } else {
            consecutive_scans_without_progress = consecutive_scans_without_progress
                .checked_add(1)
                .expect("the consecutive scan counter is policy bounded");
        }

        let reason = if consecutive_scans_without_progress >= maximum_consecutive_stalled_scans {
            Some(M1CompletionProgressWaitTerminalReasonV1::ConsecutiveScansWithoutProgress)
        } else if scans_performed >= total_scan_bound {
            Some(M1CompletionProgressWaitTerminalReasonV1::TotalScanBoundReached)
        } else {
            None
        };
        if let Some(reason) = reason {
            let lower = terminalize(pending);
            return Err(CompletionProgressWaitFailureV1::Policy {
                lower,
                diagnostic: completion_progress_diagnostic(
                    reason,
                    scans_performed,
                    consecutive_scans_without_progress,
                    Some(total_scan_bound),
                    completed_count_high_water,
                    Some(progress),
                ),
            });
        }
        pace_pending_scan();
    }
}

fn wait_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServicePublishedQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1PhysicalQueuePhaseCaseV1<ServiceCompletedQueueSessionV1<N>>>,
    M1PhysicalQueueOperationFailureV1,
> {
    let (lower, custody, step) = (*case).into_parts();
    let completed = wait_with_completion_progress_policy::<N, _, _, _>(
        lower,
        M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
        |published| {
            published.poll_with_progress().map(|outcome| match outcome {
                ServiceQueuePollWithProgressV1::Pending { session, progress } => {
                    CompletionProgressPollV1::Pending {
                        session,
                        progress: M1CompletionProgressObservationV1::from_service(progress),
                    }
                }
                ServiceQueuePollWithProgressV1::Ready { session, progress } => {
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
        Ok(lower) => Ok(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(CompletionProgressWaitFailureV1::Lower(lower)) => {
            Err(operation_failure(shape, step, lower, custody))
        }
        Err(CompletionProgressWaitFailureV1::Policy { lower, diagnostic }) => Err(
            operation_failure_with_completion_progress(shape, step, lower, custody, diagnostic),
        ),
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

#[cfg(feature = "qualification-fault-injection")]
fn inject_qualification_fault_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
) -> Result<
    Box<M1PhysicalQueuePhaseCaseV1<ServiceQualificationFaultedQueueSessionV1<N>>>,
    Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
> {
    let (lower, custody, step) = (*case).into_parts();
    match lower.inject_qualification_fault(
        ServiceQualificationQueueFaultPointV1::PostRecycleBeforeCompletedReadAttempt,
    ) {
        Ok(lower) => Ok(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            lower, custody, step,
        ))),
        Err(lower) => Err(Box::new(M1PhysicalQueuePhaseCaseV1::new(
            *lower, custody, step,
        ))),
    }
}

#[cfg(feature = "qualification-fault-injection")]
fn release_qualification_fault_case<const N: usize>(
    case: Box<M1PhysicalQueuePhaseCaseV1<ServiceQualificationFaultedQueueSessionV1<N>>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    M1QualificationQueueTransitionFaultTeardownSuccessV1,
    Box<M1QualificationQueueTransitionFaultTeardownFailureV1>,
> {
    let (lower, custody, step) = (*case).into_parts();
    let queue_epoch = step.scheduled_dispatch().epoch();
    let dispatch_generation = lower.dispatch_generation();
    let fault_point = lower.point();
    match lower.destroy_and_release() {
        Ok(release) => Ok(M1QualificationQueueTransitionFaultTeardownSuccessV1 {
            shape,
            queue_epoch,
            dispatch_generation,
            fault_point,
            release,
        }),
        Err(lower) => Err(Box::new(
            M1QualificationQueueTransitionFaultTeardownFailureV1 {
                shape,
                queue_epoch,
                dispatch_generation,
                fault_point,
                lower,
                step: Box::new(step),
                custody: Box::new(custody),
            },
        )),
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
                finish_target_only_create(create_case(ring_bytes, step, case))
            }
            M1PhysicalFixedBatchV1::PairedPrefill(case) => {
                finish_paired_prefill_create(create_case(ring_bytes, step, case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK4(case) => {
                finish_speculative_k4_create(create_case(ring_bytes, step, case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK8(case) => {
                finish_speculative_k8_create(create_case(ring_bytes, step, case))
            }
            M1PhysicalFixedBatchV1::SpeculativeK16(case) => {
                finish_speculative_k16_create(create_case(ring_bytes, step, case))
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
    /// Waits for every exact completion signal under Ferric's fixed progress policy.
    ///
    /// # Errors
    ///
    /// Returns terminal generic quarantine paired with exact Ferric custody.
    pub fn wait(
        self,
    ) -> Result<M1PhysicalCompletedQueueSessionV1, M1PhysicalQueueOperationFailureV1> {
        match self {
            Self::TargetOnly(case) => wait_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                .map(M1PhysicalCompletedQueueSessionV1::TargetOnly),
            Self::PairedPrefill(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1PhysicalCompletedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1PhysicalCompletedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1PhysicalCompletedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                wait_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
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
    SnapshotReadFailed {
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
    let canary = output.completion_canary();
    let queue_selection = case.custody.selection();
    if let Some(canary) = canary {
        if let Err(error) = preflight_m1_completion_canary_v1(canary, range) {
            return Err(Box::new(ObserveCaseFailureV1::BeforeCopy {
                error: M1CompletionObservationErrorV1::Canary(error),
                case,
            }));
        }
        let request = case
            .lower
            .completed_snapshot_request(canary.snapshot_range());
        let readback = match case.lower.read_completed_snapshot(request) {
            Ok(readback) => readback,
            Err(error) => {
                return Err(Box::new(ObserveCaseFailureV1::SnapshotReadFailed {
                    error: M1CompletionObservationErrorV1::Queue(error),
                    case,
                }))
            }
        };
        let readback = match validate_m1_completion_canary_readback_v1(canary, range, readback) {
            Ok(readback) => readback,
            Err((error, readback)) => {
                return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
                    error: M1CompletionObservationErrorV1::Canary(error),
                    case,
                    readback,
                }))
            }
        };
        let scheduled = case.step.scheduled_dispatch();
        let image = match observe_m1_guarded_completed_output_v1(
            output_shape,
            queue_selection,
            scheduled,
            Box::new(readback),
        ) {
            Ok(image) => image,
            Err((error, readback)) => {
                return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
                    error: M1CompletionObservationErrorV1::Image(error),
                    case,
                    readback: (*readback).into_readback(),
                }))
            }
        };
        return Ok(Box::new(M1ObservedCompletionCaseV1 { case, image }));
    }
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
    let image =
        match observe_m1_completed_output_v1(output_shape, queue_selection, scheduled, readback) {
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
    snapshot_read_failed: fn(
        Box<M1PhysicalQueuePhaseCaseV1<ServiceRecycledQueueSessionV1<N>>>,
    ) -> M1CompletionSnapshotReadFailedOutputV1,
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
        ObserveCaseFailureV1::SnapshotReadFailed { error, case } => {
            M1CompletionObservationFailureV1 {
                error,
                custody: M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(Box::new(
                    snapshot_read_failed(case),
                )),
            }
        }
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
    authority: M1CompletionEvidenceJoinAuthorityV1,
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
    if let Err(error) = validate_generic_observed_semantics(
        authority,
        case.case
            .custody
            .completion_output()
            .qualification_logits()
            .is_some(),
        case.case
            .custody
            .completion_output()
            .direct_diagnostic_choices()
            .is_some(),
        case.case
            .custody
            .completion_output()
            .speculative_diagnostic_choices()
            .is_some(),
        semantics,
    ) {
        return Err((error, case));
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
    let M1ObservedCompletionCaseV1 { case, image } = *case;
    let checked =
        checked.retain_completion_canary_readback(image.into_completion_canary_readback());
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

fn validate_generic_observed_semantics(
    authority: M1CompletionEvidenceJoinAuthorityV1,
    qualification_capture_enabled: bool,
    direct_diagnostic_capture_enabled: bool,
    speculative_diagnostic_capture_enabled: bool,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> Result<(), M1CompletedOutputCheckErrorV1> {
    if direct_diagnostic_capture_enabled
        != matches!(
            authority,
            M1CompletionEvidenceJoinAuthorityV1::DirectDiagnostic
        )
    {
        return Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence);
    }
    if speculative_diagnostic_capture_enabled
        != matches!(
            authority,
            M1CompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic
        )
    {
        return Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence);
    }
    if qualification_capture_enabled {
        if let Some(lane) = semantics.iter().position(|semantic| {
            !matches!(
                semantic,
                CompletionWireSemanticExpectation::QualificationPromptCommit { .. }
            )
        }) {
            return Err(
                M1CompletedOutputCheckErrorV1::QualificationCaptureRequiresEvidence { lane },
            );
        }
    }
    Ok(())
}

fn check_observed_qualification_prefill_case<const N: usize>(
    case: Box<M1ObservedCompletionCaseV1<N>>,
    final_rows: &M1QualificationFinalRowChoicesV1,
) -> CheckObservedCaseResultV1<N> {
    let scheduled = case.case.step.scheduled_dispatch();
    if final_rows.len() != scheduled.member_count() {
        return Err((
            M1CompletedOutputCheckErrorV1::QualificationFinalChoiceCount {
                expected: scheduled.member_count(),
                actual: final_rows.len(),
            },
            case,
        ));
    }
    let mut expectations = Vec::new();
    if expectations
        .try_reserve_exact(scheduled.member_count())
        .is_err()
    {
        return Err((
            M1CompletedOutputCheckErrorV1::Output(crate::M1CompletionOutputErrorV1::ExtentOverflow),
            case,
        ));
    }
    for lane in 0..scheduled.member_count() {
        let Some(plan) = case.case.step.target_plans()[lane].as_ref() else {
            return Err((
                M1CompletedOutputCheckErrorV1::ExpectationCount {
                    expected: scheduled.member_count(),
                    actual: lane,
                },
                case,
            ));
        };
        let Some(choice) = final_rows.choice(lane) else {
            return Err((
                M1CompletedOutputCheckErrorV1::QualificationFinalChoiceCount {
                    expected: scheduled.member_count(),
                    actual: final_rows.len(),
                },
                case,
            ));
        };
        expectations.push(CompletionWireExpectation::new(
            plan,
            CompletionWireSemanticExpectation::DirectFinalRow {
                choice: choice.token(),
            },
        ));
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
    let M1ObservedCompletionCaseV1 { case, image } = *case;
    let checked =
        checked.retain_completion_canary_readback(image.into_completion_canary_readback());
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

fn check_observed_qualification_final_case<const N: usize>(
    case: Box<M1ObservedCompletionCaseV1<N>>,
    contexts: &[M1ValidatedQualificationContextStepV1],
    final_rows: &M1QualificationFinalRowChoicesV1,
) -> CheckObservedCaseResultV1<N> {
    let scheduled = case.case.step.scheduled_dispatch();
    if contexts.len() != scheduled.member_count() {
        return Err((
            M1CompletedOutputCheckErrorV1::ExpectationCount {
                expected: scheduled.member_count(),
                actual: contexts.len(),
            },
            case,
        ));
    }
    let mut expectations = Vec::new();
    if expectations.try_reserve_exact(contexts.len()).is_err() {
        return Err((
            M1CompletedOutputCheckErrorV1::Output(crate::M1CompletionOutputErrorV1::ExtentOverflow),
            case,
        ));
    }
    for (lane, context) in contexts.iter().enumerate() {
        let Some(plan) = case.case.step.target_plans()[lane].as_ref() else {
            return Err((
                M1CompletedOutputCheckErrorV1::ExpectationCount {
                    expected: scheduled.member_count(),
                    actual: lane,
                },
                case,
            ));
        };
        expectations.push(CompletionWireExpectation::new(
            plan,
            CompletionWireSemanticExpectation::QualificationFinalRow { context },
        ));
    }
    let checked = match check_m1_qualification_completed_output_v1(
        &case.image,
        case.case.custody.selection(),
        scheduled,
        &expectations,
        final_rows,
    ) {
        Ok(checked) => checked,
        Err(error) => return Err((error, case)),
    };
    let M1ObservedCompletionCaseV1 { case, image } = *case;
    let checked =
        checked.retain_completion_canary_readback(image.into_completion_canary_readback());
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
        M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(failed) => {
            M1QualificationObservationFailureCustodyV1::CompactSnapshotReadFailed(failed)
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
    /// Deliberately terminalizes a real recycled queue before any completed-read attempt.
    ///
    /// Success consumes recycled custody into a release-only lower typestate and
    /// permanently faults the Ferric Engine, which denies subsequent admission.
    /// This service transition does not synthesize a KFD error or claim a native
    /// device fault. An already faulted Engine or any prior completed-read attempt
    /// rejects without mutating the Engine and returns the exact recycled queue.
    ///
    /// ```compile_fail
    /// use ferric_engine::{Engine, M1PhysicalRecycledQueueSessionV1};
    /// fn inject_twice<const C: usize>(
    ///     queue: M1PhysicalRecycledQueueSessionV1,
    ///     engine: &mut Engine<C>,
    /// ) {
    ///     let _faulted = queue.inject_qualification_queue_transition_fault(engine);
    ///     let _again = queue.destroy_and_release();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns pure rejection with unchanged recycled custody.
    #[cfg(feature = "qualification-fault-injection")]
    pub fn inject_qualification_queue_transition_fault<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1QualificationQueueTransitionFaultSessionV1,
        M1QualificationQueueTransitionFaultInjectionRejectionV1,
    > {
        if engine.is_faulted() {
            return Err(
                M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                    reason:
                        M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::EngineAlreadyFaulted,
                    queue: Box::new(self),
                },
            );
        }
        let faulted = match self {
            Self::TargetOnly(case) => match inject_qualification_fault_case(case) {
                Ok(case) => M1QualificationQueueTransitionFaultSessionV1::TargetOnly(case),
                Err(case) => {
                    return Err(
                        M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                            reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::CompletedReadAlreadyAttempted,
                            queue: Box::new(Self::TargetOnly(case)),
                        },
                    )
                }
            },
            Self::PairedPrefill(case) => match inject_qualification_fault_case(case) {
                Ok(case) => M1QualificationQueueTransitionFaultSessionV1::PairedPrefill(case),
                Err(case) => {
                    return Err(
                        M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                            reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::CompletedReadAlreadyAttempted,
                            queue: Box::new(Self::PairedPrefill(case)),
                        },
                    )
                }
            },
            Self::SpeculativeK4(case) => match inject_qualification_fault_case(case) {
                Ok(case) => M1QualificationQueueTransitionFaultSessionV1::SpeculativeK4(case),
                Err(case) => {
                    return Err(
                        M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                            reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::CompletedReadAlreadyAttempted,
                            queue: Box::new(Self::SpeculativeK4(case)),
                        },
                    )
                }
            },
            Self::SpeculativeK8(case) => match inject_qualification_fault_case(case) {
                Ok(case) => M1QualificationQueueTransitionFaultSessionV1::SpeculativeK8(case),
                Err(case) => {
                    return Err(
                        M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                            reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::CompletedReadAlreadyAttempted,
                            queue: Box::new(Self::SpeculativeK8(case)),
                        },
                    )
                }
            },
            Self::SpeculativeK16(case) => match inject_qualification_fault_case(case) {
                Ok(case) => M1QualificationQueueTransitionFaultSessionV1::SpeculativeK16(case),
                Err(case) => {
                    return Err(
                        M1QualificationQueueTransitionFaultInjectionRejectionV1 {
                            reason: M1QualificationQueueTransitionFaultInjectionRejectionReasonV1::CompletedReadAlreadyAttempted,
                            queue: Box::new(Self::SpeculativeK16(case)),
                        },
                    )
                }
            },
        };
        engine.quarantine_m1_queue_rearm_failure();
        Ok(faulted)
    }

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
    /// An ordinary generic read failure preserves its existing retryable
    /// recycled custody. Guarded pre-copy validation is also retryable, but an
    /// attempted enclosing-snapshot read is terminally opaque on error. Every
    /// guarded rejection after a successful copy retains the whole enclosing
    /// snapshot in closed rejected custody, so the copy cannot be repeated. No
    /// completion authority is created on any path.
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
                        M1CompletionSnapshotReadFailedOutputV1::target_only,
                    )
                }),
            Self::PairedPrefill(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::PairedPrefill)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::PairedPrefill,
                        M1RejectedCompletionOutputV1::PairedPrefill,
                        M1CompletionSnapshotReadFailedOutputV1::paired_prefill,
                    )
                }),
            Self::SpeculativeK4(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK4)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK4,
                        M1RejectedCompletionOutputV1::SpeculativeK4,
                        M1CompletionSnapshotReadFailedOutputV1::speculative_k4,
                    )
                }),
            Self::SpeculativeK8(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK8)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK8,
                        M1RejectedCompletionOutputV1::SpeculativeK8,
                        M1CompletionSnapshotReadFailedOutputV1::speculative_k8,
                    )
                }),
            Self::SpeculativeK16(case) => observe_case(case)
                .map(M1ObservedCompletionOutputV1::SpeculativeK16)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1PhysicalRecycledQueueSessionV1::SpeculativeK16,
                        M1RejectedCompletionOutputV1::SpeculativeK16,
                        M1CompletionSnapshotReadFailedOutputV1::speculative_k16,
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
        self.check_completion_with_evidence_authority(
            M1CompletionEvidenceJoinAuthorityV1::Generic,
            expectations,
        )
    }

    fn check_completion_with_evidence_authority(
        self,
        authority: M1CompletionEvidenceJoinAuthorityV1,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1PhysicalCompletedReadbackV1, M1CompletedReadbackJoinFailureV1> {
        match self {
            Self::TargetOnly(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::TargetOnly,
                M1PhysicalReadbackQueueSessionV1::TargetOnly,
                authority,
                expectations,
            ),
            Self::PairedPrefill(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::PairedPrefill,
                M1PhysicalReadbackQueueSessionV1::PairedPrefill,
                authority,
                expectations,
            ),
            Self::SpeculativeK4(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK4,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK4,
                authority,
                expectations,
            ),
            Self::SpeculativeK8(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK8,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK8,
                authority,
                expectations,
            ),
            Self::SpeculativeK16(case) => join_observed_output_case(
                case,
                M1ObservedCompletionOutputV1::SpeculativeK16,
                M1PhysicalReadbackQueueSessionV1::SpeculativeK16,
                authority,
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

    pub(crate) fn destroy_and_release_retaining_image(
        self,
    ) -> Result<
        (
            ServiceQueueReleaseObservationV1,
            M1ObservedCompletionImageV1,
        ),
        Box<(M1PhysicalQueueReleaseFailureV1, M1ObservedCompletionImageV1)>,
    > {
        match self {
            Self::TargetOnly(case) => release_observed_case_retaining_image(
                *case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => release_observed_case_retaining_image(
                *case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => release_observed_case_retaining_image(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => release_observed_case_retaining_image(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => release_observed_case_retaining_image(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
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

    pub(crate) fn destroy_and_release_retaining_readback(
        self,
    ) -> Result<
        (ServiceQueueReleaseObservationV1, ServiceCompletedReadbackV1),
        Box<(M1PhysicalQueueReleaseFailureV1, ServiceCompletedReadbackV1)>,
    > {
        match self {
            Self::TargetOnly(case) => release_rejected_case_retaining_readback(
                *case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => release_rejected_case_retaining_readback(
                *case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => release_rejected_case_retaining_readback(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => release_rejected_case_retaining_readback(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => release_rejected_case_retaining_readback(
                *case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
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
        lower: Box::new(lower),
        custody: Box::new(custody),
        completion_progress_wait: None,
    }
}

fn operation_failure_with_completion_progress(
    shape: M1PhysicalFixedBatchShapeV1,
    step: M1PrepublicationStepCustodyV1,
    lower: ServiceQueueOperationFailureV1,
    custody: M1PhysicalQueueBatchCustodyV1,
    completion_progress_wait: M1CompletionProgressWaitDiagnosticV1,
) -> M1PhysicalQueueOperationFailureV1 {
    M1PhysicalQueueOperationFailureV1 {
        shape,
        step: Box::new(step),
        lower: Box::new(lower),
        custody: Box::new(custody),
        completion_progress_wait: Some(Box::new(completion_progress_wait)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        checked_completion_progress_total_scan_bound, m1_completion_progress_total_scan_bound_v1,
        read_m1_diagnostic_choice_ranges_v1, validate_completion_progress_observation,
        validate_generic_observed_semantics, wait_with_completion_progress_policy,
        CompletionProgressPollV1, CompletionProgressWaitFailureV1,
        CompletionWireSemanticExpectation, M1CompletedOutputCheckErrorV1,
        M1CompletionEvidenceJoinAuthorityV1, M1CompletionProgressObservationV1,
        M1CompletionProgressWaitDiagnosticV1, M1CompletionProgressWaitTerminalReasonV1,
        M1DiagnosticChoiceCopyCustodyV1, M1DiagnosticChoiceReadBackendV1,
        M1PhysicalFixedBatchShapeV1, M1PhysicalQueueCreateFailureClassV1, M1PhysicalQueuePhaseV1,
        M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
    };
    use crate::Engine;
    use std::cell::{Cell, RefCell};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum MockPollEventV1 {
        Pending(M1CompletionProgressObservationV1),
        Ready(M1CompletionProgressObservationV1),
        Fault,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum MockPollFailureV1 {
        Terminalized(u32),
        Fault(u32),
    }

    #[derive(Debug, Eq, PartialEq)]
    enum MockWaitOutcomeV1 {
        Ready(u32),
        LowerFault(MockPollFailureV1),
        Policy {
            lower: MockPollFailureV1,
            diagnostic: M1CompletionProgressWaitDiagnosticV1,
        },
    }

    fn pending_progress<const N: usize>(
        completed_count: u16,
        first_pending_batch_index: u16,
    ) -> M1CompletionProgressObservationV1 {
        let packet_count = u16::try_from(N).expect("mock packet count fits u16");
        M1CompletionProgressObservationV1 {
            packet_count,
            completed_count,
            pending_count: packet_count - completed_count,
            first_pending_batch_index: Some(first_pending_batch_index),
        }
    }

    fn ready_progress<const N: usize>() -> M1CompletionProgressObservationV1 {
        let packet_count = u16::try_from(N).expect("mock packet count fits u16");
        M1CompletionProgressObservationV1 {
            packet_count,
            completed_count: packet_count,
            pending_count: 0,
            first_pending_batch_index: None,
        }
    }

    fn drive_mock_progress_wait<const N: usize>(
        maximum_consecutive_stalled_scans: u32,
        events: &[MockPollEventV1],
    ) -> (MockWaitOutcomeV1, usize, Vec<u32>, usize) {
        let cursor = Cell::new(0_usize);
        let paces = Cell::new(0_usize);
        let terminalized = RefCell::new(Vec::new());
        let result = wait_with_completion_progress_policy::<N, _, _, _>(
            0_u32,
            maximum_consecutive_stalled_scans,
            |owner| {
                let index = cursor.get();
                let event = events[index];
                cursor.set(index + 1);
                match event {
                    MockPollEventV1::Pending(progress) => Ok(CompletionProgressPollV1::Pending {
                        session: owner + 1,
                        progress,
                    }),
                    MockPollEventV1::Ready(progress) => Ok(CompletionProgressPollV1::Ready {
                        session: owner + 1,
                        progress,
                    }),
                    MockPollEventV1::Fault => Err(MockPollFailureV1::Fault(owner)),
                }
            },
            || paces.set(paces.get() + 1),
            |owner| {
                terminalized.borrow_mut().push(owner);
                MockPollFailureV1::Terminalized(owner)
            },
        );
        let outcome = match result {
            Ok(completed) => MockWaitOutcomeV1::Ready(completed),
            Err(CompletionProgressWaitFailureV1::Lower(lower)) => {
                MockWaitOutcomeV1::LowerFault(lower)
            }
            Err(CompletionProgressWaitFailureV1::Policy { lower, diagnostic }) => {
                MockWaitOutcomeV1::Policy { lower, diagnostic }
            }
        };
        (
            outcome,
            cursor.get(),
            terminalized.into_inner(),
            paces.get(),
        )
    }

    #[derive(Debug, Eq, PartialEq)]
    struct InjectedReadErrorV1 {
        range: &'static str,
        message: &'static str,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct InjectedReadTeardownV1 {
        calls: Vec<u64>,
        destroy_calls: usize,
    }

    #[derive(Debug)]
    struct InjectedReadBackendV1 {
        calls: Vec<u64>,
        fail_at: Option<usize>,
    }

    impl M1DiagnosticChoiceReadBackendV1 for InjectedReadBackendV1 {
        type Range = u64;
        type Readback = Vec<u32>;
        type Error = InjectedReadErrorV1;
        type TeardownSuccess = InjectedReadTeardownV1;
        type TeardownFailure = core::convert::Infallible;

        fn read_completed(
            &mut self,
            range_name: &'static str,
            range: Self::Range,
        ) -> Result<Self::Readback, Self::Error> {
            let index = self.calls.len();
            self.calls.push(range);
            if self.fail_at == Some(index) {
                Err(InjectedReadErrorV1 {
                    range: range_name,
                    message: "injected read_completed fault",
                })
            } else {
                Ok(vec![u32::try_from(range).unwrap()])
            }
        }

        fn destroy_or_quarantine(self) -> Result<Self::TeardownSuccess, Self::TeardownFailure> {
            Ok(InjectedReadTeardownV1 {
                calls: self.calls,
                destroy_calls: 1,
            })
        }
    }

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
        #[cfg(feature = "qualification-fault-injection")]
        {
            let faulted = M1PhysicalQueuePhaseV1::QualificationFaulted;
            assert!(faulted.can_release_only());
            assert_eq!(0, grants_for(faulted));
        }
    }

    #[test]
    fn only_pure_creation_rejection_allows_input_recovery() {
        assert!(M1PhysicalQueueCreateFailureClassV1::Rejected.can_recover_inputs());
        assert!(!M1PhysicalQueueCreateFailureClassV1::Rejected.denies_retry());
        assert!(!M1PhysicalQueueCreateFailureClassV1::Terminal.can_recover_inputs());
        assert!(M1PhysicalQueueCreateFailureClassV1::Terminal.denies_retry());
    }

    #[test]
    fn completion_progress_increase_resets_the_small_injected_stall_lease() {
        let events = [
            MockPollEventV1::Pending(pending_progress::<3>(0, 0)),
            MockPollEventV1::Pending(pending_progress::<3>(1, 1)),
            MockPollEventV1::Pending(pending_progress::<3>(1, 1)),
            MockPollEventV1::Pending(pending_progress::<3>(2, 2)),
            MockPollEventV1::Pending(pending_progress::<3>(2, 0)),
            MockPollEventV1::Ready(ready_progress::<3>()),
        ];
        assert_eq!(
            drive_mock_progress_wait::<3>(3, &events),
            (MockWaitOutcomeV1::Ready(6), 6, Vec::new(), 5)
        );
    }

    #[test]
    fn completion_progress_stall_terminalizes_without_an_extra_scan() {
        let progress = pending_progress::<3>(0, 0);
        let events = [
            MockPollEventV1::Pending(progress),
            MockPollEventV1::Pending(progress),
            MockPollEventV1::Pending(progress),
        ];
        let (outcome, scans, terminalized, paces) = drive_mock_progress_wait::<3>(3, &events);
        let MockWaitOutcomeV1::Policy { lower, diagnostic } = outcome else {
            panic!("stall must terminalize through the lower zero-scan wait");
        };
        assert_eq!(lower, MockPollFailureV1::Terminalized(3));
        assert_eq!(scans, 3);
        assert_eq!(terminalized, vec![3]);
        assert_eq!(paces, 2);
        assert_eq!(
            diagnostic.reason(),
            M1CompletionProgressWaitTerminalReasonV1::ConsecutiveScansWithoutProgress
        );
        assert_eq!(diagnostic.scans_performed(), 3);
        assert_eq!(diagnostic.consecutive_scans_without_progress(), 3);
        assert_eq!(diagnostic.completed_count_high_water(), 0);
        assert_eq!(diagnostic.last_observation(), Some(progress));
    }

    #[test]
    fn production_stall_threshold_paces_every_retry_but_not_terminalization() {
        let progress = pending_progress::<1>(0, 0);
        let events = vec![
            MockPollEventV1::Pending(progress);
            M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1 as usize
        ];
        let (outcome, scans, terminalized, paces) = drive_mock_progress_wait::<1>(
            M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
            &events,
        );
        assert!(matches!(outcome, MockWaitOutcomeV1::Policy { .. }));
        assert_eq!(scans, events.len());
        assert_eq!(
            terminalized,
            vec![M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1]
        );
        assert_eq!(paces + 1, events.len());
    }

    #[test]
    fn completion_progress_regression_terminalizes_with_prior_high_water() {
        let regressing = pending_progress::<3>(0, 0);
        let events = [
            MockPollEventV1::Pending(pending_progress::<3>(1, 1)),
            MockPollEventV1::Pending(regressing),
        ];
        let (outcome, scans, terminalized, paces) = drive_mock_progress_wait::<3>(3, &events);
        let MockWaitOutcomeV1::Policy { diagnostic, .. } = outcome else {
            panic!("completed-count regression must terminalize");
        };
        assert_eq!(scans, 2);
        assert_eq!(terminalized, vec![2]);
        assert_eq!(paces, 1);
        assert_eq!(
            diagnostic.reason(),
            M1CompletionProgressWaitTerminalReasonV1::CompletedCountRegressed
        );
        assert_eq!(diagnostic.completed_count_high_water(), 1);
        assert_eq!(diagnostic.last_observation(), Some(regressing));
    }

    #[test]
    fn non_atomic_first_pending_index_is_diagnostic_not_a_prefix_claim() {
        let events = [
            MockPollEventV1::Pending(pending_progress::<4>(3, 0)),
            MockPollEventV1::Ready(ready_progress::<4>()),
        ];
        assert_eq!(
            drive_mock_progress_wait::<4>(2, &events),
            (MockWaitOutcomeV1::Ready(2), 2, Vec::new(), 1)
        );
    }

    #[test]
    fn ready_wins_the_small_injected_stall_lease_boundary() {
        let events = [
            MockPollEventV1::Pending(pending_progress::<2>(0, 0)),
            MockPollEventV1::Ready(ready_progress::<2>()),
        ];
        assert_eq!(
            drive_mock_progress_wait::<2>(2, &events),
            (MockWaitOutcomeV1::Ready(2), 2, Vec::new(), 1)
        );
    }

    #[test]
    fn pure_ready_validation_rejects_malformed_and_regressing_observations() {
        let malformed = M1CompletionProgressObservationV1 {
            packet_count: 3,
            completed_count: 2,
            pending_count: 1,
            first_pending_batch_index: Some(0),
        };
        assert_eq!(
            validate_completion_progress_observation(malformed, 3, true, 2),
            Err(M1CompletionProgressWaitTerminalReasonV1::ReadyObservationInvalid)
        );

        let regressing = M1CompletionProgressObservationV1 {
            packet_count: 3,
            completed_count: 3,
            pending_count: 0,
            first_pending_batch_index: None,
        };
        assert_eq!(
            validate_completion_progress_observation(regressing, 3, true, 4),
            Err(M1CompletionProgressWaitTerminalReasonV1::CompletedCountRegressed)
        );
    }

    #[test]
    fn malformed_pending_progress_terminalizes_and_lower_faults_remain_ordinary() {
        let malformed = M1CompletionProgressObservationV1 {
            packet_count: 3,
            completed_count: 1,
            pending_count: 1,
            first_pending_batch_index: Some(3),
        };
        let (outcome, scans, terminalized, paces) =
            drive_mock_progress_wait::<3>(3, &[MockPollEventV1::Pending(malformed)]);
        let MockWaitOutcomeV1::Policy { diagnostic, .. } = outcome else {
            panic!("malformed count sum must terminalize");
        };
        assert_eq!(scans, 1);
        assert_eq!(terminalized, vec![1]);
        assert_eq!(paces, 0);
        assert_eq!(
            diagnostic.reason(),
            M1CompletionProgressWaitTerminalReasonV1::CountSumMismatch
        );

        assert_eq!(
            drive_mock_progress_wait::<3>(3, &[MockPollEventV1::Fault]),
            (
                MockWaitOutcomeV1::LowerFault(MockPollFailureV1::Fault(0)),
                1,
                Vec::new(),
                0
            )
        );
    }

    #[test]
    fn production_progress_scan_bounds_are_exact_for_every_m1_shape() {
        let cases = [
            (M1PhysicalFixedBatchShapeV1::TargetOnly, 4_472_832),
            (M1PhysicalFixedBatchShapeV1::PairedPrefill, 7_946_240),
            (M1PhysicalFixedBatchShapeV1::SpeculativeK4, 18_374_656),
            (M1PhysicalFixedBatchShapeV1::SpeculativeK8, 32_268_288),
            (M1PhysicalFixedBatchShapeV1::SpeculativeK16, 60_055_552),
        ];
        for (shape, expected) in cases {
            assert_eq!(
                m1_completion_progress_total_scan_bound_v1(shape),
                Some(expected)
            );
            assert_eq!(
                checked_completion_progress_total_scan_bound(
                    shape.packet_count(),
                    M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
                ),
                Some(expected)
            );
        }
        assert_eq!(
            checked_completion_progress_total_scan_bound(usize::MAX, u32::MAX),
            None
        );
    }

    #[test]
    fn capture_attached_generic_readback_rejects_caller_direct_choice() {
        let direct = [CompletionWireSemanticExpectation::DirectFinalRow { choice: 41 }];
        assert!(matches!(
            validate_generic_observed_semantics(
                M1CompletionEvidenceJoinAuthorityV1::Generic,
                true,
                false,
                false,
                &direct,
            ),
            Err(M1CompletedOutputCheckErrorV1::QualificationCaptureRequiresEvidence { lane: 0 })
        ));
        assert!(matches!(
            validate_generic_observed_semantics(
                M1CompletionEvidenceJoinAuthorityV1::Generic,
                false,
                true,
                false,
                &direct,
            ),
            Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence)
        ));
        assert!(matches!(
            validate_generic_observed_semantics(
                M1CompletionEvidenceJoinAuthorityV1::Generic,
                false,
                false,
                true,
                &direct,
            ),
            Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence)
        ));
        assert!(validate_generic_observed_semantics(
            M1CompletionEvidenceJoinAuthorityV1::Generic,
            false,
            false,
            false,
            &direct,
        )
        .is_ok());
        assert!(validate_generic_observed_semantics(
            M1CompletionEvidenceJoinAuthorityV1::DirectDiagnostic,
            false,
            true,
            false,
            &direct,
        )
        .is_ok());
        assert!(matches!(
            validate_generic_observed_semantics(
                M1CompletionEvidenceJoinAuthorityV1::DirectDiagnostic,
                false,
                false,
                false,
                &direct,
            ),
            Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence)
        ));
        assert!(validate_generic_observed_semantics(
            M1CompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
            false,
            false,
            true,
            &direct,
        )
        .is_ok());
        assert!(matches!(
            validate_generic_observed_semantics(
                M1CompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
                false,
                false,
                false,
                &direct,
            ),
            Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence)
        ));
    }

    #[test]
    fn partial_choice_copy_failure_retains_every_completed_range_once() {
        let empty = M1DiagnosticChoiceCopyCustodyV1::<u32>::new();
        assert_eq!(empty.copies.len(), 0);
        assert_eq!(&*empty.into_partial(), &[] as &[u32]);

        let mut after_draft = M1DiagnosticChoiceCopyCustodyV1::new();
        after_draft.retain(11_u32);
        assert_eq!(after_draft.copies.len(), 1);
        assert_eq!(&*after_draft.into_partial(), &[11]);

        let mut complete = M1DiagnosticChoiceCopyCustodyV1::new();
        for value in 11_u32..=15 {
            complete.retain(value);
        }
        assert_eq!(complete.copies.len(), 5);
        assert_eq!(complete.into_complete().unwrap(), ([11, 12, 13, 14], 15));
    }

    #[test]
    fn five_diagnostic_ranges_are_ordered_and_each_fault_retains_its_exact_prefix() {
        let ranges = [0_u64, 4, 8, 12, 20];
        let names = ["draft-0", "draft-1", "draft-2", "draft-3", "target"];
        let (backend, draft, target) = read_m1_diagnostic_choice_ranges_v1(
            InjectedReadBackendV1 {
                calls: Vec::new(),
                fail_at: None,
            },
            ranges[..4].try_into().unwrap(),
            ranges[4],
        )
        .unwrap();
        assert_eq!(backend.calls, ranges);
        assert_eq!(draft, [vec![0], vec![4], vec![8], vec![12]]);
        assert_eq!(target, vec![20]);

        for fail_at in 0..ranges.len() {
            let failure = read_m1_diagnostic_choice_ranges_v1(
                InjectedReadBackendV1 {
                    calls: Vec::new(),
                    fail_at: Some(fail_at),
                },
                ranges[..4].try_into().unwrap(),
                ranges[4],
            )
            .unwrap_err();
            assert_eq!(
                failure.error(),
                &InjectedReadErrorV1 {
                    range: names[fail_at],
                    message: "injected read_completed fault",
                }
            );
            assert_eq!(failure.copied_choice_ranges(), fail_at);

            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let teardown = failure.destroy_or_quarantine(&mut engine).unwrap();
            assert!(engine.is_faulted());
            assert_eq!(
                teardown.error,
                InjectedReadErrorV1 {
                    range: names[fail_at],
                    message: "injected read_completed fault",
                }
            );
            assert_eq!(teardown.partial.len(), fail_at);
            assert_eq!(
                teardown.partial,
                ranges[..fail_at]
                    .iter()
                    .map(|range| vec![u32::try_from(*range).unwrap()])
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            );
            assert_eq!(teardown.teardown.calls, ranges[..=fail_at]);
            assert_eq!(teardown.teardown.destroy_calls, 1);
        }
    }

    fn grants_for(phase: M1PhysicalQueuePhaseV1) -> usize {
        usize::from(phase.can_submit())
            + usize::from(phase.can_wait())
            + usize::from(phase.can_recycle())
            + usize::from(phase.can_read_detach_or_release())
    }
}
