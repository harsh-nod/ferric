#![forbid(unsafe_code)]

//! One-shot, target-only M1 qualification capture on an exclusive gfx942.

use fe2o3_kfd::{DeviceSelector, OpenedKfd};
use ferric_build::{
    authenticate_qwen3_tokenizer, build_authenticated_model_weight_layout,
    build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
    build_prepacked_deployment_bundle, decode_bundle_admission_record,
    encode_canonical_deployment_bundle, expected_preliminary_kernel_catalog_identity,
    expected_qwen3_gfx942_runner_source_identity, generate_qwen3_gfx942_runner_declaration,
    m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
    plan_authenticated_model_memory, publish_qwen3_gfx942_runner_declaration, qwen3_kv_arena_bytes,
    reopen_persisted_qwen3_weights, seal_authenticated_bundle, AuthenticatedBundleAdmission,
    AuthenticatedDeploymentAssets, AuthenticatedModelAssets, AvailableM1StepWorkspace,
    DeclaredDeviceAllocation, DeclaredM1StepWorkspaceAllocation, ExternalIdentityClosureInputs,
    M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome, ModelMemoryAllocationSet,
    ModelMemoryPlanOutcome, PrepackedDeploymentBundle, BUNDLE_ADMISSION_RECORD_BYTES,
    CANONICAL_DEPLOYMENT_BUNDLE_BYTES, DRAFT_REPOSITORY, DRAFT_REVISION,
    QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1, QWEN3_TARGET_PREPACKED_MANIFEST_BYTES,
    QWEN3_TARGET_TENSOR_DATA_BYTES, TARGET_REPOSITORY, TARGET_REVISION,
};
use ferric_engine::{
    bind_m1_kv_workspace_table_v1, bind_m1_physical_runner_v1,
    bind_m1_speculative_draft_kv_round_workspace_table_v1, complete_m1_physical_step_v1,
    initialize_m1_physical_runner_memory_v1, prelease_m1_qualification_target_pages_v1,
    prepare_m1_long_lived_queue_rearm_v1, release_m1_completed_step_kv_pages_v1,
    reopen_persisted_m1_kernel_artifacts_v1, reserve_m1_long_lived_queue_rearm_kv_v1,
    schedule_m1_long_lived_queue_rearm_v1, ActiveDeviceKvCache, CompletionWireSemanticExpectation,
    Engine, M1CompletedDeviceKvMemberV1, M1CompletedStepOutcomeV1,
    M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1,
    M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans, M1LongLivedQueueRearmKvInputsV1,
    M1LongLivedQueueReleasedRoundV1, M1PhysicalRunnerFirstCompletionOutcomeV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1QualificationCompletionEvidenceV1,
    M1RearmedQualifiedRoundReleaseOutcomeV1, M1RearmedRoundReleaseOutcomeV1,
    M1ScheduledLongLivedQueueRearmV1, M1StepDispatchIntent,
};
use ferric_engine::{
    EngineError, M1CompletedStepKvReleaseErrorV1, M1CompletedStepPoisonV1,
    M1CompletedStepRejectionTeardownFailureV1, M1CompletedStepRejectionTeardownSuccessV1,
    M1CompletedStepTeardownFailureV1, M1CompletedStepTeardownSuccessV1,
    M1CompletionEvidenceTeardownFailureV1, M1CompletionEvidenceTeardownSuccessV1,
    M1EngineQuarantinedPhysicalQueueOperationFailureV1,
    M1LongLivedQueueRearmKvReservationFailureV1, M1LongLivedQueueRearmPrepareFailureV1,
    M1LongLivedQueueRearmScheduleDetachQuarantineV1,
    M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1,
    M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1, M1LongLivedQueueRearmTeardownFailureV1,
    M1LongLivedQueueRearmTeardownSuccessV1, M1PhysicalRunnerFirstPublicationExhaustedV1,
    M1PhysicalRunnerRearmSubmissionExhaustedV1, M1QualificationEvidenceTeardownFailureV1,
    M1QualificationEvidenceTeardownSuccessV1, M1QualificationObservationTeardownFailureV1,
    M1QualificationObservationTeardownSuccessV1, M1QualificationSemanticTeardownFailureV1,
    M1QualificationSemanticTeardownSuccessV1, M1RearmedCompletionPreflightTeardownFailureV1,
    M1RearmedCompletionPreflightTeardownSuccessV1, M1RearmedObservedQualificationTeardownFailureV1,
    M1RearmedObservedQualificationTeardownSuccessV1, M1RearmedPoisonedCompletionV1,
    M1RearmedQualificationObservationTeardownFailureV1,
    M1RearmedQualificationObservationTeardownSuccessV1,
    M1RearmedQualificationSemanticTeardownFailureV1,
    M1RearmedQualificationSemanticTeardownSuccessV1,
    M1RearmedQualifiedCompletionPreflightTeardownFailureV1,
    M1RearmedQualifiedCompletionPreflightTeardownSuccessV1, M1RearmedQualifiedPoisonedCompletionV1,
    M1RearmedQualifiedReadbackTeardownFailureV1, M1RearmedQualifiedReadbackTeardownSuccessV1,
    M1RearmedQualifiedRejectedCompletionTeardownFailureV1,
    M1RearmedQualifiedRejectedCompletionTeardownSuccessV1,
    M1RearmedQualifiedRoundPageReleaseTeardownFailureV1,
    M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1, M1RearmedQualifiedTeardownFailureV1,
    M1RearmedQualifiedTeardownSuccessV1, M1RearmedQueueProgressFailureV1,
    M1RearmedReadbackTeardownFailureV1, M1RearmedReadbackTeardownSuccessV1,
    M1RearmedRejectedCompletionTeardownFailureV1, M1RearmedRejectedCompletionTeardownSuccessV1,
    M1RearmedRoundPageReleaseTeardownFailureV1, M1RearmedRoundPageReleaseTeardownSuccessV1,
    M1ReleasedQueueTeardownFailureV1, M1ReleasedQueueTeardownSuccessV1,
    M1ScheduledLongLivedQueueRearmTeardownFailureV1,
    M1ScheduledLongLivedQueueRearmTeardownSuccessV1,
    M1SpeculativeDiagnosticCompletedTeardownFailureV1,
    M1SpeculativeDiagnosticCompletedTeardownSuccessV1,
    M1SpeculativeDiagnosticObservationTeardownFailureV1,
    M1SpeculativeDiagnosticObservationTeardownSuccessV1,
    M1SpeculativeDiagnosticSemanticTeardownFailureV1,
    M1SpeculativeDiagnosticSemanticTeardownSuccessV1,
};
use ferric_spec::scheduling::RequestState;
use ferric_spec::{
    m1_qualification_context_plan, validate_m1_step_inputs, EngineLimits, Identity,
    M1QualificationExecutionBindingDeclaration, M1QualificationLaneExecutionBinding,
    M1QualificationLaneGrouping, M1StepInputCandidate, M1StepInputValidationOutcome,
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, StepPlan,
    ValidatedM1StepInputs, M1_KV_PAGE_TOKENS, M1_QUALIFICATION_CONTEXT_PLAN_STEPS,
    M1_QUALIFICATION_FINAL_INPUT_TOKEN, M1_QUALIFICATION_TOKENS_PER_LANE, QWEN3_VOCABULARY_SIZE,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Dir, FileType, Mode, OFlags,
    RenameFlags, ResolveFlags, Stat, CWD,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

mod input_bundle;
mod m1_r30_canary_partial_capture;
mod m1_r30_capture_composition;
mod m1_r30_exhaustion_partial_capture;
mod m1_r30_partial_capture;
mod m1_r30_rollback_partial_capture;
mod m1_r32_partial_capture;

const PLAN_FORMAT: &str = "FERRIC-M1-BENCHMARK-PLAN-V1";
const ROSTER_FORMAT: &str = "FERRIC-M1-QUALIFICATION-ROSTER-V1";
const WORKLOAD_FORMAT: &str = "FERRIC-M1-QUALIFICATION-WORKLOAD-V3";
const CLOSURE_FORMAT: &str = "FERRIC-M1-QUALIFICATION-CLOSURE-V1";
const ENVIRONMENT_FORMAT: &str = "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1";
const OUTPUT_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1";
const TRANSCRIPT_FORMAT: &str = "FERRIC-M1-QUALIFICATION-CAPTURE-V2";
const TARGET: &str = "gfx942:xnack-";
const DIFFERENTIAL_NONCLAIM: &str = "Structural acceptance authenticates externally collected target-only differential records only. It does not validate a logit tolerance, prove token equality, establish numerical or hardware correctness, qualify performance, or close m1.r29.";
const MAX_DOCUMENT_BYTES: usize = 8 * 1_024 * 1_024;
const R30_PREFILL_ACTIVE_TOKENS: u32 = 128;
const R30_PREFILL_INPUT_TOKEN: u32 = 1;
const R30_PREFILL_INPUT_BYTES: u64 = R30_PREFILL_ACTIVE_TOKENS as u64 * 4;
const R30_PREFILL_TARGET_PAGES: usize = 8;
const METADATA_BYTES: u64 = 64 * 1_024;
const BF16_BYTES: u64 = 2;
const DECODE_CONTEXT_LENGTH: u32 = M1_QUALIFICATION_FINAL_INPUT_TOKEN;
const QUALIFICATION_LOGICAL_KV_PAGE_TOKENS: u32 = 256;

const COMMON_IDENTITIES: &[&str] = &[
    "benchmark-executable",
    "benchmark-protocol",
    "config",
    "dispatch-graph",
    "environment",
    "fe2o3-source-closure",
    "ferric-source-closure",
    "generated-plan",
    "model",
    "schedule-catalog",
    "tokenizer",
    "weights",
    "workload-roster",
];

const DIFFERENTIAL_KINDS: &[&str] = &[
    "decode-s1-c8192",
    "decode-s32-c8192",
    "decode-s8-c8192",
    "prefill-s1-t128",
    "prefill-s1-t2048",
    "prefill-s1-t512",
    "prefill-s8-t128",
];

const DIFFERENTIAL_IDENTITIES: &[&str] = &[
    "differential-acceptance-policy",
    "reference-implementation",
    "reference-protocol",
];

const DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES: &[(&str, &str)] = &[
    ("decode-s1-c8192", "dispatch-graph-decode-s1-c8192"),
    ("decode-s32-c8192", "dispatch-graph-decode-s32-c8192"),
    ("decode-s8-c8192", "dispatch-graph-decode-s8-c8192"),
    ("prefill-s1-t128", "dispatch-graph-prefill-s1-t128"),
    ("prefill-s1-t2048", "dispatch-graph-prefill-s1-t2048"),
    ("prefill-s1-t512", "dispatch-graph-prefill-s1-t512"),
    ("prefill-s8-t128", "dispatch-graph-prefill-s8-t128"),
];

type CaptureResult<T> = Result<T, String>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct R30PhysicalCaptureBindingsV1 {
    device_identity_sha256: String,
    gpu_unique_id: u64,
    kernel_artifact_manifest_sha256: String,
    program_catalog_sha256: String,
    runner_declaration_sha256: String,
}

const CAPTURE_RECOVERY_RETRIES: usize = 2;

trait CaptureClosedCustodyV1 {}
trait CaptureTerminalCustodyV1 {}
trait CaptureInvariantCustodyV1 {}
trait CaptureDiagnosticEvidenceV1 {}

struct UnexpectedReleasedScheduleFailureV1 {
    _custody: ferric_engine::M1LongLivedQueueRearmScheduleFailureV1,
    _requests: Vec<RequestId>,
    _history: QualificationRoundCommitmentV1,
}
struct FirstQueueQuarantineV1 {
    _stage: ferric_engine::M1PhysicalRunnerQueueFailureStageV1,
    _failure: Box<ferric_engine::M1PhysicalQueueOperationFailureV1>,
    _roster: M1DeviceKvCompletionRosterV1,
}

enum DecodeInitialPhaseCustodyV1 {
    Scheduled {
        _diagnostic: String,
    },
    Input {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
    },
    Reservations {
        _diagnostic: ferric_engine::M1QualificationContextStepReservationFailureV1,
        _plans: Vec<StepPlan>,
        _inputs: ValidatedM1StepInputs,
        _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    },
    KvBinding {
        _plans: Vec<StepPlan>,
        _failure: Box<ferric_engine::M1KvWorkspaceTableBindingFailureV1>,
    },
    WorkspacePlan {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
        _table: ferric_engine::BoundM1KvWorkspaceTableV1,
    },
    Recipe {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
        _table: ferric_engine::BoundM1KvWorkspaceTableV1,
        _workspace_plan: ferric_build::AddresslessM1StepWorkspacePlan,
    },
}

struct DecodeInitialDispatchAbandonmentV1 {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<32>,
    _memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    _caches: Vec<ActiveDeviceKvCache>,
    _requests: Vec<RequestId>,
    _contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
    _scheduled: ferric_engine::M1ScheduledDispatchV1,
    _phase: Box<DecodeInitialPhaseCustodyV1>,
}

enum PrefillInitialPhaseCustodyV1 {
    Scheduled {
        _diagnostic: String,
    },
    Input {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
    },
    CacheConstruction {
        _diagnostic: ferric_engine::DeviceKvCacheError,
        _plans: Vec<StepPlan>,
        _inputs: ValidatedM1StepInputs,
        _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    },
    PageCount {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
        _inputs: ValidatedM1StepInputs,
        _current_cache: ActiveDeviceKvCache,
        _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    },
    PageLease {
        _diagnostic: ferric_engine::M1DeviceKvArenaLeaseErrorV1,
        _plans: Vec<StepPlan>,
        _inputs: ValidatedM1StepInputs,
        _current_cache: ActiveDeviceKvCache,
        _page_leases: Vec<ferric_engine::DeviceKvPageLease>,
        _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    },
    StepReservation {
        _failure: Box<ferric_engine::DeviceKvStepReservationFailure>,
        _plans: Vec<StepPlan>,
        _inputs: ValidatedM1StepInputs,
        _current_cache: ActiveDeviceKvCache,
        _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    },
    KvBinding {
        _plans: Vec<StepPlan>,
        _failure: Box<ferric_engine::M1KvWorkspaceTableBindingFailureV1>,
    },
    WorkspacePlan {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
        _table: ferric_engine::BoundM1KvWorkspaceTableV1,
    },
    Recipe {
        _diagnostic: String,
        _plans: Vec<StepPlan>,
        _table: ferric_engine::BoundM1KvWorkspaceTableV1,
        _workspace_plan: ferric_build::AddresslessM1StepWorkspacePlan,
    },
}

struct PrefillInitialDispatchAbandonmentV1 {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<32>,
    _memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    _caches: Vec<ActiveDeviceKvCache>,
    _requests: Vec<RequestId>,
    _active_lengths: Vec<u32>,
    _context_lengths: Vec<u32>,
    _scheduled: ferric_engine::M1ScheduledDispatchV1,
    _phase: Box<PrefillInitialPhaseCustodyV1>,
}

enum ReleasedRoundPhaseCustodyV1 {
    Context {
        _diagnostic: String,
    },
    WorkspacePlan {
        _diagnostic: String,
        _contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
    },
    Recipe {
        _diagnostic: String,
        _contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
        _workspace_plan: ferric_build::AddresslessM1StepWorkspacePlan,
    },
    Enqueue {
        _diagnostic: ferric_engine::EngineError,
        _contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
        _workspace_plan: ferric_build::AddresslessM1StepWorkspacePlan,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    },
}

struct PreEngineMemoryAbandonmentV1 {
    _memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    _input_tokens: Vec<u32>,
    _diagnostic: PreEngineDiagnosticV1,
}

#[allow(dead_code)]
enum PreEngineDiagnosticV1 {
    Policy(String),
    ContextPlan(ferric_engine::M1QualificationContextStepWitnessErrorV1),
    Engine(ferric_engine::EngineError),
}

enum PrePhysicalPoolCustodyV1 {
    Split {
        _memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
        _caches: Vec<ActiveDeviceKvCache>,
    },
    Preleased {
        _preleased: ferric_engine::M1QualificationTargetPagePreleaseSuccessV1,
        _initial_contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
    },
}

#[allow(dead_code)]
enum PrePhysicalDiagnosticV1 {
    Engine(ferric_engine::EngineError),
    DeviceCache(ferric_engine::DeviceKvCacheError),
    ContextWitness(ferric_engine::M1QualificationContextStepWitnessErrorV1),
    MissingBatch,
}

struct PrePhysicalEngineAbandonmentV1 {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<32>,
    _requests: Vec<RequestId>,
    _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    _pool: PrePhysicalPoolCustodyV1,
    _input_tokens: Vec<u32>,
    _diagnostic: PrePhysicalDiagnosticV1,
}

#[allow(dead_code)]
enum PreleaseCancellationOutcomeV1 {
    Cancelled(ferric_engine::M1QualificationTargetPagePreleaseCancellationSuccessV1),
    Exhausted(ferric_engine::M1QualificationTargetPagePreleaseCancellationExhaustedV1),
}

struct PreleaseAbandonmentV1 {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<32>,
    _requests: Vec<RequestId>,
    _reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    _input_tokens: Vec<u32>,
    _outcome: PreleaseCancellationOutcomeV1,
}

impl CaptureInvariantCustodyV1 for UnexpectedReleasedScheduleFailureV1 {}
impl CaptureTerminalCustodyV1 for DecodeInitialDispatchAbandonmentV1 {}
impl CaptureTerminalCustodyV1 for PrefillInitialDispatchAbandonmentV1 {}
impl CaptureInvariantCustodyV1 for PreEngineMemoryAbandonmentV1 {}
impl CaptureTerminalCustodyV1 for PrePhysicalEngineAbandonmentV1 {}
impl CaptureTerminalCustodyV1 for PreleaseAbandonmentV1 {}
impl CaptureTerminalCustodyV1 for FirstQueueQuarantineV1 {}

struct DiagnosticCustodyV1<D, T> {
    _diagnostic: D,
    _custody: T,
}

impl<D: CaptureDiagnosticEvidenceV1, T: CaptureClosedCustodyV1> CaptureClosedCustodyV1
    for DiagnosticCustodyV1<D, T>
{
}
impl<D: CaptureDiagnosticEvidenceV1, T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1
    for DiagnosticCustodyV1<D, T>
{
}
impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for Box<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for Box<T> {}

impl CaptureDiagnosticEvidenceV1 for String {}
impl CaptureDiagnosticEvidenceV1 for M1CompletedStepKvReleaseErrorV1 {}
impl CaptureDiagnosticEvidenceV1 for EngineError {}

#[allow(dead_code)]
enum QualificationRoundDiagnosticV1 {
    Message(String),
    Engine(ferric_engine::EngineError),
}

struct QualificationRoundCustodyV1<T> {
    _state: QualificationRoundCaptureStateV1,
    _diagnostic: Option<QualificationRoundDiagnosticV1>,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for QualificationRoundCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for QualificationRoundCustodyV1<T> {}

struct ReleasedRoundTeardownCustodyV1<T> {
    _state: QualificationRoundCaptureStateV1,
    _phase: Box<ReleasedRoundPhaseCustodyV1>,
    _custody: T,
}

struct CompletionRosterCustodyV1<T> {
    _roster: ferric_engine::M1DeviceKvCompletionRosterV1,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for CompletionRosterCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for CompletionRosterCustodyV1<T> {}

struct R32CacheCustodyV1<T> {
    _cache: ActiveDeviceKvCache,
    _custody: T,
}

impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for R32CacheCustodyV1<T> {}

struct R32ChoiceCustodyV1<T> {
    _choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for R32ChoiceCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for R32ChoiceCustodyV1<T> {}

struct R32CaptureCustodyV1<T> {
    _capture: m1_r32_partial_capture::CaptureArtifactV1,
    _choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    _custody: T,
}

impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for R32CaptureCustodyV1<T> {}

struct R32CaptureReadyV1 {
    capture: m1_r32_partial_capture::CaptureArtifactV1,
    _choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    _closed: M1ReleasedQueueTeardownSuccessV1,
}

struct R30RollbackCacheCustodyV1<T> {
    _cache: ActiveDeviceKvCache,
    _custody: T,
}

impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for R30RollbackCacheCustodyV1<T> {}

enum SingleSpeculativePrepublicationFailureV1 {
    Workspace {
        _failure: ferric_engine::M1PrepublicationAllocationFailureV1,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    },
    CompletionOutput {
        _allocated: Box<ferric_engine::M1AllocatedScheduledStepV1>,
        _diagnostic: ferric_engine::M1CompletionOutputErrorV1,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    },
    DiagnosticChoices {
        _allocated: Box<ferric_engine::M1AllocatedScheduledStepV1>,
        _failure: Box<ferric_engine::M1SpeculativeDiagnosticChoicesAllocationFailureV1>,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    },
}

struct SingleSpeculativePrepublicationAbandonmentV1 {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<1>,
    _cache: ActiveDeviceKvCache,
    _phase: Box<SingleSpeculativePrepublicationFailureV1>,
}

impl CaptureTerminalCustodyV1 for SingleSpeculativePrepublicationAbandonmentV1 {}

struct R30RollbackChoiceCustodyV1<T> {
    _choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for R30RollbackChoiceCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for R30RollbackChoiceCustodyV1<T> {}

struct R30RollbackCaptureReadyV1 {
    capture: m1_r30_rollback_partial_capture::CaptureArtifactV1,
    _choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    _closed: M1ReleasedQueueTeardownSuccessV1,
}

#[allow(dead_code)]
enum QualificationEvidenceDiagnosticV1 {
    Message(String),
    PageRelease(ferric_engine::M1CompletedStepKvReleaseErrorV1),
}

struct QualificationEvidenceCustodyV1<T> {
    _evidence: ferric_engine::M1QualificationCompletionEvidenceV1,
    _diagnostic: Option<QualificationEvidenceDiagnosticV1>,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for QualificationEvidenceCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for QualificationEvidenceCustodyV1<T> {}

#[allow(dead_code)]
enum PrefillLiveDiagnosticV1 {
    Message(String),
    Engine(ferric_engine::EngineError),
}

struct PrefillLiveCustodyV1<T> {
    _evidence: PrefillLiveEvidenceV1,
    _diagnostic: Option<PrefillLiveDiagnosticV1>,
    _custody: T,
}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for PrefillLiveCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for PrefillLiveCustodyV1<T> {}

impl<T: CaptureClosedCustodyV1> CaptureClosedCustodyV1 for ReleasedRoundTeardownCustodyV1<T> {}
impl<T: CaptureTerminalCustodyV1> CaptureTerminalCustodyV1 for ReleasedRoundTeardownCustodyV1<T> {}

impl CaptureClosedCustodyV1 for M1LongLivedQueueRearmTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1ScheduledLongLivedQueueRearmTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedReadbackTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedCompletionPreflightTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualifiedCompletionPreflightTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedRoundPageReleaseTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedRejectedCompletionTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualifiedRejectedCompletionTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualificationObservationTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualificationSemanticTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedObservedQualificationTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualifiedReadbackTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1CompletedStepRejectionTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1CompletedStepTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1ReleasedQueueTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1QualificationObservationTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1QualificationEvidenceTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1QualificationSemanticTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1CompletionEvidenceTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1RearmedQualifiedTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1SpeculativeDiagnosticCompletedTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1SpeculativeDiagnosticObservationTeardownSuccessV1 {}
impl CaptureClosedCustodyV1 for M1SpeculativeDiagnosticSemanticTeardownSuccessV1 {}

impl CaptureTerminalCustodyV1 for M1CompletedStepPoisonV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedPoisonedCompletionV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedPoisonedCompletionV1 {}
impl CaptureTerminalCustodyV1 for M1LongLivedQueueRearmTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1ScheduledLongLivedQueueRearmTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedReadbackTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedCompletionPreflightTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedCompletionPreflightTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedRoundPageReleaseTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedRoundPageReleaseTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedRejectedCompletionTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedRejectedCompletionTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualificationObservationTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualificationSemanticTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedObservedQualificationTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedReadbackTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1CompletedStepRejectionTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1CompletedStepTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1ReleasedQueueTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1QualificationObservationTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1QualificationEvidenceTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1QualificationSemanticTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1CompletionEvidenceTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1RearmedQueueProgressFailureV1 {}
impl CaptureTerminalCustodyV1 for M1LongLivedQueueRearmKvReservationFailureV1 {}
impl CaptureTerminalCustodyV1 for M1LongLivedQueueRearmPrepareFailureV1 {}
impl CaptureTerminalCustodyV1 for M1LongLivedQueueRearmScheduleDetachQuarantineV1 {}
impl CaptureTerminalCustodyV1 for M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1PhysicalRunnerFirstPublicationExhaustedV1<'_> {}
impl CaptureTerminalCustodyV1 for M1PhysicalRunnerRearmSubmissionExhaustedV1<'_> {}
impl CaptureTerminalCustodyV1 for M1RearmedQualifiedTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1EngineQuarantinedPhysicalQueueOperationFailureV1 {}
impl CaptureTerminalCustodyV1 for M1SpeculativeDiagnosticCompletedTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1SpeculativeDiagnosticObservationTeardownFailureV1 {}
impl CaptureTerminalCustodyV1 for M1SpeculativeDiagnosticSemanticTeardownFailureV1 {}

fn abort_with_closed_custody<T: CaptureClosedCustodyV1>(phase: &'static str, custody: T) -> ! {
    let custody = core::mem::ManuallyDrop::new(custody);
    let _ = &custody;
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP: {phase}; typed custody retained"
    );
    std::process::abort();
}

fn terminal_quarantine<T: CaptureTerminalCustodyV1>(phase: &'static str, custody: T) -> ! {
    let custody = core::mem::ManuallyDrop::new(custody);
    let _ = &custody;
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP: {phase}; typed terminal custody retained"
    );
    std::process::abort();
}

fn report_physical_queue_failure(
    phase: &'static str,
    failure: &ferric_engine::M1PhysicalQueueOperationFailureV1,
) {
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP DETAIL: {phase}; shape={:?}; epoch={}; packets={}; lower_error={:?}",
        failure.shape(),
        failure.queue_epoch().value(),
        failure.shape().packet_count(),
        failure.error(),
    );
    if let Some(observation) = failure.timeout_execution_observation() {
        let _ = writeln!(
            std::io::stderr().lock(),
            "FAIL-STOP QUEUE TIMEOUT SNAPSHOT: packet_count={}; write_counter={}; read_counter={}; first_packet_header=0x{:04x}; first_packet_setup={}; first_signal_kind={}; first_signal_value={}; first_signal_state={:?}; queue_exception_reason_mask=0x{:016x}; currentness_confirmed={}",
            observation.packet_count(),
            observation.write_counter(),
            observation.read_counter(),
            observation.first_packet_header(),
            observation.first_packet_setup(),
            observation.first_signal_kind(),
            observation.first_signal().value(),
            observation.first_signal(),
            observation.queue_exception_reason_mask(),
            observation.currentness_confirmed(),
        );
    }
    if let Some(diagnostic) = failure.completion_progress_wait_diagnostic() {
        let _ = writeln!(
            std::io::stderr().lock(),
            "FAIL-STOP COMPLETION WAIT: policy_id={}; reason={:?}; scans={}; consecutive_scans_without_progress={}; total_scan_bound={:?}; completed_count_high_water={}",
            diagnostic.policy_id(),
            diagnostic.reason(),
            diagnostic.scans_performed(),
            diagnostic.consecutive_scans_without_progress(),
            diagnostic.total_scan_bound(),
            diagnostic.completed_count_high_water(),
        );
        if let Some(observation) = diagnostic.last_observation() {
            let _ = writeln!(
                std::io::stderr().lock(),
                "FAIL-STOP COMPLETION WAIT OBSERVATION: packet_count={}; completed_count={}; pending_count={}; first_observed_pending_batch_index={:?}",
                observation.packet_count(),
                observation.completed_count(),
                observation.pending_count(),
                observation.first_pending_batch_index(),
            );
        }
        if let Some(row) = failure.first_observed_pending_recipe_row() {
            let _ = writeln!(
                std::io::stderr().lock(),
                "FAIL-STOP FIRST OBSERVED-PENDING RECIPE ROW: dispatch_index={}; segment_index={}; stage={:?}; selection={:?}; kind={:?}; program_index={}",
                row.dispatch_index(),
                row.segment_index(),
                row.stage(),
                row.selection(),
                row.kind(),
                row.program_index(),
            );
        }
    }
}

fn invariant_fail_stop<T: CaptureInvariantCustodyV1>(phase: &'static str, custody: T) -> ! {
    let custody = core::mem::ManuallyDrop::new(custody);
    let _ = &custody;
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP: {phase}; typed invariant custody retained"
    );
    std::process::abort();
}

fn closed_teardown<T: CaptureClosedCustodyV1>(phase: &'static str, custody: T) -> ! {
    abort_with_closed_custody(phase, custody)
}

fn close_or_quarantine<S: CaptureClosedCustodyV1, F: CaptureTerminalCustodyV1>(
    phase: &'static str,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(phase, closed),
        Err(quarantine) => terminal_quarantine(phase, quarantine),
    }
}

fn close_or_quarantine_with_diagnostic<
    D: CaptureDiagnosticEvidenceV1,
    S: CaptureClosedCustodyV1,
    F: CaptureTerminalCustodyV1,
>(
    phase: &'static str,
    diagnostic: D,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: closed,
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: quarantine,
            },
        ),
    }
}

fn close_or_quarantine_round<S: CaptureClosedCustodyV1, F: CaptureTerminalCustodyV1>(
    phase: &'static str,
    state: QualificationRoundCaptureStateV1,
    diagnostic: Option<QualificationRoundDiagnosticV1>,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: diagnostic,
                _custody: closed,
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: diagnostic,
                _custody: quarantine,
            },
        ),
    }
}

fn terminal_round<T: CaptureTerminalCustodyV1>(
    phase: &'static str,
    state: QualificationRoundCaptureStateV1,
    custody: T,
) -> ! {
    terminal_quarantine(
        phase,
        QualificationRoundCustodyV1 {
            _state: state,
            _diagnostic: None,
            _custody: custody,
        },
    )
}

fn terminal_queue_round(
    phase: &'static str,
    state: QualificationRoundCaptureStateV1,
    failure: Box<M1RearmedQueueProgressFailureV1>,
) -> ! {
    report_physical_queue_failure(phase, failure.source());
    terminal_round(phase, state, failure)
}

fn close_or_quarantine_roster<S: CaptureClosedCustodyV1, F: CaptureTerminalCustodyV1>(
    phase: &'static str,
    roster: ferric_engine::M1DeviceKvCompletionRosterV1,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            CompletionRosterCustodyV1 {
                _roster: roster,
                _custody: closed,
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            CompletionRosterCustodyV1 {
                _roster: roster,
                _custody: quarantine,
            },
        ),
    }
}

fn close_or_quarantine_roster_with_diagnostic<
    D: CaptureDiagnosticEvidenceV1,
    S: CaptureClosedCustodyV1,
    F: CaptureTerminalCustodyV1,
>(
    phase: &'static str,
    diagnostic: D,
    roster: M1DeviceKvCompletionRosterV1,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: closed,
                },
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: quarantine,
                },
            },
        ),
    }
}

fn close_or_quarantine_r32_choices<
    D: CaptureDiagnosticEvidenceV1,
    S: CaptureClosedCustodyV1,
    F: CaptureTerminalCustodyV1,
>(
    phase: &'static str,
    diagnostic: D,
    choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: R32ChoiceCustodyV1 {
                    _choices: choices,
                    _custody: closed,
                },
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: R32ChoiceCustodyV1 {
                    _choices: choices,
                    _custody: quarantine,
                },
            },
        ),
    }
}

fn close_or_quarantine_r30_rollback_choices<
    D: CaptureDiagnosticEvidenceV1,
    S: CaptureClosedCustodyV1,
    F: CaptureTerminalCustodyV1,
>(
    phase: &'static str,
    diagnostic: D,
    choices: ferric_engine::M1ObservedSpeculativeDiagnosticChoicesV1,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: R30RollbackChoiceCustodyV1 {
                    _choices: choices,
                    _custody: closed,
                },
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: R30RollbackChoiceCustodyV1 {
                    _choices: choices,
                    _custody: quarantine,
                },
            },
        ),
    }
}

fn close_or_quarantine_qualification_evidence<
    S: CaptureClosedCustodyV1,
    F: CaptureTerminalCustodyV1,
>(
    phase: &'static str,
    evidence: ferric_engine::M1QualificationCompletionEvidenceV1,
    diagnostic: Option<QualificationEvidenceDiagnosticV1>,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: diagnostic,
                _custody: closed,
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: diagnostic,
                _custody: quarantine,
            },
        ),
    }
}

