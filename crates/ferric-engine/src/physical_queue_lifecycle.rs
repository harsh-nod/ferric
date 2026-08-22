//! Linear Ferric custody around one complete M1 fixed-batch queue generation.
//!
//! The generic service host owns KFD queue state and enforces publication,
//! completion, recycle, readback, detach, and release. This module keeps the
//! corresponding M1 recipe and allocation custody beside every generic phase.
//! It interprets neither kernel outputs nor logical completion records.

use core::fmt;

use fe2o3_service_host::{
    QuarantinedServiceQueueV1, ServiceAllocationSessionV1, ServiceCompletedQueueSessionV1,
    ServiceCompletedReadbackV1, ServicePublishedQueueSessionV1, ServiceQueueCreateFailureV1,
    ServiceQueueErrorV1, ServiceQueueOperationFailureV1, ServiceQueueReleaseFailureV1,
    ServiceQueueReleaseObservationV1, ServiceQueueSessionV1, ServiceQueueUnboundSessionV1,
    ServiceRecycledQueueSessionV1,
};

use crate::{
    M1PhysicalFixedBatchCaseV1, M1PhysicalFixedBatchCustodyV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalFixedBatchV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
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
    /// Every completion signal was recycled; readback, reuse, detach, or release is allowed.
    Recycled,
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

    /// Whether this phase grants raw completion readback or another terminal transition.
    #[must_use]
    pub const fn can_read_reuse_detach_or_release(self) -> bool {
        matches!(self, Self::Recycled)
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
}

impl<Q> M1PhysicalQueuePhaseCaseV1<Q> {
    const fn new(lower: Q, custody: M1PhysicalFixedBatchCustodyV1) -> Self {
        Self { lower, custody }
    }

    /// Returns retained Ferric recipe and allocation custody without exposing generic authority.
    #[must_use = "the exact Ferric custody remains paired with the generic queue"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    fn into_parts(self) -> (Q, M1PhysicalFixedBatchCustodyV1) {
        (self.lower, self.custody)
    }
}

impl<Q: fmt::Debug> fmt::Debug for M1PhysicalQueuePhaseCaseV1<Q> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1PhysicalQueuePhaseCaseV1")
            .field("lower", &self.lower)
            .field("custody", &self.custody)
            .finish()
    }
}

