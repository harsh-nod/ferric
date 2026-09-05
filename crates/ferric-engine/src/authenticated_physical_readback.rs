//! Authenticated compact-completion observation and semantic join.
//!
//! This boundary copies the exact completed K7 output once while retaining
//! authenticated program history, then joins the inert image to scheduler and
//! KV authority without exposing a raw queue. One explicitly demoted,
//! first-publication-only S1/K4 path may additionally copy the four draft rows
//! and one target matrix needed by the existing Ferric diagnostic semantics.

use core::fmt;

use fe2o3_host::{
    AuthenticatedServiceQueueOperationFailureV1, AuthenticatedServiceQueueReleaseFailureV1,
    AuthenticatedServiceQueueReleaseV1, AuthenticatedServiceQueueUnboundSessionV1,
    AuthenticatedServiceRecycledQueueSessionV1,
};
use fe2o3_kfd::ComputeAqlQueueObservationV1;
use fe2o3_service_host::{
    ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1, ServiceQueueErrorV1,
    ServiceQueueReleaseFailureV1,
};
use ferric_spec::{completion::CompletionEpoch, Identity, M1_MAX_ACTIVE_SEQUENCES};

use crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1;
use crate::completed_readback_join::check_m1_completed_output_v1;
use crate::observed_completion::{
    observe_m1_completed_output_v1, observe_m1_guarded_completed_output_v1,
};
use crate::speculative_diagnostic_choices::{
    m1_speculative_diagnostic_is_s1_k4_selection_v1, observe_m1_speculative_diagnostic_choices_v1,
};
use crate::{
    preflight_m1_completion_canary_v1, validate_m1_completion_canary_readback_v1,
    CompletionWireExpectation, CompletionWireSemanticExpectation, DeclaredOperationKernelPlan,
    Engine, ExactCompletion, Gfx942DeviceBinding, M1AuthenticatedPhysicalQueuePhaseCaseV1,
    M1AuthenticatedPhysicalRecycledQueueSessionV1, M1CheckedCompletionOutputV1,
    M1CompletedOutputCheckErrorV1, M1CompletionObservationErrorV1,
    M1FullStepKvReservationCustodyV1, M1ObservedCompletionImageV1,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalQueueBatchCustodyV1, M1PrepublicationStepCustodyV1, M1ScheduledDispatchV1,
    M1SpeculativeDiagnosticChoicesErrorV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Explicit authority demotion for authenticated first-publication S1/K4 diagnostics.
pub const M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1: &str = "partial-non-evidence";
/// Compatibility-neutral name for the same non-authoritative diagnostic status.
pub const M1_AUTHENTICATED_SPECULATIVE_DIAGNOSTIC_STATUS_V1: &str =
    M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1;
const M1_AUTHENTICATED_S1_K4_FIRST_DISPATCH_GENERATION_V1: u64 = 1;
const M1_AUTHENTICATED_S1_K4_DRAFT_RANGE_NAMES_V1: [&str; 4] =
    ["draft-0", "draft-1", "draft-2", "draft-3"];
const M1_AUTHENTICATED_SPECULATIVE_DRAFT_RANGE_NAMES_V1: [&str;
    crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1] = [
    "draft-0", "draft-1", "draft-2", "draft-3", "draft-4", "draft-5", "draft-6", "draft-7",
    "draft-8", "draft-9", "draft-10", "draft-11", "draft-12", "draft-13", "draft-14", "draft-15",
];

const fn is_authenticated_s1_k4_first_dispatch_generation(generation: u64) -> bool {
    generation == M1_AUTHENTICATED_S1_K4_FIRST_DISPATCH_GENERATION_V1
}

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
        match self {
            Self::TargetOnly(case) => &case.image,
            Self::PairedPrefill(case) => &case.image,
            Self::SpeculativeK4(case) => &case.image,
            Self::SpeculativeK8(case) => &case.image,
            Self::SpeculativeK16(case) => &case.image,
        }
    }

    /// Exact scheduler dispatch retained until the semantic join succeeds.
    #[must_use = "scheduler authority remains paired with the observation"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        match self {
            Self::TargetOnly(case) => case.case.scheduled_dispatch(),
            Self::PairedPrefill(case) => case.case.scheduled_dispatch(),
            Self::SpeculativeK4(case) => case.case.scheduled_dispatch(),
            Self::SpeculativeK8(case) => case.case.scheduled_dispatch(),
            Self::SpeculativeK16(case) => case.case.scheduled_dispatch(),
        }
    }

    /// Checked physical-device receipt retained through observation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::TargetOnly(case) => case.case.device(),
            Self::PairedPrefill(case) => case.case.device(),
            Self::SpeculativeK4(case) => case.case.device(),
            Self::SpeculativeK8(case) => case.case.device(),
            Self::SpeculativeK16(case) => case.case.device(),
        }
    }

    /// Exact authenticated program-catalog identity.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case) => case.case.program_catalog_id(),
            Self::PairedPrefill(case) => case.case.program_catalog_id(),
            Self::SpeculativeK4(case) => case.case.program_catalog_id(),
            Self::SpeculativeK8(case) => case.case.program_catalog_id(),
            Self::SpeculativeK16(case) => case.case.program_catalog_id(),
        }
    }

    /// Exact generated runner declaration identity.
    #[must_use]
    pub const fn runner_declaration_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case) => case.case.runner_declaration_id(),
            Self::PairedPrefill(case) => case.case.runner_declaration_id(),
            Self::SpeculativeK4(case) => case.case.runner_declaration_id(),
            Self::SpeculativeK8(case) => case.case.runner_declaration_id(),
            Self::SpeculativeK16(case) => case.case.runner_declaration_id(),
        }
    }

    /// Exact structural kernel-catalog identity.
    #[must_use]
    pub const fn kernel_catalog_id(&self) -> Identity {
        match self {
            Self::TargetOnly(case) => case.case.kernel_catalog_id(),
            Self::PairedPrefill(case) => case.case.kernel_catalog_id(),
            Self::SpeculativeK4(case) => case.case.kernel_catalog_id(),
            Self::SpeculativeK8(case) => case.case.kernel_catalog_id(),
            Self::SpeculativeK16(case) => case.case.kernel_catalog_id(),
        }
    }

    /// Copies the exact four draft rows and one target matrix for the
    /// first-generation target S1/K4 diagnostic.
    ///
    /// The compact K7 image has already been copied exactly once by
    /// [`M1AuthenticatedPhysicalRecycledQueueSessionV1::observe_completion`].
    /// This transition retains that observed owner, its authenticated program
    /// witness, and its queue session while issuing the same ordered
    /// `draft-0` through `draft-3`, then `target`, completed copies as the raw
    /// Ferric path. It is not implemented on any authenticated rearm wrapper.
    ///
    /// # Errors
    ///
    /// Rejects any non-S1/K4 owner, an absent diagnostic attachment, range
    /// derivation failure, completed-copy failure, or copied-coordinate/token
    /// rejection. Failure never exposes another diagnostic observation attempt.
    pub fn observe_speculative_k4_diagnostic_choices(
        mut self,
    ) -> Result<
        M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1,
        Box<M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1>,
    > {
        let (draft_ranges, target_range, dispatch_generation, live_sequences) = match &self {
            Self::SpeculativeK4(case) => {
                let selection = case.case.custody().completion_output().shape().selection();
                if !m1_speculative_diagnostic_is_s1_k4_selection_v1(selection) {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::NotS1K4,
                            self,
                            Box::new([]),
                        ),
                    ));
                }
                let dispatch_generation = case.image.dispatch_generation();
                if !is_authenticated_s1_k4_first_dispatch_generation(dispatch_generation) {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::NotFirstDispatchGeneration {
                                actual: dispatch_generation,
                            },
                            self,
                            Box::new([]),
                        ),
                    ));
                }
                let Some(owner) = case
                    .case
                    .custody()
                    .completion_output()
                    .speculative_diagnostic_choices()
                else {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::CaptureNotEnabled,
                            self,
                            Box::new([]),
                        ),
                    ));
                };
                let draft_ranges = match owner.retained_draft_read_ranges() {
                    Ok(ranges) => ranges,
                    Err(error) => {
                        return Err(Box::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                                M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Choices(
                                    error,
                                ),
                                self,
                                Box::new([]),
                            ),
                        ));
                    }
                };
                (
                    draft_ranges,
                    owner.retained_target_range(),
                    dispatch_generation,
                    u32::try_from(case.case.scheduled_dispatch().member_count())
                        .unwrap_or(u32::MAX),
                )
            }
            _ => {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::NotS1K4,
                        self,
                        Box::new([]),
                    ),
                ));
            }
        };

        let mut copies = Vec::new();
        if copies.try_reserve_exact(5).is_err() {
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::HostAllocation,
                    self,
                    Box::new([]),
                ),
            ));
        }
        for (index, range_name) in M1_AUTHENTICATED_S1_K4_DRAFT_RANGE_NAMES_V1
            .iter()
            .copied()
            .enumerate()
        {
            let Some(range) = draft_ranges[index] else {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::DraftRangeMissing {
                            iteration: index,
                        },
                        self,
                        copies.into_boxed_slice(),
                    ),
                ));
            };
            match read_authenticated_speculative_k4_choice(&mut self, range_name, range) {
                Ok(readback) => copies.push(readback),
                Err(source) => {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Queue {
                                range: range_name,
                                source,
                            },
                            self,
                            copies.into_boxed_slice(),
                        ),
                    ));
                }
            }
        }
        let target =
            match read_authenticated_speculative_k4_choice(&mut self, "target", target_range) {
                Ok(readback) => readback,
                Err(source) => {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Queue {
                                range: "target",
                                source,
                            },
                            self,
                            copies.into_boxed_slice(),
                        ),
                    ));
                }
            };
        let draft = copies.into_boxed_slice();
        let owner = if let Self::SpeculativeK4(case) = &self {
            case.case
                .custody()
                .completion_output()
                .speculative_diagnostic_choices()
        } else {
            let mut copies = draft.into_vec();
            copies.push(target);
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::NotS1K4,
                    self,
                    copies.into_boxed_slice(),
                ),
            ));
        };
        let Some(owner) = owner else {
            let mut copies = draft.into_vec();
            copies.push(target);
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::CaptureNotEnabled,
                    self,
                    copies.into_boxed_slice(),
                ),
            ));
        };
        let choices = match observe_m1_speculative_diagnostic_choices_v1(
            owner,
            dispatch_generation,
            live_sequences,
            draft,
            target,
        ) {
            Ok(choices) => choices,
            Err(failure) => {
                let (error, draft, target) = *failure;
                let mut copies = draft.into_vec();
                copies.push(target);
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Choices(error),
                        self,
                        copies.into_boxed_slice(),
                    ),
                ));
            }
        };
        Ok(M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1 {
            completion: self,
            choices,
        })
    }

    /// Copies the exact `K` draft rows and target matrix for any admitted
    /// finite speculative generation, including authenticated rearm rounds.
    ///
    /// This remains a diagnostic-only transition. It derives semantic values
    /// exclusively from completed device copies and grants no publication or
    /// verification authority.
    ///
    /// # Errors
    ///
    /// Rejects a non-speculative shape, absent capture owner, incomplete copy,
    /// coordinate drift, generation drift, or an out-of-vocabulary choice.
    pub fn observe_speculative_diagnostic_choices(
        mut self,
    ) -> Result<
        M1AuthenticatedObservedSpeculativeDiagnosticOutputV1,
        Box<M1AuthenticatedSpeculativeDiagnosticObservationFailureV1>,
    > {
        let (draft_ranges, target_range, generation, live_sequences, draft_tokens) =
            match authenticated_speculative_diagnostic_inputs(&self) {
                Ok(inputs) => inputs,
                Err(error) => {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            error,
                            self,
                            Box::new([]),
                        ),
                    ));
                }
            };

        let draft_tokens = usize::from(draft_tokens);
        let mut copies = Vec::new();
        if copies.try_reserve_exact(draft_tokens + 1).is_err() {
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::HostAllocation,
                    self,
                    Box::new([]),
                ),
            ));
        }
        for index in 0..draft_tokens {
            let range_name = M1_AUTHENTICATED_SPECULATIVE_DRAFT_RANGE_NAMES_V1[index];
            let Some(range) = draft_ranges[index] else {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::DraftRangeMissing {
                            iteration: index,
                        },
                        self,
                        copies.into_boxed_slice(),
                    ),
                ));
            };
            match read_authenticated_speculative_choice(&mut self, range_name, range) {
                Ok(readback) => copies.push(readback),
                Err(source) => {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                            M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Queue {
                                range: range_name,
                                source,
                            },
                            self,
                            copies.into_boxed_slice(),
                        ),
                    ));
                }
            }
        }
        let target = match read_authenticated_speculative_choice(&mut self, "target", target_range)
        {
            Ok(readback) => readback,
            Err(source) => {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Queue {
                            range: "target",
                            source,
                        },
                        self,
                        copies.into_boxed_slice(),
                    ),
                ));
            }
        };
        let draft = copies.into_boxed_slice();
        let Some(owner) = authenticated_speculative_diagnostic_owner(&self) else {
            let mut copies = draft.into_vec();
            copies.push(target);
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::CaptureNotEnabled,
                    self,
                    copies.into_boxed_slice(),
                ),
            ));
        };
        let choices = match observe_m1_speculative_diagnostic_choices_v1(
            owner,
            generation,
            live_sequences,
            draft,
            target,
        ) {
            Ok(choices) => choices,
            Err(failure) => {
                let (error, draft, target) = *failure;
                let mut copies = draft.into_vec();
                copies.push(target);
                return Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1::new(
                        M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Choices(error),
                        self,
                        copies.into_boxed_slice(),
                    ),
                ));
            }
        };
        Ok(M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1 {
            completion: self,
            choices,
        })
    }
}