fn close_or_quarantine_prefill_live<S: CaptureClosedCustodyV1, F: CaptureTerminalCustodyV1>(
    phase: &'static str,
    evidence: PrefillLiveEvidenceV1,
    diagnostic: Option<PrefillLiveDiagnosticV1>,
    teardown: Result<S, F>,
) -> ! {
    match teardown {
        Ok(closed) => closed_teardown(
            phase,
            PrefillLiveCustodyV1 {
                _evidence: evidence,
                _diagnostic: diagnostic,
                _custody: closed,
            },
        ),
        Err(quarantine) => terminal_quarantine(
            phase,
            PrefillLiveCustodyV1 {
                _evidence: evidence,
                _diagnostic: diagnostic,
                _custody: quarantine,
            },
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn abandon_decode_initial_dispatch(
    phase_name: &'static str,
    engine: Engine<32>,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    caches: Vec<ActiveDeviceKvCache>,
    requests: Vec<RequestId>,
    contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
    scheduled: ferric_engine::M1ScheduledDispatchV1,
    phase: DecodeInitialPhaseCustodyV1,
) -> ! {
    terminal_quarantine(
        phase_name,
        DecodeInitialDispatchAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _memory: memory,
            _caches: caches,
            _requests: requests,
            _contexts: contexts,
            _scheduled: scheduled,
            _phase: Box::new(phase),
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn abandon_prefill_initial_dispatch(
    phase_name: &'static str,
    engine: Engine<32>,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    caches: Vec<ActiveDeviceKvCache>,
    requests: Vec<RequestId>,
    active_lengths: Vec<u32>,
    context_lengths: Vec<u32>,
    scheduled: ferric_engine::M1ScheduledDispatchV1,
    phase: PrefillInitialPhaseCustodyV1,
) -> ! {
    terminal_quarantine(
        phase_name,
        PrefillInitialDispatchAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _memory: memory,
            _caches: caches,
            _requests: requests,
            _active_lengths: active_lengths,
            _context_lengths: context_lengths,
            _scheduled: scheduled,
            _phase: Box::new(phase),
        },
    )
}

fn abandon_single_speculative_prepublication(
    phase_name: &'static str,
    engine: Engine<1>,
    cache: ActiveDeviceKvCache,
    phase: SingleSpeculativePrepublicationFailureV1,
) -> ! {
    terminal_quarantine(
        phase_name,
        SingleSpeculativePrepublicationAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _cache: cache,
            _phase: Box::new(phase),
        },
    )
}

fn abandon_released_round(
    phase_name: &'static str,
    mut engine: Engine<32>,
    released: QualificationReleasedRoundV1,
    state: QualificationRoundCaptureStateV1,
    phase: ReleasedRoundPhaseCustodyV1,
) -> ! {
    match released {
        QualificationReleasedRoundV1::First(released) => {
            match (*released).destroy_queue_and_retain_step(&mut engine) {
                Ok(closed) => closed_teardown(
                    phase_name,
                    ReleasedRoundTeardownCustodyV1 {
                        _state: state,
                        _phase: Box::new(phase),
                        _custody: closed,
                    },
                ),
                Err(quarantine) => terminal_quarantine(
                    phase_name,
                    ReleasedRoundTeardownCustodyV1 {
                        _state: state,
                        _phase: Box::new(phase),
                        _custody: quarantine,
                    },
                ),
            }
        }
        QualificationReleasedRoundV1::Rearmed(released) => {
            match released.destroy_queue_and_retain_round(&mut engine) {
                Ok(closed) => closed_teardown(
                    phase_name,
                    ReleasedRoundTeardownCustodyV1 {
                        _state: state,
                        _phase: Box::new(phase),
                        _custody: closed,
                    },
                ),
                Err(quarantine) => terminal_quarantine(
                    phase_name,
                    ReleasedRoundTeardownCustodyV1 {
                        _state: state,
                        _phase: Box::new(phase),
                        _custody: quarantine,
                    },
                ),
            }
        }
    }
}

fn abandon_pre_engine_memory(
    phase_name: &'static str,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    input_tokens: Vec<u32>,
    diagnostic: PreEngineDiagnosticV1,
) -> ! {
    invariant_fail_stop(
        phase_name,
        PreEngineMemoryAbandonmentV1 {
            _memory: memory,
            _input_tokens: input_tokens,
            _diagnostic: diagnostic,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn abandon_pre_physical_engine(
    phase_name: &'static str,
    engine: Engine<32>,
    requests: Vec<RequestId>,
    reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    pool: PrePhysicalPoolCustodyV1,
    input_tokens: Vec<u32>,
    diagnostic: PrePhysicalDiagnosticV1,
) -> ! {
    terminal_quarantine(
        phase_name,
        PrePhysicalEngineAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _requests: requests,
            _reservations: reservations,
            _pool: pool,
            _input_tokens: input_tokens,
            _diagnostic: diagnostic,
        },
    )
}

fn abandon_prelease(
    phase_name: &'static str,
    engine: Engine<32>,
    requests: Vec<RequestId>,
    reservations: Vec<ferric_engine::PendingDeviceKvStepWrite>,
    input_tokens: Vec<u32>,
    outcome: PreleaseCancellationOutcomeV1,
) -> ! {
    terminal_quarantine(
        phase_name,
        PreleaseAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _requests: requests,
            _reservations: reservations,
            _input_tokens: input_tokens,
            _outcome: outcome,
        },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlanCase {
    id: String,
    input_sha256: String,
    kind: String,
    workload_sha256: String,
}

#[derive(Debug)]
struct DifferentialPlan {
    bytes: Vec<u8>,
    cases: Vec<PlanCase>,
    identities: BTreeMap<String, String>,
    input_sha256: String,
}

impl DifferentialPlan {
    fn case(&self, id: &str) -> CaptureResult<&PlanCase> {
        self.cases
            .iter()
            .find(|case| case.id == id)
            .ok_or_else(|| format!("case {id:?} is absent from the benchmark plan"))
    }

    fn identity(&self, name: &str) -> CaptureResult<&str> {
        self.identities
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("benchmark plan identity is absent: {name}"))
    }

    fn sha256(&self) -> String {
        sha256_hex(&self.bytes)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LaneInput {
    active_length: u32,
    context_length: u32,
}

#[derive(Debug)]
struct Workload {
    bytes: Vec<u8>,
    input_path: PathBuf,
    input_bytes: u64,
    input_sha256: String,
    kind: String,
    lanes: Vec<LaneInput>,
    selection: Qwen3PlanSelection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClosureIdentities {
    compiler: Identity,
    compiler_configuration: Identity,
    fe2o3_source: Identity,
    ferric_source: Identity,
    kernel_abi_catalog: Identity,
    kernel_proof_set: Identity,
    qualification_protocol: Identity,
    runtime_abi: Identity,
    runtime_contract: Identity,
    target_contract: Identity,
    tcb_report: Identity,
    validator_registry: Identity,
}

#[derive(Debug)]
struct ModelInputBytes {
    admission_record: Vec<u8>,
    deployment_bundle: Vec<u8>,
    draft_config: Vec<u8>,
    draft_manifest: Vec<u8>,
    draft_tokenizer: Vec<u8>,
    draft_tokenizer_metadata: Vec<u8>,
    draft_weights: Box<[u8]>,
    target_config: Vec<u8>,
    target_manifest: Vec<u8>,
    target_tokenizer: Vec<u8>,
    target_tokenizer_metadata: Vec<u8>,
    target_weights: Box<[u8]>,
}

impl ModelInputBytes {
    fn authenticate(&self) -> CaptureResult<AuthenticatedBundleAdmission> {
        let descriptor = decode_bundle_admission_record(&self.admission_record)
            .map_err(|error| format!("cannot decode bundle admission record: {error}"))?;
        let target = reopen_persisted_qwen3_weights(
            Qwen3ModelRole::Target8B,
            descriptor.target_manifest,
            &self.target_manifest,
            Cursor::new(&self.target_weights),
        )
        .map_err(|error| format!("cannot authenticate persisted target weights: {error}"))?;
        let draft = reopen_persisted_qwen3_weights(
            Qwen3ModelRole::Draft06B,
            descriptor.draft_manifest,
            &self.draft_manifest,
            Cursor::new(&self.draft_weights),
        )
        .map_err(|error| format!("cannot authenticate persisted draft weights: {error}"))?;
        let target_tokenizer = authenticate_qwen3_tokenizer(
            Qwen3ModelRole::Target8B,
            Cursor::new(&self.target_tokenizer),
        )
        .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
        let draft_tokenizer = authenticate_qwen3_tokenizer(
            Qwen3ModelRole::Draft06B,
            Cursor::new(&self.draft_tokenizer),
        )
        .map_err(|error| format!("cannot authenticate draft tokenizer: {error}"))?;
        let prepacked = build_prepacked_deployment_bundle(
            authenticated_assets(
                &self.target_config,
                &self.target_tokenizer_metadata,
                &self.draft_config,
                &self.draft_tokenizer_metadata,
            ),
            target_tokenizer,
            draft_tokenizer,
            target,
            draft,
        )
        .map_err(|error| format!("cannot reconstruct prepacked deployment: {error}"))?;
        validate_persisted_deployment(&prepacked, &descriptor.deployment, &self.deployment_bundle)?;
        let admission = seal_authenticated_bundle(prepacked)
            .map_err(|error| format!("cannot re-seal authenticated deployment: {error}"))?;
        if admission.record().as_bytes().as_slice() != self.admission_record.as_slice() {
            return Err("persisted admission record does not re-seal exactly".to_owned());
        }
        Ok(admission)
    }
}

#[derive(Debug)]
struct SecureDirectory {
    descriptor: OwnedFd,
}

#[derive(Debug)]
struct SecureFile {
    file: File,
    initial: Stat,
}

impl SecureDirectory {
    fn open(path: &Path, description: &str) -> CaptureResult<Self> {
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        Ok(Self { descriptor })
    }

    fn read_bounded(
        &self,
        relative: &Path,
        maximum_bytes: u64,
        description: &str,
    ) -> CaptureResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        let length = input.length(description)?;
        let length_u64 =
            u64::try_from(length).map_err(|_| format!("{description} length does not fit u64"))?;
        if length == 0 || length_u64 > maximum_bytes {
            return Err(format!("{description} size is outside the admitted bound"));
        }
        input.read_exact_snapshot(length, description)
    }

    fn read_exact(
        &self,
        relative: &Path,
        expected_bytes: u64,
        description: &str,
    ) -> CaptureResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        let length = input.length(description)?;
        if u64::try_from(length).ok() != Some(expected_bytes) {
            return Err(format!("{description} length drifted"));
        }
        input.read_exact_snapshot(length, description)
    }

    fn read_canonical(
        &self,
        relative: &Path,
        description: &str,
    ) -> CaptureResult<(Value, Vec<u8>)> {
        let bytes = self.read_bounded(relative, MAX_DOCUMENT_BYTES as u64, description)?;
        let value = parse_canonical(&bytes, description)?;
        Ok((value, bytes))
    }

    fn open_file(&self, relative: &Path, description: &str) -> CaptureResult<SecureFile> {
        require_relative(relative, description)?;
        let descriptor = openat2(
            &self.descriptor,
            relative,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect opened {description}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
            return Err(format!("{description} must be a regular file"));
        }
        if initial.st_nlink != 1 {
            return Err(format!(
                "{description} must have exactly one filesystem link"
            ));
        }
        Ok(SecureFile {
            file: File::from(descriptor),
            initial,
        })
    }
}

impl SecureFile {
    fn length(&self, description: &str) -> CaptureResult<usize> {
        usize::try_from(self.initial.st_size)
            .map_err(|_| format!("{description} is too large for this host"))
    }

    fn read_exact_snapshot(&mut self, length: usize, description: &str) -> CaptureResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("cannot reserve {description} read buffer"))?;
        let read = (&mut self.file)
            .take(u64::try_from(length).unwrap_or(u64::MAX).saturating_add(1))
            .read_to_end(&mut bytes);
        let snapshot = self.validate_snapshot(description);
        if let Err(error) = read {
            snapshot?;
            return Err(format!("cannot read {description}: {error}"));
        }
        snapshot?;
        if bytes.len() != length {
            return Err(format!("{description} changed during the exact read"));
        }
        Ok(bytes)
    }

    fn validate_snapshot(&self, description: &str) -> CaptureResult<()> {
        let final_stat = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if !same_file_snapshot(&self.initial, &final_stat) {
            return Err(format!("{description} changed while being read"));
        }
        Ok(())
    }

    fn sha256_snapshot(&mut self, description: &str) -> CaptureResult<String> {
        let expected = self.length(description)?;
        if expected == 0 {
            return Err(format!("{description} must not be empty"));
        }
        let mut hasher = Sha256::new();
        let mut actual = 0_usize;
        let mut buffer = vec![0_u8; 64 * 1_024].into_boxed_slice();
        loop {
            let count = self
                .file
                .read(&mut buffer)
                .map_err(|error| format!("cannot read {description}: {error}"))?;
            if count == 0 {
                break;
            }
            actual = actual
                .checked_add(count)
                .ok_or_else(|| format!("{description} length overflowed"))?;
            if actual > expected {
                return Err(format!("{description} changed while being measured"));
            }
            hasher.update(&buffer[..count]);
        }
        self.validate_snapshot(description)?;
        if actual != expected {
            return Err(format!("{description} changed while being measured"));
        }
        Ok(hex_bytes(&hasher.finalize()))
    }
}

struct StagingOutput {
    parent: OwnedFd,
    staging: OwnedFd,
    staging_snapshot: Stat,
    staging_name: OsString,
    output_name: OsString,
    files: Vec<StagedOutputFileV1>,
    armed: bool,
}

struct StagedOutputFileV1 {
    name: OsString,
    snapshot: Stat,
}

impl StagingOutput {
    fn create(output: &Path) -> CaptureResult<Self> {
        let output_name = output
            .file_name()
            .map(OsString::from)
            .ok_or_else(|| "output bundle path has no final component".to_owned())?;
        let parent_path = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = openat2(
            CWD,
            parent_path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open output parent: {error}"))?;
        if path_exists_at(&parent, &output_name)? {
            return Err("output bundle already exists".to_owned());
        }
        for nonce in 0..1_024_u16 {
            let mut staging_name = OsString::from(".");
            staging_name.push(&output_name);
            staging_name.push(format!(".staging.{}.{nonce}", std::process::id()));
            match mkdirat(
                &parent,
                staging_name.as_os_str(),
                Mode::RUSR | Mode::WUSR | Mode::XUSR,
            ) {
                Ok(()) => {
                    let staging = match openat2(
                        &parent,
                        Path::new(&staging_name),
                        OFlags::RDONLY
                            | OFlags::DIRECTORY
                            | OFlags::NOFOLLOW
                            | OFlags::NONBLOCK
                            | OFlags::CLOEXEC,
                        Mode::empty(),
                        ResolveFlags::BENEATH
                            | ResolveFlags::NO_SYMLINKS
                            | ResolveFlags::NO_MAGICLINKS,
                    ) {
                        Ok(staging) => staging,
                        Err(error) => {
                            let _ = unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                            return Err(format!("cannot open staging output: {error}"));
                        }
                    };
                    let staging_snapshot = fstat(&staging)
                        .map_err(|error| format!("cannot inspect staging output: {error}"))?;
                    if FileType::from_raw_mode(staging_snapshot.st_mode) != FileType::Directory {
                        let _ = unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                        return Err("created staging output is not a directory".to_owned());
                    }
                    return Ok(Self {
                        parent,
                        staging,
                        staging_snapshot,
                        staging_name,
                        output_name,
                        files: Vec::new(),
                        armed: true,
                    });
                }
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => return Err(format!("cannot create staging output: {error}")),
            }
        }
        Err("staging output namespace was exhausted".to_owned())
    }

    fn write(&mut self, name: &str, bytes: &[u8]) -> CaptureResult<()> {
        self.write_with(name, |file| file.write_all(bytes))
    }

    fn write_with(
        &mut self,
        name: &str,
        writer: impl FnOnce(&mut File) -> std::io::Result<()>,
    ) -> CaptureResult<()> {
        let name = OsString::from(name);
        let descriptor = openat2(
            &self.staging,
            Path::new(&name),
            OFlags::WRONLY
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot create staged output {}: {error}", name.display()))?;
        let mut file = File::from(descriptor);
        let created = fstat(&file)
            .map_err(|error| format!("cannot inspect staged output {}: {error}", name.display()))?;
        self.files.push(StagedOutputFileV1 {
            name: name.clone(),
            snapshot: created,
        });
        if FileType::from_raw_mode(created.st_mode) != FileType::RegularFile
            || created.st_nlink != 1
        {
            return Err(format!(
                "created staged output must be a one-link regular file: {}",
                name.display()
            ));
        }
        writer(&mut file)
            .map_err(|error| format!("cannot write staged output {}: {error}", name.display()))?;
        file.sync_all()
            .map_err(|error| format!("cannot sync staged output {}: {error}", name.display()))?;
        let written = fstat(&file).map_err(|error| {
            format!(
                "cannot reinspect written staged output {}: {error}",
                name.display()
            )
        })?;
        if created.st_dev != written.st_dev
            || created.st_ino != written.st_ino
            || created.st_mode != written.st_mode
            || created.st_nlink != written.st_nlink
            || created.st_uid != written.st_uid
            || created.st_gid != written.st_gid
            || FileType::from_raw_mode(written.st_mode) != FileType::RegularFile
            || written.st_nlink != 1
        {
            return Err(format!(
                "staged output identity changed during write: {}",
                name.display()
            ));
        }
        let Some(record) = self.files.last_mut() else {
            return Err("staged output record disappeared during write".to_owned());
        };
        record.snapshot = written;
        Ok(())
    }

    fn publish(mut self) -> CaptureResult<()> {
        fsync(&self.staging).map_err(|error| format!("cannot sync staging directory: {error}"))?;
        if !self.name_binds_staging(self.staging_name.as_os_str())? {
            return Err("staging output name no longer binds the held directory".to_owned());
        }
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| format!("cannot publish output without replacement: {error}"))?;
        self.armed = false;
        if !self.name_binds_staging(self.output_name.as_os_str())? {
            return Err("published output name does not bind the held directory".to_owned());
        }
        if let Err(error) = fsync(&self.parent) {
            eprintln!("WARN: output bundle is visible but parent sync failed: {error}");
        }
        Ok(())
    }

    fn publish_exact(mut self, expected: &[(&str, &[u8])]) -> CaptureResult<()> {
        let expected_names = expected
            .iter()
            .map(|(name, _)| OsString::from(name))
            .collect::<Vec<_>>();
        if self.files.iter().map(|file| &file.name).ne(&expected_names) {
            return Err("staged output roster differs from the exact protocol".to_owned());
        }
        let staged = self.rebind_directory(self.staging_name.as_os_str(), "staged")?;
        self.verify_exact_files(&staged, expected, "staged")?;
        fsync(&self.staging).map_err(|error| format!("cannot sync staging directory: {error}"))?;
        if !self.name_binds_staging(self.staging_name.as_os_str())? {
            return Err("staging output name no longer binds the held directory".to_owned());
        }
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| format!("cannot publish output without replacement: {error}"))?;
        self.armed = false;
        let published = self.rebind_directory(self.output_name.as_os_str(), "published")?;
        self.verify_exact_files(&published, expected, "published")?;
        fsync(&self.parent)
            .map_err(|error| format!("cannot sync published output parent: {error}"))?;
        let final_binding =
            self.rebind_directory(self.output_name.as_os_str(), "final published")?;
        let published_stat = fstat(&published)
            .map_err(|error| format!("cannot inspect published output directory: {error}"))?;
        let final_stat = fstat(&final_binding)
            .map_err(|error| format!("cannot reinspect published output directory: {error}"))?;
        if published_stat.st_dev != final_stat.st_dev || published_stat.st_ino != final_stat.st_ino
        {
            return Err("published output name changed during content verification".to_owned());
        }
        Ok(())
    }

    fn rebind_directory(&self, name: &OsStr, phase: &str) -> CaptureResult<OwnedFd> {
        let reopened = openat2(
            &self.parent,
            Path::new(name),
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind {phase} output directory: {error}"))?;
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held staging directory: {error}"))?;
        let rebound = fstat(&reopened)
            .map_err(|error| format!("cannot inspect rebound {phase} output directory: {error}"))?;
        if [held, rebound].iter().any(|current| {
            current.st_dev != self.staging_snapshot.st_dev
                || current.st_ino != self.staging_snapshot.st_ino
                || current.st_mode != self.staging_snapshot.st_mode
                || current.st_nlink != self.staging_snapshot.st_nlink
                || current.st_uid != self.staging_snapshot.st_uid
                || current.st_gid != self.staging_snapshot.st_gid
                || FileType::from_raw_mode(current.st_mode) != FileType::Directory
        }) {
            return Err(format!(
                "{phase} output name does not bind the held directory"
            ));
        }
        Ok(reopened)
    }

    fn verify_exact_files(
        &self,
        directory: &OwnedFd,
        expected: &[(&str, &[u8])],
        phase: &str,
    ) -> CaptureResult<()> {
        let expected_roster = expected
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        if Self::directory_roster(directory, phase)? != expected_roster {
            return Err(format!("{phase} output file roster drifted"));
        }
        for ((name, expected_bytes), created) in expected.iter().zip(&self.files) {
            if created.name != OsStr::new(name) {
                return Err(format!("{phase} output file order drifted"));
            }
            Self::verify_exact_file(directory, name, expected_bytes, &created.snapshot, phase)?;
        }
        if Self::directory_roster(directory, phase)? != expected_roster {
            return Err(format!(
                "{phase} output file roster changed during verification"
            ));
        }
        Ok(())
    }

    fn directory_roster(directory: &OwnedFd, phase: &str) -> CaptureResult<BTreeSet<String>> {
        let mut entries = Dir::read_from(directory)
            .map_err(|error| format!("cannot enumerate {phase} output directory: {error}"))?;
        let mut names = BTreeSet::new();
        while let Some(entry) = entries.read() {
            let entry = entry
                .map_err(|error| format!("cannot enumerate {phase} output directory: {error}"))?;
            let bytes = entry.file_name().to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            let name = std::str::from_utf8(bytes)
                .map_err(|_| format!("{phase} output filename must be UTF-8"))?;
            if !name.is_ascii() || !names.insert(name.to_owned()) {
                return Err(format!("{phase} output file roster is invalid"));
            }
        }
        Ok(names)
    }

    fn verify_exact_file(
        directory: &OwnedFd,
        name: &str,
        expected: &[u8],
        created: &Stat,
        phase: &str,
    ) -> CaptureResult<()> {
        let descriptor = openat2(
            directory,
            Path::new(name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot open {phase} output file {name}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect {phase} output file {name}: {error}"))?;
        if !Self::same_file_snapshot(created, &initial)
            || FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
            || usize::try_from(initial.st_size).ok() != Some(expected.len())
        {
            return Err(format!("{phase} output file metadata drifted: {name}"));
        }
        let mut file = File::from(descriptor);
        let limit = u64::try_from(expected.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(expected.len().saturating_add(1))
            .map_err(|_| format!("cannot reserve {phase} output verification buffer: {name}"))?;
        Read::by_ref(&mut file)
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread {phase} output file {name}: {error}"))?;
        let final_stat = fstat(&file)
            .map_err(|error| format!("cannot reinspect {phase} output file {name}: {error}"))?;
        let observed_sha256 = sha256_hex(&bytes);
        let expected_sha256 = sha256_hex(expected);
        if bytes != expected
            || observed_sha256 != expected_sha256
            || !Self::same_file_snapshot(&initial, &final_stat)
        {
            return Err(format!("{phase} output bytes changed: {name}"));
        }
        let rebound = openat2(
            directory,
            Path::new(name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind {phase} output file {name}: {error}"))?;
        let rebound_stat = fstat(&rebound).map_err(|error| {
            format!("cannot inspect rebound {phase} output file {name}: {error}")
        })?;
        if !Self::same_file_snapshot(&final_stat, &rebound_stat) {
            return Err(format!(
                "{phase} output filename changed during verification: {name}"
            ));
        }
        Ok(())
    }

    fn same_file_snapshot(left: &Stat, right: &Stat) -> bool {
        left.st_dev == right.st_dev
            && left.st_ino == right.st_ino
            && left.st_mode == right.st_mode
            && left.st_nlink == right.st_nlink
            && left.st_uid == right.st_uid
            && left.st_gid == right.st_gid
            && left.st_size == right.st_size
            && left.st_mtime == right.st_mtime
            && left.st_mtime_nsec == right.st_mtime_nsec
            && left.st_ctime == right.st_ctime
            && left.st_ctime_nsec == right.st_ctime_nsec
    }

    fn name_binds_staging(&self, name: &OsStr) -> CaptureResult<bool> {
        let reopened = match openat2(
            &self.parent,
            Path::new(name),
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        ) {
            Ok(reopened) => reopened,
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(false),
            Err(error) => return Err(format!("cannot reopen staged output name: {error}")),
        };
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held staging directory: {error}"))?;
        let named = fstat(&reopened)
            .map_err(|error| format!("cannot inspect named staging directory: {error}"))?;
        Ok(held.st_dev == named.st_dev
            && held.st_ino == named.st_ino
            && FileType::from_raw_mode(named.st_mode) == FileType::Directory)
    }
}

impl Drop for StagingOutput {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for file in &self.files {
            let bound = openat2(
                &self.staging,
                Path::new(&file.name),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )
            .ok()
            .and_then(|descriptor| fstat(&descriptor).ok())
            .is_some_and(|current| {
                current.st_dev == file.snapshot.st_dev && current.st_ino == file.snapshot.st_ino
            });
            if bound {
                let _ = unlinkat(&self.staging, file.name.as_os_str(), AtFlags::empty());
            }
        }
        if self
            .name_binds_staging(self.staging_name.as_os_str())
            .unwrap_or(false)
        {
            let _ = unlinkat(
                &self.parent,
                self.staging_name.as_os_str(),
                AtFlags::REMOVEDIR,
            );
        }
    }
}

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> CaptureResult<()> {
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r30_capture_composition::COMMAND)
    {
        return m1_r30_capture_composition::run(&arguments[1..]);
    }
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r30_canary_partial_capture::COMMAND)
    {
        return run_r30_canary_capture(&arguments[1..]);
    }
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r30_partial_capture::COMMAND)
    {
        return run_r30_cancellation_capture(&arguments[1..]);
    }
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r30_exhaustion_partial_capture::COMMAND)
    {
        return run_r30_exhaustion_capture(&arguments[1..]);
    }
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r30_rollback_partial_capture::COMMAND)
    {
        return run_r30_rollback_capture(&arguments[1..]);
    }
    if arguments.first().and_then(|argument| argument.to_str())
        == Some(m1_r32_partial_capture::COMMAND)
    {
        return run_r32_speculative_capture(&arguments[1..]);
    }
    if arguments.len() != 11 {
        match arguments.first().and_then(|argument| argument.to_str()) {
            Some("generate-inputs") => return input_bundle::generate_inputs(&arguments[1..]),
            Some("validate-inputs") => return input_bundle::validate_inputs(&arguments[1..]),
            _ => {}
        }
    }
    run_capture(&arguments)
}

fn run_r30_canary_capture(arguments: &[OsString]) -> CaptureResult<()> {
    let [source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture capture-r30-canary MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let closure = load_closure(Path::new(closure_path))?;
    let _environment = load_environment(Path::new(environment_path), gpu_unique_id)?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
    let (capture, workload) = execute_r30_canary_capture(&runner, memory, gpu_unique_id)?;
    let closed = capture
        .r30_canary_closed
        .as_ref()
        .ok_or_else(|| "guarded capture lost closed queue custody".to_owned())?;
    let artifact = m1_r30_canary_partial_capture::manifest(
        m1_r30_canary_partial_capture::ClosedCaptureInputsV1 {
            closed,
            device_id: capture.device_id,
            gpu_unique_id,
            runner: &runner,
            workload: &workload,
        },
    )?;
    let capture_sha256 = sha256_hex(artifact.bytes());
    m1_r30_canary_partial_capture::publish(Path::new(output), artifact)?;
    println!("output={}", Path::new(output).display());
    println!("capture_sha256={capture_sha256}");
    println!("status=partial-non-evidence");
    Ok(())
}

fn execute_r30_canary_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    gpu_unique_id: u64,
) -> CaptureResult<(CapturedOutput, Workload)> {
    let (workload, input_tokens) = fixed_r30_canary_workload()?;
    execute_capture(
        runner,
        memory,
        &workload,
        input_tokens,
        gpu_unique_id,
        CapturePurposeV1::R30PartialCanary,
    )
    .map(|capture| (capture, workload))
}

fn fixed_r30_prefill_input_tokens() -> Vec<u32> {
    vec![R30_PREFILL_INPUT_TOKEN; R30_PREFILL_ACTIVE_TOKENS as usize]
}

fn fixed_r30_prefill_input_bytes() -> Vec<u8> {
    fixed_r30_prefill_input_tokens()
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect()
}

fn completion_wait_policy_contract() -> Value {
    json!({
        "id": ferric_engine::M1_COMPLETION_PROGRESS_WAIT_POLICY_ID_V2,
        "max_consecutive_scans_without_progress": ferric_engine::M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
        "minimum_pending_scan_pause_micros": ferric_engine::M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1,
        "timeout_basis": "paced-completion-signal-scans",
        "total_scan_bound_rule": "(packet-count+1)*max-consecutive-scans-without-progress",
    })
}

fn validate_completion_wait_policy(value: &Value) -> CaptureResult<()> {
    let object = exact_object(
        value,
        &[
            "id",
            "max_consecutive_scans_without_progress",
            "minimum_pending_scan_pause_micros",
            "timeout_basis",
            "total_scan_bound_rule",
        ],
        "qualification completion wait policy",
    )?;
    if object
        != completion_wait_policy_contract()
            .as_object()
            .expect("fixed completion wait policy is an object")
    {
        return Err("qualification completion wait policy drifted".to_owned());
    }
    Ok(())
}

fn validate_r30_prefill_page_contract() -> CaptureResult<()> {
    let pages = usize::try_from(qualification_kv_page_count(0, R30_PREFILL_ACTIVE_TOKENS)?)
        .map_err(|_| "fixed R30 target-page count does not fit usize".to_owned())?;
    if pages != R30_PREFILL_TARGET_PAGES {
        return Err("fixed R30 target-page count drifted from qualification geometry".to_owned());
    }
    Ok(())
}

fn fixed_r30_canary_workload() -> CaptureResult<(Workload, Vec<u32>)> {
    validate_r30_prefill_page_contract()?;
    let input_tokens = fixed_r30_prefill_input_tokens();
    let input_bytes = fixed_r30_prefill_input_bytes();
    let selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    };
    let workload_bytes = canonical_bytes(&json!({
        "active_length": R30_PREFILL_ACTIVE_TOKENS,
        "case": "target-prefill-s1-t128",
        "context_length": 0,
        "completion_wait_policy": completion_wait_policy_contract(),
        "format": "FERRIC-M1-R30-CANARY-WORKLOAD-V4",
        "input_bytes": R30_PREFILL_INPUT_BYTES,
        "input_token": R30_PREFILL_INPUT_TOKEN,
        "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
        "lane_count": 1,
        "selection": "target-prefill-s1-t128",
    }))?;
    let workload = Workload {
        bytes: workload_bytes,
        input_path: PathBuf::from("frozen-r30-canary-input-u32le"),
        input_bytes: u64::try_from(input_bytes.len()).unwrap_or(u64::MAX),
        input_sha256: sha256_hex(&input_bytes),
        kind: "prefill-s1-t128".to_owned(),
        lanes: vec![LaneInput {
            active_length: R30_PREFILL_ACTIVE_TOKENS,
            context_length: 0,
        }],
        selection,
    };
    validate_workload_geometry(&workload)?;
    Ok((workload, input_tokens))
}

fn run_r30_cancellation_capture(arguments: &[OsString]) -> CaptureResult<()> {
    let [source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture capture-r30-cancellation MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let (closure, closure_bytes) = load_closure_with_bytes(Path::new(closure_path))?;
    let environment_bytes = load_environment(Path::new(environment_path), gpu_unique_id)?;
    let executable_sha256 = current_executable_sha256()?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
    let (workload, input_tokens) = fixed_r30_cancellation_workload()?;
    let capture = execute_capture(
        &runner,
        memory,
        &workload,
        input_tokens,
        gpu_unique_id,
        CapturePurposeV1::R30PartialCancellation,
    )?;
    let settlement = capture.settlement.as_ref().ok_or_else(|| {
        "fixed target-prefill execution omitted partial cancellation settlement".to_owned()
    })?;
    let closure_sha256 = sha256_hex(&closure_bytes);
    let environment_sha256 = sha256_hex(&environment_bytes);
    let manifest =
        m1_r30_partial_capture::manifest(m1_r30_partial_capture::CaptureManifestInputsV4 {
            capture: &capture,
            closure_sha256: &closure_sha256,
            environment_sha256: &environment_sha256,
            executable_sha256: &executable_sha256,
            gpu_unique_id,
            runner: &runner,
            settlement,
            workload: &workload,
        })?;
    let protocol_sha256 = m1_r30_partial_capture::protocol_sha256()?;
    m1_r30_partial_capture::publish(Path::new(output), &manifest)?;
    println!("output={}", Path::new(output).display());
    println!("capture_sha256={}", sha256_hex(&manifest));
    println!("partial_protocol_sha256={protocol_sha256}");
    println!("status=partial-non-evidence");
    Ok(())
}

fn fixed_r30_cancellation_workload() -> CaptureResult<(Workload, Vec<u32>)> {
    validate_r30_prefill_page_contract()?;
    let input_tokens = fixed_r30_prefill_input_tokens();
    let input_bytes = fixed_r30_prefill_input_bytes();
    let workload_bytes = canonical_bytes(&json!({
        "active_length": R30_PREFILL_ACTIVE_TOKENS,
        "case": "target-prefill-s1-t128-retirement-before-observation",
        "context_length": 0,
        "completion_wait_policy": completion_wait_policy_contract(),
        "format": "FERRIC-M1-R30-CANCELLATION-WORKLOAD-V5",
        "input_bytes": R30_PREFILL_INPUT_BYTES,
        "input_token": R30_PREFILL_INPUT_TOKEN,
        "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
        "lane_count": 1,
        "selection": "target-prefill-s1-t128",
    }))?;
    let workload = Workload {
        bytes: workload_bytes,
        input_path: PathBuf::from("frozen-r30-cancellation-input-u32le"),
        input_bytes: u64::try_from(input_bytes.len()).unwrap_or(u64::MAX),
        input_sha256: sha256_hex(&input_bytes),
        kind: "prefill-s1-t128".to_owned(),
        lanes: vec![LaneInput {
            active_length: R30_PREFILL_ACTIVE_TOKENS,
            context_length: 0,
        }],
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        },
    };
    validate_workload_geometry(&workload)?;
    Ok((workload, input_tokens))
}

fn run_r30_exhaustion_capture(arguments: &[OsString]) -> CaptureResult<()> {
    let [source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture capture-r30-exhaustion MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let closure = load_closure(Path::new(closure_path))?;
    let _environment = load_environment(Path::new(environment_path), gpu_unique_id)?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
    let capture = execute_r30_exhaustion_capture(&runner, memory)?;
    let capture_sha256 = sha256_hex(capture.bytes());
    m1_r30_exhaustion_partial_capture::publish(Path::new(output), capture)?;
    println!("output={}", Path::new(output).display());
    println!("capture_sha256={capture_sha256}");
    println!("status=partial-non-evidence");
    Ok(())
}

fn execute_r30_exhaustion_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
) -> CaptureResult<m1_r30_exhaustion_partial_capture::CaptureArtifactV1> {
    let mut engine = Engine::<1>::new(512, 256, 8_192)
        .map_err(|error| format!("cannot construct one-lane r30 exhaustion engine: {error:?}"))?;
    let request = engine
        .admit()
        .map_err(|error| format!("cannot admit r30 exhaustion request: {error:?}"))?;
    let device = memory.device();
    let target_arena = memory.allocation_id(Qwen3ModelRole::Target8B);
    let draft_arena = memory.allocation_id(Qwen3ModelRole::Draft06B);
    let page_capacity = ferric_spec::M1_KV_PHYSICAL_PAGE_SLOTS;
    let mut leases = Vec::new();
    leases
        .try_reserve_exact(page_capacity)
        .map_err(|_| "cannot reserve r30 exhaustion page custody".to_owned())?;
    for page in 0..page_capacity {
        let page =
            u32::try_from(page).map_err(|_| "r30 exhaustion page index exceeds u32".to_owned())?;
        let lease = match memory.lease_page(request, Qwen3ModelRole::Target8B, page) {
            Ok(lease) => lease,
            Err(error) => {
                let diagnostic = format!("cannot lease r30 exhaustion page {page}: {error:?}");
                if !leases.is_empty() {
                    cleanup_unpublished_or_abort(
                        "r30 exhaustion partial page acquisition rejected",
                        &mut memory,
                        leases,
                    );
                }
                return Err(diagnostic);
            }
        };
        leases.push(lease);
    }
    let first_generation = leases
        .first()
        .map(|lease| lease.page().generation())
        .ok_or_else(|| "r30 exhaustion page roster is empty".to_owned())?;
    if leases.iter().any(|lease| {
        lease.request() != request
            || lease.allocation_id() != target_arena
            || lease.page().role() != Qwen3ModelRole::Target8B
            || lease.page().generation() != first_generation
            || memory
                .validate_page_identity(request, target_arena, lease.page())
                .is_err()
    }) {
        cleanup_unpublished_or_abort(
            "r30 exhaustion retained page identity drift",
            &mut memory,
            leases,
        );
        return Err("r30 exhaustion retained page identity drift".to_owned());
    }
    let occupied_roster_len = leases.len();
    let occupied_page_rejected = match memory.lease_page(request, Qwen3ModelRole::Target8B, 0) {
        Err(ferric_engine::M1DeviceKvArenaLeaseErrorV1::PageAlreadyLeased) => true,
        Err(error) => {
            let diagnostic =
                format!("r30 exhaustion returned the wrong occupied-slot rejection: {error:?}");
            cleanup_unpublished_or_abort(
                "r30 exhaustion returned the wrong occupied-slot rejection",
                &mut memory,
                leases,
            );
            return Err(diagnostic);
        }
        Ok(extra) => {
            leases.push(extra);
            cleanup_unpublished_or_abort(
                "r30 exhaustion re-leased an occupied page",
                &mut memory,
                leases,
            );
            return Err("r30 exhaustion re-leased an occupied page".to_owned());
        }
    };
    let rejected_page = u32::try_from(page_capacity)
        .map_err(|_| "r30 exhaustion boundary index exceeds u32".to_owned())?;
    let out_of_range_page_rejected =
        match memory.lease_page(request, Qwen3ModelRole::Target8B, rejected_page) {
            Err(ferric_engine::M1DeviceKvArenaLeaseErrorV1::PageOutOfRange) => true,
            Err(error) => {
                let diagnostic =
                    format!("r30 exhaustion returned the wrong boundary rejection: {error:?}");
                cleanup_unpublished_or_abort(
                    "r30 exhaustion returned the wrong boundary rejection",
                    &mut memory,
                    leases,
                );
                return Err(diagnostic);
            }
            Ok(extra) => {
                leases.push(extra);
                cleanup_unpublished_or_abort(
                    "r30 exhaustion admitted a page beyond exact capacity",
                    &mut memory,
                    leases,
                );
                return Err("r30 exhaustion admitted a page beyond exact capacity".to_owned());
            }
        };
    if leases.len() != occupied_roster_len
        || leases.iter().any(|lease| {
            lease.request() != request
                || lease.allocation_id() != target_arena
                || lease.page().role() != Qwen3ModelRole::Target8B
                || lease.page().generation() != first_generation
                || memory
                    .validate_page_identity(request, target_arena, lease.page())
                    .is_err()
        })
    {
        cleanup_unpublished_or_abort(
            "r30 exhaustion rejection changed retained page custody",
            &mut memory,
            leases,
        );
        return Err("r30 exhaustion rejection changed retained page custody".to_owned());
    }
    let checked_leases = leases.len();
    let returned_pages = match memory.return_unpublished_pages(leases) {
        Ok(returned) => returned,
        Err(failure) => quarantine_unpublished_pages(
            "r30 exhaustion complete page return rejected",
            memory,
            failure,
        ),
    };
    let reused = memory
        .lease_page(request, Qwen3ModelRole::Target8B, 0)
        .map_err(|error| format!("cannot re-lease returned r30 exhaustion page: {error:?}"))?;
    let reused_generation = reused.page().generation();
    let Some(expected_reused_generation) = first_generation.checked_add(1) else {
        cleanup_unpublished_or_abort(
            "r30 exhaustion page generation overflow",
            &mut memory,
            vec![reused],
        );
        return Err("r30 exhaustion page generation overflow".to_owned());
    };
    if reused_generation != expected_reused_generation
        || memory
            .validate_page_identity(request, target_arena, reused.page())
            .is_err()
    {
        cleanup_unpublished_or_abort(
            "r30 exhaustion generation-advanced reuse drift",
            &mut memory,
            vec![reused],
        );
        return Err("r30 exhaustion generation-advanced reuse drift".to_owned());
    }
    let reused_returned_pages = match memory.return_unpublished_pages(vec![reused]) {
        Ok(returned) => returned,
        Err(failure) => quarantine_unpublished_pages(
            "r30 exhaustion reused-page return rejected",
            memory,
            failure,
        ),
    };
    engine
        .retire(request)
        .map_err(|error| format!("cannot retire r30 exhaustion request: {error:?}"))?;
    let engine_reclaimed = engine
        .reclaim_one()
        .map_err(|error| format!("cannot reclaim r30 exhaustion request: {error:?}"))?
        .ok_or_else(|| "r30 exhaustion request did not become reclaimable".to_owned())?;
    m1_r30_exhaustion_partial_capture::manifest(
        m1_r30_exhaustion_partial_capture::CaptureInputsV1 {
            checked_leases,
            device,
            draft_arena,
            engine_reclaimed,
            first_generation,
            occupied_page_rejected,
            out_of_range_page_rejected,
            request,
            returned_pages,
            reused_generation,
            reused_returned_pages,
            runner,
            target_arena,
        },
    )
}

fn cleanup_unpublished_or_abort(
    phase: &'static str,
    memory: &mut ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    leases: Vec<ferric_engine::DeviceKvPageLease>,
) {
    match memory.return_unpublished_pages(leases) {
        Ok(_) => {}
        Err(failure) => {
            let failure = core::mem::ManuallyDrop::new(failure);
            let _ = &failure;
            let _ = writeln!(
                std::io::stderr().lock(),
                "FAIL-STOP: {phase}; unpublished page custody retained"
            );
            std::process::abort();
        }
    }
}

fn quarantine_unpublished_pages(
    phase: &'static str,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    failure: ferric_engine::M1UnpublishedKvPageReturnFailureV1,
) -> ! {
    let custody = core::mem::ManuallyDrop::new((memory, failure));
    let _ = &custody;
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP: {phase}; physical memory and unpublished page custody retained"
    );
    std::process::abort();
}

fn run_r30_rollback_capture(arguments: &[OsString]) -> CaptureResult<()> {
    let [source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture capture-r30-rollback MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let closure = load_closure(Path::new(closure_path))?;
    let _environment = load_environment(Path::new(environment_path), gpu_unique_id)?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
    let ready = execute_r30_rollback_capture(&runner, memory)?;
    let capture_sha256 = sha256_hex(ready.capture.bytes());
    m1_r30_rollback_partial_capture::publish(Path::new(output), ready.capture)?;
    println!("output={}", Path::new(output).display());
    println!("capture_sha256={capture_sha256}");
    println!("status=partial-non-evidence");
    Ok(())
}

fn execute_r30_rollback_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
) -> CaptureResult<R30RollbackCaptureReadyV1> {
    let target = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Speculative,
        bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
    };
    let draft_speculative = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Speculative,
        bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
    };
    let draft_decode = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: Qwen3PlanBucket::DecodeS1C8192,
    };
    let mut engine = Engine::<1>::new(512, 256, 8_192)
        .map_err(|error| format!("cannot construct one-lane r30 rollback engine: {error:?}"))?;
    let request = engine
        .admit()
        .map_err(|error| format!("cannot admit r30 rollback request: {error:?}"))?;
    engine
        .append_tentative(request, 5)
        .map_err(|error| format!("cannot append exact r30 rollback K4 round: {error:?}"))?;
    let scheduled = engine
        .dispatch_m1_ready()
        .map_err(|error| format!("cannot dispatch r30 rollback request: {error:?}"))?
        .ok_or_else(|| "r30 rollback request was not scheduler-ready".to_owned())?;
    let target_plan = runner
        .logical_runner()
        .bind_step_plan(request, scheduled.epoch(), target)
        .map_err(|error| format!("cannot bind rollback target speculative plan: {error:?}"))?;
    let draft_plan = runner
        .logical_runner()
        .bind_step_plan(request, scheduled.epoch(), draft_decode)
        .map_err(|error| format!("cannot bind rollback draft decode plan: {error:?}"))?;
    let draft_inputs = speculative_s1_k4_validated_inputs(draft_plan, vec![1], vec![0], 1)?;
    let target_inputs =
        speculative_s1_k4_validated_inputs(target_plan, vec![1, 0, 0, 0, 0], (0..5).collect(), 5)?;

    let mut cache =
        ActiveDeviceKvCache::new(memory.device(), request, target, draft_speculative)
            .map_err(|error| format!("cannot construct rollback paired KV cache: {error:?}"))?;
    let draft_lease = memory
        .lease_page(request, Qwen3ModelRole::Draft06B, 0)
        .map_err(|error| format!("cannot lease rollback draft KV page: {error:?}"))?;
    let draft_pending = cache
        .reserve_speculative_draft_round_write(
            request,
            target,
            draft_decode,
            0,
            scheduled.epoch(),
            vec![draft_lease],
        )
        .map_err(|failure| {
            format!(
                "cannot reserve rollback draft KV write: {:?}",
                failure.error()
            )
        })?;
    let target_lease = memory
        .lease_page(request, Qwen3ModelRole::Target8B, 0)
        .map_err(|error| format!("cannot lease rollback target KV page: {error:?}"))?;
    let target_pending = cache
        .reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            5,
            scheduled.epoch(),
            vec![target_lease],
        )
        .map_err(|failure| {
            format!(
                "cannot reserve rollback target KV write: {:?}",
                failure.error()
            )
        })?;
    let pre_completion = cache.projection();
    let draft_table = bind_m1_speculative_draft_kv_round_workspace_table_v1(
        target,
        draft_inputs,
        vec![draft_pending],
    )
    .map_err(|failure| format!("cannot bind rollback draft KV table: {:?}", failure.error()))?;
    let target_table =
        bind_m1_kv_workspace_table_v1(target_inputs, vec![target_pending]).map_err(|failure| {
            format!(
                "cannot bind rollback target KV table: {:?}",
                failure.error()
            )
        })?;
    let tables = M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
        draft_decode: draft_table,
        target_speculative: target_table,
    };
    let draft_workspace_identity =
        *domain_identity(b"ferric.m1.r30.rollback.draft-workspace.v1", &[]).as_bytes();
    let target_workspace_identity =
        *domain_identity(b"ferric.m1.r30.rollback.target-workspace.v1", &[]).as_bytes();
    let plans = M1FullStepWorkspacePlans::speculative_round(
        workload_workspace_plan(draft_decode, draft_workspace_identity)?,
        workload_workspace_plan(target, target_workspace_identity)?,
    );
    let recipe = match runner.derive_step_recipe(
        M1StepDispatchIntent::SpeculativeRound(target),
        M1FullStepWorkspacePlans::speculative_round(
            workload_workspace_plan(draft_decode, draft_workspace_identity)?,
            workload_workspace_plan(target, target_workspace_identity)?,
        ),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
            return Err(format!(
                "cannot derive exact rollback physical recipe: {failure:?}"
            ))
        }
    };
    let prepared = runner
        .prepare_scheduled_workspaces(scheduled, plans, tables)
        .map_err(|failure| format!("cannot prepare rollback workspaces: {failure:?}"))?;
    let mut allocated = match runner.allocate_scheduled_workspaces(memory, prepared) {
        Ok(allocated) => allocated,
        Err(failure) => abandon_single_speculative_prepublication(
            "r30 rollback workspace allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::Workspace {
                _failure: failure,
                _recipe: recipe,
            },
        ),
    };
    let completion = match allocated.allocate_completion_output(target) {
        Ok(completion) => completion,
        Err(diagnostic) => abandon_single_speculative_prepublication(
            "r30 rollback compact output allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::CompletionOutput {
                _allocated: Box::new(allocated),
                _diagnostic: diagnostic,
                _recipe: recipe,
            },
        ),
    };
    let completion = match allocated.enable_speculative_k4_diagnostic_choices_capture(completion) {
        Ok(completion) => completion,
        Err(failure) => abandon_single_speculative_prepublication(
            "r30 rollback diagnostic choice allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::DiagnosticChoices {
                _allocated: Box::new(allocated),
                _failure: failure,
                _recipe: recipe,
            },
        ),
    };
    let published =
        match publish_first_step_with_retries(runner, &mut engine, allocated, recipe, completion) {
            Ok(published) => published,
            Err(failure) => terminal_quarantine(
                "r30 rollback first publication retry exhaustion",
                R30RollbackCacheCustodyV1 {
                    _cache: cache,
                    _custody: failure.quarantine_after_retry_exhaustion(&mut engine),
                },
            ),
        };
    let roster =
        M1DeviceKvCompletionRosterV1::new(vec![M1DeviceKvCompletionMemberV1::continuing(cache)]);
    let completed = match published.wait() {
        Ok(completed) => completed,
        Err(failure) => {
            report_physical_queue_failure("r30 rollback physical dispatch wait failure", &failure);
            terminal_quarantine(
                "r30 rollback physical dispatch wait failure",
                CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: failure.quarantine_engine(&mut engine),
                },
            )
        }
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => {
            report_physical_queue_failure("r30 rollback physical queue recycle failure", &failure);
            terminal_quarantine(
                "r30 rollback physical queue recycle failure",
                CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: failure.quarantine_engine(&mut engine),
                },
            )
        }
    };
    let observed = match recycled.observe_completion() {
        Ok(observed) => observed,
        Err(failure) => match failure.retry() {
            Ok(observed) => observed,
            Err(failure) => close_or_quarantine_roster(
                "r30 rollback compact observation rejected after bounded retry",
                roster,
                (*failure).destroy_queue_and_retain_evidence(&mut engine),
            ),
        },
    };
    let diagnostic = match observed.observe_speculative_k4_diagnostic_choices() {
        Ok(diagnostic) => diagnostic,
        Err(failure) => close_or_quarantine_roster(
            "r30 rollback diagnostic choice observation rejected",
            roster,
            (*failure).destroy_queue_and_retain_evidence(&mut engine),
        ),
    };
    let joined = match diagnostic.check_completion() {
        Ok(joined) => joined,
        Err(failure) => match (*failure).retry() {
            Ok(joined) => joined,
            Err(failure) => close_or_quarantine_roster(
                "r30 rollback maximal-prefix semantic join rejected after bounded retry",
                roster,
                (*failure).destroy_queue_and_retain_evidence(&mut engine),
            ),
        },
    };
    if !joined.target_token_matches() {
        close_or_quarantine_roster_with_diagnostic(
            "r30 rollback corresponding target-token equality rejection",
            "rollback speculative token differs from corresponding target choice".to_owned(),
            roster,
            joined.destroy_queue_and_retain_evidence(&mut engine),
        );
    }
    let (physical, choices) = joined.into_parts();
    let completed = match complete_m1_physical_step_v1(&mut engine, physical, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        M1CompletedStepOutcomeV1::Rejected(rejected) => {
            let diagnostic = format!(
                "r30 rollback exact Engine completion rejected: {:?}",
                rejected.error()
            );
            close_or_quarantine_r30_rollback_choices(
                "r30 rollback Engine completion rejected",
                diagnostic,
                choices,
                rejected.destroy_queue_and_retain_rejection(&mut engine),
            )
        }
        M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
            "r30 rollback Engine completion entered terminal poison",
            R30RollbackChoiceCustodyV1 {
                _choices: choices,
                _custody: poison,
            },
        ),
    };
    let [M1CompletedDeviceKvMemberV1::Active(active)] = completed.members() else {
        close_or_quarantine_r30_rollback_choices(
            "r30 rollback completed cache roster rejected",
            "rollback completion did not retain exactly one active cache".to_owned(),
            choices,
            completed.destroy_queue_and_retain_completion(&mut engine),
        );
    };
    let post_completion_pre_release = active.projection();
    let queue = match m1_r30_rollback_partial_capture::capture_queue_bindings(&completed, runner) {
        Ok(queue) => queue,
        Err(diagnostic) => close_or_quarantine_r30_rollback_choices(
            "r30 rollback queue binding capture rejected",
            diagnostic,
            choices,
            completed.destroy_queue_and_retain_completion(&mut engine),
        ),
    };
    let released = match release_m1_completed_step_kv_pages_v1(completed) {
        Ok(released) => released,
        Err(failure) => {
            let (error, completed) = (*failure).into_parts();
            close_or_quarantine_r30_rollback_choices(
                "r30 rollback single KV page release attempt rejected",
                error,
                choices,
                completed.destroy_queue_and_retain_completion(&mut engine),
            )
        }
    };
    let closed = match released.destroy_queue_and_retain_step(&mut engine) {
        Ok(closed) => closed,
        Err(quarantine) => terminal_quarantine(
            "r30 rollback single queue destruction failed",
            R30RollbackChoiceCustodyV1 {
                _choices: choices,
                _custody: quarantine,
            },
        ),
    };
    let capture = match m1_r30_rollback_partial_capture::manifest(
        m1_r30_rollback_partial_capture::ClosedCaptureInputsV1 {
            choices: &choices,
            closed: &closed,
            post_completion_pre_release,
            pre_completion,
            queue,
        },
    ) {
        Ok(capture) => capture,
        Err(diagnostic) => closed_teardown(
            "r30 rollback closed manifest construction rejected",
            DiagnosticCustodyV1 {
                _diagnostic: diagnostic,
                _custody: R30RollbackChoiceCustodyV1 {
                    _choices: choices,
                    _custody: closed,
                },
            },
        ),
    };
    Ok(R30RollbackCaptureReadyV1 {
        capture,
        _choices: choices,
        _closed: closed,
    })
}