macro_rules! define_closed_queue_phase {
    ($name:ident, $lower:ident, $phase:expr, $must_use:literal) => {
        #[doc = $must_use]
        #[must_use = $must_use]
        #[derive(Debug)]
        pub enum $name {
            /// One complete target-only publication.
            TargetOnly(
                Box<M1PhysicalQueuePhaseCaseV1<$lower<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>>,
            ),
            /// One complete paired-prefill publication.
            PairedPrefill(
                Box<M1PhysicalQueuePhaseCaseV1<$lower<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>>,
            ),
            /// One complete K4 speculative publication, for either S1 or S8.
            SpeculativeK4(
                Box<M1PhysicalQueuePhaseCaseV1<$lower<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>>,
            ),
            /// One complete K8 speculative publication.
            SpeculativeK8(
                Box<M1PhysicalQueuePhaseCaseV1<$lower<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>>,
            ),
            /// One complete K16 speculative publication.
            SpeculativeK16(
                Box<M1PhysicalQueuePhaseCaseV1<$lower<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>>,
            ),
        }

        impl $name {
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

            /// Returns the exact phase represented by this closed owner.
            #[must_use]
            pub const fn phase(&self) -> M1PhysicalQueuePhaseV1 {
                $phase
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
        }
    };
}

define_closed_queue_phase!(
    M1PhysicalQueueSessionV1,
    ServiceQueueSessionV1,
    M1PhysicalQueuePhaseV1::Prepared,
    "a prepared M1 queue must be submitted or explicitly retained"
);
define_closed_queue_phase!(
    M1PhysicalPublishedQueueSessionV1,
    ServicePublishedQueueSessionV1,
    M1PhysicalQueuePhaseV1::Published,
    "a published M1 queue must complete or remain retained"
);
define_closed_queue_phase!(
    M1PhysicalCompletedQueueSessionV1,
    ServiceCompletedQueueSessionV1,
    M1PhysicalQueuePhaseV1::Completed,
    "a completed M1 queue must recycle every exact signal"
);
define_closed_queue_phase!(
    M1PhysicalRecycledQueueSessionV1,
    ServiceRecycledQueueSessionV1,
    M1PhysicalQueuePhaseV1::Recycled,
    "a recycled M1 queue must be reused, detached, released, or explicitly retained"
);

/// Queue creation rejection or terminal failure with exact Ferric custody.
#[must_use = "pure rejection retains exact inputs; terminal failure retains Ferric custody"]
pub enum M1PhysicalQueueCreateFailureV1<'a> {
    /// Pre-transfer rejection with unchanged allocation and fixed-batch inputs.
    Rejected {
        /// Exact generic rejection.
        error: ServiceQueueErrorV1,
        /// Unchanged generic allocation session.
        allocations: Box<ServiceAllocationSessionV1>,
        /// Exact reconstructed closed M1 fixed batch.
        batch: Box<M1PhysicalFixedBatchV1<'a>>,
    },
    /// KFD may have consumed the generic inputs; only Ferric custody remains recoverable.
    Terminal {
        /// Exact generic terminal error.
        error: ServiceQueueErrorV1,
        /// Original fixed-batch shape.
        shape: M1PhysicalFixedBatchShapeV1,
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
                .finish_non_exhaustive(),
            Self::Terminal { error, shape, .. } => formatter
                .debug_struct("Terminal")
                .field("error", error)
                .field("shape", shape)
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

    /// Recovers unchanged inputs only after pure pre-transfer rejection.
    #[must_use = "pure rejection recovery returns both unchanged construction inputs"]
    pub fn into_rejected_inputs(
        self,
    ) -> Option<(ServiceAllocationSessionV1, M1PhysicalFixedBatchV1<'a>)> {
        match self {
            Self::Rejected {
                allocations, batch, ..
            } => Some((*allocations, *batch)),
            Self::Terminal { .. } => None,
        }
    }

    /// Recovers Ferric custody only after terminal queue creation failure.
    ///
    /// The returned custody grants no retry authority; the generic allocation
    /// and batch owners are unavailable after this failure class.
    #[must_use = "terminal Ferric custody must remain retained"]
    pub fn into_terminal_custody(self) -> Option<M1PhysicalFixedBatchCustodyV1> {
        match self {
            Self::Rejected { .. } => None,
            Self::Terminal { custody, .. } => Some(*custody),
        }
    }
}

/// Terminal consuming transition failure with generic quarantine and Ferric custody.
#[must_use = "terminal failure retains generic quarantine and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueOperationFailureV1,
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
}

impl M1PhysicalQueueOperationFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
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
    ) -> (QuarantinedServiceQueueV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower.into_quarantined(), *self.custody)
    }
}

/// Detached generic queue custody paired with the exact former M1 batch custody.
#[must_use = "the live detached queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub struct M1PhysicalDetachedQueueCaseV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalFixedBatchCustodyV1,
}

impl M1PhysicalDetachedQueueCaseV1 {
    /// Returns retained Ferric recipe and allocation custody by borrow.
    #[must_use = "the exact Ferric custody remains paired with the detached queue"]
    pub const fn custody(&self) -> &M1PhysicalFixedBatchCustodyV1 {
        &self.custody
    }

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "the live generic queue and Ferric custody must both remain retained"]
    pub fn into_parts(self) -> (ServiceQueueUnboundSessionV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower, self.custody)
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

    /// Separates the still-live generic queue from inert Ferric custody.
    #[must_use = "the live generic queue and Ferric custody must both remain retained"]
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

