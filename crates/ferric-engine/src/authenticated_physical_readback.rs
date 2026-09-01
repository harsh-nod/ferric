//! Authenticated compact-completion observation and semantic join.
//!
//! This boundary copies the exact completed K7 output once while retaining
//! authenticated program history, then joins the inert image to scheduler and
//! KV authority without exposing a raw queue or any diagnostic capture path.

use core::fmt;

use fe2o3_host::{
    AuthenticatedServiceQueueOperationFailureV1, AuthenticatedServiceQueueReleaseFailureV1,
    AuthenticatedServiceQueueReleaseV1, AuthenticatedServiceQueueUnboundSessionV1,
    AuthenticatedServiceRecycledQueueSessionV1,
};
use fe2o3_kfd::ComputeAqlQueueObservationV1;
use fe2o3_service_host::{
    ServiceCompletedReadbackV1, ServiceQueueErrorV1, ServiceQueueReleaseFailureV1,
};
use ferric_spec::{completion::CompletionEpoch, Identity};

use crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1;
use crate::completed_readback_join::check_m1_completed_output_v1;
use crate::observed_completion::{
    observe_m1_completed_output_v1, observe_m1_guarded_completed_output_v1,
};
use crate::{
    preflight_m1_completion_canary_v1, validate_m1_completion_canary_readback_v1,
    CompletionWireExpectation, CompletionWireSemanticExpectation, DeclaredOperationKernelPlan,
    Engine, ExactCompletion, Gfx942DeviceBinding, M1AuthenticatedPhysicalQueuePhaseCaseV1,
    M1AuthenticatedPhysicalRecycledQueueSessionV1, M1CheckedCompletionOutputV1,
    M1CompletedOutputCheckErrorV1, M1CompletionObservationErrorV1,
    M1FullStepKvReservationCustodyV1, M1ObservedCompletionImageV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalQueueBatchCustodyV1, M1PrepublicationStepCustodyV1, M1ScheduledDispatchV1,
    M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
    M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// One authenticated recycled queue generation paired with its completed output copy.
#[must_use = "observed bytes and authenticated queue custody remain linear"]
pub struct M1AuthenticatedObservedCompletionCaseV1<const N: usize> {
    case:
        Box<M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>>,
    image: M1ObservedCompletionImageV1,
}

impl<const N: usize> fmt::Debug for M1AuthenticatedObservedCompletionCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedObservedCompletionCaseV1")
            .field("device", &self.case.device())
            .field("program_catalog_id", &self.case.program_catalog_id())
            .field("runner_declaration_id", &self.case.runner_declaration_id())
            .field("kernel_catalog_id", &self.case.kernel_catalog_id())
            .field("queue_epoch", &self.case.queue_epoch())
            .finish_non_exhaustive()
    }
}

/// Move-only authenticated queue after one exact completed K7 copy.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedObservedCompletionOutputV1;
/// fn read_twice(observed: M1AuthenticatedObservedCompletionOutputV1) {
///     let _ = observed.observe_completion();
/// }
/// ```
#[must_use = "authenticated observed completion must be checked or retained"]
#[derive(Debug)]
pub enum M1AuthenticatedObservedCompletionOutputV1 {
    TargetOnly(Box<M1AuthenticatedObservedCompletionCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    PairedPrefill(
        Box<M1AuthenticatedObservedCompletionCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK4(
        Box<M1AuthenticatedObservedCompletionCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK8(
        Box<M1AuthenticatedObservedCompletionCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK16(
        Box<M1AuthenticatedObservedCompletionCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

macro_rules! observed_case_ref {
    ($self:expr, $field:ident) => {
        match $self {
            M1AuthenticatedObservedCompletionOutputV1::TargetOnly(case) => &case.$field,
            M1AuthenticatedObservedCompletionOutputV1::PairedPrefill(case) => &case.$field,
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => &case.$field,
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => &case.$field,
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => &case.$field,
        }
    };
}

macro_rules! observed_case_call {
    ($self:expr, $method:ident) => {
        match $self {
            M1AuthenticatedObservedCompletionOutputV1::TargetOnly(case) => case.case.$method(),
            M1AuthenticatedObservedCompletionOutputV1::PairedPrefill(case) => case.case.$method(),
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => case.case.$method(),
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => case.case.$method(),
            M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => case.case.$method(),
        }
    };
}

impl M1AuthenticatedObservedCompletionOutputV1 {
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

    /// Inert copied image retained without another completed-read capability.
    #[must_use = "the observed image remains paired with authenticated custody"]
    pub const fn image(&self) -> &M1ObservedCompletionImageV1 {
        observed_case_ref!(self, image)
    }

    /// Exact scheduler dispatch retained until the semantic join succeeds.
    #[must_use = "scheduler authority remains paired with the observation"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        observed_case_call!(self, scheduled_dispatch)
    }

    /// Checked physical-device receipt retained through observation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        observed_case_call!(self, device)
    }

    /// Exact authenticated program-catalog identity.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        observed_case_call!(self, program_catalog_id)
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        observed_case_call!(self, runner_declaration_id)
    }

    /// Exact structural kernel-catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        observed_case_call!(self, kernel_catalog_id)
    }
}

/// One authenticated queue paired with a completed copy that failed structural observation.
#[must_use = "rejected completed bytes and authenticated queue custody remain retained"]
pub struct M1AuthenticatedRejectedCompletionCaseV1<const N: usize> {
    case:
        Box<M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>>,
    readback: ServiceCompletedReadbackV1,
}

impl<const N: usize> fmt::Debug for M1AuthenticatedRejectedCompletionCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedRejectedCompletionCaseV1")
            .field("device", &self.case.device())
            .field("program_catalog_id", &self.case.program_catalog_id())
            .field("runner_declaration_id", &self.case.runner_declaration_id())
            .field("kernel_catalog_id", &self.case.kernel_catalog_id())
            .field("queue_epoch", &self.case.queue_epoch())
            .finish_non_exhaustive()
    }
}