fn run_r32_speculative_capture(arguments: &[OsString]) -> CaptureResult<()> {
    let [source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture capture-r32-speculative-k4 MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let closure = load_closure(Path::new(closure_path))?;
    let _environment = load_environment(Path::new(environment_path), gpu_unique_id)?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
    let ready = execute_r32_speculative_capture(&runner, memory)?;
    let capture_sha256 = sha256_hex(ready.capture.bytes());
    m1_r32_partial_capture::publish(Path::new(output), ready.capture)?;
    println!("output={}", Path::new(output).display());
    println!("capture_sha256={capture_sha256}");
    println!("status=partial-non-evidence");
    Ok(())
}

fn execute_r32_speculative_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
) -> CaptureResult<R32CaptureReadyV1> {
    let target = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Speculative,
        bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
    };
    let draft_speculative = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Speculative,
        bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
    };
    let draft_decode = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: Qwen3PlanBucket::DecodeS1C8192,
    };
    let mut engine = Engine::<1>::new(512, 256, 8_192)
        .map_err(|error| format!("cannot construct one-lane r32 engine: {error:?}"))?;
    let request = engine
        .admit()
        .map_err(|error| format!("cannot admit r32 request: {error:?}"))?;
    engine
        .append_tentative(request, 5)
        .map_err(|error| format!("cannot append exact r32 K4 round: {error:?}"))?;
    let scheduled = engine
        .dispatch_m1_ready()
        .map_err(|error| format!("cannot dispatch r32 request: {error:?}"))?
        .ok_or_else(|| "r32 request was not scheduler-ready".to_owned())?;
    let target_plan = runner
        .logical_runner()
        .bind_step_plan(request, scheduled.epoch(), target)
        .map_err(|error| format!("cannot bind target speculative plan: {error:?}"))?;
    let draft_plan = runner
        .logical_runner()
        .bind_step_plan(request, scheduled.epoch(), draft_decode)
        .map_err(|error| format!("cannot bind draft decode plan: {error:?}"))?;
    let draft_inputs = speculative_s1_k4_validated_inputs(draft_plan, vec![1], vec![0], 1)?;
    let target_inputs =
        speculative_s1_k4_validated_inputs(target_plan, vec![1, 0, 0, 0, 0], (0..5).collect(), 5)?;

    let mut cache = ActiveDeviceKvCache::new(memory.device(), request, target, draft_speculative)
        .map_err(|error| format!("cannot construct r32 paired KV cache: {error:?}"))?;
    let draft_lease = memory
        .lease_page(request, Qwen3ModelRole::Draft06B, 0)
        .map_err(|error| format!("cannot lease r32 draft KV page: {error:?}"))?;
    let draft_pending = cache
        .reserve_speculative_draft_round_write(
            request,
            target,
            draft_decode,
            0,
            scheduled.epoch(),
            vec![draft_lease],
        )
        .map_err(|failure| format!("cannot reserve r32 draft KV write: {:?}", failure.error()))?;
    let target_lease = memory
        .lease_page(request, Qwen3ModelRole::Target8B, 0)
        .map_err(|error| format!("cannot lease r32 target KV page: {error:?}"))?;
    let target_pending = cache
        .reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            5,
            scheduled.epoch(),
            vec![target_lease],
        )
        .map_err(|failure| format!("cannot reserve r32 target KV write: {:?}", failure.error()))?;
    let draft_table = bind_m1_speculative_draft_kv_round_workspace_table_v1(
        target,
        draft_inputs,
        vec![draft_pending],
    )
    .map_err(|failure| format!("cannot bind r32 draft KV table: {:?}", failure.error()))?;
    let target_table = bind_m1_kv_workspace_table_v1(target_inputs, vec![target_pending])
        .map_err(|failure| format!("cannot bind r32 target KV table: {:?}", failure.error()))?;
    let tables = M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
        draft_decode: draft_table,
        target_speculative: target_table,
    };
    let draft_workspace_identity =
        *domain_identity(b"ferric.m1.r32.draft-workspace.v1", &[]).as_bytes();
    let target_workspace_identity =
        *domain_identity(b"ferric.m1.r32.target-workspace.v1", &[]).as_bytes();
    let plans = M1FullStepWorkspacePlans::speculative_round(
        workload_workspace_plan(draft_decode, draft_workspace_identity)?,
        workload_workspace_plan(target, target_workspace_identity)?,
    );
    let recipe = match runner.derive_step_recipe(
        M1StepDispatchIntent::SpeculativeRound(target),
        M1FullStepWorkspacePlans::speculative_round(
            workload_workspace_plan(draft_decode, draft_workspace_identity)?,
            workload_workspace_plan(target, target_workspace_identity)?,
        ),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
            return Err(format!(
                "cannot derive exact r32 physical recipe: {failure:?}"
            ))
        }
    };
    let prepared = runner
        .prepare_scheduled_workspaces(scheduled, plans, tables)
        .map_err(|failure| format!("cannot prepare r32 workspaces: {failure:?}"))?;
    let mut allocated = match runner.allocate_scheduled_workspaces(memory, prepared) {
        Ok(allocated) => allocated,
        Err(failure) => abandon_single_speculative_prepublication(
            "r32 workspace allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::Workspace {
                _failure: failure,
                _recipe: recipe,
            },
        ),
    };
    let completion = match allocated.allocate_completion_output(target) {
        Ok(completion) => completion,
        Err(diagnostic) => abandon_single_speculative_prepublication(
            "r32 compact output allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::CompletionOutput {
                _allocated: Box::new(allocated),
                _diagnostic: diagnostic,
                _recipe: recipe,
            },
        ),
    };
    let completion = match allocated.enable_speculative_k4_diagnostic_choices_capture(completion) {
        Ok(completion) => completion,
        Err(failure) => abandon_single_speculative_prepublication(
            "r32 diagnostic choice allocation failure",
            engine,
            cache,
            SingleSpeculativePrepublicationFailureV1::DiagnosticChoices {
                _allocated: Box::new(allocated),
                _failure: failure,
                _recipe: recipe,
            },
        ),
    };
    let published =
        match publish_first_step_with_retries(runner, &mut engine, allocated, recipe, completion) {
            Ok(published) => published,
            Err(failure) => terminal_quarantine(
                "r32 first publication retry exhaustion",
                R32CacheCustodyV1 {
                    _cache: cache,
                    _custody: failure.quarantine_after_retry_exhaustion(&mut engine),
                },
            ),
        };
    let roster =
        M1DeviceKvCompletionRosterV1::new(vec![M1DeviceKvCompletionMemberV1::continuing(cache)]);
    let completed = match published.wait() {
        Ok(completed) => completed,
        Err(failure) => {
            report_physical_queue_failure("r32 physical dispatch wait failure", &failure);
            terminal_quarantine(
                "r32 physical dispatch wait failure",
                CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: failure.quarantine_engine(&mut engine),
                },
            )
        }
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => {
            report_physical_queue_failure("r32 physical queue recycle failure", &failure);
            terminal_quarantine(
                "r32 physical queue recycle failure",
                CompletionRosterCustodyV1 {
                    _roster: roster,
                    _custody: failure.quarantine_engine(&mut engine),
                },
            )
        }
    };
    let observed = match recycled.observe_completion() {
        Ok(observed) => observed,
        Err(failure) => match failure.retry() {
            Ok(observed) => observed,
            Err(failure) => close_or_quarantine_roster(
                "r32 compact observation rejected after bounded retry",
                roster,
                (*failure).destroy_queue_and_retain_evidence(&mut engine),
            ),
        },
    };
    let diagnostic = match observed.observe_speculative_k4_diagnostic_choices() {
        Ok(diagnostic) => diagnostic,
        Err(failure) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "FAIL-STOP DETAIL: r32 diagnostic choice observation rejected: {:?}; copied_choice_ranges={}",
                failure.error(),
                failure.copied_choice_ranges(),
            );
            close_or_quarantine_roster(
                "r32 diagnostic choice observation rejected",
                roster,
                (*failure).destroy_queue_and_retain_evidence(&mut engine),
            )
        }
    };
    let joined = match diagnostic.check_completion() {
        Ok(joined) => joined,
        Err(failure) => match (*failure).retry() {
            Ok(joined) => joined,
            Err(failure) => close_or_quarantine_roster(
                "r32 maximal-prefix semantic join rejected after bounded retry",
                roster,
                (*failure).destroy_queue_and_retain_evidence(&mut engine),
            ),
        },
    };
    if !joined.target_token_matches() {
        close_or_quarantine_roster_with_diagnostic(
            "r32 corresponding target-token equality rejection",
            "r32 speculative token differs from corresponding target choice".to_owned(),
            roster,
            joined.destroy_queue_and_retain_evidence(&mut engine),
        );
    }
    let (completed, choices) = joined.into_parts();
    let completed = match complete_m1_physical_step_v1(&mut engine, completed, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        M1CompletedStepOutcomeV1::Rejected(rejected) => {
            let (_error, completed, roster) = rejected.into_parts();
            match complete_m1_physical_step_v1(&mut engine, completed, roster) {
                M1CompletedStepOutcomeV1::Completed(completed) => completed,
                M1CompletedStepOutcomeV1::Rejected(rejected) => close_or_quarantine_r32_choices(
                    "r32 Engine completion rejected after bounded retry",
                    "r32 exact Engine completion rejected twice".to_owned(),
                    choices,
                    rejected.destroy_queue_and_retain_rejection(&mut engine),
                ),
                M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
                    "r32 Engine completion retry entered terminal poison",
                    R32ChoiceCustodyV1 {
                        _choices: choices,
                        _custody: poison,
                    },
                ),
            }
        }
        M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
            "r32 Engine completion entered terminal poison",
            R32ChoiceCustodyV1 {
                _choices: choices,
                _custody: poison,
            },
        ),
    };
    let released = match release_first_completed_step(&mut engine, completed) {
        Ok(released) => released,
        Err(teardown) => {
            let FirstPageReleaseTeardownV1 { error, teardown } = *teardown;
            close_or_quarantine_r32_choices(
                "r32 KV page release rejected after bounded retry",
                error,
                choices,
                teardown,
            )
        }
    };
    let capture =
        match m1_r32_partial_capture::manifest(m1_r32_partial_capture::SettledCaptureInputsV1 {
            choices: &choices,
            released: &released,
            runner,
        }) {
            Ok(capture) => capture,
            Err(diagnostic) => close_or_quarantine_r32_choices(
                "r32 settled manifest construction rejected",
                diagnostic,
                choices,
                released.destroy_queue_and_retain_step(&mut engine),
            ),
        };
    let closed = match released.destroy_queue_and_retain_step(&mut engine) {
        Ok(closed) => closed,
        Err(quarantine) => terminal_quarantine(
            "r32 settled queue destruction failure",
            R32CaptureCustodyV1 {
                _capture: capture,
                _choices: choices,
                _custody: quarantine,
            },
        ),
    };
    Ok(R32CaptureReadyV1 {
        capture,
        _choices: choices,
        _closed: closed,
    })
}

fn speculative_s1_k4_validated_inputs(
    plan: StepPlan,
    tokens: Vec<u32>,
    positions: Vec<u32>,
    active_length: u32,
) -> CaptureResult<ValidatedM1StepInputs> {
    let candidate = M1StepInputCandidate::new(
        plan.selection(),
        vec![Some(plan)],
        tokens,
        positions,
        vec![active_length],
        vec![0],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(failure) => {
            Err(format!("r32 S1/K4 inputs rejected: {:?}", failure.error()))
        }
    }
}

fn run_capture(arguments: &[OsString]) -> CaptureResult<()> {
    run_capture_with_purpose(arguments, CapturePurposeV1::Qualification)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CapturePurposeV1 {
    Qualification,
    R30PartialCanary,
    R30PartialCancellation,
}

fn run_capture_with_purpose(
    arguments: &[OsString],
    purpose: CapturePurposeV1,
) -> CaptureResult<()> {
    let [plan_path, roster_path, case_id, workload_path, source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments
    else {
        return Err(match purpose {
            CapturePurposeV1::Qualification => "usage: ferric-m1-qualification-capture PLAN ROSTER CASE-ID WORKLOAD MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned(),
            CapturePurposeV1::R30PartialCanary => "capture-r30-canary uses its independent seven-argument path".to_owned(),
            CapturePurposeV1::R30PartialCancellation => "capture-r30-cancellation uses its independent seven-argument path".to_owned(),
        });
    };
    let case_id = case_id
        .to_str()
        .ok_or_else(|| "case ID must be UTF-8".to_owned())?;
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;

    let plan = load_plan(Path::new(plan_path))?;
    let case = plan.case(case_id)?.clone();
    load_roster(Path::new(roster_path), &plan)?;
    let workload = load_workload(Path::new(workload_path), &case)?;
    if purpose == CapturePurposeV1::R30PartialCancellation
        && (workload.selection.role != Qwen3ModelRole::Target8B
            || workload.selection.mode != Qwen3ExecutionMode::Prefill)
    {
        return Err(
            "partial r30 cancellation capture requires an exact target-prefill workload".to_owned(),
        );
    }
    let input_tokens = load_input_tokens(Path::new(workload_path), &workload, &case)?;
    let closure = load_closure(Path::new(closure_path))?;
    let environment_bytes = load_environment(Path::new(environment_path), gpu_unique_id)?;
    require_identity(
        plan.identity("environment")?,
        &sha256_hex(&environment_bytes),
        "environment",
    )?;
    let executable_sha256 = current_executable_sha256()?;
    require_identity(
        plan.identity("benchmark-executable")?,
        &executable_sha256,
        "benchmark executable",
    )?;

    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();

    let source = SecureDirectory::open(Path::new(source_root), "model source root")?;
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&source, &snapshot)?;
    let runner_admission = model.authenticate()?;
    let deployment = *runner_admission.prepacked().deployment();
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    validate_plan_identities(&plan, &case, &closure, &declaration, &deployment, &model)?;
    require_supported_capture(&workload)?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;

    let capture = execute_capture(
        &runner,
        memory,
        &workload,
        input_tokens,
        gpu_unique_id,
        purpose,
    )?;
    let runner_declaration = runner.declaration_id();
    let kernel_manifest = runner.kernel_artifact_manifest_id();
    let identities = CaptureIdentities {
        gpu_unique_id,
        runner_declaration,
        kernel_manifest,
        program_catalog: executable_catalog_id,
    };
    let transcript = capture_transcript(&plan, &case, &workload, &capture, identities)?;
    let transcript_sha256 = sha256_hex(&transcript);
    let output_manifest = differential_output_manifest(
        &plan,
        &case,
        &capture.logits,
        &capture.tokens,
        &transcript_sha256,
    )?;

    let mut staging = StagingOutput::create(Path::new(output))?;
    staging.write("logits.bf16le", &capture.logits)?;
    staging.write("tokens.u32le", &capture.tokens)?;
    staging.write("runner.json", &transcript)?;
    staging.write("output.json", &output_manifest)?;
    staging.publish()?;
    println!("output={}", Path::new(output).display());
    println!("case_id={}", case.id);
    println!("logits_sha256={}", sha256_hex(&capture.logits));
    println!("tokens_sha256={}", sha256_hex(&capture.tokens));
    println!("runner_transcript_sha256={transcript_sha256}");
    Ok(())
}

#[derive(Debug)]
struct CapturedOutput {
    compact_sha256: [u8; 32],
    device_id: Identity,
    execution: CapturedExecutionV1,
    logits: Vec<u8>,
    logits_row_sha256: Vec<[u8; 32]>,
    r30_canary_closed: Option<ferric_engine::M1ReleasedQueueTeardownSuccessV1>,
    settlement: Option<m1_r30_partial_capture::CancellationSettlementV1>,
    tokens: Vec<u8>,
}

#[derive(Debug)]
enum CapturedExecutionV1 {
    OneShotPrefill {
        dispatch_generation: u64,
        epoch: u64,
    },
    C8192 {
        execution_binding: M1QualificationExecutionBindingDeclaration,
        first_dispatch_generation: u64,
        first_epoch: u64,
        qualification_plan_id: Identity,
        round_count: u32,
        round_history_sha256: [u8; 32],
        terminal_dispatch_generation: u64,
        terminal_epoch: u64,
    },
}

#[derive(Clone, Copy, Debug)]
struct CaptureIdentities {
    gpu_unique_id: u64,
    runner_declaration: Identity,
    kernel_manifest: Identity,
    program_catalog: Identity,
}

#[derive(Debug)]
struct QualificationExecutionBindingV1 {
    declaration: M1QualificationExecutionBindingDeclaration,
    grouping: M1QualificationLaneGrouping,
}

#[derive(Debug)]
struct QualificationRoundCommitmentV1 {
    hasher: Sha256,
    count: u32,
    selection: Qwen3PlanSelection,
    first_epoch: u64,
    first_generation: u64,
    terminal_epoch: u64,
    terminal_generation: u64,
}

struct QualificationRoundCaptureStateV1 {
    requests: Vec<RequestId>,
    history: QualificationRoundCommitmentV1,
}

type QualificationRoundCommitmentOutputV1 = ([u8; 32], u32, u64, u64, u64, u64);

struct QualificationRoundCommitmentFailureV1 {
    diagnostic: String,
    history: QualificationRoundCommitmentV1,
}

struct QualificationRoundReceiptV1<'a> {
    logical_accepted: &'a [u32],
    externally_published: &'a [u32],
    release_counts: &'a [ferric_engine::M1CompletedKvPageReleaseCountsV1],
    device_id: Identity,
}

impl QualificationRoundCommitmentV1 {
    fn new(
        plan_id: Identity,
        declaration: &M1QualificationExecutionBindingDeclaration,
        selection: Qwen3PlanSelection,
    ) -> Self {
        let mut hasher = Sha256::new();
        hash_field(&mut hasher, b"ferric.m1.qualification-round-history.v1");
        hash_field(&mut hasher, plan_id.as_bytes());
        hash_field(&mut hasher, declaration.declared_workload_digest.as_bytes());
        hash_field(&mut hasher, &selection_bytes(selection));
        for lane in &declaration.ordered_lanes {
            hash_field(&mut hasher, &lane.lane_ordinal.to_le_bytes());
            hash_field(&mut hasher, lane.lane_identity.as_bytes());
            hash_field(&mut hasher, lane.token_sequence_identity.as_bytes());
        }
        Self {
            hasher,
            count: 0,
            selection,
            first_epoch: 0,
            first_generation: 0,
            terminal_epoch: 0,
            terminal_generation: 0,
        }
    }