/// Terminal queue-release failure retaining the lower failure and Ferric custody.
#[must_use = "terminal release failure retains all available lower and Ferric custody"]
#[derive(Debug)]
pub struct M1PhysicalQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueReleaseFailureV1,
    custody: Box<M1PhysicalFixedBatchCustodyV1>,
}

impl M1PhysicalQueueReleaseFailureV1 {
    /// Returns the exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
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
    pub fn into_parts(self) -> (ServiceQueueReleaseFailureV1, M1PhysicalFixedBatchCustodyV1) {
        (self.lower, *self.custody)
    }
}

enum CreateCaseResultV1<'a, const N: usize> {
    Ready(Box<M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>>),
    Rejected {
        error: ServiceQueueErrorV1,
        allocations: Box<ServiceAllocationSessionV1>,
        batch: Box<M1PhysicalFixedBatchCaseV1<'a, N>>,
    },
    Terminal {
        error: ServiceQueueErrorV1,
        custody: Box<M1PhysicalFixedBatchCustodyV1>,
    },
}

fn create_case<const N: usize>(
    allocations: ServiceAllocationSessionV1,
    ring_bytes: u32,
    case: M1PhysicalFixedBatchCaseV1<'_, N>,
) -> CreateCaseResultV1<'_, N> {
    let (batch, custody) = case.into_parts();
    match ServiceQueueSessionV1::create(allocations, ring_bytes, batch) {
        Ok(lower) => {
            CreateCaseResultV1::Ready(Box::new(M1PhysicalQueuePhaseCaseV1::new(lower, custody)))
        }
        Err(ServiceQueueCreateFailureV1::Rejected {
            error,
            allocations,
            batch,
        }) => CreateCaseResultV1::Rejected {
            error,
            allocations,
            batch: Box::new(M1PhysicalFixedBatchCaseV1::from_parts(*batch, custody)),
        },
        Err(ServiceQueueCreateFailureV1::Terminal { error }) => CreateCaseResultV1::Terminal {
            error,
            custody: Box::new(custody),
        },
    }
}

macro_rules! create_closed_case {
    ($allocations:expr, $ring_bytes:expr, $case:expr, $shape:expr, $variant:ident) => {
        match create_case($allocations, $ring_bytes, *$case) {
            CreateCaseResultV1::Ready(case) => Ok(M1PhysicalQueueSessionV1::$variant(case)),
            CreateCaseResultV1::Rejected {
                error,
                allocations,
                batch,
            } => Err(M1PhysicalQueueCreateFailureV1::Rejected {
                error,
                allocations,
                batch: Box::new(M1PhysicalFixedBatchV1::$variant(batch)),
            }),
            CreateCaseResultV1::Terminal { error, custody } => {
                Err(M1PhysicalQueueCreateFailureV1::Terminal {
                    error,
                    shape: $shape,
                    custody,
                })
            }
        }
    };
}