/// Closed post-copy structural rejection. It exposes no completed-read transition.
#[must_use = "rejected authenticated completion evidence must remain retained"]
#[derive(Debug)]
pub enum M1AuthenticatedRejectedCompletionOutputV1 {
    TargetOnly(Box<M1AuthenticatedRejectedCompletionCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    PairedPrefill(
        Box<M1AuthenticatedRejectedCompletionCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK4(
        Box<M1AuthenticatedRejectedCompletionCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK8(
        Box<M1AuthenticatedRejectedCompletionCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK16(
        Box<M1AuthenticatedRejectedCompletionCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedRejectedCompletionOutputV1 {
    /// Exact former publication shape.
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
}

/// Closed authenticated owner after an enclosing-snapshot read became ambiguous.
#[must_use = "snapshot-read failure custody must remain closed"]
pub enum M1AuthenticatedCompletionSnapshotReadFailedOutputV1 {
    TargetOnly(
        Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<
                AuthenticatedServiceRecycledQueueSessionV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
            >,
        >,
    ),
    PairedPrefill(
        Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<
                AuthenticatedServiceRecycledQueueSessionV1<
                    M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
                >,
            >,
        >,
    ),
    SpeculativeK4(
        Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<
                AuthenticatedServiceRecycledQueueSessionV1<
                    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
                >,
            >,
        >,
    ),
    SpeculativeK8(
        Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<
                AuthenticatedServiceRecycledQueueSessionV1<
                    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
                >,
            >,
        >,
    ),
    SpeculativeK16(
        Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<
                AuthenticatedServiceRecycledQueueSessionV1<
                    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
                >,
            >,
        >,
    ),
}

impl fmt::Debug for M1AuthenticatedCompletionSnapshotReadFailedOutputV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedCompletionSnapshotReadFailedOutputV1")
            .field("shape", &self.shape())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedCompletionSnapshotReadFailedOutputV1 {
    /// Exact former publication shape.
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
}

/// Linear custody retained by authenticated compact-observation failure.
#[must_use = "authenticated observation failure custody must remain retained"]
pub enum M1AuthenticatedCompletionObservationFailureCustodyV1 {
    /// No completed copy was attempted successfully, so ordinary retry remains available.
    Recycled(Box<M1AuthenticatedPhysicalRecycledQueueSessionV1>),
    /// An enclosing snapshot read was attempted and cannot be repeated safely.
    SnapshotReadFailed(Box<M1AuthenticatedCompletionSnapshotReadFailedOutputV1>),
    /// A completed copy succeeded and the copied evidence closes another read.
    Rejected(Box<M1AuthenticatedRejectedCompletionOutputV1>),
}

impl fmt::Debug for M1AuthenticatedCompletionObservationFailureCustodyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (state, shape) = match self {
            Self::Recycled(queue) => ("Recycled", queue.shape()),
            Self::SnapshotReadFailed(queue) => ("SnapshotReadFailed", queue.shape()),
            Self::Rejected(queue) => ("Rejected", queue.shape()),
        };
        formatter
            .debug_struct("M1AuthenticatedCompletionObservationFailureCustodyV1")
            .field("state", &state)
            .field("shape", &shape)
            .finish_non_exhaustive()
    }
}

/// Authenticated observation rejection with exhaustive queue and evidence custody.
#[must_use = "authenticated observation failure must be retried or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedCompletionObservationFailureV1 {
    error: M1CompletionObservationErrorV1,
    custody: M1AuthenticatedCompletionObservationFailureCustodyV1,
}

impl M1AuthenticatedCompletionObservationFailureV1 {
    /// Exact completed-copy or structural-observation rejection.
    #[must_use]
    pub const fn error(&self) -> &M1CompletionObservationErrorV1 {
        &self.error
    }

    /// Exact retained pre-copy or post-copy custody.
    #[must_use = "authenticated failure custody remains retained"]
    pub const fn custody(&self) -> &M1AuthenticatedCompletionObservationFailureCustodyV1 {
        &self.custody
    }

    /// Recovers the exact diagnostic and linear custody.
    #[must_use = "authenticated failure custody remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletionObservationErrorV1,
        M1AuthenticatedCompletionObservationFailureCustodyV1,
    ) {
        (self.error, self.custody)
    }

    /// Retries only a rejection that occurred before an ordinary completed copy.
    ///
    /// # Errors
    ///
    /// Returns renewed observation failure, or the unchanged closed owner when
    /// a snapshot read or successful completed copy already occurred.
    pub fn retry(self) -> Result<M1AuthenticatedObservedCompletionOutputV1, Box<Self>> {
        let Self { error, custody } = self;
        match custody {
            M1AuthenticatedCompletionObservationFailureCustodyV1::Recycled(queue) => {
                queue.observe_completion().map_err(Box::new)
            }
            custody
            @ (M1AuthenticatedCompletionObservationFailureCustodyV1::SnapshotReadFailed(
                _,
            )
            | M1AuthenticatedCompletionObservationFailureCustodyV1::Rejected(_)) => {
                Err(Box::new(Self { error, custody }))
            }
        }
    }

    /// Faults the logical Engine, destroys the authenticated queue, and retains
    /// any completed bytes already copied before structural rejection.
    ///
    /// # Errors
    ///
    /// Returns authenticated lower release quarantine joined to the original
    /// diagnostic, copied evidence, and every retained Ferric owner.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedReadbackTeardownSuccessV1,
        Box<M1AuthenticatedReadbackTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { error, custody } = self;
        let diagnostic = M1AuthenticatedReadbackTeardownDiagnosticV1::Observation(error);
        match custody {
            M1AuthenticatedCompletionObservationFailureCustodyV1::Recycled(queue) => {
                finish_authenticated_readback_teardown(
                    diagnostic,
                    M1AuthenticatedReadbackTeardownEvidenceV1::None,
                    release_authenticated_recycled_queue(*queue),
                )
            }
            M1AuthenticatedCompletionObservationFailureCustodyV1::SnapshotReadFailed(output) => {
                finish_authenticated_readback_teardown(
                    diagnostic,
                    M1AuthenticatedReadbackTeardownEvidenceV1::None,
                    release_authenticated_snapshot_failed_output(*output),
                )
            }
            M1AuthenticatedCompletionObservationFailureCustodyV1::Rejected(output) => {
                match release_authenticated_rejected_output(*output) {
                    Ok((queue_release, readback)) => Ok(M1AuthenticatedReadbackTeardownSuccessV1 {
                        diagnostic,
                        evidence: M1AuthenticatedReadbackTeardownEvidenceV1::Rejected(readback),
                        queue_release,
                    }),
                    Err(failure) => {
                        let (source, readback) = *failure;
                        Err(Box::new(M1AuthenticatedReadbackTeardownFailureV1 {
                            diagnostic,
                            evidence: M1AuthenticatedReadbackTeardownEvidenceV1::Rejected(readback),
                            source,
                        }))
                    }
                }
            }
        }
    }
}

enum ObserveCaseFailureV1<const N: usize> {
    BeforeCopy {
        error: M1CompletionObservationErrorV1,
        case: Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
        >,
    },
    SnapshotReadFailed {
        error: M1CompletionObservationErrorV1,
        case: Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
        >,
    },
    AfterCopy {
        error: M1CompletionObservationErrorV1,
        case: Box<
            M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
        >,
        readback: ServiceCompletedReadbackV1,
    },
}

fn observe_case<const N: usize>(
    mut case: Box<
        M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
    >,
) -> Result<Box<M1AuthenticatedObservedCompletionCaseV1<N>>, Box<ObserveCaseFailureV1<N>>> {
    let (lower, custody, step) = case.observation_parts();
    let output = custody.completion_output();
    let output_shape = output.shape();
    let range = output.retained_host_dispatch_range();
    let canary = output.completion_canary();
    let queue_selection = custody.selection();
    let scheduled = step.scheduled_dispatch();
    if let Some(canary) = canary {
        if let Err(error) = preflight_m1_completion_canary_v1(canary, range) {
            return Err(Box::new(ObserveCaseFailureV1::BeforeCopy {
                error: M1CompletionObservationErrorV1::Canary(error),
                case,
            }));
        }
        let request = lower.completed_snapshot_request(canary.snapshot_range());
        let readback = match lower.read_completed_snapshot(request) {
            Ok(readback) => readback,
            Err(error) => {
                return Err(Box::new(ObserveCaseFailureV1::SnapshotReadFailed {
                    error: M1CompletionObservationErrorV1::Queue(error),
                    case,
                }));
            }
        };
        let readback = match validate_m1_completion_canary_readback_v1(canary, range, readback) {
            Ok(readback) => readback,
            Err((error, readback)) => {
                return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
                    error: M1CompletionObservationErrorV1::Canary(error),
                    case,
                    readback,
                }));
            }
        };
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
                }));
            }
        };
        return Ok(Box::new(M1AuthenticatedObservedCompletionCaseV1 {
            case,
            image,
        }));
    }

    let request = lower.completed_read_request(range);
    let readback = match lower.read_completed(request) {
        Ok(readback) => readback,
        Err(error) => {
            return Err(Box::new(ObserveCaseFailureV1::BeforeCopy {
                error: M1CompletionObservationErrorV1::Queue(error),
                case,
            }));
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
    let image =
        match observe_m1_completed_output_v1(output_shape, queue_selection, scheduled, readback) {
            Ok(image) => image,
            Err((error, readback)) => {
                return Err(Box::new(ObserveCaseFailureV1::AfterCopy {
                    error: M1CompletionObservationErrorV1::Image(error),
                    case,
                    readback,
                }));
            }
        };
    Ok(Box::new(M1AuthenticatedObservedCompletionCaseV1 {
        case,
        image,
    }))
}

fn retain_observation_failure<const N: usize>(
    failure: ObserveCaseFailureV1<N>,
    recycled: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>>,
    ) -> M1AuthenticatedPhysicalRecycledQueueSessionV1,
    rejected: fn(
        Box<M1AuthenticatedRejectedCompletionCaseV1<N>>,
    ) -> M1AuthenticatedRejectedCompletionOutputV1,
    snapshot_read_failed: fn(
        Box<M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>>,
    ) -> M1AuthenticatedCompletionSnapshotReadFailedOutputV1,
) -> M1AuthenticatedCompletionObservationFailureV1 {
    match failure {
        ObserveCaseFailureV1::BeforeCopy { error, case } => {
            M1AuthenticatedCompletionObservationFailureV1 {
                error,
                custody: M1AuthenticatedCompletionObservationFailureCustodyV1::Recycled(Box::new(
                    recycled(case),
                )),
            }
        }
        ObserveCaseFailureV1::SnapshotReadFailed { error, case } => {
            M1AuthenticatedCompletionObservationFailureV1 {
                error,
                custody: M1AuthenticatedCompletionObservationFailureCustodyV1::SnapshotReadFailed(
                    Box::new(snapshot_read_failed(case)),
                ),
            }
        }
        ObserveCaseFailureV1::AfterCopy {
            error,
            case,
            readback,
        } => M1AuthenticatedCompletionObservationFailureV1 {
            error,
            custody: M1AuthenticatedCompletionObservationFailureCustodyV1::Rejected(Box::new(
                rejected(Box::new(M1AuthenticatedRejectedCompletionCaseV1 {
                    case,
                    readback,
                })),
            )),
        },
    }
}