    fn observe(
        &mut self,
        ordinal: u32,
        checked: &ferric_engine::M1CheckedCompletionOutputV1,
        expected_requests: &[RequestId],
        receipt: QualificationRoundReceiptV1<'_>,
    ) -> CaptureResult<()> {
        let terminal = ordinal == M1_QUALIFICATION_FINAL_INPUT_TOKEN;
        if ordinal != self.count
            || checked.selection() != self.selection
            || checked.records().len() != expected_requests.len()
            || receipt.logical_accepted.len() != expected_requests.len()
            || receipt.externally_published.len() != expected_requests.len()
            || receipt.release_counts.len() != expected_requests.len()
            || receipt.logical_accepted.iter().any(|count| *count != 1)
            || receipt
                .externally_published
                .iter()
                .any(|count| *count != u32::from(terminal))
        {
            return Err(format!(
                "qualification round {ordinal} history cardinality or order drifted"
            ));
        }
        let epoch = checked.epoch().value();
        let generation = checked.dispatch_generation();
        for (lane, (record, expected)) in
            checked.records().iter().zip(expected_requests).enumerate()
        {
            let record = record.record();
            if record.request != *expected || record.epoch.value() != epoch {
                return Err(format!(
                    "qualification round {ordinal} lane {lane} checked request order drifted"
                ));
            }
        }
        let next_count = self
            .count
            .checked_add(1)
            .ok_or_else(|| "qualification round count overflowed".to_owned())?;
        if self.count == 0 {
            self.first_epoch = epoch;
            self.first_generation = generation;
        }
        self.terminal_epoch = epoch;
        self.terminal_generation = generation;
        hash_field(&mut self.hasher, &ordinal.to_le_bytes());
        hash_field(&mut self.hasher, &epoch.to_le_bytes());
        hash_field(&mut self.hasher, &generation.to_le_bytes());
        hash_field(&mut self.hasher, receipt.device_id.as_bytes());
        for (((record, logical), external), released) in checked
            .records()
            .iter()
            .zip(receipt.logical_accepted)
            .zip(receipt.externally_published)
            .zip(receipt.release_counts)
        {
            let record = record.record();
            hash_field(&mut self.hasher, &record.request.slot().to_le_bytes());
            hash_field(&mut self.hasher, &record.request.generation().to_le_bytes());
            hash_field(&mut self.hasher, record.plan_id.as_bytes());
            hash_field(&mut self.hasher, &[record.accepted_draft_tokens]);
            hash_field(&mut self.hasher, &[record.emitted_token_count]);
            for token in record
                .emitted_tokens
                .iter()
                .take(usize::from(record.emitted_token_count))
            {
                hash_field(&mut self.hasher, &token.to_le_bytes());
            }
            hash_field(&mut self.hasher, &logical.to_le_bytes());
            hash_field(&mut self.hasher, &external.to_le_bytes());
            hash_field(
                &mut self.hasher,
                &u64::try_from(released.draft())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            hash_field(
                &mut self.hasher,
                &u64::try_from(released.target())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
        }
        self.count = next_count;
        Ok(())
    }

    fn finish(
        self,
    ) -> Result<QualificationRoundCommitmentOutputV1, Box<QualificationRoundCommitmentFailureV1>>
    {
        if usize::try_from(self.count).ok() != Some(M1_QUALIFICATION_CONTEXT_PLAN_STEPS) {
            return Err(Box::new(QualificationRoundCommitmentFailureV1 {
                diagnostic: format!(
                    "qualification completed {} rounds instead of {}",
                    self.count, M1_QUALIFICATION_CONTEXT_PLAN_STEPS
                ),
                history: self,
            }));
        }
        Ok((
            self.hasher.finalize().into(),
            self.count,
            self.first_epoch,
            self.first_generation,
            self.terminal_epoch,
            self.terminal_generation,
        ))
    }
}

fn qualification_grouping(
    selection: Qwen3PlanSelection,
) -> CaptureResult<M1QualificationLaneGrouping> {
    if selection.role != Qwen3ModelRole::Target8B || selection.mode != Qwen3ExecutionMode::Decode {
        return Err("qualification capture accepts target-only decode selections".to_owned());
    }
    match selection.bucket {
        Qwen3PlanBucket::DecodeS1C8192 => Ok(M1QualificationLaneGrouping::S1),
        Qwen3PlanBucket::DecodeS8C8192 => Ok(M1QualificationLaneGrouping::S8),
        Qwen3PlanBucket::DecodeS32C8192 => Ok(M1QualificationLaneGrouping::S32),
        _ => Err("qualification capture accepts exactly DecodeS1/S8/S32C8192".to_owned()),
    }
}

fn qualification_engine_page_capacity(grouping: M1QualificationLaneGrouping) -> CaptureResult<u32> {
    let pages_per_lane = M1_QUALIFICATION_TOKENS_PER_LANE
        .checked_add(QUALIFICATION_LOGICAL_KV_PAGE_TOKENS - 1)
        .ok_or_else(|| "qualification logical page count overflowed".to_owned())?
        / QUALIFICATION_LOGICAL_KV_PAGE_TOKENS;
    grouping
        .sequences()
        .checked_mul(pages_per_lane)
        .ok_or_else(|| "qualification logical page capacity overflowed".to_owned())
}

fn qualification_execution_binding(
    workload: &Workload,
    input_tokens: &[u32],
) -> CaptureResult<QualificationExecutionBindingV1> {
    let grouping = qualification_grouping(workload.selection)?;
    let lanes = usize::try_from(grouping.sequences())
        .map_err(|_| "qualification grouping does not fit usize".to_owned())?;
    let tokens_per_lane = usize::try_from(M1_QUALIFICATION_TOKENS_PER_LANE)
        .map_err(|_| "qualification lane width does not fit usize".to_owned())?;
    let expected = lanes
        .checked_mul(tokens_per_lane)
        .ok_or_else(|| "qualification token extent overflowed".to_owned())?;
    if workload.lanes.len() != lanes || input_tokens.len() != expected {
        return Err("qualification binding token or lane extent drifted".to_owned());
    }
    let selection = selection_bytes(workload.selection);
    let grouping_bytes = grouping.sequences().to_le_bytes();
    let workload_digest = domain_identity(
        b"ferric.m1.qualification-workload-binding.v1",
        &[&grouping_bytes, &selection, &workload.bytes],
    );
    let mut ordered_lanes = Vec::new();
    ordered_lanes
        .try_reserve_exact(lanes)
        .map_err(|_| "cannot reserve qualification lane bindings".to_owned())?;
    for lane in 0..lanes {
        let lane_u32 = u32::try_from(lane)
            .map_err(|_| "qualification lane ordinal does not fit u32".to_owned())?;
        let start = lane
            .checked_mul(tokens_per_lane)
            .ok_or_else(|| "qualification lane token offset overflowed".to_owned())?;
        let end = start
            .checked_add(tokens_per_lane)
            .ok_or_else(|| "qualification lane token extent overflowed".to_owned())?;
        let mut canonical_tokens = Vec::new();
        canonical_tokens
            .try_reserve_exact(tokens_per_lane.saturating_mul(4))
            .map_err(|_| "cannot reserve canonical qualification token bytes".to_owned())?;
        for token in &input_tokens[start..end] {
            canonical_tokens.extend_from_slice(&token.to_le_bytes());
        }
        let token_sequence_identity = domain_identity(
            b"ferric.m1.qualification-token-sequence.v1",
            &[
                &grouping_bytes,
                &selection,
                &lane_u32.to_le_bytes(),
                &canonical_tokens,
            ],
        );
        let lane_identity = domain_identity(
            b"ferric.m1.qualification-lane.v1",
            &[
                workload_digest.as_bytes(),
                &grouping_bytes,
                &selection,
                &lane_u32.to_le_bytes(),
                token_sequence_identity.as_bytes(),
            ],
        );
        ordered_lanes.push(M1QualificationLaneExecutionBinding {
            lane_ordinal: lane_u32,
            lane_identity,
            token_sequence_identity,
        });
    }
    Ok(QualificationExecutionBindingV1 {
        declaration: M1QualificationExecutionBindingDeclaration {
            declared_workload_digest: workload_digest,
            ordered_lanes,
        },
        grouping,
    })
}

fn validate_scheduled_roster(
    scheduled: &ferric_engine::M1ScheduledDispatchV1,
    expected: &[RequestId],
    ordinal: u32,
) -> CaptureResult<()> {
    if scheduled.member_count() != expected.len() {
        return Err(format!(
            "qualification ordinal {ordinal} scheduler member count drifted"
        ));
    }
    for (lane, expected_request) in expected.iter().copied().enumerate() {
        if scheduled.member(lane) != Some(expected_request) {
            return Err(format!(
                "qualification ordinal {ordinal} scheduler lane {lane} request order drifted"
            ));
        }
    }
    Ok(())
}

fn qualification_step_inputs(
    workload: &Workload,
    plans: &[StepPlan],
    input_tokens: &[u32],
    ordinal: u32,
) -> CaptureResult<ValidatedM1StepInputs> {
    let lane_width = usize::try_from(M1_QUALIFICATION_TOKENS_PER_LANE)
        .map_err(|_| "qualification lane width does not fit usize".to_owned())?;
    if ordinal >= M1_QUALIFICATION_TOKENS_PER_LANE || plans.len() != workload.lanes.len() {
        return Err(format!(
            "qualification ordinal {ordinal} or plan roster is outside the admitted context"
        ));
    }
    let ordinal_index = usize::try_from(ordinal)
        .map_err(|_| "qualification ordinal does not fit usize".to_owned())?;
    let mut tokens = Vec::new();
    tokens
        .try_reserve_exact(plans.len())
        .map_err(|_| "cannot reserve qualification step tokens".to_owned())?;
    for lane in 0..plans.len() {
        let index = lane
            .checked_mul(lane_width)
            .and_then(|start| start.checked_add(ordinal_index))
            .ok_or_else(|| "qualification lane-major input offset overflowed".to_owned())?;
        tokens.push(
            *input_tokens
                .get(index)
                .ok_or_else(|| "qualification lane-major input is truncated".to_owned())?,
        );
    }
    if input_tokens.len() != plans.len().saturating_mul(lane_width) {
        return Err("qualification lane-major input has trailing or missing tokens".to_owned());
    }
    let candidate = M1StepInputCandidate::new(
        workload.selection,
        plans.iter().copied().map(Some).collect(),
        tokens,
        vec![ordinal; plans.len()],
        vec![1; plans.len()],
        vec![ordinal; plans.len()],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(rejection) => Err(format!(
            "qualification ordinal {ordinal} inputs were rejected: {:?}",
            rejection.error()
        )),
    }
}

fn qualification_contexts(
    plan: ferric_engine::M1ValidatedQualificationContextPlanV1<'_>,
    lanes: usize,
    ordinal: u32,
) -> CaptureResult<Vec<ferric_engine::M1ValidatedQualificationContextStepV1>> {
    (0..lanes)
        .map(|lane| plan.step(ordinal, u32::try_from(lane).unwrap_or(u32::MAX)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!("cannot derive qualification ordinal {ordinal} context witnesses: {error}")
        })
}

fn bind_qualification_step_plans(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    scheduled: &ferric_engine::M1ScheduledDispatchV1,
    selection: Qwen3PlanSelection,
    requests: &[RequestId],
    ordinal: u32,
) -> CaptureResult<Vec<StepPlan>> {
    validate_scheduled_roster(scheduled, requests, ordinal)?;
    requests
        .iter()
        .copied()
        .map(|request| {
            runner
                .logical_runner()
                .bind_step_plan(request, scheduled.epoch(), selection)
                .map_err(|error| {
                    format!("cannot bind qualification ordinal {ordinal} step plan: {error:?}")
                })
        })
        .collect()
}

fn derive_qualification_recipe(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    selection: Qwen3PlanSelection,
    workspace_identity: [u8; 32],
) -> CaptureResult<ferric_engine::AddresslessM1PhysicalBufferRecipeV1> {
    match runner.derive_step_recipe(
        M1StepDispatchIntent::TargetOnly(selection),
        M1FullStepWorkspacePlans::target_only(workload_workspace_plan(
            selection,
            workspace_identity,
        )?),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => Ok(recipe),
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => Err(format!(
            "cannot derive qualification physical recipe: {error:?}"
        )),
    }
}

fn publish_first_step_with_retries<'runner, const C: usize>(
    runner: &'runner ferric_engine::M1PhysicalRunnerV1,
    engine: &mut Engine<C>,
    allocated: ferric_engine::M1AllocatedScheduledStepV1,
    recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    completion: ferric_engine::BoundM1CompletionOutputV1,
) -> Result<
    ferric_engine::M1PhysicalPublishedQueueSessionV1,
    ferric_engine::M1PhysicalRunnerFirstPublicationFailureV1<'runner>,
> {
    let failure = match runner.publish_first_step(engine, 1 << 20, allocated, recipe, completion) {
        Ok(published) => return Ok(published),
        Err(failure) => failure,
    };
    let outcome = retry_with_bounded_policy(failure, CAPTURE_RECOVERY_RETRIES, |failure| {
        failure.retry(runner, engine, 1 << 20)
    });
    if let Err(failure) = &outcome {
        let _ = writeln!(
            std::io::stderr().lock(),
            "FAIL-STOP DETAIL: first publication did not recover under bounded policy (retry_budget={CAPTURE_RECOVERY_RETRIES}): {:?}",
            failure.diagnostic()
        );
    }
    outcome
}

fn retry_with_bounded_policy<Owner, Success>(
    mut owner: Owner,
    attempts: usize,
    mut retry: impl FnMut(Owner) -> Result<Success, Owner>,
) -> Result<Success, Owner> {
    for _ in 0..attempts {
        owner = match retry(owner) {
            Ok(success) => return Ok(success),
            Err(owner) => owner,
        };
    }
    Err(owner)
}

fn submit_rearm_or_fail_stop(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    engine: &mut Engine<32>,
    prepared: ferric_engine::M1PreparedLongLivedQueueRearmV1,
    recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
    state: QualificationRoundCaptureStateV1,
) -> (
    ferric_engine::M1RearmedPublishedQueueV1,
    QualificationRoundCaptureStateV1,
) {
    let mut failure = match runner.submit_rearm(engine, prepared, recipe) {
        Ok(published) => return (published, state),
        Err(failure) => failure,
    };
    failure = match retry_with_bounded_policy(failure, CAPTURE_RECOVERY_RETRIES, |failure| {
        failure.retry(runner, engine)
    }) {
        Ok(published) => return (published, state),
        Err(failure) => failure,
    };
    terminal_round(
        "qualification rearm submission retry exhaustion",
        state,
        failure.quarantine_after_retry_exhaustion(engine),
    )
}

enum CaptureAllocationFailureV1 {
    Preflight {
        _diagnostic: ferric_engine::InitializedM1FullStepWorkspacePreflightErrorV1,
        _memory: Box<ferric_engine::M1PartitionedModelMemoryKvPoolV1>,
        _prepared: Box<ferric_engine::M1PreparedScheduledWorkspaceImagesV1>,
    },
    Terminal {
        _failure: ferric_engine::M1PrepublicationAllocationFailureV1,
    },
}

enum CapturePreparationFailureV1 {
    Join {
        _diagnostic: ferric_engine::M1PrepublicationJoinErrorV1,
        _scheduled: Box<ferric_engine::M1ScheduledDispatchV1>,
        _plans: M1FullStepWorkspacePlans,
        _tables: Box<M1FullStepKvWorkspaceTablesV1>,
    },
    Composition {
        _failure: ferric_engine::M1PrepublicationCompositionFailureV1,
    },
}

enum FirstGenerationLiveEvidenceV1 {
    Decode {
        _caches: Vec<ActiveDeviceKvCache>,
        _requests: Vec<RequestId>,
        _contexts: Vec<ferric_engine::M1ValidatedQualificationContextStepV1>,
    },
    Prefill {
        _caches: Vec<ActiveDeviceKvCache>,
        _requests: Vec<RequestId>,
        _active_lengths: Vec<u32>,
        _context_lengths: Vec<u32>,
    },
}

struct PrefillLiveEvidenceV1 {
    _caches: Vec<ActiveDeviceKvCache>,
    requests: Vec<RequestId>,
    _active_lengths: Vec<u32>,
    _context_lengths: Vec<u32>,
}

enum FirstGenerationPhaseFailureV1<'runner> {
    Preparation {
        _memory: Box<ferric_engine::M1PartitionedModelMemoryKvPoolV1>,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
        _failure: Box<CapturePreparationFailureV1>,
    },
    Allocation {
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
        _failure: Box<CaptureAllocationFailureV1>,
    },
    CompletionOutput {
        _allocated: Box<ferric_engine::M1AllocatedScheduledStepV1>,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
        _diagnostic: ferric_engine::M1CompletionOutputErrorV1,
    },
    QualificationLogits {
        _allocated: Box<ferric_engine::M1AllocatedScheduledStepV1>,
        _recipe: ferric_engine::AddresslessM1PhysicalBufferRecipeV1,
        _failure: Box<ferric_engine::M1QualificationLogitsAllocationFailureV1>,
    },
    Publication {
        _failure: ferric_engine::M1PhysicalRunnerFirstPublicationExhaustedV1<'runner>,
    },
}

struct FirstGenerationAbandonmentV1<'runner> {
    _engine: ferric_engine::M1CaptureQuarantinedEngineV1<32>,
    _evidence: FirstGenerationLiveEvidenceV1,
    _phase: Box<FirstGenerationPhaseFailureV1<'runner>>,
}

impl CaptureTerminalCustodyV1 for FirstGenerationAbandonmentV1<'_> {}

fn abandon_first_generation(
    phase_name: &'static str,
    engine: Engine<32>,
    evidence: FirstGenerationLiveEvidenceV1,
    phase: FirstGenerationPhaseFailureV1<'_>,
) -> ! {
    terminal_quarantine(
        phase_name,
        FirstGenerationAbandonmentV1 {
            _engine: engine.into_m1_capture_quarantine(),
            _evidence: evidence,
            _phase: Box::new(phase),
        },
    )
}

fn prepare_scheduled_workspaces_with_retries(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    scheduled: ferric_engine::M1ScheduledDispatchV1,
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> Result<ferric_engine::M1PreparedScheduledWorkspaceImagesV1, Box<CapturePreparationFailureV1>> {
    let mut failure = match runner.prepare_scheduled_workspaces(scheduled, plans, tables) {
        Ok(prepared) => return Ok(prepared),
        Err(failure) => failure,
    };
    let mut attempts = 0;
    loop {
        let (diagnostic, scheduled, plans, tables) = match failure {
            ferric_engine::M1PrepareFailureV1::Join(failure) => failure.into_parts(),
            ferric_engine::M1PrepareFailureV1::Composition(failure) => {
                return Err(Box::new(CapturePreparationFailureV1::Composition {
                    _failure: failure,
                }));
            }
        };
        if attempts == CAPTURE_RECOVERY_RETRIES {
            return Err(Box::new(CapturePreparationFailureV1::Join {
                _diagnostic: diagnostic,
                _scheduled: Box::new(scheduled),
                _plans: plans,
                _tables: Box::new(tables),
            }));
        }
        attempts += 1;
        failure = match runner.prepare_scheduled_workspaces(scheduled, plans, tables) {
            Ok(prepared) => return Ok(prepared),
            Err(failure) => failure,
        };
    }
}

fn allocate_scheduled_workspaces_with_retries(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    prepared: ferric_engine::M1PreparedScheduledWorkspaceImagesV1,
) -> Result<ferric_engine::M1AllocatedScheduledStepV1, Box<CaptureAllocationFailureV1>> {
    let mut failure = match runner.allocate_scheduled_workspaces(memory, prepared) {
        Ok(allocated) => return Ok(allocated),
        Err(failure) => failure,
    };
    let mut attempts = 0;
    loop {
        let (diagnostic, memory, prepared) = match failure.into_preflight_prepared() {
            Ok(parts) => parts,
            Err(failure) => {
                return Err(Box::new(CaptureAllocationFailureV1::Terminal {
                    _failure: failure,
                }));
            }
        };
        if attempts == CAPTURE_RECOVERY_RETRIES {
            return Err(Box::new(CaptureAllocationFailureV1::Preflight {
                _diagnostic: diagnostic,
                _memory: Box::new(memory),
                _prepared: Box::new(prepared),
            }));
        }
        attempts += 1;
        failure = match runner.allocate_scheduled_workspaces(memory, prepared) {
            Ok(allocated) => return Ok(allocated),
            Err(failure) => failure,
        };
    }
}

fn schedule_qualification_round(
    released: QualificationReleasedRoundV1,
    engine: &mut Engine<32>,
    state: QualificationRoundCaptureStateV1,
) -> (
    M1ScheduledLongLivedQueueRearmV1,
    QualificationRoundCaptureStateV1,
) {
    match released.schedule(engine) {
        Ok(scheduled) => (scheduled, state),
        Err(failure) => {
            let unscheduled = match failure.into_unscheduled() {
                Ok(unscheduled) => unscheduled,
                Err(failure) => close_terminal_schedule_failure(failure, state),
            };
            match unscheduled.retry(engine) {
                Ok(scheduled) => (scheduled, state),
                Err(failure) => {
                    let unscheduled = match failure.into_unscheduled() {
                        Ok(unscheduled) => unscheduled,
                        Err(failure) => close_terminal_schedule_failure(failure, state),
                    };
                    let teardown = unscheduled.destroy_queue_and_retain_round(engine);
                    close_or_quarantine_round(
                        "qualification scheduling rejected after retry",
                        state,
                        None,
                        teardown,
                    )
                }
            }
        }
    }
}

fn close_terminal_schedule_failure(
    failure: ferric_engine::M1LongLivedQueueRearmScheduleFailureV1,
    state: QualificationRoundCaptureStateV1,
) -> ! {
    match failure.close_terminal() {
        ferric_engine::M1LongLivedQueueRearmScheduleClosureOutcomeV1::Released(failure) => {
            invariant_fail_stop(
                "qualification scheduling closure retained a released retry owner",
                UnexpectedReleasedScheduleFailureV1 {
                    _custody: failure,
                    _requests: state.requests,
                    _history: state.history,
                },
            )
        }
        ferric_engine::M1LongLivedQueueRearmScheduleClosureOutcomeV1::QueueDetach(quarantine) => {
            terminal_round(
                "qualification scheduling detach quarantine",
                state,
                quarantine,
            )
        }
        ferric_engine::M1LongLivedQueueRearmScheduleClosureOutcomeV1::Detached(teardown) => {
            close_or_quarantine_round(
                "qualification scheduling detached teardown",
                state,
                None,
                *teardown,
            )
        }
    }
}

fn release_intermediate_round_or_fail_stop(
    engine: &mut Engine<32>,
    mut completion: ferric_engine::M1RearmedCompletionOutcomeV1,
    mut state: QualificationRoundCaptureStateV1,
) -> (
    M1LongLivedQueueReleasedRoundV1,
    QualificationRoundCaptureStateV1,
) {
    let mut completion_retries = 0;
    'completion: loop {
        match completion.release_completed() {
            M1RearmedRoundReleaseOutcomeV1::Released(released) => return (released, state),
            M1RearmedRoundReleaseOutcomeV1::Rejected(mut failure) => {
                for _ in 0..CAPTURE_RECOVERY_RETRIES {
                    match (*failure).retry() {
                        M1RearmedRoundReleaseOutcomeV1::Released(released) => {
                            return (released, state);
                        }
                        M1RearmedRoundReleaseOutcomeV1::Rejected(next) => failure = next,
                        M1RearmedRoundReleaseOutcomeV1::NotCompleted(next) => {
                            let returned = consume_intermediate_non_rejected(
                                engine,
                                next,
                                state,
                                "qualification intermediate release lost completed state",
                            );
                            completion = returned.0;
                            state = returned.1;
                            continue 'completion;
                        }
                    }
                }
                close_or_quarantine_round(
                    "qualification intermediate page-release retry exhaustion",
                    state,
                    None,
                    failure.destroy_queue_and_retain_round(engine),
                );
            }
            M1RearmedRoundReleaseOutcomeV1::NotCompleted(next) => {
                if completion_retries == CAPTURE_RECOVERY_RETRIES {
                    let returned = match next.destroy_queue_and_retain_rejected(engine) {
                        Ok(teardown) => close_or_quarantine_round(
                            "qualification intermediate completion retry exhaustion",
                            state,
                            None,
                            teardown,
                        ),
                        Err(non_rejected) => consume_intermediate_non_rejected(
                            engine,
                            *non_rejected,
                            state,
                            "qualification intermediate completion retry exhaustion",
                        ),
                    };
                    completion = returned.0;
                    state = returned.1;
                    continue;
                }
                completion_retries += 1;
                let returned = match next.retry_rejected(engine) {
                    Ok(completion) => (completion, state),
                    Err(non_rejected) => consume_intermediate_non_rejected(
                        engine,
                        *non_rejected,
                        state,
                        "qualification intermediate completion retry",
                    ),
                };
                completion = returned.0;
                state = returned.1;
            }
        }
    }
}

fn release_terminal_round_or_fail_stop(
    engine: &mut Engine<32>,
    mut completion: ferric_engine::M1RearmedQualifiedCompletionOutcomeV1,
    mut state: QualificationRoundCaptureStateV1,
) -> (
    ferric_engine::M1RearmedQualifiedReleasedRoundV1,
    QualificationRoundCaptureStateV1,
) {
    let mut completion_retries = 0;
    'completion: loop {
        match completion.release_completed() {
            M1RearmedQualifiedRoundReleaseOutcomeV1::Released(released) => {
                return (released, state);
            }
            M1RearmedQualifiedRoundReleaseOutcomeV1::Rejected(mut failure) => {
                for _ in 0..CAPTURE_RECOVERY_RETRIES {
                    match (*failure).retry() {
                        M1RearmedQualifiedRoundReleaseOutcomeV1::Released(released) => {
                            return (released, state);
                        }
                        M1RearmedQualifiedRoundReleaseOutcomeV1::Rejected(next) => failure = next,
                        M1RearmedQualifiedRoundReleaseOutcomeV1::NotCompleted(next) => {
                            let returned = consume_terminal_non_rejected(
                                engine,
                                next,
                                state,
                                "terminal qualification release lost completed state",
                            );
                            completion = returned.0;
                            state = returned.1;
                            continue 'completion;
                        }
                    }
                }
                close_or_quarantine_round(
                    "terminal qualification page-release retry exhaustion",
                    state,
                    None,
                    failure.destroy_queue_and_retain_round(engine),
                );
            }
            M1RearmedQualifiedRoundReleaseOutcomeV1::NotCompleted(next) => {
                if completion_retries == CAPTURE_RECOVERY_RETRIES {
                    let returned = match next.destroy_queue_and_retain_rejected(engine) {
                        Ok(teardown) => close_or_quarantine_round(
                            "terminal qualification completion retry exhaustion",
                            state,
                            None,
                            teardown,
                        ),
                        Err(non_rejected) => consume_terminal_non_rejected(
                            engine,
                            *non_rejected,
                            state,
                            "terminal qualification completion retry exhaustion",
                        ),
                    };
                    completion = returned.0;
                    state = returned.1;
                    continue;
                }
                completion_retries += 1;
                let returned = match next.retry_rejected(engine) {
                    Ok(completion) => (completion, state),
                    Err(non_rejected) => consume_terminal_non_rejected(
                        engine,
                        *non_rejected,
                        state,
                        "terminal qualification completion retry",
                    ),
                };
                completion = returned.0;
                state = returned.1;
            }
        }
    }
}

fn consume_intermediate_non_rejected(
    engine: &mut Engine<32>,
    completion: ferric_engine::M1RearmedCompletionOutcomeV1,
    state: QualificationRoundCaptureStateV1,
    phase: &'static str,
) -> (
    ferric_engine::M1RearmedCompletionOutcomeV1,
    QualificationRoundCaptureStateV1,
) {
    if matches!(completion.outcome(), M1CompletedStepOutcomeV1::Completed(_)) {
        return (completion, state);
    }
    match completion.into_terminal_poison() {
        Ok(poison) => terminal_round(phase, state, poison),
        Err(rejected) => match rejected.destroy_queue_and_retain_rejected(engine) {
            Ok(teardown) => close_or_quarantine_round(phase, state, None, teardown),
            Err(completed) => (*completed, state),
        },
    }
}

fn consume_terminal_non_rejected(
    engine: &mut Engine<32>,
    completion: ferric_engine::M1RearmedQualifiedCompletionOutcomeV1,
    state: QualificationRoundCaptureStateV1,
    phase: &'static str,
) -> (
    ferric_engine::M1RearmedQualifiedCompletionOutcomeV1,
    QualificationRoundCaptureStateV1,
) {
    if matches!(
        completion.completion().outcome(),
        M1CompletedStepOutcomeV1::Completed(_)
    ) {
        return (completion, state);
    }
    match completion.into_terminal_poison() {
        Ok(poison) => terminal_round(phase, state, poison),
        Err(rejected) => match rejected.destroy_queue_and_retain_rejected(engine) {
            Ok(teardown) => close_or_quarantine_round(phase, state, None, teardown),
            Err(completed) => (*completed, state),
        },
    }
}

fn validate_round_counts(
    ordinal: u32,
    logical: &[u32],
    external: &[u32],
    lanes: usize,
    terminal: bool,
) -> CaptureResult<()> {
    if logical.len() != lanes
        || external.len() != lanes
        || logical.iter().any(|count| *count != 1)
        || external.iter().any(|count| *count != u32::from(terminal))
        || terminal != (ordinal == M1_QUALIFICATION_FINAL_INPUT_TOKEN)
    {
        return Err(format!(
            "qualification ordinal {ordinal} logical/external completion policy drifted"
        ));
    }
    Ok(())
}

fn validate_checked_terminal_counts(
    checked: &ferric_engine::M1CheckedCompletionOutputV1,
    lanes: usize,
) -> CaptureResult<()> {
    if checked.records().len() != lanes
        || checked.records().iter().any(|record| {
            record.record().accepted_draft_tokens != 0 || record.record().emitted_token_count != 1
        })
    {
        return Err("terminal compact completion count or target-only shape drifted".to_owned());
    }
    Ok(())
}