type AuthenticatedSpeculativeDiagnosticInputsV1 = (
    [Option<ServiceHostDispatchRangeV1>; crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1],
    ServiceHostDispatchRangeV1,
    u64,
    u32,
    u8,
);

fn authenticated_speculative_diagnostic_inputs(
    completion: &M1AuthenticatedObservedCompletionOutputV1,
) -> Result<
    AuthenticatedSpeculativeDiagnosticInputsV1,
    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
> {
    match completion {
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => {
            authenticated_speculative_diagnostic_case_inputs(case)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => {
            authenticated_speculative_diagnostic_case_inputs(case)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => {
            authenticated_speculative_diagnostic_case_inputs(case)
        }
        _ => Err(M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::NotSpeculativeShape),
    }
}

fn authenticated_speculative_diagnostic_case_inputs<const N: usize>(
    case: &M1AuthenticatedObservedCompletionCaseV1<N>,
) -> Result<
    AuthenticatedSpeculativeDiagnosticInputsV1,
    M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
> {
    let Some(owner) = case
        .case
        .custody()
        .completion_output()
        .speculative_diagnostic_choices()
    else {
        return Err(M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::CaptureNotEnabled);
    };
    Ok((
        owner
            .retained_draft_read_ranges()
            .map_err(M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1::Choices)?,
        owner.retained_target_range(),
        case.image.dispatch_generation(),
        u32::try_from(case.case.scheduled_dispatch().member_count()).unwrap_or(u32::MAX),
        owner.shape().draft_tokens(),
    ))
}

fn authenticated_speculative_diagnostic_owner(
    completion: &M1AuthenticatedObservedCompletionOutputV1,
) -> Option<&crate::BoundM1SpeculativeDiagnosticChoicesV1> {
    match completion {
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => case
            .case
            .custody()
            .completion_output()
            .speculative_diagnostic_choices(),
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => case
            .case
            .custody()
            .completion_output()
            .speculative_diagnostic_choices(),
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => case
            .case
            .custody()
            .completion_output()
            .speculative_diagnostic_choices(),
        _ => None,
    }
}

fn read_authenticated_speculative_choice(
    completion: &mut M1AuthenticatedObservedCompletionOutputV1,
    _range_name: &'static str,
    range: ServiceHostDispatchRangeV1,
) -> Result<ServiceCompletedReadbackV1, ServiceQueueErrorV1> {
    match completion {
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => {
            let (lower, _custody, _step) = case.case.observation_parts();
            let request = lower.completed_read_request(range);
            lower.read_completed(request)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => {
            let (lower, _custody, _step) = case.case.observation_parts();
            let request = lower.completed_read_request(range);
            lower.read_completed(request)
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => {
            let (lower, _custody, _step) = case.case.observation_parts();
            let request = lower.completed_read_request(range);
            lower.read_completed(request)
        }
        _ => unreachable!("authenticated diagnostic read was preflighted as speculative"),
    }
}

fn read_authenticated_speculative_k4_choice(
    completion: &mut M1AuthenticatedObservedCompletionOutputV1,
    _range_name: &'static str,
    range: ServiceHostDispatchRangeV1,
) -> Result<ServiceCompletedReadbackV1, ServiceQueueErrorV1> {
    let M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) = completion else {
        unreachable!("authenticated diagnostic read was preflighted as S1/K4")
    };
    let (lower, _custody, _step) = case.case.observation_parts();
    let request = lower.completed_read_request(range);
    lower.read_completed(request)
}

/// First-publication authenticated S1/K4 diagnostic observation rejection.
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {
    /// The owner is not the exact target `SpeculativeS1K4C8192` case.
    NotS1K4,
    /// The owner is not one of the finite speculative queue shapes.
    NotSpeculativeShape,
    /// The queue was directly reused; diagnostic sentinels were not reset.
    NotFirstDispatchGeneration { actual: u64 },
    /// Diagnostic choice allocations were not attached before publication.
    CaptureNotEnabled,
    /// Host custody for the five completed copies could not be reserved.
    HostAllocation,
    /// An exact draft-row range was unexpectedly absent.
    DraftRangeMissing { iteration: usize },
    /// One exact ordered completed copy failed.
    Queue {
        range: &'static str,
        source: ServiceQueueErrorV1,
    },
    /// Copied coordinates, extents, generation, or token values rejected.
    Choices(M1SpeculativeDiagnosticChoicesErrorV1),
}

impl fmt::Display for M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authenticated first-publication S1/K4 diagnostic observation rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {}

/// Failure retaining authenticated compact/queue/program custody and every copy.
#[must_use = "authenticated diagnostic failure custody must be torn down or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1 {
    error: M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
    completion: Box<M1AuthenticatedObservedCompletionOutputV1>,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
}

/// Shape-generic name for diagnostic observation failure custody.
pub type M1AuthenticatedSpeculativeDiagnosticObservationFailureV1 =
    M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1;

impl M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1 {
    fn new(
        error: M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
        completion: M1AuthenticatedObservedCompletionOutputV1,
        partial_choices: Box<[ServiceCompletedReadbackV1]>,
    ) -> Self {
        Self {
            error,
            completion: Box::new(completion),
            partial_choices,
        }
    }

    #[must_use]
    pub const fn error(&self) -> &M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {
        &self.error
    }

    /// Explicitly demoted diagnostic status; this owner is not M1 evidence.
    #[must_use]
    pub const fn status(&self) -> &'static str {
        M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    #[must_use = "authenticated compact and queue custody remain retained"]
    pub const fn compact(&self) -> &M1AuthenticatedObservedCompletionOutputV1 {
        &self.completion
    }

    /// Faults the logical engine, destroys the queue, and retains all inert bytes.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated release quarantine paired with the
    /// compact image and every completed choice copy.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            error,
            completion,
            partial_choices,
        } = self;
        match release_authenticated_observed_output(*completion) {
            Ok((queue_release, compact)) => Ok(
                M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1 {
                    error,
                    compact,
                    partial_choices,
                    queue_release,
                },
            ),
            Err(failure) => {
                let (source, compact) = *failure;
                Err(Box::new(
                    M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownFailureV1 {
                        error,
                        compact,
                        partial_choices,
                        source,
                    },
                ))
            }
        }
    }
}