impl M1PhysicalQueueSessionV1 {
    /// Consumes one allocation session and exact closed M1 fixed batch into a queue.
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
        batch: M1PhysicalFixedBatchV1<'_>,
    ) -> Result<Self, M1PhysicalQueueCreateFailureV1<'_>> {
        match batch {
            M1PhysicalFixedBatchV1::TargetOnly(case) => create_closed_case!(
                allocations,
                ring_bytes,
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                TargetOnly
            ),
            M1PhysicalFixedBatchV1::PairedPrefill(case) => create_closed_case!(
                allocations,
                ring_bytes,
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                PairedPrefill
            ),
            M1PhysicalFixedBatchV1::SpeculativeK4(case) => create_closed_case!(
                allocations,
                ring_bytes,
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                SpeculativeK4
            ),
            M1PhysicalFixedBatchV1::SpeculativeK8(case) => create_closed_case!(
                allocations,
                ring_bytes,
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                SpeculativeK8
            ),
            M1PhysicalFixedBatchV1::SpeculativeK16(case) => create_closed_case!(
                allocations,
                ring_bytes,
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                SpeculativeK16
            ),
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
        macro_rules! submit_case {
            ($case:expr, $shape:expr, $variant:ident) => {{
                let (lower, custody) = (*$case).into_parts();
                match lower.submit() {
                    Ok(lower) => Ok(M1PhysicalPublishedQueueSessionV1::$variant(Box::new(
                        M1PhysicalQueuePhaseCaseV1::new(lower, custody),
                    ))),
                    Err(lower) => Err(operation_failure($shape, lower, custody)),
                }
            }};
        }
        match self {
            Self::TargetOnly(case) => {
                submit_case!(case, M1PhysicalFixedBatchShapeV1::TargetOnly, TargetOnly)
            }
            Self::PairedPrefill(case) => submit_case!(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                PairedPrefill
            ),
            Self::SpeculativeK4(case) => submit_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                SpeculativeK4
            ),
            Self::SpeculativeK8(case) => submit_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                SpeculativeK8
            ),
            Self::SpeculativeK16(case) => submit_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                SpeculativeK16
            ),
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
        macro_rules! wait_case {
            ($case:expr, $shape:expr, $variant:ident) => {{
                let (lower, custody) = (*$case).into_parts();
                match lower.wait(polls) {
                    Ok(lower) => Ok(M1PhysicalCompletedQueueSessionV1::$variant(Box::new(
                        M1PhysicalQueuePhaseCaseV1::new(lower, custody),
                    ))),
                    Err(lower) => Err(operation_failure($shape, lower, custody)),
                }
            }};
        }
        match self {
            Self::TargetOnly(case) => {
                wait_case!(case, M1PhysicalFixedBatchShapeV1::TargetOnly, TargetOnly)
            }
            Self::PairedPrefill(case) => wait_case!(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                PairedPrefill
            ),
            Self::SpeculativeK4(case) => wait_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                SpeculativeK4
            ),
            Self::SpeculativeK8(case) => wait_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                SpeculativeK8
            ),
            Self::SpeculativeK16(case) => wait_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                SpeculativeK16
            ),
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
        macro_rules! recycle_case {
            ($case:expr, $shape:expr, $variant:ident) => {{
                let (lower, custody) = (*$case).into_parts();
                match lower.recycle() {
                    Ok(lower) => Ok(M1PhysicalRecycledQueueSessionV1::$variant(Box::new(
                        M1PhysicalQueuePhaseCaseV1::new(lower, custody),
                    ))),
                    Err(lower) => Err(operation_failure($shape, lower, custody)),
                }
            }};
        }
        match self {
            Self::TargetOnly(case) => {
                recycle_case!(case, M1PhysicalFixedBatchShapeV1::TargetOnly, TargetOnly)
            }
            Self::PairedPrefill(case) => recycle_case!(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                PairedPrefill
            ),
            Self::SpeculativeK4(case) => recycle_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                SpeculativeK4
            ),
            Self::SpeculativeK8(case) => recycle_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                SpeculativeK8
            ),
            Self::SpeculativeK16(case) => recycle_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                SpeculativeK16
            ),
        }
    }
}

impl M1PhysicalRecycledQueueSessionV1 {
    /// Copies the exact raw K7 completion-output range for this recycled generation.
    ///
    /// The request is minted from the current generic recycled owner and the
    /// range retained in this queue's Ferric custody. Returned bytes are not
    /// decoded or granted logical-completion meaning by this layer.
    ///
    /// # Errors
    ///
    /// Returns the generic generation, owner, range, or readback failure.
    pub fn read_raw_completion_output(
        &mut self,
    ) -> Result<ServiceCompletedReadbackV1, ServiceQueueErrorV1> {
        macro_rules! read_case {
            ($case:expr) => {{
                let range = $case
                    .custody
                    .completion_output()
                    .retained_host_dispatch_range();
                let request = $case.lower.completed_read_request(range);
                $case.lower.read_completed(request)
            }};
        }
        match self {
            Self::TargetOnly(case) => read_case!(case),
            Self::PairedPrefill(case) => read_case!(case),
            Self::SpeculativeK4(case) => read_case!(case),
            Self::SpeculativeK8(case) => read_case!(case),
            Self::SpeculativeK16(case) => read_case!(case),
        }
    }