fn preflight_engine_retirement(engine: &Engine<32>, requests: &[RequestId]) -> CaptureResult<()> {
    let mut seen = BTreeSet::new();
    for (lane, request) in requests.iter().copied().enumerate() {
        if !seen.insert(request) {
            return Err(format!(
                "terminal retirement roster repeats the request at lane {lane}"
            ));
        }
        if engine.state(request) != Some(RequestState::InFlight) {
            return Err(format!(
                "terminal retirement request at lane {lane} is not in flight"
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
enum QualificationReleasedRoundV1 {
    First(Box<ferric_engine::M1ReleasedCompletedStepV1>),
    Rearmed(Box<M1LongLivedQueueReleasedRoundV1>),
}

impl QualificationReleasedRoundV1 {
    fn schedule(
        self,
        engine: &mut Engine<32>,
    ) -> Result<
        M1ScheduledLongLivedQueueRearmV1,
        ferric_engine::M1LongLivedQueueRearmScheduleFailureV1,
    > {
        match self {
            Self::First(released) => schedule_m1_long_lived_queue_rearm_v1(engine, *released),
            Self::Rearmed(released) => (*released).schedule_next(engine),
        }
    }
}

fn execute_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    workload: &Workload,
    input_tokens: Vec<u32>,
    gpu_unique_id: u64,
    purpose: CapturePurposeV1,
) -> CaptureResult<CapturedOutput> {
    if let Err(diagnostic) = validate_workload_geometry(workload) {
        abandon_pre_engine_memory(
            "qualification capture workload geometry rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(diagnostic),
        );
    }
    match workload.selection.mode {
        Qwen3ExecutionMode::Prefill => Ok(execute_prefill_capture(
            runner,
            memory,
            workload,
            input_tokens,
            gpu_unique_id,
            purpose,
        )),
        Qwen3ExecutionMode::Decode => Ok(execute_decode_capture(
            runner,
            memory,
            workload,
            input_tokens,
            gpu_unique_id,
        )),
        Qwen3ExecutionMode::Speculative => abandon_pre_engine_memory(
            "qualification capture speculative selection rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(
                "qualification capture does not admit speculative selections".to_owned(),
            ),
        ),
    }
}

fn execute_decode_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    workload: &Workload,
    input_tokens: Vec<u32>,
    _gpu_unique_id: u64,
) -> CapturedOutput {
    if let Err(diagnostic) = require_supported_capture(workload) {
        abandon_pre_engine_memory(
            "qualification decode policy rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(diagnostic),
        );
    }
    let binding = match qualification_execution_binding(workload, &input_tokens) {
        Ok(binding) => binding,
        Err(diagnostic) => abandon_pre_engine_memory(
            "qualification execution binding rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(diagnostic),
        ),
    };
    let context_plan = m1_qualification_context_plan(binding.grouping, binding.declaration.clone());
    let validated_context_plan = match ferric_engine::validate_m1_qualification_context_plan_v1(
        &context_plan,
        binding.grouping,
        &binding.declaration,
    ) {
        Ok(plan) => plan,
        Err(diagnostic) => abandon_pre_engine_memory(
            "qualification context plan rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::ContextPlan(diagnostic),
        ),
    };
    let selection = workload.selection;
    let draft_selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: selection.bucket,
    };
    let logical_page_capacity = match qualification_engine_page_capacity(binding.grouping) {
        Ok(capacity) => capacity,
        Err(diagnostic) => abandon_pre_engine_memory(
            "qualification Engine capacity rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(diagnostic),
        ),
    };
    let mut requests = Vec::new();
    let mut caches = Vec::new();
    let mut reservations = Vec::new();
    if requests.try_reserve_exact(workload.lanes.len()).is_err()
        || caches.try_reserve_exact(workload.lanes.len()).is_err()
        || reservations
            .try_reserve_exact(workload.lanes.len())
            .is_err()
    {
        abandon_pre_engine_memory(
            "qualification admission-roster allocation",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(
                "cannot allocate qualification admission rosters".to_owned(),
            ),
        );
    }
    let mut engine = match Engine::<32>::new(
        logical_page_capacity,
        QUALIFICATION_LOGICAL_KV_PAGE_TOKENS,
        M1_QUALIFICATION_TOKENS_PER_LANE,
    ) {
        Ok(engine) => engine,
        Err(diagnostic) => abandon_pre_engine_memory(
            "qualification Engine construction",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Engine(diagnostic),
        ),
    };
    for _ in 0..workload.lanes.len() {
        let request = match engine.admit() {
            Ok(request) => request,
            Err(diagnostic) => abandon_pre_physical_engine(
                "qualification request admission",
                engine,
                requests,
                reservations,
                PrePhysicalPoolCustodyV1::Split {
                    _memory: memory,
                    _caches: caches,
                },
                input_tokens,
                PrePhysicalDiagnosticV1::Engine(diagnostic),
            ),
        };
        requests.push(request);
        let cache =
            match ActiveDeviceKvCache::new(memory.device(), request, selection, draft_selection) {
                Ok(cache) => cache,
                Err(diagnostic) => abandon_pre_physical_engine(
                    "qualification device-KV cache construction",
                    engine,
                    requests,
                    reservations,
                    PrePhysicalPoolCustodyV1::Split {
                        _memory: memory,
                        _caches: caches,
                    },
                    input_tokens,
                    PrePhysicalDiagnosticV1::DeviceCache(diagnostic),
                ),
            };
        caches.push(cache);
    }
    let initial_contexts = match (0..requests.len())
        .map(|lane| validated_context_plan.step(0, u32::try_from(lane).unwrap_or(u32::MAX)))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(contexts) => contexts,
        Err(diagnostic) => abandon_pre_physical_engine(
            "ordinal-zero context witness derivation",
            engine,
            requests,
            reservations,
            PrePhysicalPoolCustodyV1::Split {
                _memory: memory,
                _caches: caches,
            },
            input_tokens,
            PrePhysicalDiagnosticV1::ContextWitness(diagnostic),
        ),
    };
    let preleased = match prelease_m1_qualification_target_pages_v1(
        memory,
        caches,
        initial_contexts.clone(),
        binding.grouping,
    ) {
        Ok(preleased) => preleased,
        Err(failure) => {
            let (_error, recovery) = (*failure).into_parts();
            match recovery.retry() {
                Ok(preleased) => preleased,
                Err(failure) => {
                    let (_error, recovery) = (*failure).into_parts();
                    match recovery.cancel() {
                        Ok(cancelled) => abandon_prelease(
                            "qualification target-page prelease cancelled after retry exhaustion",
                            engine,
                            requests,
                            reservations,
                            input_tokens,
                            PreleaseCancellationOutcomeV1::Cancelled(cancelled),
                        ),
                        Err(failure) => match (*failure).retry() {
                            Ok(cancelled) => abandon_prelease(
                                "qualification target-page prelease cancellation retry completed",
                                engine,
                                requests,
                                reservations,
                                input_tokens,
                                PreleaseCancellationOutcomeV1::Cancelled(cancelled),
                            ),
                            Err(failure) => abandon_prelease(
                                "qualification target-page prelease cancellation retry exhaustion",
                                engine,
                                requests,
                                reservations,
                                input_tokens,
                                PreleaseCancellationOutcomeV1::Exhausted(
                                    (*failure).exhaust_retry_policy(),
                                ),
                            ),
                        },
                    }
                }
            }
        }
    };
    for request in requests.iter().copied() {
        if let Err(diagnostic) = engine.append_tentative(request, 1) {
            abandon_pre_physical_engine(
                "ordinal-zero Engine enqueue",
                engine,
                requests,
                reservations,
                PrePhysicalPoolCustodyV1::Preleased {
                    _preleased: preleased,
                    _initial_contexts: initial_contexts,
                },
                input_tokens,
                PrePhysicalDiagnosticV1::Engine(diagnostic),
            );
        }
    }
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled)) => scheduled,
        Ok(None) => abandon_pre_physical_engine(
            "ordinal-zero missing scheduler batch",
            engine,
            requests,
            reservations,
            PrePhysicalPoolCustodyV1::Preleased {
                _preleased: preleased,
                _initial_contexts: initial_contexts,
            },
            input_tokens,
            PrePhysicalDiagnosticV1::MissingBatch,
        ),
        Err(diagnostic) => abandon_pre_physical_engine(
            "ordinal-zero Engine dispatch",
            engine,
            requests,
            reservations,
            PrePhysicalPoolCustodyV1::Preleased {
                _preleased: preleased,
                _initial_contexts: initial_contexts,
            },
            input_tokens,
            PrePhysicalDiagnosticV1::Engine(diagnostic),
        ),
    };
    let (memory, mut caches) = preleased.into_parts();
    let device_id = memory.device().device_id();
    if let Err(diagnostic) = validate_scheduled_roster(&scheduled, &requests, 0) {
        abandon_decode_initial_dispatch(
            "ordinal-zero scheduled roster validation",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::Scheduled {
                _diagnostic: diagnostic,
            },
        );
    }
    let plans = match bind_qualification_step_plans(runner, &scheduled, selection, &requests, 0) {
        Ok(plans) => plans,
        Err(diagnostic) => abandon_decode_initial_dispatch(
            "ordinal-zero step-plan binding",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::Scheduled {
                _diagnostic: diagnostic,
            },
        ),
    };
    let inputs = match qualification_step_inputs(workload, &plans, &input_tokens, 0) {
        Ok(inputs) => inputs,
        Err(diagnostic) => abandon_decode_initial_dispatch(
            "ordinal-zero input binding",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::Input {
                _diagnostic: diagnostic,
                _plans: plans,
            },
        ),
    };
    for (lane, ((cache, request), context)) in caches
        .iter_mut()
        .zip(requests.iter().copied())
        .zip(initial_contexts.iter().copied())
        .enumerate()
    {
        let reservation = match cache.reserve_m1_qualification_context_step_write_v1(
            request,
            u32::try_from(lane).unwrap_or(u32::MAX),
            context,
            scheduled.epoch(),
        ) {
            Ok(reservation) => reservation,
            Err(diagnostic) => abandon_decode_initial_dispatch(
                "ordinal-zero qualification KV reservation",
                engine,
                memory,
                caches,
                requests,
                initial_contexts,
                scheduled,
                DecodeInitialPhaseCustodyV1::Reservations {
                    _diagnostic: diagnostic,
                    _plans: plans,
                    _inputs: inputs,
                    _reservations: reservations,
                },
            ),
        };
        reservations.push(reservation.into_pending_step_write());
    }
    let table = match bind_m1_kv_workspace_table_v1(inputs, reservations) {
        Ok(table) => table,
        Err(failure) => abandon_decode_initial_dispatch(
            "ordinal-zero KV workspace binding",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::KvBinding {
                _plans: plans,
                _failure: failure,
            },
        ),
    };
    let workspace_identity = *binding.declaration.declared_workload_digest.as_bytes();
    let workspace_plan = match workload_workspace_plan(selection, workspace_identity) {
        Ok(plan) => plan,
        Err(diagnostic) => abandon_decode_initial_dispatch(
            "ordinal-zero workspace plan",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::WorkspacePlan {
                _diagnostic: diagnostic,
                _plans: plans,
                _table: table,
            },
        ),
    };
    let recipe = match derive_qualification_recipe(runner, selection, workspace_identity) {
        Ok(recipe) => recipe,
        Err(diagnostic) => abandon_decode_initial_dispatch(
            "ordinal-zero recipe derivation",
            engine,
            memory,
            caches,
            requests,
            initial_contexts,
            scheduled,
            DecodeInitialPhaseCustodyV1::Recipe {
                _diagnostic: diagnostic,
                _plans: plans,
                _table: table,
                _workspace_plan: workspace_plan,
            },
        ),
    };
    let prepared = match prepare_scheduled_workspaces_with_retries(
        runner,
        scheduled,
        M1FullStepWorkspacePlans::target_only(workspace_plan),
        M1FullStepKvWorkspaceTablesV1::TargetOnly { target: table },
    ) {
        Ok(prepared) => prepared,
        Err(failure) => abandon_first_generation(
            "ordinal-zero workspace preparation retry exhaustion",
            engine,
            FirstGenerationLiveEvidenceV1::Decode {
                _caches: caches,
                _requests: requests,
                _contexts: initial_contexts,
            },
            FirstGenerationPhaseFailureV1::Preparation {
                _memory: Box::new(memory),
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let mut allocated = match allocate_scheduled_workspaces_with_retries(runner, memory, prepared) {
        Ok(allocated) => allocated,
        Err(failure) => abandon_first_generation(
            "ordinal-zero allocation retry exhaustion",
            engine,
            FirstGenerationLiveEvidenceV1::Decode {
                _caches: caches,
                _requests: requests,
                _contexts: initial_contexts,
            },
            FirstGenerationPhaseFailureV1::Allocation {
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let completion = match allocated.allocate_completion_output(selection) {
        Ok(completion) => completion,
        Err(diagnostic) => abandon_first_generation(
            "ordinal-zero compact completion allocation",
            engine,
            FirstGenerationLiveEvidenceV1::Decode {
                _caches: caches,
                _requests: requests,
                _contexts: initial_contexts,
            },
            FirstGenerationPhaseFailureV1::CompletionOutput {
                _allocated: Box::new(allocated),
                _recipe: recipe,
                _diagnostic: diagnostic,
            },
        ),
    };
    let completion = match allocated.enable_qualification_logits_capture(completion) {
        Ok(completion) => completion,
        Err(failure) => abandon_first_generation(
            "ordinal-zero qualification logits allocation",
            engine,
            FirstGenerationLiveEvidenceV1::Decode {
                _caches: caches,
                _requests: requests,
                _contexts: initial_contexts,
            },
            FirstGenerationPhaseFailureV1::QualificationLogits {
                _allocated: Box::new(allocated),
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let published =
        match publish_first_step_with_retries(runner, &mut engine, allocated, recipe, completion) {
            Ok(published) => published,
            Err(failure) => {
                let failure = failure.quarantine_after_retry_exhaustion(&mut engine);
                abandon_first_generation(
                    "ordinal-zero first publication retry exhaustion",
                    engine,
                    FirstGenerationLiveEvidenceV1::Decode {
                        _caches: caches,
                        _requests: requests,
                        _contexts: initial_contexts,
                    },
                    FirstGenerationPhaseFailureV1::Publication { _failure: failure },
                )
            }
        };
    let semantics = initial_contexts
        .iter()
        .map(|context| CompletionWireSemanticExpectation::QualificationPromptCommit { context })
        .collect::<Vec<_>>();
    let roster = M1DeviceKvCompletionRosterV1::new(
        caches
            .into_iter()
            .map(M1DeviceKvCompletionMemberV1::continuing)
            .collect(),
    );
    let first_outcome = runner.complete_first_step(&mut engine, published, &semantics, roster);
    let first = consume_first_completion_outcome(&mut engine, first_outcome, &semantics);
    let history =
        QualificationRoundCommitmentV1::new(context_plan.plan_id, &binding.declaration, selection);
    let mut state = QualificationRoundCaptureStateV1 { requests, history };
    if let Err(error) = validate_round_counts(
        0,
        first.logical_accepted_counts(),
        first.externally_published_counts(),
        state.requests.len(),
        false,
    ) {
        let teardown = first.destroy_queue_and_retain_step(&mut engine);
        close_or_quarantine_round(
            "ordinal-zero completion-count validation",
            state,
            Some(QualificationRoundDiagnosticV1::Message(error)),
            teardown,
        );
    }
    if let Err(error) = state.history.observe(
        0,
        first.checked(),
        &state.requests,
        QualificationRoundReceiptV1 {
            logical_accepted: first.logical_accepted_counts(),
            externally_published: first.externally_published_counts(),
            release_counts: first.release_counts(),
            device_id,
        },
    ) {
        let teardown = first.destroy_queue_and_retain_step(&mut engine);
        close_or_quarantine_round(
            "ordinal-zero round-history validation",
            state,
            Some(QualificationRoundDiagnosticV1::Message(error)),
            teardown,
        );
    }
    let mut released = QualificationReleasedRoundV1::First(Box::new(first));

    for ordinal in 1..M1_QUALIFICATION_FINAL_INPUT_TOKEN {
        let contexts =
            match qualification_contexts(validated_context_plan, state.requests.len(), ordinal) {
                Ok(contexts) => contexts,
                Err(diagnostic) => abandon_released_round(
                    "qualification intermediate context witnesses",
                    engine,
                    released,
                    state,
                    ReleasedRoundPhaseCustodyV1::Context {
                        _diagnostic: diagnostic,
                    },
                ),
            };
        let workspace_plan = match workload_workspace_plan(selection, workspace_identity) {
            Ok(plan) => plan,
            Err(diagnostic) => abandon_released_round(
                "qualification intermediate workspace plan",
                engine,
                released,
                state,
                ReleasedRoundPhaseCustodyV1::WorkspacePlan {
                    _diagnostic: diagnostic,
                    _contexts: contexts,
                },
            ),
        };
        let recipe = match derive_qualification_recipe(runner, selection, workspace_identity) {
            Ok(recipe) => recipe,
            Err(diagnostic) => abandon_released_round(
                "qualification intermediate recipe derivation",
                engine,
                released,
                state,
                ReleasedRoundPhaseCustodyV1::Recipe {
                    _diagnostic: diagnostic,
                    _contexts: contexts,
                    _workspace_plan: workspace_plan,
                },
            ),
        };
        for request in state.requests.iter().copied() {
            if let Err(diagnostic) = engine.append_tentative(request, 1) {
                abandon_released_round(
                    "qualification intermediate Engine enqueue",
                    engine,
                    released,
                    state,
                    ReleasedRoundPhaseCustodyV1::Enqueue {
                        _diagnostic: diagnostic,
                        _contexts: contexts,
                        _workspace_plan: workspace_plan,
                        _recipe: recipe,
                    },
                );
            }
        }
        let (scheduled, returned_state) =
            schedule_qualification_round(released, &mut engine, state);
        state = returned_state;
        if let Err(error) =
            validate_scheduled_roster(scheduled.scheduled_dispatch(), &state.requests, ordinal)
        {
            let teardown = scheduled.destroy_queue_and_retain_round(&mut engine);
            close_or_quarantine_round(
                "qualification intermediate scheduled-roster drift",
                state,
                Some(QualificationRoundDiagnosticV1::Message(error)),
                teardown,
            );
        }
        if scheduled
            .selected_requests()
            .ne(state.requests.iter().copied())
            || scheduled.parked_count() != 0
            || scheduled.terminal_count() != 0
        {
            close_or_quarantine_round(
                "qualification intermediate scheduler-lineage drift",
                state,
                None,
                scheduled.destroy_queue_and_retain_round(&mut engine),
            );
        }
        let plans = match bind_qualification_step_plans(
            runner,
            scheduled.scheduled_dispatch(),
            selection,
            &state.requests,
            ordinal,
        ) {
            Ok(plans) => plans,
            Err(diagnostic) => close_or_quarantine_round(
                "qualification intermediate step-plan binding",
                state,
                Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
                scheduled.destroy_queue_and_retain_round(&mut engine),
            ),
        };
        let inputs = match qualification_step_inputs(workload, &plans, &input_tokens, ordinal) {
            Ok(inputs) => inputs,
            Err(diagnostic) => close_or_quarantine_round(
                "qualification intermediate input binding",
                state,
                Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
                scheduled.destroy_queue_and_retain_round(&mut engine),
            ),
        };
        let reserved = match reserve_m1_long_lived_queue_rearm_kv_v1(
            &mut engine,
            scheduled,
            M1LongLivedQueueRearmKvInputsV1::qualification_target_only(inputs, contexts.clone()),
        ) {
            Ok(reserved) => reserved,
            Err(failure) => {
                terminal_round("qualification intermediate KV reservation", state, failure)
            }
        };
        let prepared = match prepare_m1_long_lived_queue_rearm_v1(
            &mut engine,
            reserved,
            runner.logical_runner(),
            M1FullStepWorkspacePlans::target_only(workspace_plan),
        ) {
            Ok(prepared) => prepared,
            Err(failure) => terminal_round(
                "qualification intermediate workspace preparation",
                state,
                failure,
            ),
        };
        let (published, returned_state) =
            submit_rearm_or_fail_stop(runner, &mut engine, prepared, recipe, state);
        state = returned_state;
        let completed = match published.wait(&mut engine) {
            Ok(completed) => completed,
            Err(failure) => {
                terminal_queue_round("qualification intermediate queue wait", state, failure)
            }
        };
        let recycled = match completed.recycle(&mut engine) {
            Ok(recycled) => recycled,
            Err(failure) => {
                terminal_queue_round("qualification intermediate queue recycle", state, failure)
            }
        };
        let semantics = contexts
            .iter()
            .map(|context| CompletionWireSemanticExpectation::QualificationPromptCommit { context })
            .collect::<Vec<_>>();
        let readback = match recycled.read_and_check_completion(&semantics) {
            Ok(readback) => readback,
            Err(failure) => match failure.retry(&semantics) {
                Ok(readback) => readback,
                Err(failure) => {
                    let teardown = failure.destroy_queue_and_retain_custody(&mut engine);
                    close_or_quarantine_round(
                        "qualification intermediate readback rejected after retry",
                        state,
                        None,
                        teardown,
                    );
                }
            },
        };
        let completion = match readback.complete(
            &mut engine,
            vec![M1DeviceKvCompletionDispositionV1::Continue; state.requests.len()],
        ) {
            Ok(completion) => completion,
            Err(failure) => match failure.retry(&mut engine) {
                Ok(completion) => completion,
                Err(failure) => close_or_quarantine_round(
                    "qualification intermediate completion preflight teardown",
                    state,
                    None,
                    failure.destroy_queue_and_retain_custody(&mut engine),
                ),
            },
        };
        let (released_round, returned_state) =
            release_intermediate_round_or_fail_stop(&mut engine, completion, state);
        state = returned_state;
        if released_round.parked_count() != 0
            || released_round.terminal_lineage_count() != 0
            || released_round.round_history_len() != usize::try_from(ordinal).unwrap_or(usize::MAX)
        {
            let teardown = released_round.destroy_queue_and_retain_round(&mut engine);
            close_or_quarantine_round(
                "qualification intermediate released-lineage drift",
                state,
                None,
                teardown,
            );
        }
        let current = released_round.current_released();
        if let Err(error) = validate_round_counts(
            ordinal,
            current.logical_accepted_counts(),
            current.externally_published_counts(),
            state.requests.len(),
            false,
        ) {
            let teardown = released_round.destroy_queue_and_retain_round(&mut engine);
            close_or_quarantine_round(
                "qualification intermediate completion-count drift",
                state,
                Some(QualificationRoundDiagnosticV1::Message(error)),
                teardown,
            );
        }
        if let Err(error) = state.history.observe(
            ordinal,
            current.checked(),
            &state.requests,
            QualificationRoundReceiptV1 {
                logical_accepted: current.logical_accepted_counts(),
                externally_published: current.externally_published_counts(),
                release_counts: current.release_counts(),
                device_id,
            },
        ) {
            let teardown = released_round.destroy_queue_and_retain_round(&mut engine);
            close_or_quarantine_round(
                "qualification intermediate round-history drift",
                state,
                Some(QualificationRoundDiagnosticV1::Message(error)),
                teardown,
            );
        }
        released = QualificationReleasedRoundV1::Rearmed(Box::new(released_round));
    }

    let terminal_ordinal = M1_QUALIFICATION_FINAL_INPUT_TOKEN;
    let terminal_contexts = match qualification_contexts(
        validated_context_plan,
        state.requests.len(),
        terminal_ordinal,
    ) {
        Ok(contexts) => contexts,
        Err(diagnostic) => abandon_released_round(
            "terminal qualification context witnesses",
            engine,
            released,
            state,
            ReleasedRoundPhaseCustodyV1::Context {
                _diagnostic: diagnostic,
            },
        ),
    };
    let workspace_plan = match workload_workspace_plan(selection, workspace_identity) {
        Ok(plan) => plan,
        Err(diagnostic) => abandon_released_round(
            "terminal qualification workspace plan",
            engine,
            released,
            state,
            ReleasedRoundPhaseCustodyV1::WorkspacePlan {
                _diagnostic: diagnostic,
                _contexts: terminal_contexts,
            },
        ),
    };
    let recipe = match derive_qualification_recipe(runner, selection, workspace_identity) {
        Ok(recipe) => recipe,
        Err(diagnostic) => abandon_released_round(
            "terminal qualification recipe derivation",
            engine,
            released,
            state,
            ReleasedRoundPhaseCustodyV1::Recipe {
                _diagnostic: diagnostic,
                _contexts: terminal_contexts,
                _workspace_plan: workspace_plan,
            },
        ),
    };
    for request in state.requests.iter().copied() {
        if let Err(diagnostic) = engine.append_tentative(request, 1) {
            abandon_released_round(
                "terminal qualification Engine enqueue",
                engine,
                released,
                state,
                ReleasedRoundPhaseCustodyV1::Enqueue {
                    _diagnostic: diagnostic,
                    _contexts: terminal_contexts,
                    _workspace_plan: workspace_plan,
                    _recipe: recipe,
                },
            );
        }
    }
    let (scheduled, returned_state) = schedule_qualification_round(released, &mut engine, state);
    state = returned_state;
    if let Err(error) = validate_scheduled_roster(
        scheduled.scheduled_dispatch(),
        &state.requests,
        terminal_ordinal,
    ) {
        let teardown = scheduled.destroy_queue_and_retain_round(&mut engine);
        close_or_quarantine_round(
            "terminal qualification scheduled-roster drift",
            state,
            Some(QualificationRoundDiagnosticV1::Message(error)),
            teardown,
        );
    }
    if scheduled
        .selected_requests()
        .ne(state.requests.iter().copied())
        || scheduled.parked_count() != 0
        || scheduled.terminal_count() != 0
        || scheduled.round_history_len() != M1_QUALIFICATION_CONTEXT_PLAN_STEPS - 2
    {
        close_or_quarantine_round(
            "terminal qualification scheduler-lineage drift",
            state,
            None,
            scheduled.destroy_queue_and_retain_round(&mut engine),
        );
    }
    let plans = match bind_qualification_step_plans(
        runner,
        scheduled.scheduled_dispatch(),
        selection,
        &state.requests,
        terminal_ordinal,
    ) {
        Ok(plans) => plans,
        Err(diagnostic) => close_or_quarantine_round(
            "terminal qualification step-plan binding",
            state,
            Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
            scheduled.destroy_queue_and_retain_round(&mut engine),
        ),
    };
    let inputs = match qualification_step_inputs(workload, &plans, &input_tokens, terminal_ordinal)
    {
        Ok(inputs) => inputs,
        Err(diagnostic) => close_or_quarantine_round(
            "terminal qualification input binding",
            state,
            Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
            scheduled.destroy_queue_and_retain_round(&mut engine),
        ),
    };
    let reserved = match reserve_m1_long_lived_queue_rearm_kv_v1(
        &mut engine,
        scheduled,
        M1LongLivedQueueRearmKvInputsV1::qualification_target_only(
            inputs,
            terminal_contexts.clone(),
        ),
    ) {
        Ok(reserved) => reserved,
        Err(failure) => terminal_round("terminal qualification KV reservation", state, failure),
    };
    let prepared = match prepare_m1_long_lived_queue_rearm_v1(
        &mut engine,
        reserved,
        runner.logical_runner(),
        M1FullStepWorkspacePlans::target_only(workspace_plan),
    ) {
        Ok(prepared) => prepared,
        Err(failure) => terminal_round(
            "terminal qualification workspace preparation",
            state,
            failure,
        ),
    };
    let (published, returned_state) =
        submit_rearm_or_fail_stop(runner, &mut engine, prepared, recipe, state);
    state = returned_state;
    let completed = match published.wait(&mut engine) {
        Ok(completed) => completed,
        Err(failure) => terminal_queue_round("terminal qualification queue wait", state, failure),
    };
    let recycled = match completed.recycle(&mut engine) {
        Ok(recycled) => recycled,
        Err(failure) => {
            terminal_queue_round("terminal qualification queue recycle", state, failure)
        }
    };
    let observed = match recycled.observe_qualification_completion() {
        Ok(observed) => observed,
        Err(failure) => match (*failure).retry() {
            Ok(observed) => observed,
            Err(failure) => {
                let teardown = (*failure).destroy_queue_and_retain_custody(&mut engine);
                close_or_quarantine_round(
                    "terminal qualification observation rejected after retry",
                    state,
                    None,
                    teardown,
                );
            }
        },
    };
    if observed
        .selected_requests()
        .ne(state.requests.iter().copied())
        || observed.parked_count() != 0
    {
        close_or_quarantine_round(
            "terminal qualification observation roster drift",
            state,
            None,
            observed.destroy_queue_and_retain_custody(&mut engine),
        );
    }
    let qualified = match observed.check_final_completion(&terminal_contexts) {
        Ok(qualified) => qualified,
        Err(failure) => {
            let teardown = failure.destroy_queue_and_retain_custody(&mut engine);
            close_or_quarantine_round(
                "terminal qualification semantic join rejected",
                state,
                None,
                teardown,
            );
        }
    };
    if let Err(error) = validate_checked_terminal_counts(qualified.checked(), state.requests.len())
    {
        close_or_quarantine_round(
            "terminal qualification compact-count drift",
            state,
            Some(QualificationRoundDiagnosticV1::Message(error)),
            qualified.destroy_queue_and_retain_custody(&mut engine),
        );
    }
    let retiring = qualified.selected_requests().collect::<Vec<_>>();
    if retiring != state.requests {
        close_or_quarantine_round(
            "terminal qualification retirement roster drift",
            state,
            None,
            qualified.destroy_queue_and_retain_custody(&mut engine),
        );
    }
    if let Err(error) = preflight_engine_retirement(&engine, &retiring) {
        close_or_quarantine_round(
            "terminal qualification Engine retirement preflight",
            state,
            Some(QualificationRoundDiagnosticV1::Message(error)),
            qualified.destroy_queue_and_retain_custody(&mut engine),
        );
    }
    for request in &retiring {
        if let Err(error) = engine.retire(*request) {
            close_or_quarantine_round(
                "terminal qualification Engine retirement failure",
                state,
                Some(QualificationRoundDiagnosticV1::Engine(error)),
                qualified.destroy_queue_and_retain_custody(&mut engine),
            );
        }
    }
    let completion = match qualified.complete_retiring(&mut engine) {
        Ok(completion) => completion,
        Err(failure) => match (*failure).retry(&mut engine) {
            Ok(completion) => completion,
            Err(failure) => close_or_quarantine_round(
                "terminal qualification completion preflight teardown",
                state,
                None,
                failure.destroy_queue_and_retain_custody(&mut engine),
            ),
        },
    };
    let (released, returned_state) =
        release_terminal_round_or_fail_stop(&mut engine, completion, state);
    state = returned_state;
    let teardown = match released.destroy_queue_and_retain_round(&mut engine) {
        Ok(teardown) => teardown,
        Err(quarantine) => terminal_quarantine(
            "terminal qualification queue teardown",
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: None,
                _custody: quarantine,
            },
        ),
    };
    if teardown.round_history_len() != M1_QUALIFICATION_CONTEXT_PLAN_STEPS - 1
        || teardown.teardown().parked_count() != 0
        || teardown.teardown().terminal_count() != 0
        || teardown.teardown().released().members().len() != state.requests.len()
        || teardown
            .teardown()
            .released()
            .members()
            .iter()
            .any(|member| matches!(member, ferric_engine::M1ReleasedDeviceKvMemberV1::Active(_)))
    {
        closed_teardown(
            "terminal qualification teardown custody drifted",
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: Some(QualificationRoundDiagnosticV1::Message(
                    "terminal qualification teardown custody drifted".to_owned(),
                )),
                _custody: teardown,
            },
        );
    }
    if let Err(diagnostic) = validate_round_counts(
        terminal_ordinal,
        teardown.teardown().released().logical_accepted_counts(),
        teardown.teardown().released().externally_published_counts(),
        state.requests.len(),
        true,
    ) {
        closed_teardown(
            "terminal qualification completion-count drift",
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
                _custody: teardown,
            },
        );
    }
    if let Err(diagnostic) = state.history.observe(
        terminal_ordinal,
        teardown.teardown().released().checked(),
        &state.requests,
        QualificationRoundReceiptV1 {
            logical_accepted: teardown.teardown().released().logical_accepted_counts(),
            externally_published: teardown.teardown().released().externally_published_counts(),
            release_counts: teardown.teardown().released().release_counts(),
            device_id,
        },
    ) {
        closed_teardown(
            "terminal qualification round-history drift",
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
                _custody: teardown,
            },
        );
    }
    let copied = match copy_capture_candidate(
        teardown.teardown().released().checked(),
        teardown.evidence(),
        state.requests.len(),
    ) {
        Ok(copied) => copied,
        Err(diagnostic) => closed_teardown(
            "terminal qualification output-copy rejection",
            QualificationRoundCustodyV1 {
                _state: state,
                _diagnostic: Some(QualificationRoundDiagnosticV1::Message(diagnostic)),
                _custody: teardown,
            },
        ),
    };
    let (
        round_history_sha256,
        round_count,
        first_epoch,
        first_dispatch_generation,
        terminal_epoch,
        terminal_dispatch_generation,
    ) = match state.history.finish() {
        Ok(history) => history,
        Err(failure) => closed_teardown(
            "terminal qualification history finalization",
            QualificationRoundCustodyV1 {
                _state: QualificationRoundCaptureStateV1 {
                    requests: state.requests,
                    history: failure.history,
                },
                _diagnostic: Some(QualificationRoundDiagnosticV1::Message(failure.diagnostic)),
                _custody: teardown,
            },
        ),
    };
    CapturedOutput {
        compact_sha256: copied.compact_sha256,
        device_id,
        execution: CapturedExecutionV1::C8192 {
            execution_binding: binding.declaration,
            first_dispatch_generation,
            first_epoch,
            qualification_plan_id: context_plan.plan_id,
            round_count,
            round_history_sha256,
            terminal_dispatch_generation,
            terminal_epoch,
        },
        logits: copied.logits,
        logits_row_sha256: copied.logits_row_sha256,
        r30_canary_closed: None,
        settlement: None,
        tokens: copied.tokens,
    }
}

fn consume_first_completion_outcome(
    engine: &mut Engine<32>,
    outcome: M1PhysicalRunnerFirstCompletionOutcomeV1,
    semantics: &[CompletionWireSemanticExpectation<'_>],
) -> ferric_engine::M1ReleasedCompletedStepV1 {
    let completed = match outcome {
        M1PhysicalRunnerFirstCompletionOutcomeV1::Released(released) => return released,
        M1PhysicalRunnerFirstCompletionOutcomeV1::CompletionNotCommitted(
            M1CompletedStepOutcomeV1::Completed(completed),
        ) => completed,
        M1PhysicalRunnerFirstCompletionOutcomeV1::CompletionNotCommitted(
            M1CompletedStepOutcomeV1::Rejected(rejected),
        ) => {
            let (_error, readback, roster) = rejected.into_parts();
            match complete_m1_physical_step_v1(engine, readback, roster) {
                M1CompletedStepOutcomeV1::Completed(completed) => completed,
                M1CompletedStepOutcomeV1::Rejected(rejected) => close_or_quarantine(
                    "ordinal-zero completion rejection teardown",
                    rejected.destroy_queue_and_retain_rejection(engine),
                ),
                M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
                    "ordinal-zero completion retry entered terminal poison",
                    poison,
                ),
            }
        }
        M1PhysicalRunnerFirstCompletionOutcomeV1::CompletionNotCommitted(
            M1CompletedStepOutcomeV1::Poisoned(poisoned),
        ) => terminal_quarantine("ordinal-zero completion entered terminal poison", poisoned),
        M1PhysicalRunnerFirstCompletionOutcomeV1::PageReleaseRejected(failure) => {
            let (_error, completed) = (*failure).into_parts();
            return match release_first_completed_step(engine, completed) {
                Ok(released) => released,
                Err(teardown) => close_first_page_release_teardown(
                    "ordinal-zero page-release teardown",
                    teardown,
                ),
            };
        }
        M1PhysicalRunnerFirstCompletionOutcomeV1::ObservationRejected { failure, roster } => {
            let failure = match (*failure).retry() {
                Ok(observed) => match observed.check_completion(semantics) {
                    Ok(readback) => {
                        return consume_first_readback_completion(engine, readback, roster)
                    }
                    Err(failure) => {
                        let teardown = failure.destroy_queue_and_retain_evidence(engine);
                        close_or_quarantine_roster(
                            "ordinal-zero observation retry semantic rejection",
                            roster,
                            teardown,
                        )
                    }
                },
                Err(failure) => failure,
            };
            let teardown = (*failure).destroy_queue_and_retain_evidence(engine);
            close_or_quarantine_roster(
                "ordinal-zero completion observation rejected",
                roster,
                teardown,
            );
        }
        M1PhysicalRunnerFirstCompletionOutcomeV1::ReadbackRejected { failure, roster } => {
            let failure = match (*failure).retry(semantics) {
                Ok(readback) => return consume_first_readback_completion(engine, readback, roster),
                Err(failure) => failure,
            };
            let teardown = failure.destroy_queue_and_retain_evidence(engine);
            close_or_quarantine_roster(
                "ordinal-zero completion semantic join rejected",
                roster,
                teardown,
            );
        }
        M1PhysicalRunnerFirstCompletionOutcomeV1::QueueQuarantined {
            stage,
            failure,
            roster,
        } => {
            report_physical_queue_failure(
                "ordinal-zero queue entered terminal quarantine",
                &failure,
            );
            terminal_quarantine(
                "ordinal-zero queue entered terminal quarantine",
                FirstQueueQuarantineV1 {
                    _stage: stage,
                    _failure: failure,
                    _roster: roster,
                },
            )
        }
    };
    match release_first_completed_step(engine, completed) {
        Ok(released) => released,
        Err(teardown) => {
            close_first_page_release_teardown("ordinal-zero page-release teardown", teardown)
        }
    }
}

fn consume_first_readback_completion(
    engine: &mut Engine<32>,
    readback: ferric_engine::M1PhysicalCompletedReadbackV1,
    roster: M1DeviceKvCompletionRosterV1,
) -> ferric_engine::M1ReleasedCompletedStepV1 {
    let completed = match complete_m1_physical_step_v1(engine, readback, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        M1CompletedStepOutcomeV1::Rejected(rejected) => {
            let (_error, readback, roster) = rejected.into_parts();
            match complete_m1_physical_step_v1(engine, readback, roster) {
                M1CompletedStepOutcomeV1::Completed(completed) => completed,
                M1CompletedStepOutcomeV1::Rejected(rejected) => close_or_quarantine(
                    "first completion observation retry rejected completion",
                    rejected.destroy_queue_and_retain_rejection(engine),
                ),
                M1CompletedStepOutcomeV1::Poisoned(poison) => {
                    terminal_quarantine("first completion observation retry entered poison", poison)
                }
            }
        }
        M1CompletedStepOutcomeV1::Poisoned(poison) => {
            terminal_quarantine("first completion observation entered poison", poison)
        }
    };
    match release_first_completed_step(engine, completed) {
        Ok(released) => released,
        Err(teardown) => close_first_page_release_teardown(
            "first completion observation page-release teardown",
            teardown,
        ),
    }
}

struct FirstPageReleaseTeardownV1 {
    error: ferric_engine::M1CompletedStepKvReleaseErrorV1,
    teardown: Result<
        ferric_engine::M1CompletedStepTeardownSuccessV1,
        Box<ferric_engine::M1CompletedStepTeardownFailureV1>,
    >,
}

fn release_first_completed_step<const C: usize>(
    engine: &mut Engine<C>,
    completed: ferric_engine::M1CompletedStepSuccessV1,
) -> Result<ferric_engine::M1ReleasedCompletedStepV1, Box<FirstPageReleaseTeardownV1>> {
    match release_m1_completed_step_kv_pages_v1(completed) {
        Ok(released) => Ok(released),
        Err(failure) => {
            let (_error, completed) = (*failure).into_parts();
            match release_m1_completed_step_kv_pages_v1(completed) {
                Ok(released) => Ok(released),
                Err(failure) => {
                    let (error, completed) = (*failure).into_parts();
                    Err(Box::new(FirstPageReleaseTeardownV1 {
                        error,
                        teardown: completed.destroy_queue_and_retain_completion(engine),
                    }))
                }
            }
        }
    }
}

fn close_first_page_release_teardown(
    phase: &'static str,
    teardown: Box<FirstPageReleaseTeardownV1>,
) -> ! {
    let FirstPageReleaseTeardownV1 { error, teardown } = *teardown;
    close_or_quarantine_with_diagnostic(phase, error, teardown)
}

fn execute_prefill_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    workload: &Workload,
    input_tokens: Vec<u32>,
    _gpu_unique_id: u64,
    purpose: CapturePurposeV1,
) -> CapturedOutput {
    let selection = workload.selection;
    if selection.role != Qwen3ModelRole::Target8B || selection.mode != Qwen3ExecutionMode::Prefill {
        abandon_pre_engine_memory(
            "prefill selection rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(
                "one-shot qualification capture accepts target prefill only".to_owned(),
            ),
        );
    }
    let dimensions = match selection.bucket.dimensions(selection.role, selection.mode) {
        Some(dimensions) => dimensions,
        None => abandon_pre_engine_memory(
            "prefill dimensions rejection",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Policy(
                "prefill workload selection has no admitted dimensions".to_owned(),
            ),
        ),
    };
    let draft_selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: selection.mode,
        bucket: selection.bucket,
    };
    let mut engine = match Engine::<32>::new(512, 256, 8_192) {
        Ok(engine) => engine,
        Err(diagnostic) => abandon_pre_engine_memory(
            "prefill Engine construction",
            memory,
            input_tokens,
            PreEngineDiagnosticV1::Engine(diagnostic),
        ),
    };
    let mut requests = Vec::with_capacity(workload.lanes.len());
    for lane in &workload.lanes {
        let request = match engine.admit() {
            Ok(request) => request,
            Err(diagnostic) => abandon_pre_physical_engine(
                "prefill request admission",
                engine,
                requests,
                Vec::new(),
                PrePhysicalPoolCustodyV1::Split {
                    _memory: memory,
                    _caches: Vec::new(),
                },
                input_tokens,
                PrePhysicalDiagnosticV1::Engine(diagnostic),
            ),
        };
        if let Err(diagnostic) = engine.append_tentative(request, lane.active_length) {
            requests.push(request);
            abandon_pre_physical_engine(
                "prefill Engine enqueue",
                engine,
                requests,
                Vec::new(),
                PrePhysicalPoolCustodyV1::Split {
                    _memory: memory,
                    _caches: Vec::new(),
                },
                input_tokens,
                PrePhysicalDiagnosticV1::Engine(diagnostic),
            );
        }
        requests.push(request);
    }
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled)) => scheduled,
        Ok(None) => abandon_pre_physical_engine(
            "prefill Engine dispatch returned no scheduler batch",
            engine,
            requests,
            Vec::new(),
            PrePhysicalPoolCustodyV1::Split {
                _memory: memory,
                _caches: Vec::new(),
            },
            input_tokens,
            PrePhysicalDiagnosticV1::MissingBatch,
        ),
        Err(diagnostic) => abandon_pre_physical_engine(
            "prefill Engine dispatch",
            engine,
            requests,
            Vec::new(),
            PrePhysicalPoolCustodyV1::Split {
                _memory: memory,
                _caches: Vec::new(),
            },
            input_tokens,
            PrePhysicalDiagnosticV1::Engine(diagnostic),
        ),
    };
    if let Err(diagnostic) = validate_scheduled_roster(&scheduled, &requests, 0) {
        abandon_prefill_initial_dispatch(
            "prefill scheduled roster validation",
            engine,
            memory,
            Vec::new(),
            requests,
            Vec::new(),
            Vec::new(),
            scheduled,
            PrefillInitialPhaseCustodyV1::Scheduled {
                _diagnostic: diagnostic,
            },
        );
    }
    let plans = match bind_qualification_step_plans(runner, &scheduled, selection, &requests, 0) {
        Ok(plans) => plans,
        Err(diagnostic) => abandon_prefill_initial_dispatch(
            "prefill step-plan binding",
            engine,
            memory,
            Vec::new(),
            requests,
            Vec::new(),
            Vec::new(),
            scheduled,
            PrefillInitialPhaseCustodyV1::Scheduled {
                _diagnostic: diagnostic,
            },
        ),
    };
    let inputs = match validated_inputs(workload, &plans, input_tokens, dimensions.active_tokens) {
        Ok(inputs) => inputs,
        Err(diagnostic) => abandon_prefill_initial_dispatch(
            "prefill input binding",
            engine,
            memory,
            Vec::new(),
            requests,
            Vec::new(),
            Vec::new(),
            scheduled,
            PrefillInitialPhaseCustodyV1::Input {
                _diagnostic: diagnostic,
                _plans: plans,
            },
        ),
    };
    let active_lengths = inputs.active_lengths().to_vec();
    let context_lengths = inputs.context_lengths().to_vec();
    let mut caches = Vec::with_capacity(requests.len());
    let mut expected_target_pages = Vec::with_capacity(requests.len());
    let mut reservations = Vec::with_capacity(requests.len());
    for ((request, active_length), context_length) in requests
        .iter()
        .copied()
        .zip(active_lengths.iter().copied())
        .zip(context_lengths.iter().copied())
    {
        let mut cache =
            match ActiveDeviceKvCache::new(memory.device(), request, selection, draft_selection) {
                Ok(cache) => cache,
                Err(diagnostic) => abandon_prefill_initial_dispatch(
                    "prefill device-KV cache construction",
                    engine,
                    memory,
                    caches,
                    requests,
                    active_lengths,
                    context_lengths,
                    scheduled,
                    PrefillInitialPhaseCustodyV1::CacheConstruction {
                        _diagnostic: diagnostic,
                        _plans: plans,
                        _inputs: inputs,
                        _reservations: reservations,
                    },
                ),
            };
        let pages = match qualification_kv_page_count(context_length, active_length) {
            Ok(pages) => pages,
            Err(diagnostic) => abandon_prefill_initial_dispatch(
                "prefill KV page-count derivation",
                engine,
                memory,
                caches,
                requests,
                active_lengths,
                context_lengths,
                scheduled,
                PrefillInitialPhaseCustodyV1::PageCount {
                    _diagnostic: diagnostic,
                    _plans: plans,
                    _inputs: inputs,
                    _current_cache: cache,
                    _reservations: reservations,
                },
            ),
        };
        let mut leases = Vec::with_capacity(pages as usize);
        for page in 0..pages {
            let lease = match memory.lease_page(request, Qwen3ModelRole::Target8B, page) {
                Ok(lease) => lease,
                Err(diagnostic) => abandon_prefill_initial_dispatch(
                    "prefill physical KV page lease",
                    engine,
                    memory,
                    caches,
                    requests,
                    active_lengths,
                    context_lengths,
                    scheduled,
                    PrefillInitialPhaseCustodyV1::PageLease {
                        _diagnostic: diagnostic,
                        _plans: plans,
                        _inputs: inputs,
                        _current_cache: cache,
                        _page_leases: leases,
                        _reservations: reservations,
                    },
                ),
            };
            leases.push(lease);
        }
        let reservation = match cache.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            context_length,
            active_length,
            scheduled.epoch(),
            leases,
        ) {
            Ok(reservation) => reservation,
            Err(failure) => abandon_prefill_initial_dispatch(
                "prefill KV step reservation",
                engine,
                memory,
                caches,
                requests,
                active_lengths,
                context_lengths,
                scheduled,
                PrefillInitialPhaseCustodyV1::StepReservation {
                    _failure: failure,
                    _plans: plans,
                    _inputs: inputs,
                    _current_cache: cache,
                    _reservations: reservations,
                },
            ),
        };
        expected_target_pages.push(pages as usize);
        reservations.push(reservation);
        caches.push(cache);
    }
    let table = match bind_m1_kv_workspace_table_v1(inputs, reservations) {
        Ok(table) => table,
        Err(failure) => abandon_prefill_initial_dispatch(
            "prefill KV workspace binding",
            engine,
            memory,
            caches,
            requests,
            active_lengths,
            context_lengths,
            scheduled,
            PrefillInitialPhaseCustodyV1::KvBinding {
                _plans: plans,
                _failure: failure,
            },
        ),
    };
    let workspace_identity = sha256_array(&workload.bytes);
    let workspace_plan = match workload_workspace_plan(selection, workspace_identity) {
        Ok(plan) => plan,
        Err(diagnostic) => abandon_prefill_initial_dispatch(
            "prefill workspace plan",
            engine,
            memory,
            caches,
            requests,
            active_lengths,
            context_lengths,
            scheduled,
            PrefillInitialPhaseCustodyV1::WorkspacePlan {
                _diagnostic: diagnostic,
                _plans: plans,
                _table: table,
            },
        ),
    };
    let recipe = match derive_qualification_recipe(runner, selection, workspace_identity) {
        Ok(recipe) => recipe,
        Err(diagnostic) => abandon_prefill_initial_dispatch(
            "prefill recipe derivation",
            engine,
            memory,
            caches,
            requests,
            active_lengths,
            context_lengths,
            scheduled,
            PrefillInitialPhaseCustodyV1::Recipe {
                _diagnostic: diagnostic,
                _plans: plans,
                _table: table,
                _workspace_plan: workspace_plan,
            },
        ),
    };
    let prepared = match prepare_scheduled_workspaces_with_retries(
        runner,
        scheduled,
        M1FullStepWorkspacePlans::target_only(workspace_plan),
        M1FullStepKvWorkspaceTablesV1::TargetOnly { target: table },
    ) {
        Ok(prepared) => prepared,
        Err(failure) => abandon_first_generation(
            "prefill workspace preparation retry exhaustion",
            engine,
            FirstGenerationLiveEvidenceV1::Prefill {
                _caches: caches,
                _requests: requests,
                _active_lengths: active_lengths,
                _context_lengths: context_lengths,
            },
            FirstGenerationPhaseFailureV1::Preparation {
                _memory: Box::new(memory),
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let mut allocated = match allocate_scheduled_workspaces_with_retries(runner, memory, prepared) {
        Ok(allocated) => allocated,
        Err(failure) => abandon_first_generation(
            "prefill allocation retry exhaustion",
            engine,
            FirstGenerationLiveEvidenceV1::Prefill {
                _caches: caches,
                _requests: requests,
                _active_lengths: active_lengths,
                _context_lengths: context_lengths,
            },
            FirstGenerationPhaseFailureV1::Allocation {
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let completion = match purpose {
        CapturePurposeV1::R30PartialCanary => {
            allocated.allocate_guarded_completion_output(selection)
        }
        CapturePurposeV1::Qualification | CapturePurposeV1::R30PartialCancellation => {
            allocated.allocate_completion_output(selection)
        }
    };
    let completion = match completion {
        Ok(completion) => completion,
        Err(diagnostic) => abandon_first_generation(
            "prefill compact completion allocation",
            engine,
            FirstGenerationLiveEvidenceV1::Prefill {
                _caches: caches,
                _requests: requests,
                _active_lengths: active_lengths,
                _context_lengths: context_lengths,
            },
            FirstGenerationPhaseFailureV1::CompletionOutput {
                _allocated: Box::new(allocated),
                _recipe: recipe,
                _diagnostic: diagnostic,
            },
        ),
    };
    let completion = match allocated.enable_qualification_logits_capture(completion) {
        Ok(completion) => completion,
        Err(failure) => abandon_first_generation(
            "prefill qualification logits allocation",
            engine,
            FirstGenerationLiveEvidenceV1::Prefill {
                _caches: caches,
                _requests: requests,
                _active_lengths: active_lengths,
                _context_lengths: context_lengths,
            },
            FirstGenerationPhaseFailureV1::QualificationLogits {
                _allocated: Box::new(allocated),
                _recipe: recipe,
                _failure: failure,
            },
        ),
    };
    let published =
        match publish_first_step_with_retries(runner, &mut engine, allocated, recipe, completion) {
            Ok(published) => published,
            Err(failure) => {
                let failure = failure.quarantine_after_retry_exhaustion(&mut engine);
                abandon_first_generation(
                    "prefill first publication retry exhaustion",
                    engine,
                    FirstGenerationLiveEvidenceV1::Prefill {
                        _caches: caches,
                        _requests: requests,
                        _active_lengths: active_lengths,
                        _context_lengths: context_lengths,
                    },
                    FirstGenerationPhaseFailureV1::Publication { _failure: failure },
                )
            }
        };
    let evidence = PrefillLiveEvidenceV1 {
        _caches: caches,
        requests,
        _active_lengths: active_lengths,
        _context_lengths: context_lengths,
    };
    let (qualified, evidence, device_id, precompletion_cancellation) =
        qualify_prefill_live_generation(&mut engine, published, evidence, purpose);
    let PrefillLiveEvidenceV1 {
        _caches: caches,
        requests,
        _active_lengths: _,
        _context_lengths: _,
    } = evidence;
    let (completed, evidence) = qualified.into_parts();
    let roster = M1DeviceKvCompletionRosterV1::new(
        caches
            .into_iter()
            .map(M1DeviceKvCompletionMemberV1::retiring)
            .collect(),
    );
    let completed = match complete_m1_physical_step_v1(&mut engine, completed, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        M1CompletedStepOutcomeV1::Rejected(rejected) => {
            let (_error, readback, roster) = rejected.into_parts();
            match complete_m1_physical_step_v1(&mut engine, readback, roster) {
                M1CompletedStepOutcomeV1::Completed(completed) => completed,
                M1CompletedStepOutcomeV1::Rejected(rejected) => {
                    close_or_quarantine_qualification_evidence(
                        "prefill completion rejected after retry",
                        evidence,
                        None,
                        rejected.destroy_queue_and_retain_rejection(&mut engine),
                    )
                }
                M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
                    "prefill completion retry entered terminal poison",
                    QualificationEvidenceCustodyV1 {
                        _evidence: evidence,
                        _diagnostic: None,
                        _custody: poison,
                    },
                ),
            }
        }
        M1CompletedStepOutcomeV1::Poisoned(poison) => terminal_quarantine(
            "prefill completion entered terminal poison",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: None,
                _custody: poison,
            },
        ),
    };
    let final_absent_count = if purpose == CapturePurposeV1::R30PartialCancellation {
        requests
            .iter()
            .filter(|request| engine.state(**request).is_none())
            .count()
    } else {
        0
    };
    let released = match release_first_completed_step(&mut engine, completed) {
        Ok(released) => released,
        Err(teardown) => {
            let FirstPageReleaseTeardownV1 { error, teardown } = *teardown;
            close_or_quarantine_qualification_evidence(
                "prefill page release rejected after retry",
                evidence,
                Some(QualificationEvidenceDiagnosticV1::PageRelease(error)),
                teardown,
            )
        }
    };
    let teardown = match released.destroy_queue_and_retain_step(&mut engine) {
        Ok(teardown) => teardown,
        Err(quarantine) => terminal_quarantine(
            "prefill final queue teardown",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: None,
                _custody: quarantine,
            },
        ),
    };
    if teardown.logical_accepted_counts().len() != requests.len()
        || teardown.externally_published_counts().len() != requests.len()
        || teardown
            .logical_accepted_counts()
            .iter()
            .any(|count| *count != 1)
        || teardown
            .externally_published_counts()
            .iter()
            .any(|count| *count != 1)
    {
        closed_teardown(
            "prefill qualification completion counts drifted",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: None,
                _custody: teardown,
            },
        );
    }
    if teardown.members().len() != requests.len()
        || teardown
            .members()
            .iter()
            .any(|member| matches!(member, ferric_engine::M1ReleasedDeviceKvMemberV1::Active(_)))
    {
        closed_teardown(
            "prefill teardown retained a nonterminal KV member",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: None,
                _custody: teardown,
            },
        );
    }
    let copied = match copy_capture_candidate(teardown.checked(), &evidence, workload.lanes.len()) {
        Ok(copied) => copied,
        Err(diagnostic) => closed_teardown(
            "prefill qualification output-copy rejection",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: Some(QualificationEvidenceDiagnosticV1::Message(diagnostic)),
                _custody: teardown,
            },
        ),
    };
    let epoch = teardown.checked().epoch().value();
    let dispatch_generation = teardown.checked().dispatch_generation();
    let expected_total_target_pages = match expected_target_pages
        .iter()
        .try_fold(0usize, |total, pages| total.checked_add(*pages))
    {
        Some(total) => total,
        None => closed_teardown(
            "prefill expected target-page total overflowed",
            QualificationEvidenceCustodyV1 {
                _evidence: evidence,
                _diagnostic: None,
                _custody: teardown,
            },
        ),
    };
    let cancellation_plan_id = if purpose == CapturePurposeV1::R30PartialCancellation {
        match teardown.checked().records() {
            [record] => Some(record.record().plan_id),
            _ => closed_teardown(
                "fixed cancellation checked roster drifted",
                QualificationEvidenceCustodyV1 {
                    _evidence: evidence,
                    _diagnostic: None,
                    _custody: teardown,
                },
            ),
        }
    } else {
        None
    };
    let settlement = precompletion_cancellation.map(|precompletion| {
        let terminal_members = teardown
            .members()
            .iter()
            .filter(|member| {
                matches!(
                    member,
                    ferric_engine::M1ReleasedDeviceKvMemberV1::Terminal(_)
                )
            })
            .count();
        let released_pages = teardown
            .release_counts()
            .iter()
            .map(|counts| (counts.draft(), counts.target()))
            .collect();
        m1_r30_partial_capture::CancellationSettlementV1 {
            checked_records: teardown.checked().records().len(),
            completed_members: teardown.completed_members(),
            dispatch_generation,
            epoch,
            expected_target_pages,
            expected_total_target_pages,
            externally_published_counts: teardown.externally_published_counts().to_vec(),
            final_absent_count,
            in_flight_count: precompletion.in_flight_count,
            logical_accepted_counts: teardown.logical_accepted_counts().to_vec(),
            plan_id: cancellation_plan_id
                .expect("partial cancellation always records the checked plan identity"),
            precompletion_reclaim_count: precompletion.precompletion_reclaim_count,
            released_pages,
            requests: precompletion.requests,
            retiring_count: precompletion.retiring_count,
            terminal_members,
            total_released_pages: teardown.total_released(),
        }
    });
    let r30_canary_closed = if purpose == CapturePurposeV1::R30PartialCanary {
        if teardown.checked().completion_canary().is_none() {
            closed_teardown(
                "guarded prefill lost checked canary summary",
                QualificationEvidenceCustodyV1 {
                    _evidence: evidence,
                    _diagnostic: None,
                    _custody: teardown,
                },
            );
        }
        Some(teardown)
    } else {
        if teardown.checked().completion_canary().is_some() {
            closed_teardown(
                "ordinary prefill acquired guarded completion summary",
                QualificationEvidenceCustodyV1 {
                    _evidence: evidence,
                    _diagnostic: None,
                    _custody: teardown,
                },
            );
        }
        None
    };
    CapturedOutput {
        compact_sha256: copied.compact_sha256,
        device_id,
        execution: CapturedExecutionV1::OneShotPrefill {
            dispatch_generation,
            epoch,
        },
        logits: copied.logits,
        logits_row_sha256: copied.logits_row_sha256,
        r30_canary_closed,
        settlement,
        tokens: copied.tokens,
    }
}