/// Clean authenticated teardown after diagnostic observation rejection.
#[must_use = "diagnostic bytes and authenticated queue release remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1 {
    error: M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    queue_release: AuthenticatedServiceQueueReleaseV1,
}

impl M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    #[must_use = "the compact image remains retained"]
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use = "authenticated program release remains retained"]
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
    }
}

/// Terminal authenticated release quarantine after diagnostic observation rejection.
#[must_use = "diagnostic bytes and authenticated release quarantine remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownFailureV1 {
    error: M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1,
    compact: M1ObservedCompletionImageV1,
    partial_choices: Box<[ServiceCompletedReadbackV1]>,
    source: M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
}

impl M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1AuthenticatedSpeculativeK4DiagnosticObservationErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn copied_choice_ranges(&self) -> usize {
        self.partial_choices.len()
    }

    #[must_use = "the compact image remains retained"]
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        &self.compact
    }

    #[must_use = "authenticated release quarantine remains retained"]
    pub const fn source(&self) -> &M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }
}

/// One authenticated compact K7 observation paired with exactly five S1/K4 copies.
#[must_use = "authenticated diagnostic observation must be checked or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1 {
    completion: M1AuthenticatedObservedCompletionOutputV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

/// Shape-generic name for one authenticated finite speculative observation.
pub type M1AuthenticatedObservedSpeculativeDiagnosticOutputV1 =
    M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1;