impl M1AuthenticatedPhysicalRecycledQueueSessionV1 {
    /// Copies and structurally observes the exact completed K7 output once.
    ///
    /// Ordinary failures before a successful copy preserve retryable recycled
    /// custody. Snapshot-read ambiguity and every post-copy rejection are closed
    /// against another completed read. No exact completion is minted here.
    ///
    /// # Errors
    ///
    /// Returns retryable pre-copy custody or closed snapshot/post-copy custody
    /// paired with the exact structural diagnostic.
    pub fn observe_completion(
        self,
    ) -> Result<
        M1AuthenticatedObservedCompletionOutputV1,
        M1AuthenticatedCompletionObservationFailureV1,
    > {
        macro_rules! observe_variant {
            ($case:expr, $observed:ident, $recycled:ident, $rejected:ident, $snapshot:ident) => {
                observe_case($case)
                    .map(M1AuthenticatedObservedCompletionOutputV1::$observed)
                    .map_err(|failure| {
                        retain_observation_failure(
                            *failure,
                            M1AuthenticatedPhysicalRecycledQueueSessionV1::$recycled,
                            M1AuthenticatedRejectedCompletionOutputV1::$rejected,
                            M1AuthenticatedCompletionSnapshotReadFailedOutputV1::$snapshot,
                        )
                    })
            };
        }

        match self {
            Self::TargetOnly(case) => {
                observe_variant!(case, TargetOnly, TargetOnly, TargetOnly, TargetOnly)
            }
            Self::PairedPrefill(case) => observe_variant!(
                case,
                PairedPrefill,
                PairedPrefill,
                PairedPrefill,
                PairedPrefill
            ),
            Self::SpeculativeK4(case) => observe_variant!(
                case,
                SpeculativeK4,
                SpeculativeK4,
                SpeculativeK4,
                SpeculativeK4
            ),
            Self::SpeculativeK8(case) => observe_variant!(
                case,
                SpeculativeK8,
                SpeculativeK8,
                SpeculativeK8,
                SpeculativeK8
            ),
            Self::SpeculativeK16(case) => observe_variant!(
                case,
                SpeculativeK16,
                SpeculativeK16,
                SpeculativeK16,
                SpeculativeK16
            ),
        }
    }
}

/// Authenticated post-readback queue custody with no scheduler step attached.
#[must_use = "authenticated post-readback queue custody must remain retained"]
pub struct M1AuthenticatedPhysicalReadbackQueueCaseV1<const N: usize> {
    lower: AuthenticatedServiceRecycledQueueSessionV1<N>,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl<const N: usize> fmt::Debug for M1AuthenticatedPhysicalReadbackQueueCaseV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalReadbackQueueCaseV1")
            .field("device", &self.custody.device())
            .field("program_catalog_id", &self.witness.catalog_id())
            .field(
                "runner_declaration_id",
                &self.operations.runner_declaration_id(),
            )
            .field("kernel_catalog_id", &self.operations.kernel_catalog_id())
            .finish_non_exhaustive()
    }
}