#[derive(Debug)]
struct CopiedCaptureCandidateV1 {
    compact_sha256: [u8; 32],
    logits: Vec<u8>,
    logits_row_sha256: Vec<[u8; 32]>,
    tokens: Vec<u8>,
}

fn copy_capture_candidate(
    checked: &ferric_engine::M1CheckedCompletionOutputV1,
    evidence: &M1QualificationCompletionEvidenceV1,
    expected_lanes: usize,
) -> CaptureResult<CopiedCaptureCandidateV1> {
    let records = checked.records();
    if records.len() != expected_lanes {
        return Err("compact live record count differs from workload lanes".to_owned());
    }
    if evidence.logits().rows().len() != records.len() {
        return Err("captured logits row count differs from compact records".to_owned());
    }
    let mut tokens = Vec::with_capacity(records.len() * 4);
    for (lane, record) in records.iter().enumerate() {
        if record.record().emitted_token_count != 1 || record.record().accepted_draft_tokens != 0 {
            return Err(format!(
                "lane {lane} compact target-only record is not exactly one emitted token"
            ));
        }
    }
    let mut logits = Vec::new();
    let row_bytes = usize::try_from(
        u64::from(QWEN3_VOCABULARY_SIZE)
            .checked_mul(BF16_BYTES)
            .ok_or_else(|| "logits row byte count overflowed".to_owned())?,
    )
    .map_err(|_| "logits row byte count does not fit usize".to_owned())?;
    logits
        .try_reserve_exact(row_bytes.saturating_mul(evidence.logits().rows().len()))
        .map_err(|_| "cannot reserve captured logits output".to_owned())?;
    let mut logits_row_sha256 = Vec::with_capacity(evidence.logits().rows().len());
    for (lane, (row, record)) in evidence.logits().rows().iter().zip(records).enumerate() {
        if row.lane() != lane || row.raw_bytes().len() != row_bytes {
            return Err(format!("captured logits row {lane} geometry drifted"));
        }
        let compact_choice = record.record().emitted_tokens[0];
        let choice = checked_bf16_row_choice(row.raw_bytes(), lane, compact_choice)?;
        tokens.extend_from_slice(&choice.to_le_bytes());
        logits.extend_from_slice(row.raw_bytes());
        logits_row_sha256.push(*row.raw_sha256());
    }
    let output = CopiedCaptureCandidateV1 {
        compact_sha256: *evidence.compact_raw_sha256(),
        logits,
        logits_row_sha256,
        tokens,
    };
    Ok(output)
}

fn checked_bf16_row_choice(bytes: &[u8], lane: usize, compact_choice: u32) -> CaptureResult<u32> {
    let choice = lowest_id_finite_bf16_argmax(bytes, lane)?;
    if choice != compact_choice {
        return Err(format!(
            "captured logits row {lane} argmax differs from checked compact choice"
        ));
    }
    Ok(choice)
}

fn lowest_id_finite_bf16_argmax(bytes: &[u8], lane: usize) -> CaptureResult<u32> {
    let expected = usize::try_from(u64::from(QWEN3_VOCABULARY_SIZE) * BF16_BYTES)
        .map_err(|_| "BF16 logits row extent does not fit usize".to_owned())?;
    if bytes.len() != expected {
        return Err(format!("captured logits row {lane} has an invalid extent"));
    }
    let mut best_token = 0_u32;
    let mut best_value = f32::NEG_INFINITY;
    for (token, encoded) in bytes.chunks_exact(2).enumerate() {
        let bits = u16::from_le_bytes([encoded[0], encoded[1]]);
        let value = f32::from_bits(u32::from(bits) << 16);
        if !value.is_finite() {
            return Err(format!(
                "captured logits row {lane} contains a non-finite BF16 value at token {token}"
            ));
        }
        if value > best_value {
            best_value = value;
            best_token = u32::try_from(token)
                .map_err(|_| "BF16 argmax token index does not fit u32".to_owned())?;
        }
    }
    Ok(best_token)
}

fn qualify_prefill_live_generation(
    engine: &mut Engine<32>,
    published: ferric_engine::M1PhysicalPublishedQueueSessionV1,
    evidence: PrefillLiveEvidenceV1,
    purpose: CapturePurposeV1,
) -> (
    ferric_engine::M1QualifiedPhysicalCompletedReadbackV1,
    PrefillLiveEvidenceV1,
    Identity,
    Option<m1_r30_partial_capture::PreCompletionCancellationV1>,
) {
    let mut partial_failure = None;
    let mut precompletion_cancellation = None;
    if purpose == CapturePurposeV1::R30PartialCancellation {
        if let Err(error) = preflight_engine_retirement(engine, &evidence.requests) {
            partial_failure = Some((
                "partial cancellation in-flight retirement preflight",
                PrefillLiveDiagnosticV1::Message(error),
            ));
        }
        if partial_failure.is_none() {
            for request in &evidence.requests {
                if let Err(error) = engine.retire(*request) {
                    partial_failure = Some((
                        "partial cancellation in-flight retirement",
                        PrefillLiveDiagnosticV1::Engine(error),
                    ));
                    break;
                }
            }
        }
        let retiring_count = if partial_failure.is_none() {
            evidence
                .requests
                .iter()
                .filter(|request| engine.state(**request) == Some(RequestState::Retiring))
                .count()
        } else {
            0
        };
        if partial_failure.is_none() && retiring_count != evidence.requests.len() {
            partial_failure = Some((
                "partial cancellation retirement state drift",
                PrefillLiveDiagnosticV1::Message(
                    "cancellation did not retain the exact retiring roster".to_owned(),
                ),
            ));
        }
        if partial_failure.is_none() {
            match engine.reclaim_one() {
                Ok(None) => {
                    precompletion_cancellation =
                        Some(m1_r30_partial_capture::PreCompletionCancellationV1 {
                            in_flight_count: evidence.requests.len(),
                            precompletion_reclaim_count: 0,
                            requests: evidence
                                .requests
                                .iter()
                                .copied()
                                .map(m1_r30_partial_capture::RequestIdentityV1::from)
                                .collect(),
                            retiring_count,
                        });
                }
                Ok(Some(_)) => {
                    partial_failure = Some((
                        "partial cancellation premature reclamation",
                        PrefillLiveDiagnosticV1::Message(
                            "a request reclaimed before physical completion observation".to_owned(),
                        ),
                    ));
                }
                Err(error) => {
                    partial_failure = Some((
                        "partial cancellation precompletion reclamation probe",
                        PrefillLiveDiagnosticV1::Engine(error),
                    ));
                }
            }
        }
    }
    let completed = match published.wait() {
        Ok(completed) => completed,
        Err(failure) => {
            report_physical_queue_failure("prefill queue wait terminal quarantine", &failure);
            terminal_quarantine(
                "prefill queue wait terminal quarantine",
                PrefillLiveCustodyV1 {
                    _evidence: evidence,
                    _diagnostic: None,
                    _custody: failure.quarantine_engine(engine),
                },
            )
        }
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => {
            report_physical_queue_failure("prefill queue recycle terminal quarantine", &failure);
            terminal_quarantine(
                "prefill queue recycle terminal quarantine",
                PrefillLiveCustodyV1 {
                    _evidence: evidence,
                    _diagnostic: None,
                    _custody: failure.quarantine_engine(engine),
                },
            )
        }
    };
    let device_id = recycled.custody().device().device_id();
    let observed = match recycled.observe_qualification_completion() {
        Ok(observed) => observed,
        Err(failure) => match (*failure).retry() {
            Ok(observed) => observed,
            Err(failure) => close_or_quarantine_prefill_live(
                "prefill qualification observation failed after retry",
                evidence,
                None,
                (*failure).destroy_queue_and_retain_evidence(engine),
            ),
        },
    };
    if let Some((phase, diagnostic)) = partial_failure {
        close_or_quarantine_prefill_live(
            phase,
            evidence,
            Some(diagnostic),
            observed.destroy_queue_and_retain_evidence(engine),
        );
    }
    if matches!(
        purpose,
        CapturePurposeV1::Qualification | CapturePurposeV1::R30PartialCanary
    ) {
        if let Err(error) = preflight_engine_retirement(engine, &evidence.requests) {
            close_or_quarantine_prefill_live(
                "prefill observed qualification retirement preflight",
                evidence,
                Some(PrefillLiveDiagnosticV1::Message(error)),
                observed.destroy_queue_and_retain_evidence(engine),
            );
        }
        for request in &evidence.requests {
            if let Err(error) = engine.retire(*request) {
                close_or_quarantine_prefill_live(
                    "prefill observed qualification retirement",
                    evidence,
                    Some(PrefillLiveDiagnosticV1::Engine(error)),
                    observed.destroy_queue_and_retain_evidence(engine),
                );
            }
        }
    }
    let qualified = match observed.check_prefill_completion() {
        Ok(qualified) => qualified,
        Err(failure) => match failure.retry_prefill_completion() {
            Ok(qualified) => qualified,
            Err(failure) => close_or_quarantine_prefill_live(
                "prefill qualification semantic join failed",
                evidence,
                None,
                failure.destroy_queue_and_retain_evidence(engine),
            ),
        },
    };
    (qualified, evidence, device_id, precompletion_cancellation)
}

fn require_supported_capture(workload: &Workload) -> CaptureResult<()> {
    match workload.selection.mode {
        Qwen3ExecutionMode::Prefill => Ok(()),
        Qwen3ExecutionMode::Decode => qualification_grouping(workload.selection).map(|_| ()),
        Qwen3ExecutionMode::Speculative => {
            Err("qualification capture does not admit speculative selections".to_owned())
        }
    }
}

fn qualification_kv_page_count(context: u32, active: u32) -> CaptureResult<u32> {
    let end = context
        .checked_add(active)
        .ok_or_else(|| "context extent overflowed".to_owned())?;
    if active == 0 || end == 0 {
        return Err("active KV extent must be nonzero".to_owned());
    }
    Ok(end.div_ceil(M1_KV_PAGE_TOKENS))
}

fn validated_inputs(
    workload: &Workload,
    plans: &[StepPlan],
    input_tokens: Vec<u32>,
    width: u32,
) -> CaptureResult<ValidatedM1StepInputs> {
    let width = usize::try_from(width).map_err(|_| "active width does not fit usize".to_owned())?;
    let rows = workload.lanes.len();
    let extent = rows
        .checked_mul(width)
        .ok_or_else(|| "fixed workload array extent overflowed".to_owned())?;
    let mut tokens = vec![0; extent];
    let mut positions = vec![0; extent];
    let mut input_offset = 0_usize;
    for (lane, lane_input) in workload.lanes.iter().copied().enumerate() {
        let active = lane_input.active_length as usize;
        let row = lane
            .checked_mul(width)
            .ok_or_else(|| "workload row offset overflowed".to_owned())?;
        let context = lane_input.context_length as usize;
        let active_start = input_offset
            .checked_add(context)
            .ok_or_else(|| "workload context input offset overflowed".to_owned())?;
        let source_end = active_start
            .checked_add(active)
            .ok_or_else(|| "workload input offset overflowed".to_owned())?;
        let source = input_tokens
            .get(active_start..source_end)
            .ok_or_else(|| "workload input token payload is truncated".to_owned())?;
        tokens[row..row + active].copy_from_slice(source);
        for active_index in 0..active {
            positions[row + active_index] = lane_input
                .context_length
                .checked_add(u32::try_from(active_index).unwrap_or(u32::MAX))
                .ok_or_else(|| format!("lane {lane} position overflowed"))?;
        }
        input_offset = source_end;
    }
    if input_offset != input_tokens.len() {
        return Err("workload input token payload has trailing tokens".to_owned());
    }
    let candidate = M1StepInputCandidate::new(
        workload.selection,
        plans.iter().copied().map(Some).collect(),
        tokens,
        positions,
        workload
            .lanes
            .iter()
            .map(|lane| lane.active_length)
            .collect(),
        workload
            .lanes
            .iter()
            .map(|lane| lane.context_length)
            .collect(),
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(rejection) => Err(format!(
            "workload step inputs were rejected: {:?}",
            rejection.error()
        )),
    }
}

fn workload_workspace_plan(
    selection: Qwen3PlanSelection,
    workload_identity: [u8; 32],
) -> CaptureResult<ferric_build::AddresslessM1StepWorkspacePlan> {
    let requirements = m1_step_workspace_requirements(selection)
        .map_err(|error| format!("cannot derive workspace requirements: {error:?}"))?;
    let identity = domain_identity(
        b"ferric.m1.qualification-workspace.v1",
        &[&workload_identity, selection_bytes(selection).as_slice()],
    );
    let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
        selection,
        DeclaredM1StepWorkspaceAllocation::new(
            identity,
            requirements.allocation_byte_len(),
            requirements.allocation_alignment(),
        ),
        requirements.ranges().to_vec().into_boxed_slice(),
    ));
    match plan_addressless_m1_step_workspace(selection, available) {
        M1StepWorkspacePlanOutcome::Planned(plan) => Ok(plan),
        M1StepWorkspacePlanOutcome::Rejected(error) => {
            Err(format!("workload workspace plan rejected: {error:?}"))
        }
    }
}

fn load_plan(path: &Path) -> CaptureResult<DifferentialPlan> {
    let (root, relative) = secure_parent(path, "benchmark plan")?;
    let (value, bytes) = root.read_canonical(&relative, "benchmark plan")?;
    parse_plan_document(&value, bytes)
}

fn parse_plan_document(value: &Value, bytes: Vec<u8>) -> CaptureResult<DifferentialPlan> {
    let object = exact_object(
        value,
        &[
            "authority",
            "cases",
            "format",
            "identities",
            "input_sha256",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "source_path",
            "suite",
            "target",
        ],
        "benchmark plan",
    )?;
    expect_string(object, "authority", "benchmark-run-plan-only")?;
    expect_string(object, "format", PLAN_FORMAT)?;
    expect_string(object, "milestone", "M1")?;
    expect_string(object, "nonclaim", DIFFERENTIAL_NONCLAIM)?;
    expect_string(object, "obligation_id", "m1.r29")?;
    expect_string(object, "path_id", "differential-bench")?;
    expect_string(object, "source_path", "benches/m1/differential.rs")?;
    expect_string(object, "suite", "differential")?;
    expect_string(object, "target", TARGET)?;
    let input_sha256 = string_field(object, "input_sha256")?.to_owned();
    require_sha256(&input_sha256)?;
    let identities = parse_identities(field(object, "identities")?)?;
    let cases = parse_cases(field(object, "cases")?)?;
    Ok(DifferentialPlan {
        bytes,
        cases,
        identities,
        input_sha256,
    })
}

fn parse_identities(value: &Value) -> CaptureResult<BTreeMap<String, String>> {
    let object = value
        .as_object()
        .ok_or_else(|| "benchmark identities must be an object".to_owned())?;
    let mut expected = COMMON_IDENTITIES.to_vec();
    expected.extend_from_slice(DIFFERENTIAL_IDENTITIES);
    expected.extend(
        DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES
            .iter()
            .map(|(_, identity)| *identity),
    );
    expected.sort_unstable();
    exact_keys(object, &expected, "benchmark identities")?;
    let mut identities = BTreeMap::new();
    for (name, value) in object {
        let identity = value
            .as_str()
            .ok_or_else(|| format!("benchmark identity {name} must be a string"))?;
        require_sha256(identity)?;
        identities.insert(name.clone(), identity.to_owned());
    }
    Ok(identities)
}