impl M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1 {
    /// Explicitly demoted diagnostic status; this owner is not M1 evidence.
    #[must_use]
    pub const fn status(&self) -> &'static str {
        M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1
    }

    #[must_use = "the compact K7 image remains paired with authenticated custody"]
    pub const fn compact(&self) -> &M1ObservedCompletionImageV1 {
        self.completion.image()
    }

    #[must_use = "the exact draft/target copies remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.completion.program_catalog_id()
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
        match self {
            Self::TargetOnly(case) => observe_case(case)
                .map(M1AuthenticatedObservedCompletionOutputV1::TargetOnly)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1AuthenticatedPhysicalRecycledQueueSessionV1::TargetOnly,
                        M1AuthenticatedRejectedCompletionOutputV1::TargetOnly,
                        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::TargetOnly,
                    )
                }),
            Self::PairedPrefill(case) => observe_case(case)
                .map(M1AuthenticatedObservedCompletionOutputV1::PairedPrefill)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1AuthenticatedPhysicalRecycledQueueSessionV1::PairedPrefill,
                        M1AuthenticatedRejectedCompletionOutputV1::PairedPrefill,
                        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::PairedPrefill,
                    )
                }),
            Self::SpeculativeK4(case) => observe_case(case)
                .map(M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK4,
                        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK4,
                        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK4,
                    )
                }),
            Self::SpeculativeK8(case) => observe_case(case)
                .map(M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK8,
                        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK8,
                        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK8,
                    )
                }),
            Self::SpeculativeK16(case) => observe_case(case)
                .map(M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16)
                .map_err(|failure| {
                    retain_observation_failure(
                        *failure,
                        M1AuthenticatedPhysicalRecycledQueueSessionV1::SpeculativeK16,
                        M1AuthenticatedRejectedCompletionOutputV1::SpeculativeK16,
                        M1AuthenticatedCompletionSnapshotReadFailedOutputV1::SpeculativeK16,
                    )
                }),
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
    pub(crate) const fn operations(&self) -> &DeclaredOperationKernelPlan {
        &self.operations
    }

    pub(crate) fn program_families_match(&self) -> bool {
        self.witness.family_artifacts() == self.operations.families()
    }

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
    pub(crate) const fn operations(&self) -> &DeclaredOperationKernelPlan {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.operations(),
        }
    }

    pub(crate) fn program_families_match(&self) -> bool {
        match self {
            Self::TargetOnly(case)
            | Self::PairedPrefill(case)
            | Self::SpeculativeK4(case)
            | Self::SpeculativeK8(case)
            | Self::SpeculativeK16(case) => case.program_families_match(),
        }
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1AuthenticatedCompletionEvidenceJoinAuthorityV1 {
    Generic,
    SpeculativeDiagnostic,
}

fn validate_generic_observed_semantics(
    authority: M1AuthenticatedCompletionEvidenceJoinAuthorityV1,
    qualification_capture_enabled: bool,
    direct_diagnostic_capture_enabled: bool,
    speculative_diagnostic_capture_enabled: bool,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> Result<(), M1CompletedOutputCheckErrorV1> {
    if direct_diagnostic_capture_enabled {
        return Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence);
    }
    if speculative_diagnostic_capture_enabled
        != matches!(
            authority,
            M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic
        )
    {
        return Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence);
    }
    if qualification_capture_enabled {
        if let Some(lane) = semantics.iter().position(|semantic| {
            let CompletionWireSemanticExpectation::QualificationPromptCommit { .. } = semantic
            else {
                return true;
            };
            false
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
    authority: M1AuthenticatedCompletionEvidenceJoinAuthorityV1,
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
        authority,
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
    let (lower, witness, operations, custody, step) = (*case).into_parts();
    let (scheduled, _target_plans, kv, speculative_lineage, speculative_rollover_intent) =
        step.into_parts_with_speculative_lineage();
    let checked = checked
        .retain_completion_canary_readback(image.into_completion_canary_readback())
        .retain_speculative_lineage(speculative_lineage)
        .retain_speculative_rollover_intent(speculative_rollover_intent);
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
    authority: M1AuthenticatedCompletionEvidenceJoinAuthorityV1,
    expectations: &[CompletionWireSemanticExpectation<'_>],
) -> Result<M1AuthenticatedPhysicalCompletedReadbackV1, M1AuthenticatedCompletedReadbackJoinFailureV1>
{
    match check_observed_case(case, authority, expectations) {
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
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                expectations,
            ),
            Self::PairedPrefill(case) => join_observed_output_case(
                case,
                Self::PairedPrefill,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::PairedPrefill,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                expectations,
            ),
            Self::SpeculativeK4(case) => join_observed_output_case(
                case,
                Self::SpeculativeK4,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK4,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                expectations,
            ),
            Self::SpeculativeK8(case) => join_observed_output_case(
                case,
                Self::SpeculativeK8,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK8,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                expectations,
            ),
            Self::SpeculativeK16(case) => join_observed_output_case(
                case,
                Self::SpeculativeK16,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK16,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                expectations,
            ),
        }
    }
}

fn authenticated_speculative_semantics(
    choices: &M1ObservedSpeculativeDiagnosticChoicesV1,
) -> (
    [CompletionWireSemanticExpectation<'_>; M1_MAX_ACTIVE_SEQUENCES as usize],
    usize,
) {
    let empty = CompletionWireSemanticExpectation::Speculative {
        draft_tokens: &[],
        target_choices: &[],
    };
    let mut semantics = [empty; M1_MAX_ACTIVE_SEQUENCES as usize];
    let live = choices.live_sequences() as usize;
    for (lane, semantic) in semantics.iter_mut().take(live).enumerate() {
        *semantic = CompletionWireSemanticExpectation::Speculative {
            draft_tokens: choices.draft_choices_for_lane(lane).unwrap_or(&[]),
            target_choices: choices.target_choices_for_lane(lane).unwrap_or(&[]),
        };
    }
    (semantics, live)
}

fn check_authenticated_speculative_diagnostic(
    completion: M1AuthenticatedObservedCompletionOutputV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
) -> Result<
    M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1,
    Box<M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1>,
> {
    let (semantics, live) = authenticated_speculative_semantics(&choices);
    let joined = match completion {
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4(case) => {
            join_observed_output_case(
                case,
                M1AuthenticatedObservedCompletionOutputV1::SpeculativeK4,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK4,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
                &semantics[..live],
            )
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8(case) => {
            join_observed_output_case(
                case,
                M1AuthenticatedObservedCompletionOutputV1::SpeculativeK8,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK8,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
                &semantics[..live],
            )
        }
        M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16(case) => {
            join_observed_output_case(
                case,
                M1AuthenticatedObservedCompletionOutputV1::SpeculativeK16,
                M1AuthenticatedPhysicalReadbackQueueSessionV1::SpeculativeK16,
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic,
                &semantics[..live],
            )
        }
        completion => {
            return Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1 {
                    failure: M1AuthenticatedCompletedReadbackJoinFailureV1 {
                        error: M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence,
                        observed: Box::new(completion),
                    },
                    choices,
                },
            ));
        }
    };
    match joined {
        Ok(completed) => Ok(M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1 {
            corresponding_target_only_token: choices
                .target_choices()
                .first()
                .copied()
                .unwrap_or(u32::MAX),
            completed,
            choices,
        }),
        Err(failure) => Err(Box::new(
            M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1 {
                failure,
                choices,
            },
        )),
    }
}

impl M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1 {
    /// Joins copied S1/K4 choices to compact wire, scheduler, plan, epoch, and KV custody.
    ///
    /// This specialized private-authority route is the only authenticated join
    /// that admits a speculative diagnostic attachment. It derives all token
    /// semantics from the retained completed copies and accepts no caller-supplied
    /// semantic values.
    ///
    /// # Errors
    ///
    /// Returns the unchanged authenticated compact owner and choice copies when
    /// roster, epoch, plan, wire, request, or maximal-prefix semantics reject.
    pub fn check_completion(
        self,
    ) -> Result<
        M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1,
        Box<M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1>,
    > {
        check_authenticated_speculative_diagnostic(self.completion, self.choices)
    }
}

/// Positive authenticated S1/K4 maximal-prefix join retaining exact choice copies.
#[must_use = "authenticated completed readback and diagnostic choices remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1 {
    completed: M1AuthenticatedPhysicalCompletedReadbackV1,
    corresponding_target_only_token: ferric_spec::TokenId,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

/// Shape-generic name for a joined authenticated diagnostic readback.
pub type M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1 =
    M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1;

/// Clean fail-closed release of a joined diagnostic that cannot be completed.
#[must_use = "authenticated diagnostic evidence and program release remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownSuccessV1 {
    queue_release: AuthenticatedServiceQueueReleaseV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownSuccessV1 {
    /// Authenticated program release and native queue-destruction observation.
    #[must_use = "released authenticated programs remain explicitly owned"]
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
    }

    /// Structurally and semantically checked compact completion.
    #[must_use = "checked completion remains retained"]
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    /// Exact copied diagnostic choices.
    #[must_use = "diagnostic choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Exact completion epoch retained during fail-closed release.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    /// Pending KV reservations retained after scheduler quarantine.
    #[must_use = "pending KV reservations remain retained"]
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }
}