impl<const N: usize> M1AuthenticatedPhysicalReadbackQueueCaseV1<N> {
    fn into_parts(
        self,
    ) -> (
        AuthenticatedServiceRecycledQueueSessionV1<N>,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        (self.lower, self.witness, self.operations, self.custody)
    }
}

/// Detached authenticated queue with retained program history and Ferric custody.
#[must_use = "the live detached authenticated queue and Ferric custody must remain retained"]
pub struct M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1 {
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl fmt::Debug for M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1")
            .field("device", &self.device())
            .field("program_catalog_id", &self.program_catalog_id())
            .field("runner_declaration_id", &self.runner_declaration_id())
            .field("kernel_catalog_id", &self.kernel_catalog_id())
            .field(
                "detached_dispatch_generation",
                &self.detached_dispatch_generation(),
            )
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1 {
    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric custody remains paired with the detached authenticated queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Checked physical-device receipt retained beside the live queue.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device()
    }

    /// Redacted lower queue observation retained after detachment.
    #[must_use]
    pub const fn observation(&self) -> ComputeAqlQueueObservationV1 {
        self.lower.observation()
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

    /// Completed lower dispatch generation that authorized detachment.
    #[must_use]
    pub const fn detached_dispatch_generation(&self) -> u64 {
        self.lower.detached_dispatch_generation()
    }
}

/// Closed former M1 shape after authenticated readback join and detachment.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1;
/// fn extract_raw(queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1) {
///     let _ = queue.into_raw();
/// }
/// ```
#[must_use = "the live detached authenticated queue and Ferric custody must remain retained"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1 {
    /// Detached target-only queue.
    TargetOnly(Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>),
    /// Detached paired-prefill queue.
    PairedPrefill(Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K4 speculative queue.
    SpeculativeK4(Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K8 speculative queue.
    SpeculativeK8(Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>),
    /// Detached K16 speculative queue.
    SpeculativeK16(Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>),
}

impl M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1 {
    /// Exact closed shape of the completed generation that was detached.
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

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric custody remains paired with the detached authenticated queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.custody(),
        }
    }

    /// Checked physical-device receipt retained beside the live queue.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.device(),
        }
    }

    /// Redacted lower queue observation retained after detachment.
    #[must_use]
    pub const fn observation(&self) -> ComputeAqlQueueObservationV1 {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.observation(),
        }
    }

    /// Exact authenticated program-catalog identity retained as history.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.program_catalog_id(),
        }
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.runner_declaration_id(),
        }
    }

    /// Exact structural kernel-catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.kernel_catalog_id(),
        }
    }

    /// Completed lower dispatch generation that authorized detachment.
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

    #[expect(
        dead_code,
        reason = "consumed by the staged authenticated retained-rearm integration"
    )]
    pub(crate) fn into_rearm_parts(
        self,
    ) -> (
        M1PhysicalFixedBatchShapeV1,
        AuthenticatedServiceQueueUnboundSessionV1,
        M1AuthenticatedProgramCatalogWitnessV1,
        DeclaredOperationKernelPlan,
        M1PhysicalQueueBatchCustodyV1,
    ) {
        let (shape, case) = match self {
            Self::TargetOnly(case) => (M1PhysicalFixedBatchShapeV1::TargetOnly, case),
            Self::PairedPrefill(case) => (M1PhysicalFixedBatchShapeV1::PairedPrefill, case),
            Self::SpeculativeK4(case) => (M1PhysicalFixedBatchShapeV1::SpeculativeK4, case),
            Self::SpeculativeK8(case) => (M1PhysicalFixedBatchShapeV1::SpeculativeK8, case),
            Self::SpeculativeK16(case) => (M1PhysicalFixedBatchShapeV1::SpeculativeK16, case),
        };
        let M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1 {
            lower,
            witness,
            operations,
            custody,
        } = *case;
        (shape, lower, witness, operations, custody)
    }
}

/// Terminal authenticated post-readback transition failure retaining every owner.
#[must_use = "terminal failure retains authenticated quarantine and Ferric custody"]
pub struct M1AuthenticatedPhysicalReadbackQueueOperationFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: AuthenticatedServiceQueueOperationFailureV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl fmt::Debug for M1AuthenticatedPhysicalReadbackQueueOperationFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalReadbackQueueOperationFailureV1")
            .field("shape", &self.shape)
            .field("error", &self.error())
            .field("device", &self.custody.device())
            .field("program_catalog_id", &self.witness.catalog_id())
            .field(
                "runner_declaration_id",
                &self.operations.runner_declaration_id(),
            )
            .field("kernel_catalog_id", &self.operations.kernel_catalog_id())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPhysicalReadbackQueueOperationFailureV1 {
    /// Exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Exact lower queue error without exposing the lower queue owner.
    #[must_use]
    pub const fn error(&self) -> &ServiceQueueErrorV1 {
        self.lower.error()
    }

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric custody remains paired with authenticated quarantine"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Exact authenticated program-catalog identity retained by quarantine.
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
}

/// Terminal authenticated post-readback release failure retaining every owner.
#[must_use = "post-readback release failure retains authenticated and Ferric custody"]
pub struct M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: AuthenticatedServiceQueueReleaseFailureV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
}

/// Ferric-only residue after authenticated post-readback release quarantine
/// is separated from lower program custody.
#[must_use = "authenticated release residue retains Ferric identity and allocation custody"]
pub struct M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
}

impl fmt::Debug for M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1")
            .field("shape", &self.shape)
            .field("device", &self.custody.device())
            .field("program_catalog_id", &self.witness.catalog_id())
            .field(
                "runner_declaration_id",
                &self.operations.runner_declaration_id(),
            )
            .field("kernel_catalog_id", &self.operations.kernel_catalog_id())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1 {
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    #[must_use = "Ferric custody remains paired with authenticated release residue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.witness.catalog_id()
    }

    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        self.operations.runner_declaration_id()
    }

    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        self.operations.kernel_catalog_id()
    }
}

impl fmt::Debug for M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1")
            .field("shape", &self.shape)
            .field("error", &self.error())
            .field("device", &self.custody.device())
            .field("program_catalog_id", &self.witness.catalog_id())
            .field(
                "runner_declaration_id",
                &self.operations.runner_declaration_id(),
            )
            .field("kernel_catalog_id", &self.operations.kernel_catalog_id())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
    /// Exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Exact lower terminal release error without exposing queue custody.
    #[must_use = "the lower release failure remains retained by authenticated quarantine"]
    pub const fn error(&self) -> &ServiceQueueReleaseFailureV1 {
        self.lower.error()
    }

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric custody remains paired with authenticated quarantine"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Exact authenticated program-catalog identity retained by quarantine.
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

    /// Separates lower authenticated release quarantine from Ferric-only
    /// identity and allocation residue without exposing a raw queue.
    #[must_use = "lower program custody and Ferric release residue remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        AuthenticatedServiceQueueReleaseFailureV1,
        M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1,
    ) {
        (
            self.lower,
            M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1 {
                shape: self.shape,
                witness: self.witness,
                operations: self.operations,
                custody: self.custody,
            },
        )
    }
}