fn parse_cases(value: &Value) -> CaptureResult<Vec<PlanCase>> {
    let values = value
        .as_array()
        .ok_or_else(|| "benchmark plan cases must be an array".to_owned())?;
    if values.len() != DIFFERENTIAL_KINDS.len() {
        return Err("benchmark plan must contain exactly seven differential cases".to_owned());
    }
    let mut cases = Vec::with_capacity(values.len());
    let mut prior: Option<&str> = None;
    let mut kinds = BTreeSet::new();
    for value in values {
        let object = exact_object(
            value,
            &["id", "input_sha256", "kind", "workload_sha256"],
            "benchmark case",
        )?;
        let id = string_field(object, "id")?;
        require_safe_id(id, "benchmark case ID")?;
        if prior.is_some_and(|previous| previous >= id) {
            return Err("benchmark cases must be uniquely sorted by ID".to_owned());
        }
        prior = Some(id);
        let kind = string_field(object, "kind")?;
        if !DIFFERENTIAL_KINDS.contains(&kind) {
            return Err(format!("unknown differential case kind: {kind}"));
        }
        let input_sha256 = string_field(object, "input_sha256")?;
        let workload_sha256 = string_field(object, "workload_sha256")?;
        require_sha256(input_sha256)?;
        require_sha256(workload_sha256)?;
        kinds.insert(kind);
        cases.push(PlanCase {
            id: id.to_owned(),
            input_sha256: input_sha256.to_owned(),
            kind: kind.to_owned(),
            workload_sha256: workload_sha256.to_owned(),
        });
    }
    if kinds != DIFFERENTIAL_KINDS.iter().copied().collect() {
        return Err("benchmark plan case-kind roster drifted".to_owned());
    }
    Ok(cases)
}

fn load_roster(path: &Path, plan: &DifferentialPlan) -> CaptureResult<()> {
    let (root, relative) = secure_parent(path, "workload roster")?;
    let (value, bytes) = root.read_canonical(&relative, "workload roster")?;
    validate_roster_document(&value, &bytes, plan)
}

fn validate_roster_document(
    value: &Value,
    bytes: &[u8],
    plan: &DifferentialPlan,
) -> CaptureResult<()> {
    require_identity(
        plan.identity("workload-roster")?,
        &sha256_hex(bytes),
        "workload roster",
    )?;
    let object = exact_object(value, &["cases", "format", "suite"], "workload roster")?;
    expect_string(object, "format", ROSTER_FORMAT)?;
    expect_string(object, "suite", "differential")?;
    if parse_cases(field(object, "cases")?)? != plan.cases {
        return Err("workload roster differs from benchmark plan cases".to_owned());
    }
    Ok(())
}

fn load_workload(path: &Path, case: &PlanCase) -> CaptureResult<Workload> {
    let (root, relative) = secure_parent(path, "qualification workload")?;
    let (value, bytes) = root.read_canonical(&relative, "qualification workload")?;
    parse_workload_document(&value, bytes, case)
}

fn parse_workload_document(
    value: &Value,
    bytes: Vec<u8>,
    case: &PlanCase,
) -> CaptureResult<Workload> {
    require_identity(
        &case.workload_sha256,
        &sha256_hex(&bytes),
        "qualification workload",
    )?;
    let object = exact_object(
        value,
        &[
            "case_id",
            "completion_wait_policy",
            "format",
            "input",
            "kind",
            "lanes",
            "selection",
        ],
        "qualification workload",
    )?;
    expect_string(object, "format", WORKLOAD_FORMAT)?;
    expect_string(object, "case_id", &case.id)?;
    expect_string(object, "kind", &case.kind)?;
    validate_completion_wait_policy(field(object, "completion_wait_policy")?)?;
    let selection = kind_selection(&case.kind)?;
    validate_selection(field(object, "selection")?, selection)?;
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "qualification selection is not admitted".to_owned())?;
    let lanes = parse_lanes(field(object, "lanes")?, selection, dimensions.sequences)?;
    let input = exact_object(
        field(object, "input")?,
        &["bytes", "encoding", "path", "sha256"],
        "qualification input payload",
    )?;
    expect_string(input, "encoding", "u32-le")?;
    let input_path = PathBuf::from(string_field(input, "path")?);
    require_relative(&input_path, "qualification input payload")?;
    let input_sha256 = string_field(input, "sha256")?.to_owned();
    require_sha256(&input_sha256)?;
    require_identity(&case.input_sha256, &input_sha256, "case input payload")?;
    let input_bytes = integer_field(input, "bytes")?;
    let expected_tokens = lanes.iter().try_fold(0_u64, |count, lane| {
        let lane_tokens = u64::from(lane.context_length)
            .checked_add(u64::from(lane.active_length))
            .ok_or_else(|| "qualification lane token count overflowed".to_owned())?;
        count
            .checked_add(lane_tokens)
            .ok_or_else(|| "qualification input token count overflowed".to_owned())
    })?;
    let expected_bytes = expected_tokens
        .checked_mul(4)
        .ok_or_else(|| "qualification input byte count overflowed".to_owned())?;
    if input_bytes != expected_bytes {
        return Err("qualification input byte count differs from live lane widths".to_owned());
    }
    Ok(Workload {
        bytes,
        input_path,
        input_bytes,
        input_sha256,
        kind: case.kind.clone(),
        lanes,
        selection,
    })
}

fn parse_lanes(
    value: &Value,
    selection: Qwen3PlanSelection,
    expected: u32,
) -> CaptureResult<Vec<LaneInput>> {
    let values = value
        .as_array()
        .ok_or_else(|| "qualification lanes must be an array".to_owned())?;
    let mut lanes = Vec::with_capacity(values.len());
    for (lane, value) in values.iter().enumerate() {
        let object = exact_object(
            value,
            &["active_length", "context_length"],
            "qualification lane",
        )?;
        let active = u32::try_from(integer_field(object, "active_length")?)
            .map_err(|_| format!("lane {lane} active length does not fit u32"))?;
        let context = u32::try_from(integer_field(object, "context_length")?)
            .map_err(|_| format!("lane {lane} context length does not fit u32"))?;
        lanes.push(LaneInput {
            active_length: active,
            context_length: context,
        });
    }
    validate_lane_geometry(selection, &lanes, expected)?;
    Ok(lanes)
}

fn validate_workload_geometry(workload: &Workload) -> CaptureResult<()> {
    let dimensions = workload
        .selection
        .bucket
        .dimensions(workload.selection.role, workload.selection.mode)
        .ok_or_else(|| "qualification selection has no dimensions".to_owned())?;
    validate_lane_geometry(workload.selection, &workload.lanes, dimensions.sequences)
}

fn validate_lane_geometry(
    selection: Qwen3PlanSelection,
    lanes: &[LaneInput],
    expected: u32,
) -> CaptureResult<()> {
    if lanes.len() != expected as usize {
        return Err("qualification lane count differs from selected bucket".to_owned());
    }
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "qualification selection has no dimensions".to_owned())?;
    for (lane, input) in lanes.iter().copied().enumerate() {
        match selection.mode {
            Qwen3ExecutionMode::Prefill => {
                if input.active_length != dimensions.active_tokens || input.context_length != 0 {
                    return Err(format!(
                        "lane {lane} canonical prefill geometry requires the full declared active width at empty context"
                    ));
                }
            }
            Qwen3ExecutionMode::Decode => {
                if input.active_length != 1 || input.context_length != DECODE_CONTEXT_LENGTH {
                    return Err(format!(
                        "lane {lane} canonical c8192 decode geometry requires one active token after exactly 8191 committed context tokens"
                    ));
                }
            }
            Qwen3ExecutionMode::Speculative => {
                return Err("qualification capture accepts target-only modes only".to_owned());
            }
        }
    }
    Ok(())
}

fn validate_selection(value: &Value, expected: Qwen3PlanSelection) -> CaptureResult<()> {
    let object = exact_object(value, &["bucket", "mode", "role"], "workload selection")?;
    expect_string(object, "role", "target-8b")?;
    let mode = match expected.mode {
        Qwen3ExecutionMode::Prefill => "prefill",
        Qwen3ExecutionMode::Decode => "decode",
        Qwen3ExecutionMode::Speculative => "speculative",
    };
    expect_string(object, "mode", mode)?;
    expect_string(object, "bucket", bucket_name(expected.bucket))
}

fn load_input_tokens(
    workload_path: &Path,
    workload: &Workload,
    case: &PlanCase,
) -> CaptureResult<Vec<u32>> {
    let parent = workload_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let root = SecureDirectory::open(parent, "qualification workload parent")?;
    load_input_tokens_from_root(&root, workload, case)
}

fn load_input_tokens_from_root(
    root: &SecureDirectory,
    workload: &Workload,
    case: &PlanCase,
) -> CaptureResult<Vec<u32>> {
    let bytes = root.read_exact(
        &workload.input_path,
        workload.input_bytes,
        "qualification token payload",
    )?;
    parse_input_tokens(&bytes, workload, case)
}

fn parse_input_tokens(
    bytes: &[u8],
    workload: &Workload,
    case: &PlanCase,
) -> CaptureResult<Vec<u32>> {
    let actual = sha256_hex(bytes);
    require_identity(&workload.input_sha256, &actual, "workload input payload")?;
    require_identity(&case.input_sha256, &actual, "benchmark case input")?;
    let mut tokens = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let encoded: [u8; 4] = chunk
            .try_into()
            .map_err(|_| "qualification token payload ended inside a u32".to_owned())?;
        let token = u32::from_le_bytes(encoded);
        if token >= QWEN3_VOCABULARY_SIZE {
            return Err(format!(
                "qualification input token is out of range: {token}"
            ));
        }
        tokens.push(token);
    }
    Ok(tokens)
}

fn load_closure(path: &Path) -> CaptureResult<ClosureIdentities> {
    load_closure_with_bytes(path).map(|(identities, _)| identities)
}

fn load_closure_with_bytes(path: &Path) -> CaptureResult<(ClosureIdentities, Vec<u8>)> {
    let (root, relative) = secure_parent(path, "qualification closure")?;
    let (value, bytes) = root.read_canonical(&relative, "qualification closure")?;
    Ok((parse_closure_document(&value)?, bytes))
}

fn parse_closure_document(value: &Value) -> CaptureResult<ClosureIdentities> {
    let object = exact_object(
        value,
        &[
            "compiler",
            "compiler_configuration",
            "fe2o3_source",
            "ferric_source",
            "format",
            "kernel_abi_catalog",
            "kernel_proof_set",
            "qualification_protocol",
            "runtime_abi",
            "runtime_contract",
            "target_contract",
            "tcb_report",
            "validator_registry",
        ],
        "qualification closure",
    )?;
    expect_string(object, "format", CLOSURE_FORMAT)?;
    Ok(ClosureIdentities {
        compiler: identity_field(object, "compiler")?,
        compiler_configuration: identity_field(object, "compiler_configuration")?,
        fe2o3_source: identity_field(object, "fe2o3_source")?,
        ferric_source: identity_field(object, "ferric_source")?,
        kernel_abi_catalog: identity_field(object, "kernel_abi_catalog")?,
        kernel_proof_set: identity_field(object, "kernel_proof_set")?,
        qualification_protocol: identity_field(object, "qualification_protocol")?,
        runtime_abi: identity_field(object, "runtime_abi")?,
        runtime_contract: identity_field(object, "runtime_contract")?,
        target_contract: identity_field(object, "target_contract")?,
        tcb_report: identity_field(object, "tcb_report")?,
        validator_registry: identity_field(object, "validator_registry")?,
    })
}

fn complete_closure(
    closure: &ClosureIdentities,
    catalog: &ferric_build::SequentialPlanCatalog,
    executable_catalog: Identity,
) -> CaptureResult<ExternalIdentityClosureInputs> {
    let mut external = ExternalIdentityClosureInputs {
        ferric_source: closure.ferric_source,
        fe2o3_source: closure.fe2o3_source,
        compiler: closure.compiler,
        compiler_configuration: closure.compiler_configuration,
        target_contract: closure.target_contract,
        kernel_catalog: domain_identity(b"ferric.m1.pending-kernel-catalog.v1", &[b"pending"]),
        kernel_proof_set: closure.kernel_proof_set,
        kernel_abi_catalog: closure.kernel_abi_catalog,
        executable_catalog,
        runtime_contract: closure.runtime_contract,
        runtime_abi: closure.runtime_abi,
        generated_runner: expected_qwen3_gfx942_runner_source_identity(),
        validator_registry: closure.validator_registry,
        qualification_protocol: closure.qualification_protocol,
        tcb_report: closure.tcb_report,
    };
    external.kernel_catalog = expected_preliminary_kernel_catalog_identity(catalog, &external)
        .map_err(|error| format!("cannot derive kernel catalog identity: {error:?}"))?;
    Ok(external)
}

fn load_environment(path: &Path, gpu_unique_id: u64) -> CaptureResult<Vec<u8>> {
    let (root, relative) = secure_parent(path, "qualification environment")?;
    let (value, bytes) = root.read_canonical(&relative, "qualification environment")?;
    let actual_gpu_unique_id = parse_environment_document(&value)?;
    if actual_gpu_unique_id != gpu_unique_id {
        return Err("environment GPU unique ID differs from the selected device".to_owned());
    }
    Ok(bytes)
}

fn parse_environment_document(value: &Value) -> CaptureResult<u64> {
    let object = exact_object(
        value,
        &["format", "gpu_unique_id", "target"],
        "qualification environment",
    )?;
    expect_string(object, "format", ENVIRONMENT_FORMAT)?;
    expect_string(object, "target", TARGET)?;
    integer_field(object, "gpu_unique_id")
}

fn load_model_inputs(
    source: &SecureDirectory,
    snapshot: &SecureDirectory,
) -> CaptureResult<ModelInputBytes> {
    Ok(ModelInputBytes {
        admission_record: snapshot.read_exact(
            Path::new("bundle.admission.bin"),
            BUNDLE_ADMISSION_RECORD_BYTES as u64,
            "bundle admission record",
        )?,
        deployment_bundle: snapshot.read_exact(
            Path::new("deployment.bundle.bin"),
            CANONICAL_DEPLOYMENT_BUNDLE_BYTES as u64,
            "canonical deployment bundle",
        )?,
        draft_config: source.read_bounded(
            Path::new("draft/config.json"),
            METADATA_BYTES,
            "draft config",
        )?,
        draft_manifest: snapshot.read_exact(
            Path::new("draft.weights.manifest.bin"),
            u64::from(QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES),
            "draft weight manifest",
        )?,
        draft_tokenizer: source.read_bounded(
            Path::new("draft/tokenizer.json"),
            64 * 1_024 * 1_024,
            "draft tokenizer",
        )?,
        draft_tokenizer_metadata: source.read_bounded(
            Path::new("draft/tokenizer_config.json"),
            METADATA_BYTES,
            "draft tokenizer metadata",
        )?,
        draft_weights: snapshot
            .read_exact(
                Path::new("draft.weights.bin"),
                QWEN3_DRAFT_TENSOR_DATA_BYTES,
                "draft prepacked weights",
            )?
            .into_boxed_slice(),
        target_config: source.read_bounded(
            Path::new("target/config.json"),
            METADATA_BYTES,
            "target config",
        )?,
        target_manifest: snapshot.read_exact(
            Path::new("target.weights.manifest.bin"),
            u64::from(QWEN3_TARGET_PREPACKED_MANIFEST_BYTES),
            "target weight manifest",
        )?,
        target_tokenizer: source.read_bounded(
            Path::new("target/tokenizer.json"),
            64 * 1_024 * 1_024,
            "target tokenizer",
        )?,
        target_tokenizer_metadata: source.read_bounded(
            Path::new("target/tokenizer_config.json"),
            METADATA_BYTES,
            "target tokenizer metadata",
        )?,
        target_weights: snapshot
            .read_exact(
                Path::new("target.weights.bin"),
                QWEN3_TARGET_TENSOR_DATA_BYTES,
                "target prepacked weights",
            )?
            .into_boxed_slice(),
    })
}

fn authenticated_assets<'a>(
    target_config: &'a [u8],
    target_tokenizer_metadata: &'a [u8],
    draft_config: &'a [u8],
    draft_tokenizer_metadata: &'a [u8],
) -> AuthenticatedDeploymentAssets<'a> {
    AuthenticatedDeploymentAssets {
        target: AuthenticatedModelAssets {
            repository: TARGET_REPOSITORY,
            revision: TARGET_REVISION,
            config_json: target_config,
            tokenizer_metadata_json: target_tokenizer_metadata,
        },
        draft: AuthenticatedModelAssets {
            repository: DRAFT_REPOSITORY,
            revision: DRAFT_REVISION,
            config_json: draft_config,
            tokenizer_metadata_json: draft_tokenizer_metadata,
        },
        limits: EngineLimits {
            max_context_tokens: 8_192,
            max_active_sequences: 32,
            kv_page_tokens: 256,
            max_draft_tokens: 16,
        },
    }
}

fn validate_persisted_deployment(
    prepacked: &PrepackedDeploymentBundle,
    expected: &ferric_spec::DeploymentBundle,
    persisted: &[u8],
) -> CaptureResult<()> {
    if prepacked.deployment() != expected {
        return Err("reconstructed deployment differs from admission record".to_owned());
    }
    let canonical = encode_canonical_deployment_bundle(prepacked.deployment())
        .map_err(|error| format!("cannot encode reconstructed deployment: {error}"))?;
    if canonical.as_bytes() != persisted {
        return Err("persisted canonical deployment bytes differ".to_owned());
    }
    Ok(())
}

fn model_memory_plan(
    admission: AuthenticatedBundleAdmission,
) -> CaptureResult<ferric_build::AddresslessModelMemoryPlan> {
    let deployment = *admission.prepacked().deployment();
    let target_manifest = admission.prepacked().target_manifest().aggregate_id();
    let draft_manifest = admission.prepacked().draft_manifest().aggregate_id();
    let layout = build_authenticated_model_weight_layout(admission)
        .map_err(|error| format!("cannot build authenticated model layout: {error:?}"))?;
    let target_kv = domain_identity(
        b"ferric.m1.target-kv-allocation.v1",
        &[deployment.bundle_id.as_bytes()],
    );
    let draft_kv = domain_identity(
        b"ferric.m1.draft-kv-allocation.v1",
        &[deployment.bundle_id.as_bytes()],
    );
    let declarations = ModelMemoryAllocationSet::new(
        DeclaredDeviceAllocation::new(
            Identity::new(target_manifest),
            QWEN3_TARGET_TENSOR_DATA_BYTES,
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            Identity::new(draft_manifest),
            QWEN3_DRAFT_TENSOR_DATA_BYTES,
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            target_kv,
            qwen3_kv_arena_bytes(Qwen3ModelRole::Target8B),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            draft_kv,
            qwen3_kv_arena_bytes(Qwen3ModelRole::Draft06B),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
    );
    match plan_authenticated_model_memory(layout, declarations) {
        ModelMemoryPlanOutcome::Planned(plan) => Ok(plan),
        ModelMemoryPlanOutcome::Rejected(error) => Err(format!(
            "authenticated model memory plan rejected: {error:?}"
        )),
    }
}

fn validate_plan_identities(
    plan: &DifferentialPlan,
    case: &PlanCase,
    closure: &ClosureIdentities,
    declaration: &ferric_build::GeneratedRunnerDeclaration,
    deployment: &ferric_spec::DeploymentBundle,
    model: &ModelInputBytes,
) -> CaptureResult<()> {
    require_identity(
        plan.identity("ferric-source-closure")?,
        &hex_identity(closure.ferric_source),
        "Ferric source closure",
    )?;
    require_identity(
        plan.identity("fe2o3-source-closure")?,
        &hex_identity(closure.fe2o3_source),
        "fe2o3 source closure",
    )?;
    require_identity(
        plan.identity("benchmark-protocol")?,
        &hex_identity(closure.qualification_protocol),
        "qualification protocol",
    )?;
    require_identity(
        plan.identity("model")?,
        &hex_identity(deployment.bundle_id),
        "deployment bundle",
    )?;
    require_identity(
        plan.identity("generated-plan")?,
        &hex_identity(declaration.declaration_id()),
        "generated runner declaration",
    )?;
    require_identity(
        plan.identity("schedule-catalog")?,
        &hex_identity(declaration.kernel_catalog_id()),
        "kernel schedule catalog",
    )?;
    let selection = kind_selection(&case.kind)?;
    let selected = declaration
        .plans()
        .iter()
        .find(|candidate| candidate.selection == selection)
        .ok_or_else(|| "generated declaration lacks selected workload plan".to_owned())?;
    validate_dispatch_graph_identities(
        plan,
        case,
        declaration.plan_catalog_id(),
        selected.plan_id,
    )?;
    let config = aggregate_identity(
        b"ferric.m1.deployment-configs.v1",
        &[
            deployment.target_model.config.config_id,
            deployment.draft_model.config.config_id,
        ],
    );
    require_identity(
        plan.identity("config")?,
        &hex_identity(config),
        "deployment configs",
    )?;
    let tokenizer = aggregate_identity(
        b"ferric.m1.deployment-tokenizers.v1",
        &[
            deployment.target_model.tokenizer.tokenizer_id,
            deployment.target_model.tokenizer.vocabulary_id,
            deployment.draft_model.tokenizer.tokenizer_id,
            deployment.draft_model.tokenizer.vocabulary_id,
        ],
    );
    require_identity(
        plan.identity("tokenizer")?,
        &hex_identity(tokenizer),
        "deployment tokenizers",
    )?;
    let weights = domain_identity(
        b"ferric.m1.deployment-prepacked-weights.v1",
        &[
            &sha256_array(&model.target_manifest),
            &sha256_array(&model.draft_manifest),
            &sha256_array(&model.target_weights),
            &sha256_array(&model.draft_weights),
        ],
    );
    require_identity(
        plan.identity("weights")?,
        &hex_identity(weights),
        "prepacked deployment weights",
    )?;
    Ok(())
}

fn validate_dispatch_graph_identities(
    plan: &DifferentialPlan,
    case: &PlanCase,
    plan_catalog_id: Identity,
    selected_plan_id: Identity,
) -> CaptureResult<()> {
    require_identity(
        plan.identity("dispatch-graph")?,
        &hex_identity(plan_catalog_id),
        "generated dispatch graph catalog",
    )?;
    require_identity(
        plan.identity(dispatch_graph_identity_name(&case.kind)?)?,
        &hex_identity(selected_plan_id),
        "selected dispatch graph",
    )
}

fn capture_transcript(
    plan: &DifferentialPlan,
    case: &PlanCase,
    workload: &Workload,
    capture: &CapturedOutput,
    identities: CaptureIdentities,
) -> CaptureResult<Vec<u8>> {
    let row_hashes = capture
        .logits_row_sha256
        .iter()
        .map(|digest| hex_bytes(digest))
        .collect::<Vec<_>>();
    let (dispatch_generation, execution) = match &capture.execution {
        CapturedExecutionV1::OneShotPrefill {
            dispatch_generation,
            epoch,
        } => (
            *dispatch_generation,
            json!({
                "dispatch_generation": dispatch_generation,
                "epoch": epoch,
                "mode": "one-shot-prefill",
                "round_count": 1,
            }),
        ),
        CapturedExecutionV1::C8192 {
            execution_binding,
            first_dispatch_generation,
            first_epoch,
            qualification_plan_id,
            round_count,
            round_history_sha256,
            terminal_dispatch_generation,
            terminal_epoch,
        } => {
            let ordered_lanes = execution_binding
                .ordered_lanes
                .iter()
                .map(|lane| {
                    json!({
                        "lane_identity_sha256": hex_identity(lane.lane_identity),
                        "lane_ordinal": lane.lane_ordinal,
                        "token_sequence_identity_sha256": hex_identity(lane.token_sequence_identity),
                    })
                })
                .collect::<Vec<_>>();
            (
                *terminal_dispatch_generation,
                json!({
                    "context_plan_sha256": hex_identity(*qualification_plan_id),
                    "declared_workload_binding_sha256": hex_identity(execution_binding.declared_workload_digest),
                    "first_dispatch_generation": first_dispatch_generation,
                    "first_epoch": first_epoch,
                    "mode": "teacher-forced-c8192",
                    "ordered_lane_bindings": ordered_lanes,
                    "round_count": round_count,
                    "round_history_sha256": hex_bytes(round_history_sha256),
                    "terminal_dispatch_generation": terminal_dispatch_generation,
                    "terminal_epoch": terminal_epoch,
                    "terminal_ordinal": M1_QUALIFICATION_FINAL_INPUT_TOKEN,
                }),
            )
        }
    };
    canonical_bytes(&json!({
        "authority": "observed-target-only-qualification-capture",
        "benchmark_executable_sha256": plan.identity("benchmark-executable")?,
        "benchmark_protocol_sha256": plan.identity("benchmark-protocol")?,
        "case_id": case.id,
        "compact_sha256": hex_bytes(&capture.compact_sha256),
        "device_identity_sha256": hex_identity(capture.device_id),
        "dispatch_generation": dispatch_generation,
        "environment_sha256": plan.identity("environment")?,
        "execution": execution,
        "format": TRANSCRIPT_FORMAT,
        "gpu_unique_id": identities.gpu_unique_id,
        "input_sha256": case.input_sha256,
        "kernel_artifact_manifest_sha256": hex_identity(identities.kernel_manifest),
        "kind": workload.kind,
        "logits_row_sha256": row_hashes,
        "logits_sha256": sha256_hex(&capture.logits),
        "nonclaim": "Observed bytes only; this transcript does not establish a reference comparison, tolerance, numerical correctness, hardware correctness, performance, qualification, or m1.r29 closure.",
        "plan_sha256": plan.sha256(),
        "program_catalog_sha256": hex_identity(identities.program_catalog),
        "runner_declaration_sha256": hex_identity(identities.runner_declaration),
        "selection": selection_json(workload.selection),
        "status": "OBSERVED",
        "target": TARGET,
        "tokens_sha256": sha256_hex(&capture.tokens),
        "workload_sha256": sha256_hex(&workload.bytes),
    }))
}

fn differential_output_manifest(
    plan: &DifferentialPlan,
    case: &PlanCase,
    logits: &[u8],
    tokens: &[u8],
    transcript_sha256: &str,
) -> CaptureResult<Vec<u8>> {
    let rows = rows_for_kind(&case.kind)?;
    let logits_bytes = rows
        .checked_mul(u64::from(QWEN3_VOCABULARY_SIZE))
        .and_then(|values| values.checked_mul(BF16_BYTES))
        .ok_or_else(|| "output logits extent overflowed".to_owned())?;
    if usize::try_from(logits_bytes).ok() != Some(logits.len()) {
        return Err("captured logits extent differs from producer contract".to_owned());
    }
    if usize::try_from(rows.saturating_mul(4)).ok() != Some(tokens.len()) {
        return Err("captured token extent differs from producer contract".to_owned());
    }
    canonical_bytes(&json!({
        "authority": "externally-collected-model-output-only",
        "case_id": case.id,
        "environment_sha256": plan.identity("environment")?,
        "format": OUTPUT_FORMAT,
        "input_sha256": case.input_sha256,
        "kind": case.kind,
        "logits": {
            "bytes": logits_bytes,
            "encoding": "bf16-le",
            "path": "logits.bf16le",
            "sha256": sha256_hex(logits),
        },
        "plan_sha256": plan.sha256(),
        "producer": "ferric",
        "producer_sha256": plan.identity("benchmark-executable")?,
        "protocol_sha256": plan.identity("benchmark-protocol")?,
        "runner_transcript_sha256": transcript_sha256,
        "shape": {
            "rows": rows,
            "vocabulary_size": QWEN3_VOCABULARY_SIZE,
        },
        "tokens": {
            "bytes": rows * 4,
            "encoding": "u32-le",
            "path": "tokens.u32le",
            "sha256": sha256_hex(tokens),
        },
        "workload_sha256": case.workload_sha256,
    }))
}

fn kind_selection(kind: &str) -> CaptureResult<Qwen3PlanSelection> {
    let (mode, bucket) = match kind {
        "decode-s1-c8192" => (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
        "decode-s8-c8192" => (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
        "decode-s32-c8192" => (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
        "prefill-s1-t128" => (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
        "prefill-s8-t128" => (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
        "prefill-s1-t512" => (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
        "prefill-s1-t2048" => (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
        _ => return Err(format!("unsupported differential case kind: {kind}")),
    };
    Ok(Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode,
        bucket,
    })
}

fn dispatch_graph_identity_name(kind: &str) -> CaptureResult<&'static str> {
    DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES
        .iter()
        .find_map(|(candidate, identity)| (*candidate == kind).then_some(*identity))
        .ok_or_else(|| format!("unsupported differential case kind: {kind}"))
}

fn rows_for_kind(kind: &str) -> CaptureResult<u64> {
    kind_selection(kind)?
        .bucket
        .dimensions(Qwen3ModelRole::Target8B, kind_selection(kind)?.mode)
        .map(|dimensions| u64::from(dimensions.sequences))
        .ok_or_else(|| "case kind has no target dimensions".to_owned())
}

fn bucket_name(bucket: Qwen3PlanBucket) -> &'static str {
    match bucket {
        Qwen3PlanBucket::PrefillS1T128 => "prefill-s1-t128",
        Qwen3PlanBucket::PrefillS8T128 => "prefill-s8-t128",
        Qwen3PlanBucket::PrefillS1T512 => "prefill-s1-t512",
        Qwen3PlanBucket::PrefillS1T2048 => "prefill-s1-t2048",
        Qwen3PlanBucket::DecodeS1C8192 => "decode-s1-c8192",
        Qwen3PlanBucket::DecodeS8C8192 => "decode-s8-c8192",
        Qwen3PlanBucket::DecodeS32C8192 => "decode-s32-c8192",
        Qwen3PlanBucket::SpeculativeS1K4C8192 => "speculative-s1-k4-c8192",
        Qwen3PlanBucket::SpeculativeS8K4C8192 => "speculative-s8-k4-c8192",
        Qwen3PlanBucket::SpeculativeS1K8C8192 => "speculative-s1-k8-c8192",
        Qwen3PlanBucket::SpeculativeS1K16C8192 => "speculative-s1-k16-c8192",
    }
}

fn selection_json(selection: Qwen3PlanSelection) -> Value {
    json!({
        "bucket": bucket_name(selection.bucket),
        "mode": match selection.mode {
            Qwen3ExecutionMode::Prefill => "prefill",
            Qwen3ExecutionMode::Decode => "decode",
            Qwen3ExecutionMode::Speculative => "speculative",
        },
        "role": "target-8b",
    })
}

fn selection_bytes(selection: Qwen3PlanSelection) -> Vec<u8> {
    format!(
        "target-8b\0{}\0{}",
        match selection.mode {
            Qwen3ExecutionMode::Prefill => "prefill",
            Qwen3ExecutionMode::Decode => "decode",
            Qwen3ExecutionMode::Speculative => "speculative",
        },
        bucket_name(selection.bucket)
    )
    .into_bytes()
}

fn current_executable_sha256() -> CaptureResult<String> {
    // `/proc/self/exe` is the deliberate magic-link exception: opening it binds
    // the descriptor to the inode executing this process, even if its pathname
    // is concurrently replaced. All reads and both metadata checks use that fd.
    let file = File::open("/proc/self/exe")
        .map_err(|error| format!("cannot open running benchmark executable: {error}"))?;
    let initial = fstat(&file)
        .map_err(|error| format!("cannot inspect running benchmark executable: {error}"))?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
        return Err("running benchmark executable must be a regular file".to_owned());
    }
    if initial.st_nlink != 1 {
        return Err(
            "running benchmark executable must have exactly one filesystem link".to_owned(),
        );
    }
    let mut executable = SecureFile { file, initial };
    let length = executable.length("running benchmark executable")?;
    if length == 0 {
        return Err("running benchmark executable must not be empty".to_owned());
    }
    let bytes = executable.read_exact_snapshot(length, "running benchmark executable")?;
    Ok(sha256_hex(&bytes))
}

fn secure_parent(path: &Path, description: &str) -> CaptureResult<(SecureDirectory, PathBuf)> {
    let relative = path
        .file_name()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{description} path has no file name"))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok((SecureDirectory::open(parent, description)?, relative))
}

fn path_exists_at(parent: &OwnedFd, name: &OsStr) -> CaptureResult<bool> {
    match openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(_) => Ok(true),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(false),
        Err(error) => Err(format!("cannot safely inspect output path: {error}")),
    }
}

fn require_relative(path: &Path, description: &str) -> CaptureResult<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description} path must be a safe relative path"));
    }
    Ok(())
}

fn same_file_snapshot(initial: &Stat, final_stat: &Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

fn parse_canonical(bytes: &[u8], description: &str) -> CaptureResult<Value> {
    if !bytes.is_ascii() {
        return Err(format!("{description} must be ASCII JSON"));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse {description}: {error}"))?;
    if canonical_bytes(&value)? != bytes {
        return Err(format!("{description} is not canonical JSON"));
    }
    Ok(value)
}

fn canonical_bytes(value: &Value) -> CaptureResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot serialize canonical JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> CaptureResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    exact_keys(object, expected, description)?;
    Ok(object)
}

fn exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    description: &str,
) -> CaptureResult<()> {
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!("{description} field roster drifted"));
    }
    Ok(())
}

fn field<'a>(object: &'a Map<String, Value>, name: &str) -> CaptureResult<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| format!("required field is absent: {name}"))
}

fn string_field<'a>(object: &'a Map<String, Value>, name: &str) -> CaptureResult<&'a str> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| format!("field {name} must be a string"))
}

fn integer_field(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("field {name} must be a nonnegative integer"))
}

fn identity_field(object: &Map<String, Value>, name: &str) -> CaptureResult<Identity> {
    decode_identity(string_field(object, name)?)
}

fn expect_string(object: &Map<String, Value>, name: &str, expected: &str) -> CaptureResult<()> {
    if string_field(object, name)? != expected {
        return Err(format!("field {name} has an unexpected value"));
    }
    Ok(())
}

fn require_sha256(value: &str) -> CaptureResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == b'0')
    {
        return Err("invalid lowercase SHA-256 identity".to_owned());
    }
    Ok(())
}

fn require_safe_id(value: &str, description: &str) -> CaptureResult<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("{description} is not a safe identifier"));
    }
    Ok(())
}

fn require_identity(expected: &str, actual: &str, description: &str) -> CaptureResult<()> {
    if expected != actual {
        return Err(format!("{description} SHA-256 identity drifted"));
    }
    Ok(())
}

fn decode_identity(value: &str) -> CaptureResult<Identity> {
    require_sha256(value)?;
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_digit(pair[0]).ok_or_else(|| "invalid SHA-256 identity".to_owned())?;
        let low = hex_digit(pair[1]).ok_or_else(|| "invalid SHA-256 identity".to_owned())?;
        bytes[index] = (high << 4) | low;
    }
    Ok(Identity::new(bytes))
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn sha256_array(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(&sha256_array(bytes))
}