/// Terminal release quarantine for a joined diagnostic that cannot be completed.
#[must_use = "authenticated diagnostic evidence and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownFailureV1 {
    source: M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownFailureV1 {
    /// Authenticated lower release quarantine and every Ferric owner.
    #[must_use = "authenticated release quarantine remains retained"]
    pub const fn source(&self) -> &M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
        &self.source
    }

    /// Structurally and semantically checked compact completion.
    #[must_use = "checked completion remains retained"]
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    /// Exact copied diagnostic choices.
    #[must_use = "diagnostic choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Exact completion epoch retained by terminal quarantine.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    /// Pending KV reservations retained after scheduler quarantine.
    #[must_use = "pending KV reservations remain retained"]
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }
}

impl M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1 {
    /// Explicitly demoted diagnostic status; this owner is not M1 evidence.
    #[must_use]
    pub const fn status(&self) -> &'static str {
        M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1
    }

    #[must_use = "authenticated completed readback authority remains retained"]
    pub const fn completed(&self) -> &M1AuthenticatedPhysicalCompletedReadbackV1 {
        &self.completed
    }

    #[must_use = "exact diagnostic choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Target's S1 next-token choice for the pre-round context.
    #[must_use]
    pub const fn corresponding_target_only_token(&self) -> ferric_spec::TokenId {
        self.corresponding_target_only_token
    }

    /// Whether compact output's first token equals the same-round target choice.
    #[must_use]
    pub fn target_token_matches(&self) -> bool {
        self.completed
            .checked()
            .records()
            .first()
            .is_some_and(|record| {
                record.record().emitted_token_count > 0
                    && record.record().emitted_tokens[0] == self.corresponding_target_only_token
            })
    }

    /// Faults the logical Engine, destroys the authenticated queue, and retains
    /// the checked compact image, exact completion, KV reservations, and choices.
    ///
    /// This is the fail-closed terminal transition for an invariant rejection
    /// after the private diagnostic semantic join. It does not complete the
    /// scheduler request or release any KV page.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated lower release quarantine paired with all
    /// post-join Ferric owners when native queue destruction fails.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            completed,
            corresponding_target_only_token: _,
            choices,
        } = self;
        let (queue, checked, completion, kv) = completed.into_parts();
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(
                M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownSuccessV1 {
                    queue_release,
                    checked,
                    completion,
                    kv,
                    choices,
                },
            ),
            Err(source) => Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticCompletedTeardownFailureV1 {
                    source: *source,
                    checked,
                    completion,
                    kv,
                    choices,
                },
            )),
        }
    }

    /// Separates authenticated completion authority and inert choice copies once.
    #[must_use = "authenticated completion and diagnostic choices remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedPhysicalCompletedReadbackV1,
        M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        (self.completed, self.choices)
    }
}