/// Terminal authenticated release failure retaining every pre-join owner.
#[must_use = "terminal release failure retains authenticated and Ferric custody"]
pub struct M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: AuthenticatedServiceQueueReleaseFailureV1,
    witness: M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
}

impl fmt::Debug for M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1")
            .field("shape", &self.shape)
            .field("error", &self.error())
            .field("device", &self.custody.device())
            .field("program_catalog_id", &self.witness.catalog_id())
            .field(
                "runner_declaration_id",
                &self.operations.runner_declaration_id(),
            )
            .field("kernel_catalog_id", &self.operations.kernel_catalog_id())
            .field("queue_epoch", &self.queue_epoch())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
    /// Exact failed M1 shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    /// Exact lower terminal release error without exposing queue custody.
    #[must_use = "the lower release failure remains retained by authenticated quarantine"]
    pub const fn error(&self) -> &ServiceQueueReleaseFailureV1 {
        self.lower.error()
    }

    /// Exact Ferric allocation, recipe, and model-memory custody.
    #[must_use = "Ferric custody remains paired with authenticated quarantine"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody
    }

    /// Exact scheduler dispatch retained by the failed release.
    #[must_use = "scheduler authority remains paired with authenticated quarantine"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.step.scheduled_dispatch()
    }

    /// Immutable scheduler-issued queue epoch.
    #[must_use]
    pub const fn queue_epoch(&self) -> CompletionEpoch {
        self.step.scheduled_dispatch().epoch()
    }

    /// Exact authenticated program-catalog identity retained by quarantine.
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
}

/// Diagnostic retained after an authenticated readback failure is torn down.
#[derive(Debug)]
pub enum M1AuthenticatedReadbackTeardownDiagnosticV1 {
    /// Structural completed-copy or observation rejection.
    Observation(M1CompletionObservationErrorV1),
    /// Scheduler, plan, wire, or token-semantic rejection.
    Semantic(M1CompletedOutputCheckErrorV1),
}

/// Copied evidence retained after authenticated readback teardown.
pub enum M1AuthenticatedReadbackTeardownEvidenceV1 {
    /// No completed byte owner exists; a snapshot read may have become ambiguous.
    None,
    /// One completed copy was structurally rejected.
    Rejected(ServiceCompletedReadbackV1),
    /// Structurally decoded output used by a rejected semantic join.
    Observed(M1ObservedCompletionImageV1),
}

impl fmt::Debug for M1AuthenticatedReadbackTeardownEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("None"),
            Self::Rejected(readback) => formatter
                .debug_struct("Rejected")
                .field("dispatch_generation", &readback.dispatch_generation())
                .field("data_index", &readback.data_index())
                .field("offset_bytes", &readback.offset_bytes())
                .field("extent_bytes", &readback.bytes().len())
                .finish_non_exhaustive(),
            Self::Observed(image) => formatter
                .debug_struct("Observed")
                .field("selection", &image.selection())
                .field("epoch", &image.epoch())
                .field("dispatch_generation", &image.dispatch_generation())
                .field("data_index", &image.data_index())
                .field("offset_bytes", &image.offset_bytes())
                .field("extent_bytes", &image.extent_bytes())
                .finish_non_exhaustive(),
        }
    }
}

/// Clean authenticated teardown retaining diagnostic, evidence, and program release.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedReadbackTeardownSuccessV1;
/// fn split_twice(teardown: M1AuthenticatedReadbackTeardownSuccessV1) {
///     let _first = teardown.into_parts();
///     let _second = teardown.into_parts();
/// }
/// ```
#[must_use = "diagnostic, evidence, and authenticated program release remain retained"]
pub struct M1AuthenticatedReadbackTeardownSuccessV1 {
    diagnostic: M1AuthenticatedReadbackTeardownDiagnosticV1,
    evidence: M1AuthenticatedReadbackTeardownEvidenceV1,
    queue_release: AuthenticatedServiceQueueReleaseV1,
}

impl fmt::Debug for M1AuthenticatedReadbackTeardownSuccessV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedReadbackTeardownSuccessV1")
            .field("diagnostic", &self.diagnostic)
            .field("evidence", &self.evidence)
            .field("queue_release", &self.queue_release)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedReadbackTeardownSuccessV1 {
    /// Exact diagnostic that caused terminal teardown.
    #[must_use]
    pub const fn diagnostic(&self) -> &M1AuthenticatedReadbackTeardownDiagnosticV1 {
        &self.diagnostic
    }

    /// Exact copied evidence, if any.
    #[must_use = "copied evidence remains retained"]
    pub const fn evidence(&self) -> &M1AuthenticatedReadbackTeardownEvidenceV1 {
        &self.evidence
    }

    /// Authenticated program release and native queue-destruction evidence.
    #[must_use = "released authenticated program sets remain explicitly owned"]
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
    }

    /// Separates the diagnostic, copied evidence, and authenticated release.
    #[must_use = "all teardown outputs must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedReadbackTeardownDiagnosticV1,
        M1AuthenticatedReadbackTeardownEvidenceV1,
        AuthenticatedServiceQueueReleaseV1,
    ) {
        (self.diagnostic, self.evidence, self.queue_release)
    }
}

/// Terminal authenticated release quarantine retaining diagnostic and evidence.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedReadbackTeardownFailureV1;
/// fn split_twice(teardown: M1AuthenticatedReadbackTeardownFailureV1) {
///     let _first = teardown.into_parts();
///     let _second = teardown.into_parts();
/// }
/// ```
#[must_use = "release quarantine, diagnostic, and copied evidence remain retained"]
pub struct M1AuthenticatedReadbackTeardownFailureV1 {
    diagnostic: M1AuthenticatedReadbackTeardownDiagnosticV1,
    evidence: M1AuthenticatedReadbackTeardownEvidenceV1,
    source: M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
}