fn hex_identity(identity: Identity) -> String {
    hex_bytes(identity.as_bytes())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn domain_identity(domain: &[u8], fields: &[&[u8]]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    for field in fields {
        hash_field(&mut hasher, field);
    }
    Identity::new(hasher.finalize().into())
}

fn aggregate_identity(domain: &[u8], identities: &[Identity]) -> Identity {
    let fields = identities
        .iter()
        .map(|identity| identity.as_bytes().as_slice())
        .collect::<Vec<_>>();
    domain_identity(domain, &fields)
}

fn hash_field(hasher: &mut Sha256, field: &[u8]) {
    hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(field);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::RequestId;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-qualification-capture-test.{}.{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn canonical(value: Value) -> Vec<u8> {
        canonical_bytes(&value).unwrap()
    }

    fn digest(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn workload_value(kind: &str, case_id: &str, lanes: usize) -> Value {
        let selection = kind_selection(kind).unwrap();
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .unwrap();
        let context_length = match selection.mode {
            Qwen3ExecutionMode::Decode => DECODE_CONTEXT_LENGTH,
            Qwen3ExecutionMode::Prefill => 0,
            Qwen3ExecutionMode::Speculative => unreachable!(),
        };
        let active_length = dimensions.active_tokens;
        json!({
            "case_id": case_id,
            "completion_wait_policy": completion_wait_policy_contract(),
            "format": WORKLOAD_FORMAT,
            "input": {
                "bytes": lanes * usize::try_from(context_length + active_length).unwrap() * 4,
                "encoding": "u32-le",
                "path": "tokens.u32le",
                "sha256": digest("input"),
            },
            "kind": kind,
            "lanes": (0..lanes).map(|_| json!({
                "active_length": active_length,
                "context_length": context_length,
            })).collect::<Vec<_>>(),
            "selection": selection_json(kind_selection(kind).unwrap()),
        })
    }

    #[test]
    fn input_subcommands_do_not_reserve_legacy_eleven_argument_plan_names() {
        let new_mode_error = run(vec![OsString::from("generate-inputs")]).unwrap_err();
        assert!(new_mode_error
            .starts_with("usage: ferric-m1-qualification-capture generate-inputs MODEL-SOURCE"));

        let mut legacy = vec![OsString::from("unused"); 11];
        legacy[0] = OsString::from("generate-inputs");
        let legacy_error = run(legacy).unwrap_err();
        assert!(!legacy_error.contains("generate-inputs MODEL-SOURCE"));

        let r30_error = run(vec![OsString::from(m1_r30_partial_capture::COMMAND)]).unwrap_err();
        assert!(r30_error.contains("capture-r30-cancellation MODEL-SOURCE"));
        assert!(!r30_error.contains("PLAN ROSTER"));

        let composed_error =
            run(vec![OsString::from(m1_r30_capture_composition::COMMAND)]).unwrap_err();
        assert!(composed_error.contains("compose-r30-runner CANARY-BUNDLE"));

        let mut legacy = vec![OsString::from("unused"); 11];
        legacy[0] = OsString::from(m1_r30_partial_capture::COMMAND);
        let legacy_error = run(legacy).unwrap_err();
        assert!(legacy_error.contains("capture-r30-cancellation MODEL-SOURCE"));
        assert!(!legacy_error.contains("PLAN ROSTER"));

        let canary_error =
            run(vec![OsString::from(m1_r30_canary_partial_capture::COMMAND)]).unwrap_err();
        assert!(canary_error.contains("capture-r30-canary MODEL-SOURCE"));

        let mut canary_wrong_width = vec![OsString::from("unused"); 11];
        canary_wrong_width[0] = OsString::from(m1_r30_canary_partial_capture::COMMAND);
        let canary_error = run(canary_wrong_width).unwrap_err();
        assert!(canary_error.contains("capture-r30-canary MODEL-SOURCE"));

        let rollback_error = run(vec![OsString::from(
            m1_r30_rollback_partial_capture::COMMAND,
        )])
        .unwrap_err();
        assert!(rollback_error.contains("capture-r30-rollback MODEL-SOURCE"));

        let mut wrong_width = vec![OsString::from("unused"); 11];
        wrong_width[0] = OsString::from(m1_r30_rollback_partial_capture::COMMAND);
        let rollback_error = run(wrong_width).unwrap_err();
        assert!(rollback_error.contains("capture-r30-rollback MODEL-SOURCE"));

        let exhaustion_error = run(vec![OsString::from(
            m1_r30_exhaustion_partial_capture::COMMAND,
        )])
        .unwrap_err();
        assert!(exhaustion_error.contains("capture-r30-exhaustion MODEL-SOURCE"));

        let mut exhaustion_wrong_width = vec![OsString::from("unused"); 11];
        exhaustion_wrong_width[0] = OsString::from(m1_r30_exhaustion_partial_capture::COMMAND);
        let exhaustion_error = run(exhaustion_wrong_width).unwrap_err();
        assert!(exhaustion_error.contains("capture-r30-exhaustion MODEL-SOURCE"));

        let r32_error = run(vec![OsString::from(m1_r32_partial_capture::COMMAND)]).unwrap_err();
        assert!(r32_error.contains("capture-r32-speculative-k4 MODEL-SOURCE"));

        let mut wrong_width = vec![OsString::from("unused"); 11];
        wrong_width[0] = OsString::from(m1_r32_partial_capture::COMMAND);
        let r32_error = run(wrong_width).unwrap_err();
        assert!(r32_error.contains("capture-r32-speculative-k4 MODEL-SOURCE"));
    }

    #[test]
    fn r30_canary_workload_binds_exact_canonical_bytes_path_and_payload_hash() {
        let (workload, tokens) = fixed_r30_canary_workload().unwrap();
        let input = fixed_r30_prefill_input_bytes();
        assert_eq!(tokens, vec![R30_PREFILL_INPUT_TOKEN; 128]);
        assert_eq!(
            workload.input_path,
            Path::new("frozen-r30-canary-input-u32le")
        );
        assert_eq!(workload.input_bytes, R30_PREFILL_INPUT_BYTES);
        assert_eq!(
            workload.input_sha256,
            "d585e10d1e2240e9af79fc1cf8d11e11420b5306480b469b587e85630fcb0c9f"
        );
        assert_eq!(workload.input_sha256, sha256_hex(&input));
        assert_eq!(
            workload.bytes,
            canonical(json!({
                "active_length": R30_PREFILL_ACTIVE_TOKENS,
                "case": "target-prefill-s1-t128",
                "completion_wait_policy": completion_wait_policy_contract(),
                "context_length": 0,
                "format": "FERRIC-M1-R30-CANARY-WORKLOAD-V4",
                "input_bytes": R30_PREFILL_INPUT_BYTES,
                "input_token": R30_PREFILL_INPUT_TOKEN,
                "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
                "lane_count": 1,
                "selection": "target-prefill-s1-t128",
            }))
        );
        assert_eq!(
            sha256_hex(&workload.bytes),
            "a50aa2bd495fbd936cd15ff82f351f398bce38c1317750a8ee020305d1e93b7b"
        );
    }

    #[test]
    fn r30_cancellation_workload_is_fixed_and_policy_independent() {
        let (workload, tokens) = fixed_r30_cancellation_workload().unwrap();
        assert_eq!(tokens, vec![R30_PREFILL_INPUT_TOKEN; 128]);
        assert_eq!(workload.input_bytes, R30_PREFILL_INPUT_BYTES);
        assert_eq!(
            workload.input_sha256,
            sha256_hex(&fixed_r30_prefill_input_bytes())
        );
        assert_eq!(workload.lanes.len(), 1);
        assert_eq!(workload.lanes[0].context_length, 0);
        assert_eq!(workload.lanes[0].active_length, R30_PREFILL_ACTIVE_TOKENS);
        assert_eq!(workload.selection.role, Qwen3ModelRole::Target8B);
        assert_eq!(workload.selection.mode, Qwen3ExecutionMode::Prefill);
        assert_eq!(workload.selection.bucket, Qwen3PlanBucket::PrefillS1T128);
        assert_eq!(
            workload.bytes,
            canonical(json!({
                "active_length": R30_PREFILL_ACTIVE_TOKENS,
                "case": "target-prefill-s1-t128-retirement-before-observation",
                "context_length": 0,
                "completion_wait_policy": completion_wait_policy_contract(),
                "format": "FERRIC-M1-R30-CANCELLATION-WORKLOAD-V5",
                "input_bytes": R30_PREFILL_INPUT_BYTES,
                "input_token": R30_PREFILL_INPUT_TOKEN,
                "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
                "lane_count": 1,
                "selection": "target-prefill-s1-t128",
            }))
        );
    }

    #[test]
    fn completion_wait_policy_is_exact_and_rejects_caller_control() {
        validate_completion_wait_policy(&completion_wait_policy_contract()).unwrap();
        let mutations: &[fn(&mut Value)] = &[
            |value| value["id"] = json!("other"),
            |value| value["max_consecutive_scans_without_progress"] = json!(8191),
            |value| value["minimum_pending_scan_pause_micros"] = json!(9_999),
            |value| value["timeout_basis"] = json!("wall-clock"),
            |value| value["total_scan_bound_rule"] = json!("caller-selected"),
            |value| value["caller_override"] = json!(100_000_000),
        ];
        for mutate in mutations {
            let mut policy = completion_wait_policy_contract();
            mutate(&mut policy);
            assert!(validate_completion_wait_policy(&policy).is_err());
        }
    }

    #[test]
    fn direct_r30_workloads_are_full_width_and_cannot_bypass_geometry_validation() {
        for (workload, tokens) in [
            fixed_r30_canary_workload().unwrap(),
            fixed_r30_cancellation_workload().unwrap(),
        ] {
            assert_eq!(tokens.len(), R30_PREFILL_ACTIVE_TOKENS as usize);
            assert!(tokens.iter().all(|token| *token == R30_PREFILL_INPUT_TOKEN));
            assert_eq!(workload.input_bytes, R30_PREFILL_INPUT_BYTES);
            assert_eq!(workload.lanes[0].active_length, R30_PREFILL_ACTIVE_TOKENS);
            validate_workload_geometry(&workload).unwrap();

            let mut partial = workload;
            partial.lanes[0].active_length = 1;
            let error = validate_workload_geometry(&partial).unwrap_err();
            assert!(error.contains("full declared active width"));
        }
    }

    #[test]
    fn live_capture_failure_paths_use_exact_typed_consumers() {
        type FirstCompletionConsumer = fn(
            &mut Engine<32>,
            M1PhysicalRunnerFirstCompletionOutcomeV1,
            &[CompletionWireSemanticExpectation<'_>],
        ) -> ferric_engine::M1ReleasedCompletedStepV1;

        fn assert_terminal<T: CaptureTerminalCustodyV1>() {}
        fn assert_closed<T: CaptureClosedCustodyV1>() {}

        assert_terminal::<ferric_engine::M1LongLivedQueueRearmKvReservationFailureV1>();
        assert_terminal::<Box<ferric_engine::M1LongLivedQueueRearmPrepareFailureV1>>();
        assert_terminal::<Box<ferric_engine::M1RearmedQueueProgressFailureV1>>();
        assert_terminal::<ferric_engine::M1EngineQuarantinedPhysicalQueueOperationFailureV1>();
        assert_closed::<ferric_engine::M1CompletionEvidenceTeardownSuccessV1>();
        let _: FirstCompletionConsumer = consume_first_completion_outcome;
    }

    #[test]
    fn bounded_retry_policy_preserves_owner_and_attempts_exact_limit() {
        use std::cell::Cell;

        let attempts = Cell::new(0_usize);
        let owner = Box::new(Identity::new([94; 32]));
        let pointer = core::ptr::from_ref(owner.as_ref());
        let owner = retry_with_bounded_policy(owner, CAPTURE_RECOVERY_RETRIES, |owner| {
            attempts.set(attempts.get() + 1);
            Err::<(), _>(owner)
        })
        .unwrap_err();
        assert_eq!(attempts.get(), CAPTURE_RECOVERY_RETRIES);
        assert_eq!(core::ptr::from_ref(owner.as_ref()), pointer);
        assert_eq!(*owner, Identity::new([94; 32]));

        let attempts = Cell::new(0_usize);
        let owner = retry_with_bounded_policy(owner, CAPTURE_RECOVERY_RETRIES, |owner| {
            attempts.set(attempts.get() + 1);
            if attempts.get() == CAPTURE_RECOVERY_RETRIES {
                Ok(owner)
            } else {
                Err(owner)
            }
        })
        .unwrap();
        assert_eq!(attempts.get(), CAPTURE_RECOVERY_RETRIES);
        assert_eq!(core::ptr::from_ref(owner.as_ref()), pointer);
    }

    #[test]
    fn bf16_argmax_is_finite_and_uses_lowest_token_id() {
        let mut row = vec![0_u8; usize::try_from(u64::from(QWEN3_VOCABULARY_SIZE) * 2).unwrap()];
        for encoded in row.chunks_exact_mut(2) {
            encoded.copy_from_slice(&(((-2.0_f32).to_bits() >> 16) as u16).to_le_bytes());
        }
        let maximum = (((-1.0_f32).to_bits() >> 16) as u16).to_le_bytes();
        row[4 * 2..4 * 2 + 2].copy_from_slice(&maximum);
        row[7 * 2..7 * 2 + 2].copy_from_slice(&maximum);
        assert_eq!(lowest_id_finite_bf16_argmax(&row, 0).unwrap(), 4);
        assert_eq!(checked_bf16_row_choice(&row, 0, 4).unwrap(), 4);
        assert!(checked_bf16_row_choice(&row, 0, 7).is_err());

        row[4 * 2..4 * 2 + 2].copy_from_slice(&0x8000_u16.to_le_bytes());
        row[7 * 2..7 * 2 + 2].copy_from_slice(&0_u16.to_le_bytes());
        assert_eq!(lowest_id_finite_bf16_argmax(&row, 0).unwrap(), 4);

        row[9 * 2..9 * 2 + 2].copy_from_slice(&0x7fc0_u16.to_le_bytes());
        assert!(lowest_id_finite_bf16_argmax(&row, 0).is_err());
    }

    #[test]
    fn qualification_engine_capacity_covers_every_lane_full_context() {
        assert_eq!(
            qualification_engine_page_capacity(M1QualificationLaneGrouping::S1).unwrap(),
            32
        );
        assert_eq!(
            qualification_engine_page_capacity(M1QualificationLaneGrouping::S8).unwrap(),
            256
        );
        assert_eq!(
            qualification_engine_page_capacity(M1QualificationLaneGrouping::S32).unwrap(),
            1_024
        );
    }

    #[test]
    fn running_executable_hash_reads_the_live_proc_inode() {
        let expected = sha256_hex(&fs::read("/proc/self/exe").unwrap());
        assert_eq!(current_executable_sha256().unwrap(), expected);
    }

    #[test]
    fn canonical_parser_rejects_noncanonical_json() {
        let value = json!({"format": ENVIRONMENT_FORMAT, "gpu_unique_id": 7, "target": TARGET});
        let bytes = canonical(value);
        assert!(parse_canonical(&bytes, "test").is_ok());
        let compact = b"{\"format\":\"FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1\"}\n";
        assert!(parse_canonical(compact, "test").is_err());
    }

    #[test]
    fn every_differential_kind_maps_to_exact_target_geometry() {
        assert_eq!(
            DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES
                .iter()
                .map(|(kind, _)| *kind)
                .collect::<Vec<_>>(),
            DIFFERENTIAL_KINDS
        );
        for kind in DIFFERENTIAL_KINDS {
            let selection = kind_selection(kind).unwrap();
            assert_eq!(
                dispatch_graph_identity_name(kind).unwrap(),
                format!("dispatch-graph-{kind}")
            );
            assert_eq!(selection.role, Qwen3ModelRole::Target8B);
            assert_eq!(
                rows_for_kind(kind).unwrap(),
                u64::from(
                    selection
                        .bucket
                        .dimensions(selection.role, selection.mode)
                        .unwrap()
                        .sequences
                )
            );
        }
    }

    #[test]
    fn one_plan_binds_catalog_and_every_selected_dispatch_graph() {
        let catalog_id = Identity::new([1; 32]);
        let mut identities =
            BTreeMap::from([("dispatch-graph".to_owned(), hex_identity(catalog_id))]);
        let mut cases = Vec::new();
        let mut selected = Vec::new();
        for (index, (kind, identity_name)) in
            DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES.iter().enumerate()
        {
            let plan_id = Identity::new([u8::try_from(index + 2).unwrap(); 32]);
            identities.insert((*identity_name).to_owned(), hex_identity(plan_id));
            cases.push(PlanCase {
                id: format!("{kind}.001"),
                input_sha256: digest(&format!("{kind}:input")),
                kind: (*kind).to_owned(),
                workload_sha256: digest(&format!("{kind}:workload")),
            });
            selected.push(plan_id);
        }
        let plan = DifferentialPlan {
            bytes: canonical(json!({"plan": "fixture"})),
            cases,
            identities,
            input_sha256: digest("benchmark input"),
        };

        for (case, selected_plan_id) in plan.cases.iter().zip(selected) {
            validate_dispatch_graph_identities(&plan, case, catalog_id, selected_plan_id).unwrap();
        }

        let first = &plan.cases[0];
        let first_plan_id = Identity::new([2; 32]);
        let mut wrong_catalog = plan.identities.clone();
        wrong_catalog.insert("dispatch-graph".to_owned(), digest("wrong catalog"));
        let wrong_catalog = DifferentialPlan {
            bytes: plan.bytes.clone(),
            cases: plan.cases.clone(),
            identities: wrong_catalog,
            input_sha256: plan.input_sha256.clone(),
        };
        assert!(validate_dispatch_graph_identities(
            &wrong_catalog,
            first,
            catalog_id,
            first_plan_id,
        )
        .is_err());

        let first_identity = dispatch_graph_identity_name(&first.kind).unwrap();
        let mut missing_selected = plan.identities.clone();
        missing_selected.remove(first_identity);
        let missing_selected = DifferentialPlan {
            bytes: plan.bytes.clone(),
            cases: plan.cases.clone(),
            identities: missing_selected,
            input_sha256: plan.input_sha256.clone(),
        };
        assert!(validate_dispatch_graph_identities(
            &missing_selected,
            first,
            catalog_id,
            first_plan_id,
        )
        .is_err());

        let mut wrong_selected = plan.identities.clone();
        wrong_selected.insert(first_identity.to_owned(), digest("wrong selected plan"));
        let wrong_selected = DifferentialPlan {
            bytes: plan.bytes.clone(),
            cases: plan.cases.clone(),
            identities: wrong_selected,
            input_sha256: plan.input_sha256.clone(),
        };
        assert!(validate_dispatch_graph_identities(
            &wrong_selected,
            first,
            catalog_id,
            first_plan_id,
        )
        .is_err());
    }

    #[test]
    fn decode_workload_requires_full_authenticated_context() {
        let case = PlanCase {
            id: "decode.001".to_owned(),
            input_sha256: digest("input"),
            kind: "decode-s1-c8192".to_owned(),
            workload_sha256: digest("placeholder"),
        };
        let mut value = workload_value(&case.kind, &case.id, 1);
        value["lanes"][0]["context_length"] = json!(0);
        let bytes = canonical(value.clone());
        let root = exact_object(
            &value,
            &[
                "case_id",
                "completion_wait_policy",
                "format",
                "input",
                "kind",
                "lanes",
                "selection",
            ],
            "workload",
        )
        .unwrap();
        let selection = kind_selection(&case.kind).unwrap();
        assert!(parse_lanes(field(root, "lanes").unwrap(), selection, 1).is_err());
        value["lanes"][0]["context_length"] = json!(DECODE_CONTEXT_LENGTH);
        let root = exact_object(
            &value,
            &[
                "case_id",
                "completion_wait_policy",
                "format",
                "input",
                "kind",
                "lanes",
                "selection",
            ],
            "workload",
        )
        .unwrap();
        assert_eq!(
            parse_lanes(field(root, "lanes").unwrap(), selection, 1).unwrap(),
            vec![LaneInput {
                active_length: 1,
                context_length: DECODE_CONTEXT_LENGTH,
            }]
        );
        assert!(!bytes.is_empty());
    }

    #[test]
    fn qualification_kv_leases_follow_the_exact_p16_contract() {
        validate_r30_prefill_page_contract().unwrap();
        assert_eq!(
            usize::try_from(qualification_kv_page_count(0, 128).unwrap()).unwrap(),
            R30_PREFILL_TARGET_PAGES
        );
        for (context, active, expected_pages) in [
            (0, 128, 8),
            (0, 512, 32),
            (0, 2_048, 128),
            (8_191, 1, 512),
            (15, 1, 1),
            (16, 1, 2),
        ] {
            assert_eq!(
                qualification_kv_page_count(context, active).unwrap(),
                expected_pages
            );
        }
        assert!(qualification_kv_page_count(0, 0).is_err());
        assert!(qualification_kv_page_count(u32::MAX, 1).is_err());
    }

    #[test]
    fn prefill_workload_requires_full_declared_width() {
        let selection = kind_selection("prefill-s1-t512").unwrap();
        let partial = json!([{"active_length": 511, "context_length": 0}]);
        assert!(parse_lanes(&partial, selection, 1).is_err());
        let full = json!([{"active_length": 512, "context_length": 0}]);
        assert_eq!(
            parse_lanes(&full, selection, 1).unwrap(),
            vec![LaneInput {
                active_length: 512,
                context_length: 0,
            }]
        );
    }

    #[test]
    fn validated_inputs_preserve_full_prefill_rows() {
        let selection = kind_selection("prefill-s8-t128").unwrap();
        let mut workload = Workload {
            bytes: Vec::new(),
            input_path: PathBuf::from("tokens.u32le"),
            input_bytes: 8 * 128 * 4,
            input_sha256: digest("tokens"),
            kind: "prefill-s8-t128".to_owned(),
            lanes: vec![
                LaneInput {
                    active_length: 128,
                    context_length: 0
                };
                8
            ],
            selection,
        };
        workload.bytes = canonical(workload_value(&workload.kind, "prefill.001", 8));
        let plans = (0..8)
            .map(|slot| {
                StepPlan::new(
                    RequestId::new(slot, 1),
                    ferric_spec::completion::CompletionEpoch::new(1),
                    Identity::new([7; 32]),
                    selection,
                )
            })
            .collect::<Vec<_>>();
        let inputs = validated_inputs(&workload, &plans, vec![3; 8 * 128], 128).unwrap();
        assert_eq!(inputs.live_lane_count(), 8);
        for lane in 0..8 {
            let row = lane * 128;
            assert!(inputs.token_ids()[row..row + 128]
                .iter()
                .all(|token| *token == 3));
        }
    }

    #[test]
    fn decode_steps_use_exact_lane_major_prompt_ordinals() {
        let selection = kind_selection("decode-s1-c8192").unwrap();
        let workload = Workload {
            bytes: canonical(workload_value("decode-s1-c8192", "decode.001", 1)),
            input_path: PathBuf::from("tokens.u32le"),
            input_bytes: 8_192 * 4,
            input_sha256: digest("tokens"),
            kind: "decode-s1-c8192".to_owned(),
            lanes: vec![LaneInput {
                active_length: 1,
                context_length: DECODE_CONTEXT_LENGTH,
            }],
            selection,
        };
        let plan = StepPlan::new(
            RequestId::new(0, 1),
            ferric_spec::completion::CompletionEpoch::new(1),
            Identity::new([7; 32]),
            selection,
        );
        let mut tokens = vec![3; 8_192];
        tokens[0] = 11;
        tokens[8_191] = 17;
        let initial = qualification_step_inputs(&workload, &[plan], &tokens, 0).unwrap();
        assert_eq!(initial.token_ids(), &[11]);
        assert_eq!(initial.position_ids(), &[0]);
        assert_eq!(initial.context_lengths(), &[0]);
        let terminal =
            qualification_step_inputs(&workload, &[plan], &tokens, DECODE_CONTEXT_LENGTH).unwrap();
        assert_eq!(terminal.token_ids(), &[17]);
        assert_eq!(terminal.position_ids(), &[DECODE_CONTEXT_LENGTH]);
        assert_eq!(terminal.context_lengths(), &[DECODE_CONTEXT_LENGTH]);
        assert_eq!(require_supported_capture(&workload), Ok(()));
    }

    #[test]
    fn qualification_binding_authenticates_workload_selection_lane_order_and_tokens() {
        let selection = kind_selection("decode-s8-c8192").unwrap();
        let mut workload = Workload {
            bytes: canonical(workload_value("decode-s8-c8192", "decode.008", 8)),
            input_path: PathBuf::from("tokens.u32le"),
            input_bytes: 8 * 8_192 * 4,
            input_sha256: digest("tokens"),
            kind: "decode-s8-c8192".to_owned(),
            lanes: vec![
                LaneInput {
                    active_length: 1,
                    context_length: DECODE_CONTEXT_LENGTH,
                };
                8
            ],
            selection,
        };
        let mut tokens = (0..8 * 8_192)
            .map(|index| u32::try_from(index).unwrap() % QWEN3_VOCABULARY_SIZE)
            .collect::<Vec<_>>();
        let original = qualification_execution_binding(&workload, &tokens).unwrap();
        let repeated = qualification_execution_binding(&workload, &tokens).unwrap();
        assert_eq!(original.grouping, M1QualificationLaneGrouping::S8);
        assert_eq!(original.declaration, repeated.declaration);
        assert_eq!(original.declaration.ordered_lanes.len(), 8);

        tokens[8_192 + 17] ^= 1;
        let token_mutation = qualification_execution_binding(&workload, &tokens).unwrap();
        assert_ne!(
            original.declaration.ordered_lanes[1].token_sequence_identity,
            token_mutation.declaration.ordered_lanes[1].token_sequence_identity
        );
        assert_eq!(
            original.declaration.ordered_lanes[0].token_sequence_identity,
            token_mutation.declaration.ordered_lanes[0].token_sequence_identity
        );

        workload.bytes.push(b' ');
        let workload_mutation = qualification_execution_binding(&workload, &tokens).unwrap();
        assert_ne!(
            original.declaration.declared_workload_digest,
            workload_mutation.declaration.declared_workload_digest
        );
        assert_ne!(
            original.declaration.ordered_lanes[0].lane_identity,
            workload_mutation.declaration.ordered_lanes[0].lane_identity
        );
    }

    #[test]
    fn qualification_lane_major_input_rejects_truncation_trailing_and_reorder() {
        let selection = kind_selection("decode-s8-c8192").unwrap();
        let workload = Workload {
            bytes: canonical(workload_value("decode-s8-c8192", "decode.008", 8)),
            input_path: PathBuf::from("tokens.u32le"),
            input_bytes: 8 * 8_192 * 4,
            input_sha256: digest("tokens"),
            kind: "decode-s8-c8192".to_owned(),
            lanes: vec![
                LaneInput {
                    active_length: 1,
                    context_length: DECODE_CONTEXT_LENGTH,
                };
                8
            ],
            selection,
        };
        let requests = (0..8)
            .map(|slot| RequestId::new(slot, 1))
            .collect::<Vec<_>>();
        let plans = requests
            .iter()
            .map(|request| {
                StepPlan::new(
                    *request,
                    ferric_spec::completion::CompletionEpoch::new(1),
                    Identity::new([7; 32]),
                    selection,
                )
            })
            .collect::<Vec<_>>();
        let mut tokens = vec![0; 8 * 8_192];
        for lane in 0..8 {
            tokens[lane * 8_192 + 41] = u32::try_from(lane + 101).unwrap();
        }
        let inputs = qualification_step_inputs(&workload, &plans, &tokens, 41).unwrap();
        assert_eq!(
            inputs.token_ids(),
            &[101, 102, 103, 104, 105, 106, 107, 108]
        );
        assert!(
            qualification_step_inputs(&workload, &plans, &tokens[..tokens.len() - 1], 41).is_err()
        );
        tokens.push(9);
        assert!(qualification_step_inputs(&workload, &plans, &tokens, 41).is_err());

        let mut engine = Engine::<8>::new(16, 8, 128).unwrap();
        let mut live = Vec::new();
        for _ in 0..8 {
            let request = engine.admit().unwrap();
            engine.append_tentative(request, 1).unwrap();
            live.push(request);
        }
        let scheduled = engine.dispatch_m1_ready().unwrap().unwrap();
        let mut hostile = live.clone();
        hostile.swap(0, 1);
        assert!(validate_scheduled_roster(&scheduled, &hostile, 41).is_err());
    }

    #[test]
    fn qualification_completion_policy_is_terminal_only() {
        assert_eq!(validate_round_counts(0, &[1; 8], &[0; 8], 8, false), Ok(()));
        assert_eq!(
            validate_round_counts(DECODE_CONTEXT_LENGTH, &[1; 8], &[1; 8], 8, true),
            Ok(())
        );
        assert!(validate_round_counts(17, &[1; 8], &[1; 8], 8, false).is_err());
        assert!(validate_round_counts(DECODE_CONTEXT_LENGTH, &[1; 8], &[0; 8], 8, true).is_err());
    }

    #[test]
    fn c8192_transcript_binds_exact_plan_lanes_and_round_receipts() {
        let selection = kind_selection("decode-s1-c8192").unwrap();
        let workload = Workload {
            bytes: canonical(workload_value("decode-s1-c8192", "decode.001", 1)),
            input_path: PathBuf::from("tokens.u32le"),
            input_bytes: 8_192 * 4,
            input_sha256: digest("tokens"),
            kind: "decode-s1-c8192".to_owned(),
            lanes: vec![LaneInput {
                active_length: 1,
                context_length: DECODE_CONTEXT_LENGTH,
            }],
            selection,
        };
        let binding = qualification_execution_binding(&workload, &vec![3; 8_192])
            .unwrap()
            .declaration;
        let case = PlanCase {
            id: "decode.001".to_owned(),
            input_sha256: digest("input"),
            kind: workload.kind.clone(),
            workload_sha256: sha256_hex(&workload.bytes),
        };
        let plan = DifferentialPlan {
            bytes: canonical(json!({"plan": "fixture"})),
            cases: vec![case.clone()],
            identities: BTreeMap::from([
                ("benchmark-executable".to_owned(), digest("executable")),
                ("benchmark-protocol".to_owned(), digest("protocol")),
                ("environment".to_owned(), digest("environment")),
            ]),
            input_sha256: digest("benchmark input"),
        };
        let capture = CapturedOutput {
            compact_sha256: [7; 32],
            device_id: Identity::new([8; 32]),
            execution: CapturedExecutionV1::C8192 {
                execution_binding: binding.clone(),
                first_dispatch_generation: 11,
                first_epoch: 17,
                qualification_plan_id: Identity::new([9; 32]),
                round_count: 8_192,
                round_history_sha256: [10; 32],
                terminal_dispatch_generation: 8_202,
                terminal_epoch: 8_208,
            },
            logits: vec![0; QWEN3_VOCABULARY_SIZE as usize * 2],
            logits_row_sha256: vec![[12; 32]],
            r30_canary_closed: None,
            settlement: None,
            tokens: 0_u32.to_le_bytes().to_vec(),
        };
        let transcript = capture_transcript(
            &plan,
            &case,
            &workload,
            &capture,
            CaptureIdentities {
                gpu_unique_id: 23,
                runner_declaration: Identity::new([13; 32]),
                kernel_manifest: Identity::new([14; 32]),
                program_catalog: Identity::new([15; 32]),
            },
        )
        .unwrap();
        let value = parse_canonical(&transcript, "transcript").unwrap();
        assert_eq!(value["format"], TRANSCRIPT_FORMAT);
        assert_eq!(value["execution"]["mode"], "teacher-forced-c8192");
        assert_eq!(value["execution"]["round_count"], 8_192);
        assert_eq!(value["execution"]["terminal_ordinal"], 8_191);
        assert_eq!(value["execution"]["first_epoch"], 17);
        assert_eq!(value["execution"]["terminal_epoch"], 8_208);
        assert_eq!(value["input_sha256"], case.input_sha256);
        assert_eq!(value["benchmark_executable_sha256"], digest("executable"));
        assert_eq!(value["benchmark_protocol_sha256"], digest("protocol"));
        assert_eq!(value["environment_sha256"], digest("environment"));
        assert_eq!(
            value["execution"]["declared_workload_binding_sha256"],
            hex_identity(binding.declared_workload_digest)
        );
        assert_eq!(
            value["execution"]["ordered_lane_bindings"][0]["token_sequence_identity_sha256"],
            hex_identity(binding.ordered_lanes[0].token_sequence_identity)
        );
    }

    #[test]
    fn bare_input_and_output_paths_use_current_directory() {
        let bare_workload = Path::new("workload.json");
        let bare_output = Path::new("capture.bundle");
        let workload_parent = bare_workload
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let output_parent = bare_output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        assert_eq!(workload_parent, Path::new("."));
        assert_eq!(output_parent, Path::new("."));
    }

    #[test]
    fn staged_output_failures_are_cleanup_and_retry_safe() {
        let temporary = TestDirectory::new();
        let output = temporary.0.join("capture.bundle");

        {
            let mut staging = StagingOutput::create(&output).unwrap();
            staging.write("payload", b"first\n").unwrap();
            assert!(staging.write("payload", b"duplicate\n").is_err());
        }
        assert!(fs::read_dir(&temporary.0).unwrap().next().is_none());

        {
            let mut staging = StagingOutput::create(&output).unwrap();
            assert!(staging
                .write_with("payload", |_| Err(std::io::Error::other("injected")))
                .is_err());
        }
        assert!(fs::read_dir(&temporary.0).unwrap().next().is_none());

        let mut retry = StagingOutput::create(&output).unwrap();
        retry.write("payload", b"retry\n").unwrap();
        retry.publish().unwrap();
        assert_eq!(fs::read(output.join("payload")).unwrap(), b"retry\n");
        assert!(StagingOutput::create(&output).is_err());
    }

    #[test]
    fn staged_output_refuses_name_substitution_without_deleting_substitute() {
        let temporary = TestDirectory::new();
        let output = temporary.0.join("capture.bundle");
        let mut staging = StagingOutput::create(&output).unwrap();
        staging.write("payload", b"held\n").unwrap();
        let original_name = staging.staging_name.clone();
        let original = temporary.0.join(&original_name);
        let displaced = temporary.0.join("displaced.staging");
        fs::rename(&original, &displaced).unwrap();
        fs::create_dir(&original).unwrap();
        fs::write(original.join("substitute"), b"do not delete\n").unwrap();

        assert!(staging.publish().is_err());
        assert_eq!(
            fs::read(original.join("substitute")).unwrap(),
            b"do not delete\n"
        );
        assert!(fs::read_dir(displaced).unwrap().next().is_none());
    }

    #[test]
    fn producer_manifest_has_exact_payload_contract() {
        let identities = COMMON_IDENTITIES
            .iter()
            .chain([
                &"differential-acceptance-policy",
                &"reference-implementation",
                &"reference-protocol",
            ])
            .map(|name| ((*name).to_owned(), digest(name)))
            .collect();
        let case = PlanCase {
            id: "decode.001".to_owned(),
            input_sha256: digest("input"),
            kind: "decode-s1-c8192".to_owned(),
            workload_sha256: digest("workload"),
        };
        let plan = DifferentialPlan {
            bytes: canonical(json!({"plan": "fixture"})),
            cases: vec![case.clone()],
            identities,
            input_sha256: digest("benchmark input"),
        };
        let logits = vec![0_u8; QWEN3_VOCABULARY_SIZE as usize * 2];
        let tokens = 0_u32.to_le_bytes();
        let bytes =
            differential_output_manifest(&plan, &case, &logits, &tokens, &digest("transcript"))
                .unwrap();
        let value = parse_canonical(&bytes, "manifest").unwrap();
        assert_eq!(value["format"], OUTPUT_FORMAT);
        assert_eq!(value["shape"]["rows"], 1);
        assert_eq!(value["logits"]["encoding"], "bf16-le");
        assert_eq!(value["tokens"]["encoding"], "u32-le");
    }
}