/// Semantic rejection retaining authenticated observed queue/program custody and choices.
#[must_use = "authenticated diagnostic join failure must be retried, torn down, or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1 {
    failure: M1AuthenticatedCompletedReadbackJoinFailureV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

/// Shape-generic name for semantic-join rejection with choice custody.
pub type M1AuthenticatedSpeculativeDiagnosticCompletedReadbackJoinFailureV1 =
    M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1;

impl M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1CompletedOutputCheckErrorV1 {
        self.failure.error()
    }

    #[must_use]
    pub const fn status(&self) -> &'static str {
        M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1
    }

    #[must_use = "the same copied choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Rechecks the same compact image and choices without another device read.
    ///
    /// # Errors
    ///
    /// Returns the same closed failure owner if the immutable join still rejects.
    pub fn retry(
        self,
    ) -> Result<M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackV1, Box<Self>> {
        let Self { failure, choices } = self;
        match check_authenticated_speculative_diagnostic(*failure.observed, choices) {
            Ok(completed) => Ok(completed),
            Err(failure) => Err(failure),
        }
    }

    /// Faults the logical engine and tears down while retaining both evidence owners.
    ///
    /// # Errors
    ///
    /// Returns authenticated release quarantine paired with the compact image,
    /// join diagnostic, and exact choice copies.
    pub fn destroy_queue_and_retain_evidence<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownFailureV1>,
    > {
        let Self { failure, choices } = self;
        match failure.destroy_queue_and_retain_evidence(engine) {
            Ok(teardown) => Ok(
                M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1 {
                    choices,
                    teardown,
                },
            ),
            Err(teardown) => Err(Box::new(
                M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownFailureV1 {
                    choices,
                    teardown,
                },
            )),
        }
    }
}