impl fmt::Debug for M1AuthenticatedReadbackTeardownFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedReadbackTeardownFailureV1")
            .field("diagnostic", &self.diagnostic)
            .field("evidence", &self.evidence)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedReadbackTeardownFailureV1 {
    /// Exact diagnostic that caused terminal teardown.
    #[must_use]
    pub const fn diagnostic(&self) -> &M1AuthenticatedReadbackTeardownDiagnosticV1 {
        &self.diagnostic
    }

    /// Exact copied evidence, if any.
    #[must_use = "copied evidence remains retained"]
    pub const fn evidence(&self) -> &M1AuthenticatedReadbackTeardownEvidenceV1 {
        &self.evidence
    }

    /// Authenticated lower release quarantine and every retained owner.
    #[must_use = "authenticated release quarantine remains retained"]
    pub const fn source(&self) -> &M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }

    /// Separates the diagnostic, copied evidence, and terminal release quarantine.
    #[must_use = "all teardown failure owners must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedReadbackTeardownDiagnosticV1,
        M1AuthenticatedReadbackTeardownEvidenceV1,
        M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
    ) {
        (self.diagnostic, self.evidence, self.source)
    }
}

type AuthenticatedRejectedReleaseResultV1 = Result<
    (
        AuthenticatedServiceQueueReleaseV1,
        ServiceCompletedReadbackV1,
    ),
    Box<(
        M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
        ServiceCompletedReadbackV1,
    )>,
>;

type AuthenticatedObservedReleaseResultV1 = Result<
    (
        AuthenticatedServiceQueueReleaseV1,
        M1ObservedCompletionImageV1,
    ),
    Box<(
        M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
        M1ObservedCompletionImageV1,
    )>,
>;

fn release_authenticated_phase_case<const N: usize>(
    case: Box<
        M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
    >,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    AuthenticatedServiceQueueReleaseV1,
    Box<M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1>,
> {
    let (lower, witness, operations, custody, step) = (*case).into_parts();
    match lower.destroy_and_release() {
        Ok(release) => Ok(release),
        Err(lower) => Err(Box::new(
            M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
                shape,
                lower,
                witness,
                operations,
                custody,
                step,
            },
        )),
    }
}

fn release_authenticated_recycled_queue(
    queue: M1AuthenticatedPhysicalRecycledQueueSessionV1,
) -> Result<
    AuthenticatedServiceQueueReleaseV1,
    Box<M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1>,
> {
    match queue {
        M1AuthenticatedPhysicalRecycledQueueSessionV1::TargetOnly(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
        }
        M1AuthenticatedPhysicalRecycledQueueSessionV1::PairedPrefill(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
        }
        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK4(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
        }
        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK8(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
        }
        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK16(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
        }
    }
}

fn release_authenticated_snapshot_failed_output(
    output: M1AuthenticatedCompletionSnapshotReadFailedOutputV1,
) -> Result<
    AuthenticatedServiceQueueReleaseV1,
    Box<M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1>,
> {
    match output {
        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::TargetOnly(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
        }
        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::PairedPrefill(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
        }
        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK4(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
        }
        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK8(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
        }
        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK16(case) => {
            release_authenticated_phase_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
        }
    }
}