    /// Reuses the exact attached fixed batch without rebuilding queue resources.
    #[must_use = "the prepared queue must remain retained"]
    pub fn reuse(self) -> M1PhysicalQueueSessionV1 {
        macro_rules! reuse_case {
            ($case:expr, $variant:ident) => {{
                let (lower, custody) = (*$case).into_parts();
                M1PhysicalQueueSessionV1::$variant(Box::new(M1PhysicalQueuePhaseCaseV1::new(
                    lower.reuse(),
                    custody,
                )))
            }};
        }
        match self {
            Self::TargetOnly(case) => reuse_case!(case, TargetOnly),
            Self::PairedPrefill(case) => reuse_case!(case, PairedPrefill),
            Self::SpeculativeK4(case) => reuse_case!(case, SpeculativeK4),
            Self::SpeculativeK8(case) => reuse_case!(case, SpeculativeK8),
            Self::SpeculativeK16(case) => reuse_case!(case, SpeculativeK16),
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
        macro_rules! detach_case {
            ($case:expr, $shape:expr, $variant:ident) => {{
                let (lower, custody) = (*$case).into_parts();
                match lower.detach() {
                    Ok(lower) => Ok(M1PhysicalDetachedQueueSessionV1::$variant(Box::new(
                        M1PhysicalDetachedQueueCaseV1 { lower, custody },
                    ))),
                    Err(lower) => Err(operation_failure($shape, lower, custody)),
                }
            }};
        }
        match self {
            Self::TargetOnly(case) => {
                detach_case!(case, M1PhysicalFixedBatchShapeV1::TargetOnly, TargetOnly)
            }
            Self::PairedPrefill(case) => detach_case!(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                PairedPrefill
            ),
            Self::SpeculativeK4(case) => detach_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                SpeculativeK4
            ),
            Self::SpeculativeK8(case) => detach_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                SpeculativeK8
            ),
            Self::SpeculativeK16(case) => detach_case!(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                SpeculativeK16
            ),
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
        macro_rules! release_case {
            ($case:expr, $shape:expr) => {{
                let (lower, custody) = (*$case).into_parts();
                match lower.destroy_and_release() {
                    Ok(observation) => Ok(observation),
                    Err(lower) => Err(M1PhysicalQueueReleaseFailureV1 {
                        shape: $shape,
                        lower,
                        custody: Box::new(custody),
                    }),
                }
            }};
        }
        match self {
            Self::TargetOnly(case) => {
                release_case!(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                release_case!(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                release_case!(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                release_case!(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => {
                release_case!(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
            }
        }
    }
}

fn operation_failure(
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueOperationFailureV1,
    custody: M1PhysicalFixedBatchCustodyV1,
) -> M1PhysicalQueueOperationFailureV1 {
    M1PhysicalQueueOperationFailureV1 {
        shape,
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
            M1PhysicalQueuePhaseV1::Detached,
            M1PhysicalQueuePhaseV1::Quarantined,
        ];
        for phase in phases {
            let grants = usize::from(phase.can_submit())
                + usize::from(phase.can_wait())
                + usize::from(phase.can_recycle())
                + usize::from(phase.can_read_reuse_detach_or_release());
            assert!(grants <= 1);
        }
        assert!(M1PhysicalQueuePhaseV1::Prepared.can_submit());
        assert!(M1PhysicalQueuePhaseV1::Published.can_wait());
        assert!(M1PhysicalQueuePhaseV1::Completed.can_recycle());
        assert!(M1PhysicalQueuePhaseV1::Recycled.can_read_reuse_detach_or_release());
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
            + usize::from(phase.can_read_reuse_detach_or_release())
    }
}