/// Clean teardown retaining authenticated semantic rejection and S1/K4 choices.
#[must_use = "authenticated diagnostic teardown and choice custody remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1 {
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: M1AuthenticatedReadbackTeardownSuccessV1,
}

impl M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1 {
    #[must_use = "exact diagnostic choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    #[must_use = "authenticated program release remains retained"]
    pub const fn teardown(&self) -> &M1AuthenticatedReadbackTeardownSuccessV1 {
        &self.teardown
    }
}

/// Terminal teardown retaining authenticated release quarantine and S1/K4 choices.
#[must_use = "authenticated diagnostic teardown quarantine remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownFailureV1 {
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: Box<M1AuthenticatedReadbackTeardownFailureV1>,
}

impl M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownFailureV1 {
    #[must_use = "exact diagnostic choices remain retained"]
    pub const fn choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    #[must_use = "authenticated release quarantine remains retained"]
    pub const fn teardown(&self) -> &M1AuthenticatedReadbackTeardownFailureV1 {
        &self.teardown
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_authenticated_s1_k4_first_dispatch_generation, validate_generic_observed_semantics,
        M1AuthenticatedCompletionEvidenceJoinAuthorityV1,
        M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1,
    };
    use crate::M1CompletedOutputCheckErrorV1;