fn release_authenticated_rejected_case<const N: usize>(
    rejected: M1AuthenticatedRejectedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> AuthenticatedRejectedReleaseResultV1 {
    let M1AuthenticatedRejectedCompletionCaseV1 { case, readback } = rejected;
    match release_authenticated_phase_case(case, shape) {
        Ok(release) => Ok((release, readback)),
        Err(source) => Err(Box::new((*source, readback))),
    }
}

fn release_authenticated_rejected_output(
    output: M1AuthenticatedRejectedCompletionOutputV1,
) -> AuthenticatedRejectedReleaseResultV1 {
    match output {
        M1AuthenticatedRejectedCompletionOutputV1::TargetOnly(case) => {
            release_authenticated_rejected_case(*case, M1PhysicalFixedBatchShapeV1::TargetOnly)
        }
        M1AuthenticatedRejectedCompletionOutputV1::PairedPrefill(case) => {
            release_authenticated_rejected_case(*case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
        }
        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK4(case) => {
            release_authenticated_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
        }
        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK8(case) => {
            release_authenticated_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
        }
        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK16(case) => {
            release_authenticated_rejected_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
        }
    }
}

fn release_authenticated_observed_case<const N: usize>(
    observed: M1AuthenticatedObservedCompletionCaseV1<N>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> AuthenticatedObservedReleaseResultV1 {
    let M1AuthenticatedObservedCompletionCaseV1 { case, image } = observed;
    match release_authenticated_phase_case(case, shape) {
        Ok(release) => Ok((release, image)),
        Err(source) => Err(Box::new((*source, image))),
    }
}

fn release_authenticated_observed_output(
    output: M1AuthenticatedObservedCompletionOutputV1,
) -> AuthenticatedObservedReleaseResultV1 {
    match output {
        M1AuthenticatedObservedCompletionOutputV1::TargetOnly(case) => {
            release_authenticated_observed_case(*case, M1PhysicalFixedBatchShapeV1::TargetOnly)
        }
        M1AuthenticatedObservedCompletionOutputV1::PairedPrefill(case) => {
            release_authenticated_observed_case(*case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => {
            release_authenticated_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => {
            release_authenticated_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => {
            release_authenticated_observed_case(*case, M1PhysicalFixedBatchShapeV1::SpeculativeK16)
        }
    }
}

fn finish_authenticated_readback_teardown(
    diagnostic: M1AuthenticatedReadbackTeardownDiagnosticV1,
    evidence: M1AuthenticatedReadbackTeardownEvidenceV1,
    release: Result<
        AuthenticatedServiceQueueReleaseV1,
        Box<M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1>,
    >,
) -> Result<M1AuthenticatedReadbackTeardownSuccessV1, Box<M1AuthenticatedReadbackTeardownFailureV1>>
{
    match release {
        Ok(queue_release) => Ok(M1AuthenticatedReadbackTeardownSuccessV1 {
            diagnostic,
            evidence,
            queue_release,
        }),
        Err(source) => Err(Box::new(M1AuthenticatedReadbackTeardownFailureV1 {
            diagnostic,
            evidence,
            source: *source,
        })),
    }
}

/// Closed authenticated queue after compact semantic readback joined successfully.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPhysicalReadbackQueueSessionV1;
/// fn extract_raw(queue: M1AuthenticatedPhysicalReadbackQueueSessionV1) {
///     let _ = queue.into_raw();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPhysicalReadbackQueueSessionV1;
/// fn detach_twice(queue: M1AuthenticatedPhysicalReadbackQueueSessionV1) {
///     let _first = queue.detach();
///     let _second = queue.detach();
/// }
/// ```
#[must_use = "authenticated post-readback queue custody must remain retained"]
#[derive(Debug)]
pub enum M1AuthenticatedPhysicalReadbackQueueSessionV1 {
    TargetOnly(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>,
    ),
    PairedPrefill(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK4(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK8(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    ),
    SpeculativeK16(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    ),
}

impl M1AuthenticatedPhysicalReadbackQueueSessionV1 {
    /// Exact former publication shape.
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

    /// Exact Ferric allocation and model-memory custody.
    #[must_use = "Ferric custody remains paired with the authenticated queue"]
    pub const fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => &case.custody,
            Self::PairedPrefill(case) => &case.custody,
            Self::SpeculativeK4(case) => &case.custody,
            Self::SpeculativeK8(case) => &case.custody,
            Self::SpeculativeK16(case) => &case.custody,
        }
    }

    pub(crate) fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1 {
        match self {
            Self::TargetOnly(case) => &mut case.custody,
            Self::PairedPrefill(case) => &mut case.custody,
            Self::SpeculativeK4(case) => &mut case.custody,
            Self::SpeculativeK8(case) => &mut case.custody,
            Self::SpeculativeK16(case) => &mut case.custody,
        }
    }

    /// Exact authenticated program-catalog identity retained after the join.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case) => case.witness.catalog_id(),
            Self::PairedPrefill(case) => case.witness.catalog_id(),
            Self::SpeculativeK4(case) => case.witness.catalog_id(),
            Self::SpeculativeK8(case) => case.witness.catalog_id(),
            Self::SpeculativeK16(case) => case.witness.catalog_id(),
        }
    }

    /// Detaches the completed batch while retaining authenticated program history.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated quarantine paired with every Ferric owner.
    pub fn detach(
        self,
    ) -> Result<
        M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
        Box<M1AuthenticatedPhysicalReadbackQueueOperationFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => {
                detach_authenticated_readback_case(case, M1PhysicalFixedBatchShapeV1::TargetOnly)
                    .map(M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1::TargetOnly)
            }
            Self::PairedPrefill(case) => {
                detach_authenticated_readback_case(case, M1PhysicalFixedBatchShapeV1::PairedPrefill)
                    .map(M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1::PairedPrefill)
            }
            Self::SpeculativeK4(case) => {
                detach_authenticated_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                    .map(M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1::SpeculativeK4)
            }
            Self::SpeculativeK8(case) => {
                detach_authenticated_readback_case(case, M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                    .map(M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1::SpeculativeK8)
            }
            Self::SpeculativeK16(case) => detach_authenticated_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            )
            .map(M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1::SpeculativeK16),
        }
    }

    /// Destroys the post-readback queue and releases authenticated programs.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated lower release quarantine paired with the
    /// exact witness, operation plan, and Ferric custody. No retry is claimed.
    pub fn destroy_and_release(
        self,
    ) -> Result<
        AuthenticatedServiceQueueReleaseV1,
        Box<M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1>,
    > {
        match self {
            Self::TargetOnly(case) => release_authenticated_post_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::TargetOnly,
            ),
            Self::PairedPrefill(case) => release_authenticated_post_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
            ),
            Self::SpeculativeK4(case) => release_authenticated_post_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            Self::SpeculativeK8(case) => release_authenticated_post_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            Self::SpeculativeK16(case) => release_authenticated_post_readback_case(
                case,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
        }
    }
}

fn release_authenticated_post_readback_case<const N: usize>(
    case: Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    AuthenticatedServiceQueueReleaseV1,
    Box<M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1>,
> {
    let (lower, witness, operations, custody) = (*case).into_parts();
    match lower.destroy_and_release() {
        Ok(release) => Ok(release),
        Err(lower) => Err(Box::new(
            M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
                shape,
                lower,
                witness,
                operations,
                custody,
            },
        )),
    }
}

fn detach_authenticated_readback_case<const N: usize>(
    case: Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<N>>,
    shape: M1PhysicalFixedBatchShapeV1,
) -> Result<
    Box<M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1>,
    Box<M1AuthenticatedPhysicalReadbackQueueOperationFailureV1>,
> {
    let (lower, witness, operations, custody) = (*case).into_parts();
    match lower.detach() {
        Ok(lower) => Ok(Box::new(
            M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1 {
                lower,
                witness,
                operations,
                custody,
            },
        )),
        Err(lower) => Err(Box::new(
            M1AuthenticatedPhysicalReadbackQueueOperationFailureV1 {
                shape,
                lower,
                witness,
                operations,
                custody,
            },
        )),
    }
}

type CheckObservedCaseResultV1<const N: usize> = Result<
    (
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<N>>,
        M1CheckedCompletionOutputV1,
        ExactCompletion,
        M1FullStepKvReservationCustodyV1,
    ),
    (
        M1CompletedOutputCheckErrorV1,
        Box<M1AuthenticatedObservedCompletionCaseV1<N>>,
    ),
>;

fn validate_generic_observed_semantics(
    qualification_capture_enabled: bool,
    direct_diagnostic_capture_enabled: bool,
    speculative_diagnostic_capture_enabled: bool,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> Result<(), M1CompletedOutputCheckErrorV1> {
    if direct_diagnostic_capture_enabled {
        return Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence);
    }
    if speculative_diagnostic_capture_enabled {
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

fn check_observed_case<const N: usize>(
    case: Box<M1AuthenticatedObservedCompletionCaseV1<N>>,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> CheckObservedCaseResultV1<N> {
    let scheduled = case.case.scheduled_dispatch();
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
        case.case
            .custody()
            .completion_output()
            .qualification_logits()
            .is_some(),
        case.case
            .custody()
            .completion_output()
            .direct_diagnostic_choices()
            .is_some(),
        case.case
            .custody()
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
        let Some(plan) = case.case.step().target_plans()[lane].as_ref() else {
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
        case.case.custody().selection(),
        scheduled,
        &expectations,
    ) {
        Ok(checked) => checked,
        Err(error) => return Err((error, case)),
    };
    let M1AuthenticatedObservedCompletionCaseV1 { case, image } = *case;
    let checked =
        checked.retain_completion_canary_readback(image.into_completion_canary_readback());
    let (lower, witness, operations, custody, step) = (*case).into_parts();
    let (scheduled, _target_plans, kv) = step.into_parts();
    let completion = ExactCompletion::from_completed_m1_queue_readback(scheduled);
    Ok((
        Box::new(M1AuthenticatedPhysicalReadbackQueueCaseV1 {
            lower,
            witness,
            operations,
            custody,
        }),
        checked,
        completion,
        kv,
    ))
}

/// Semantic-join rejection retaining the unchanged authenticated observation.
#[must_use = "authenticated join failure retains observed completion custody"]
#[derive(Debug)]
pub struct M1AuthenticatedCompletedReadbackJoinFailureV1 {
    error: M1CompletedOutputCheckErrorV1,
    observed: Box<M1AuthenticatedObservedCompletionOutputV1>,
}

impl M1AuthenticatedCompletedReadbackJoinFailureV1 {
    /// Exact scheduler, plan, wire, or token-semantic rejection.
    #[must_use]
    pub const fn error(&self) -> &M1CompletedOutputCheckErrorV1 {
        &self.error
    }

    /// Unchanged observed image and authenticated queue custody.
    #[must_use = "authenticated observed custody remains retained"]
    pub const fn observed(&self) -> &M1AuthenticatedObservedCompletionOutputV1 {
        &self.observed
    }

    /// Recovers the rejection and unchanged observed owner.
    #[must_use = "authenticated observed custody remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedOutputCheckErrorV1,
        M1AuthenticatedObservedCompletionOutputV1,
    ) {
        (self.error, *self.observed)
    }

    /// Rechecks corrected semantic expectations without another device read.
    ///
    /// # Errors
    ///
    /// Returns the same observed owner when corrected semantics still reject.
    pub fn retry(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1AuthenticatedPhysicalCompletedReadbackV1, Self> {
        (*self.observed).check_completion(expectations)
    }

    /// Faults the logical Engine, destroys the semantically rejected
    /// authenticated queue, and retains the exact observed image.
    ///
    /// # Errors
    ///
    /// Returns authenticated lower release quarantine joined to the semantic
    /// diagnostic, unchanged image, and every retained Ferric owner.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedReadbackTeardownSuccessV1,
        Box<M1AuthenticatedReadbackTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self { error, observed } = self;
        let diagnostic = M1AuthenticatedReadbackTeardownDiagnosticV1::Semantic(error);
        match release_authenticated_observed_output(*observed) {
            Ok((queue_release, image)) => Ok(M1AuthenticatedReadbackTeardownSuccessV1 {
                diagnostic,
                evidence: M1AuthenticatedReadbackTeardownEvidenceV1::Observed(image),
                queue_release,
            }),
            Err(failure) => {
                let (source, image) = *failure;
                Err(Box::new(M1AuthenticatedReadbackTeardownFailureV1 {
                    diagnostic,
                    evidence: M1AuthenticatedReadbackTeardownEvidenceV1::Observed(image),
                    source,
                }))
            }
        }
    }
}

/// Joined authenticated queue, checked records, exact completion, and KV custody.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPhysicalCompletedReadbackV1;
/// fn split_twice(joined: M1AuthenticatedPhysicalCompletedReadbackV1) {
///     let _first = joined.into_parts();
///     let _second = joined.into_parts();
/// }
/// ```
#[must_use = "authenticated completed readback authority must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedPhysicalCompletedReadbackV1 {
    queue: M1AuthenticatedPhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
}

impl M1AuthenticatedPhysicalCompletedReadbackV1 {
    /// Post-readback authenticated queue with no scheduler step attached.
    #[must_use = "authenticated queue custody remains retained"]
    pub const fn queue(&self) -> &M1AuthenticatedPhysicalReadbackQueueSessionV1 {
        &self.queue
    }

    /// Checked live records in scheduler-member order.
    #[must_use = "checked completion records remain retained"]
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    /// Exact completed scheduler epoch.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    pub(crate) const fn completion_authority(&self) -> &ExactCompletion {
        &self.completion
    }

    /// Pending KV reservations split from scheduler completion authority.
    #[must_use = "pending KV custody remains retained"]
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }

    /// Separates every post-readback owner exactly once.
    #[must_use = "all authenticated readback outputs must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedPhysicalReadbackQueueSessionV1,
        M1CheckedCompletionOutputV1,
        ExactCompletion,
        M1FullStepKvReservationCustodyV1,
    ) {
        (self.queue, self.checked, self.completion, self.kv)
    }
}

fn join_observed_output_case<const N: usize>(
    case: Box<M1AuthenticatedObservedCompletionCaseV1<N>>,
    observed: fn(
        Box<M1AuthenticatedObservedCompletionCaseV1<N>>,
    ) -> M1AuthenticatedObservedCompletionOutputV1,
    readback: fn(
        Box<M1AuthenticatedPhysicalReadbackQueueCaseV1<N>>,
    ) -> M1AuthenticatedPhysicalReadbackQueueSessionV1,
    expectations: &[CompletionWireSemanticExpectation<'_>],
) -> Result<M1AuthenticatedPhysicalCompletedReadbackV1, M1AuthenticatedCompletedReadbackJoinFailureV1>
{
    match check_observed_case(case, expectations) {
        Ok((case, checked, completion, kv)) => Ok(M1AuthenticatedPhysicalCompletedReadbackV1 {
            queue: readback(case),
            checked,
            completion,
            kv,
        }),
        Err((error, case)) => Err(M1AuthenticatedCompletedReadbackJoinFailureV1 {
            error,
            observed: Box::new(observed(case)),
        }),
    }
}

impl M1AuthenticatedObservedCompletionOutputV1 {
    /// Joins the copied image to exact target plans and generic semantic expectations.
    ///
    /// Failure retains the unchanged observation, so corrected semantics can be
    /// retried without another completed read. Success splits scheduler authority
    /// into one `ExactCompletion` and the still-pending KV reservation custody.
    ///
    /// # Errors
    ///
    /// Returns the unchanged observed owner on roster, plan, epoch, wire, or
    /// semantic rejection.
    pub fn check_completion(
        self,
        expectations: &[CompletionWireSemanticExpectation<'_>],
    ) -> Result<
        M1AuthenticatedPhysicalCompletedReadbackV1,
        M1AuthenticatedCompletedReadbackJoinFailureV1,
    > {
        match self {
            Self::TargetOnly(case) => join_observed_output_case(
                case,
                Self::TargetOnly,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::TargetOnly,
                expectations,
            ),
            Self::PairedPrefill(case) => join_observed_output_case(
                case,
                Self::PairedPrefill,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::PairedPrefill,
                expectations,
            ),
            Self::SpeculativeK4(case) => join_observed_output_case(
                case,
                Self::SpeculativeK4,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK4,
                expectations,
            ),
            Self::SpeculativeK8(case) => join_observed_output_case(
                case,
                Self::SpeculativeK8,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK8,
                expectations,
            ),
            Self::SpeculativeK16(case) => join_observed_output_case(
                case,
                Self::SpeculativeK16,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK16,
                expectations,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_generic_observed_semantics;
    use crate::M1CompletedOutputCheckErrorV1;

    #[test]
    fn generic_readback_denies_diagnostic_capture_routes() {
        assert!(matches!(
            validate_generic_observed_semantics(false, true, false, &[]),
            Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence)
        ));
        assert!(matches!(
            validate_generic_observed_semantics(false, false, true, &[]),
            Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence)
        ));
    }

    #[test]
    fn generic_readback_accepts_capture_free_semantic_route() {
        assert!(validate_generic_observed_semantics(false, false, false, &[]).is_ok());
    }
}