    #[test]
    fn generic_readback_denies_diagnostic_capture_routes() {
        assert!(matches!(
            validate_generic_observed_semantics(
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                false,
                true,
                false,
                &[]
            ),
            Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence)
        ));
        assert!(matches!(
            validate_generic_observed_semantics(
                M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
                false,
                false,
                true,
                &[]
            ),
            Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence)
        ));
    }

    #[test]
    fn generic_readback_accepts_capture_free_semantic_route() {
        assert!(validate_generic_observed_semantics(
            M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic,
            false,
            false,
            false,
            &[]
        )
        .is_ok());
    }

    #[test]
    fn private_diagnostic_authority_requires_exactly_one_speculative_attachment() {
        let authority = M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic;
        assert!(validate_generic_observed_semantics(authority, false, false, true, &[]).is_ok());
        assert!(matches!(
            validate_generic_observed_semantics(authority, false, false, false, &[]),
            Err(M1CompletedOutputCheckErrorV1::SpeculativeDiagnosticCaptureRequiresEvidence)
        ));
        assert!(matches!(
            validate_generic_observed_semantics(authority, false, true, true, &[]),
            Err(M1CompletedOutputCheckErrorV1::DirectDiagnosticCaptureRequiresEvidence)
        ));
        assert_eq!(
            M1_AUTHENTICATED_S1_K4_DIAGNOSTIC_STATUS_V1,
            "partial-non-evidence"
        );
    }

    #[test]
    fn authenticated_s1_k4_diagnostic_admits_only_first_dispatch_generation() {
        assert!(is_authenticated_s1_k4_first_dispatch_generation(1));
        for hostile in [0, 2, u64::MAX] {
            assert!(!is_authenticated_s1_k4_first_dispatch_generation(hostile));
        }
    }
}
