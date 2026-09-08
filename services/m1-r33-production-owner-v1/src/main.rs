//! Externally supervised owner for one bounded authenticated R33 instance.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use fe2o3_host::{
    InheritedWorkerV3CompilerCurrentRecordAuditorV1, WorkerV3ProtectedRosterVerifierAdapterV1,
};
use fe2o3_kfd::{DeviceSelector, OpenedKfd};
use ferric_build::{
    AuthenticatedBundleAdmission, AuthenticatedDeploymentAssets, AuthenticatedModelAssets,
    AvailableM1StepWorkspace, BUNDLE_ADMISSION_RECORD_BYTES, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    DRAFT_REPOSITORY, DRAFT_REVISION, DeclaredDeviceAllocation, DeclaredM1StepWorkspaceAllocation,
    ExternalIdentityClosureInputs, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
    ModelMemoryAllocationSet, ModelMemoryPlanOutcome, PrepackedDeploymentBundle,
    QWEN3_DRAFT_CONFIG_BYTES, QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1, QWEN3_TARGET_CONFIG_BYTES,
    QWEN3_TARGET_PREPACKED_MANIFEST_BYTES, QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TOKENIZER_BYTES,
    QWEN3_TOKENIZER_METADATA_BYTES, TARGET_REPOSITORY, TARGET_REVISION,
    authenticate_qwen3_tokenizer, build_authenticated_model_weight_layout,
    build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
    build_prepacked_deployment_bundle, decode_bundle_admission_record,
    encode_canonical_deployment_bundle, expected_preliminary_kernel_catalog_identity,
    expected_qwen3_gfx942_runner_source_identity, generate_qwen3_gfx942_runner_declaration,
    m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
    plan_authenticated_model_memory, publish_qwen3_gfx942_runner_declaration, qwen3_kv_arena_bytes,
    reopen_persisted_qwen3_weights, seal_authenticated_bundle,
};
use ferric_engine::{
    M1AuthenticatedResidentRoundPlansV1, M1AuthenticatedResidentWindowInputV1,
    M1AuthenticatedS1T128PrefillBootstrapInputV1, M1FullStepWorkspacePlans, M1QueueWaitTimeoutV1,
    bind_m1_authenticated_physical_runner_from_selector_v1,
    decode_m1_worker_v3_selector_manifest_v2, initialize_m1_physical_runner_memory_v1,
};
use ferric_m1_engineering_execution_v1::r33_production_backend::{
    M1R33AuthenticatedProductionBackendV1, M1R33AuthenticatedResidentWindowBindingV1,
};
use ferric_m1_engineering_execution_v1::r33_service::{
    HeldM1R33ServiceBundleV1, M1R33DaemonCoordinatorV1, M1R33UnixServerV1,
};
use ferric_m1_engineering_execution_v1::r33_wire::M1_R33_WINDOWS_PER_START_V1;
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::{
    M1AllKernelsProductionProtectedVerifierV1,
    protected_receipt::M1AllKernelsProtectedVerifierTrustPolicyV1,
    protected_verifier_client::{
        M1AllKernelsProtectedVerifierBeginChallengeV2, M1AllKernelsProtectedVerifierClientV2,
        M1AllKernelsProtectedVerifierServiceIdentityV1,
    },
};
use ferric_spec::{
    EngineLimits, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
};
use rustix::fd::OwnedFd;
use rustix::fs::{CWD, Dir, FileType, Mode, OFlags, ResolveFlags, SealFlags, Stat, fstat, openat2};
use rustix::net::{AddressFamily, SocketType, getpeername, getsockname, sockopt::socket_type};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Cursor, Read};
use std::mem::ManuallyDrop;
use std::os::fd::{BorrowedFd, FromRawFd, IntoRawFd};
use std::path::{Component, Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::Duration;

type OwnerResult<T> = Result<T, String>;

const OWNER_PLAN_FORMAT_V1: &str = "FERRIC-M1-R33-PRODUCTION-OWNER-PLAN-V1";
const OWNER_AUTHORITY_V1: &str = "externally-supervised-production-capabilities-only";
const COMPILER_CURRENT_RECORD_FD_V1: i32 = 195;
const PROTECTED_VERIFIER_FD_V1: i32 = 196;
const BEGIN_CHALLENGE_FD_V1: i32 = 197;
const OWNER_PLAN_FD_V1: i32 = 198;
const MAX_OWNER_PLAN_BYTES_V1: u64 = 64 * 1024;
const MAX_SELECTOR_BYTES_V1: u64 = 64 * 1024;
const EXPECTED_SERVICE_EXCHANGES_V1: usize = 3 + M1_R33_WINDOWS_PER_START_V1;
const LOWER_HEX_V1: &[u8; 16] = b"0123456789abcdef";

const TARGET_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS1T128,
};
const DRAFT_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS1T128,
};
const DRAFT_DECODE: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Decode,
    bucket: Qwen3PlanBucket::DecodeS1C8192,
};
const TARGET_SPECULATIVE: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Speculative,
    bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClosurePlanV1 {
    compiler: String,
    compiler_configuration: String,
    executable_catalog: String,
    fe2o3_source: String,
    ferric_source: String,
    kernel_abi_catalog: String,
    kernel_proof_set: String,
    qualification_protocol: String,
    runtime_abi: String,
    runtime_contract: String,
    target_contract: String,
    tcb_report: String,
    validator_registry: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtectedVerifierPlanV1 {
    checker_measurement_sha256: String,
    expected_gid: u32,
    expected_path: String,
    expected_uid: u32,
    timeout_ms: u64,
    verifier_measurement_sha256: String,
    verifying_key_hex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerPlanV1 {
    authority: String,
    closure: ClosurePlanV1,
    diagnostic_ring_bytes: u32,
    format: String,
    gpu_unique_id: u64,
    model_snapshot_path: String,
    protected_verifier: ProtectedVerifierPlanV1,
    queue_wait_timeout_ms: u32,
    selector_manifest_path: String,
    selector_manifest_sha256: String,
    server_start: u64,
    service_plan_path: String,
    service_plan_sha256: String,
}

impl OwnerPlanV1 {
    fn validate(&self) -> OwnerResult<()> {
        if self.authority != OWNER_AUTHORITY_V1 || self.format != OWNER_PLAN_FORMAT_V1 {
            return Err("owner-plan-fixed-fields: authority or format rejected".to_owned());
        }
        if self.server_start >= 3 {
            return Err("owner-plan-server-start: expected 0..=2".to_owned());
        }
        if self.diagnostic_ring_bytes == 0 {
            return Err("owner-plan-diagnostic-ring: zero bytes rejected".to_owned());
        }
        M1QueueWaitTimeoutV1::new(self.queue_wait_timeout_ms)
            .ok_or_else(|| "owner-plan-queue-timeout: unsupported value".to_owned())?;
        let verifier = &self.protected_verifier;
        if verifier.timeout_ms == 0 || verifier.timeout_ms > 60_000 {
            return Err("protected-verifier-timeout: expected 1..=60000ms".to_owned());
        }
        M1AllKernelsProtectedVerifierServiceIdentityV1::new(
            verifier.expected_uid,
            verifier.expected_gid,
        )
        .map_err(|error| format!("protected-verifier-identity: {error}"))?;
        for (path, name) in [
            (&self.model_snapshot_path, "model-snapshot"),
            (&verifier.expected_path, "protected-verifier-socket"),
            (&self.selector_manifest_path, "selector-manifest"),
            (&self.service_plan_path, "service-plan"),
        ] {
            require_canonical_absolute_path(Path::new(path), name)?;
        }
        for (value, name) in [
            (&self.selector_manifest_sha256, "selector-manifest-sha256"),
            (&self.service_plan_sha256, "service-plan-sha256"),
            (
                &verifier.checker_measurement_sha256,
                "checker-measurement-sha256",
            ),
            (
                &verifier.verifier_measurement_sha256,
                "verifier-measurement-sha256",
            ),
            (&self.closure.compiler, "closure.compiler"),
            (
                &self.closure.compiler_configuration,
                "closure.compiler-configuration",
            ),
            (
                &self.closure.executable_catalog,
                "closure.executable-catalog",
            ),
            (&self.closure.fe2o3_source, "closure.fe2o3-source"),
            (&self.closure.ferric_source, "closure.ferric-source"),
            (
                &self.closure.kernel_abi_catalog,
                "closure.kernel-abi-catalog",
            ),
            (&self.closure.kernel_proof_set, "closure.kernel-proof-set"),
            (
                &self.closure.qualification_protocol,
                "closure.qualification-protocol",
            ),
            (&self.closure.runtime_abi, "closure.runtime-abi"),
            (&self.closure.runtime_contract, "closure.runtime-contract"),
            (&self.closure.target_contract, "closure.target-contract"),
            (&self.closure.tcb_report, "closure.tcb-report"),
            (
                &self.closure.validator_registry,
                "closure.validator-registry",
            ),
        ] {
            decode_identity(value).map_err(|_| format!("{name}: invalid nonzero SHA-256"))?;
        }
        decode_bytes_32(&verifier.verifying_key_hex, false)
            .map_err(|_| "protected-verifier-key: invalid 32-byte hex".to_owned())?;
        let checker = decode_bytes_32(&verifier.checker_measurement_sha256, true)?;
        let measurement = decode_bytes_32(&verifier.verifier_measurement_sha256, true)?;
        let key = decode_bytes_32(&verifier.verifying_key_hex, false)?;
        M1AllKernelsProtectedVerifierTrustPolicyV1::new(key, measurement, checker)
            .map_err(|error| format!("protected-verifier-trust-policy: {error}"))?;
        Ok(())
    }
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAIL-CLOSED: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[OsString]) -> OwnerResult<&'static str> {
    match arguments {
        [command, plan_path] if command == "validate-plan" => {
            let _ = load_owner_plan(Path::new(plan_path))?;
            Ok("status=OWNER_PLAN_VALIDATED authority=none")
        }
        [command, expected_plan_sha256] if command == "serve" => {
            let expected_plan_sha256 = expected_plan_sha256
                .to_str()
                .ok_or_else(|| "owner-plan-sha256: UTF-8 required".to_owned())?;
            let expected_plan_sha256 = decode_bytes_32(expected_plan_sha256, true)
                .map_err(|error| format!("owner-plan-sha256: {error}"))?;
            serve(expected_plan_sha256)?;
            Ok("status=BOUNDED_R33_INSTANCE_CLOSED")
        }
        _ => Err(
            "usage: ferric-m1-r33-production-owner-v1 validate-plan OWNER-PLAN.json | serve OWNER-PLAN-SHA256"
                .to_owned(),
        ),
    }
}

fn load_owner_plan(path: &Path) -> OwnerResult<OwnerPlanV1> {
    let bytes = read_secure_file(path, MAX_OWNER_PLAN_BYTES_V1, "owner-plan")?;
    decode_owner_plan(&bytes)
}

fn decode_owner_plan(bytes: &[u8]) -> OwnerResult<OwnerPlanV1> {
    let plan: OwnerPlanV1 =
        serde_json::from_slice(bytes).map_err(|error| format!("owner-plan-json: {error}"))?;
    let mut canonical = serde_json::to_vec_pretty(&plan)
        .map_err(|error| format!("owner-plan-canonical-serialization: {error}"))?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(
            "owner-plan-canonical-json: exact pretty JSON plus newline required".to_owned(),
        );
    }
    plan.validate()?;
    Ok(plan)
}

fn read_bound_selector(plan: &OwnerPlanV1) -> OwnerResult<Vec<u8>> {
    let bytes = read_secure_file(
        Path::new(&plan.selector_manifest_path),
        MAX_SELECTOR_BYTES_V1,
        "selector-manifest",
    )?;
    if sha256_hex(&bytes) != plan.selector_manifest_sha256 {
        return Err("selector-manifest-sha256: bytes differ from owner plan".to_owned());
    }
    Ok(bytes)
}

fn external_closure(
    plan: &ClosurePlanV1,
    catalog: &ferric_build::SequentialPlanCatalog,
) -> OwnerResult<ExternalIdentityClosureInputs> {
    let mut external = ExternalIdentityClosureInputs {
        ferric_source: decode_identity(&plan.ferric_source)?,
        fe2o3_source: decode_identity(&plan.fe2o3_source)?,
        compiler: decode_identity(&plan.compiler)?,
        compiler_configuration: decode_identity(&plan.compiler_configuration)?,
        target_contract: decode_identity(&plan.target_contract)?,
        kernel_catalog: domain_identity(b"ferric.m1.pending-kernel-catalog.v1", &[b"pending"]),
        kernel_proof_set: decode_identity(&plan.kernel_proof_set)?,
        kernel_abi_catalog: decode_identity(&plan.kernel_abi_catalog)?,
        executable_catalog: decode_identity(&plan.executable_catalog)?,
        runtime_contract: decode_identity(&plan.runtime_contract)?,
        runtime_abi: decode_identity(&plan.runtime_abi)?,
        generated_runner: expected_qwen3_gfx942_runner_source_identity(),
        validator_registry: decode_identity(&plan.validator_registry)?,
        qualification_protocol: decode_identity(&plan.qualification_protocol)?,
        tcb_report: decode_identity(&plan.tcb_report)?,
    };
    external.kernel_catalog = expected_preliminary_kernel_catalog_identity(catalog, &external)
        .map_err(|error| format!("runner-kernel-catalog: {error:?}"))?;
    Ok(external)
}

fn workspace_plan(
    owner_plan_sha256: &[u8; 32],
    server_start: u64,
    window: usize,
    round: usize,
    copy: u8,
    selection: Qwen3PlanSelection,
) -> OwnerResult<ferric_build::AddresslessM1StepWorkspacePlan> {
    let requirements = m1_step_workspace_requirements(selection)
        .map_err(|error| format!("workspace-requirements: {error:?}"))?;
    let identity = domain_identity(
        b"ferric.m1.r33-production-owner.workspace.v1",
        &[
            owner_plan_sha256,
            &server_start.to_le_bytes(),
            &u64::try_from(window).unwrap_or(u64::MAX).to_le_bytes(),
            &u64::try_from(round).unwrap_or(u64::MAX).to_le_bytes(),
            &[copy],
            &[
                selection.role as u8,
                selection.mode as u8,
                selection.bucket as u8,
            ],
        ],
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
        M1StepWorkspacePlanOutcome::Rejected(failure) => {
            Err(format!("workspace-plan-rejected: {failure:?}"))
        }
    }
}

fn resident_windows(
    plan: &OwnerPlanV1,
    owner_plan_sha256: [u8; 32],
    bundle: &HeldM1R33ServiceBundleV1,
) -> OwnerResult<Vec<M1R33AuthenticatedResidentWindowBindingV1>> {
    let timeout = M1QueueWaitTimeoutV1::new(plan.queue_wait_timeout_ms)
        .ok_or_else(|| "owner-plan-queue-timeout: unsupported value".to_owned())?;
    let start = usize::try_from(plan.server_start)
        .ok()
        .and_then(|start| start.checked_mul(M1_R33_WINDOWS_PER_START_V1))
        .ok_or_else(|| "owner-plan-server-start: row offset overflow".to_owned())?;
    let rows = bundle
        .workload()
        .rows
        .get(start..start + M1_R33_WINDOWS_PER_START_V1)
        .ok_or_else(|| "service-workload: selected 20-row roster absent".to_owned())?;
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(rows.len())
        .map_err(|_| "resident-window-roster: allocation failed".to_owned())?;
    for (window_index, window) in rows.iter().enumerate() {
        let [request] = window.requests.as_slice() else {
            return Err(format!(
                "resident-window-{window_index}: singleton request required"
            ));
        };
        let expected_output_tokens = u32::try_from(request.expected_output_tokens)
            .map_err(|_| format!("resident-window-{window_index}: output bound overflow"))?;
        let successor = expected_output_tokens
            .checked_sub(1)
            .ok_or_else(|| format!("resident-window-{window_index}: zero output rejected"))?;
        let prefill = |copy| -> OwnerResult<M1FullStepWorkspacePlans> {
            Ok(M1FullStepWorkspacePlans::paired_prefill(
                workspace_plan(
                    &owner_plan_sha256,
                    plan.server_start,
                    window_index,
                    0,
                    copy,
                    DRAFT_PREFILL,
                )?,
                workspace_plan(
                    &owner_plan_sha256,
                    plan.server_start,
                    window_index,
                    0,
                    copy,
                    TARGET_PREFILL,
                )?,
            ))
        };
        let bootstrap = M1AuthenticatedS1T128PrefillBootstrapInputV1::new(
            request.prompt_tokens.clone(),
            successor,
            prefill(0)?,
            prefill(1)?,
        )
        .map_err(|failure| {
            format!(
                "resident-window-{window_index}: S1/T128 bootstrap rejected: {:?}",
                failure.error()
            )
        })?;
        let mut rounds = Vec::new();
        rounds
            .try_reserve_exact(successor as usize)
            .map_err(|_| format!("resident-window-{window_index}: round allocation failed"))?;
        for round_index in 0..successor as usize {
            let speculative = |copy| -> OwnerResult<M1FullStepWorkspacePlans> {
                Ok(M1FullStepWorkspacePlans::speculative_round(
                    workspace_plan(
                        &owner_plan_sha256,
                        plan.server_start,
                        window_index,
                        round_index,
                        copy,
                        DRAFT_DECODE,
                    )?,
                    workspace_plan(
                        &owner_plan_sha256,
                        plan.server_start,
                        window_index,
                        round_index,
                        copy,
                        TARGET_SPECULATIVE,
                    )?,
                ))
            };
            rounds.push(
                M1AuthenticatedResidentRoundPlansV1::new(speculative(2)?, speculative(3)?)
                    .map_err(|_| {
                        format!("resident-window-{window_index}: round-plan identity mismatch")
                    })?,
            );
        }
        let input = M1AuthenticatedResidentWindowInputV1::new(
            bootstrap,
            rounds,
            plan.diagnostic_ring_bytes,
            timeout,
        )
        .map_err(|_| format!("resident-window-{window_index}: resident input rejected"))?;
        bindings.push(
            M1R33AuthenticatedResidentWindowBindingV1::bind(window, input).map_err(|failure| {
                format!(
                    "resident-window-{window_index}: workload bind rejected: {:?}",
                    failure.error()
                )
            })?,
        );
    }
    Ok(bindings)
}

#[derive(Clone, Copy)]
struct SnapshotFileV1 {
    name: &'static str,
    bytes: u64,
}

const MODEL_SNAPSHOT_FILES_V1: [SnapshotFileV1; 11] = [
    SnapshotFileV1 {
        name: "bundle.admission.bin",
        bytes: BUNDLE_ADMISSION_RECORD_BYTES as u64,
    },
    SnapshotFileV1 {
        name: "deployment.bundle.bin",
        bytes: CANONICAL_DEPLOYMENT_BUNDLE_BYTES as u64,
    },
    SnapshotFileV1 {
        name: "draft.config.json",
        bytes: QWEN3_DRAFT_CONFIG_BYTES,
    },
    SnapshotFileV1 {
        name: "draft.tokenizer_config.json",
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
    SnapshotFileV1 {
        name: "draft.weights.bin",
        bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
    },
    SnapshotFileV1 {
        name: "draft.weights.manifest.bin",
        bytes: QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES as u64,
    },
    SnapshotFileV1 {
        name: "target.config.json",
        bytes: QWEN3_TARGET_CONFIG_BYTES,
    },
    SnapshotFileV1 {
        name: "target.tokenizer_config.json",
        bytes: QWEN3_TOKENIZER_METADATA_BYTES,
    },
    SnapshotFileV1 {
        name: "target.weights.bin",
        bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
    },
    SnapshotFileV1 {
        name: "target.weights.manifest.bin",
        bytes: QWEN3_TARGET_PREPACKED_MANIFEST_BYTES as u64,
    },
    SnapshotFileV1 {
        name: "tokenizer.json",
        bytes: QWEN3_TOKENIZER_BYTES,
    },
];

struct ModelInputBytesV1 {
    admission_record: Vec<u8>,
    deployment_bundle: Vec<u8>,
    draft_config: Vec<u8>,
    draft_manifest: Vec<u8>,
    draft_tokenizer_metadata: Vec<u8>,
    draft_weights: Box<[u8]>,
    target_config: Vec<u8>,
    target_manifest: Vec<u8>,
    target_tokenizer_metadata: Vec<u8>,
    target_weights: Box<[u8]>,
    tokenizer: Vec<u8>,
}

impl ModelInputBytesV1 {
    fn authenticate(&self) -> OwnerResult<AuthenticatedBundleAdmission> {
        let descriptor = decode_bundle_admission_record(&self.admission_record)
            .map_err(|error| format!("model-admission-record: {error}"))?;
        let target = reopen_persisted_qwen3_weights(
            Qwen3ModelRole::Target8B,
            descriptor.target_manifest,
            &self.target_manifest,
            Cursor::new(&self.target_weights),
        )
        .map_err(|error| format!("target-model-authentication: {error}"))?;
        let draft = reopen_persisted_qwen3_weights(
            Qwen3ModelRole::Draft06B,
            descriptor.draft_manifest,
            &self.draft_manifest,
            Cursor::new(&self.draft_weights),
        )
        .map_err(|error| format!("draft-model-authentication: {error}"))?;
        let target_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&self.tokenizer))
                .map_err(|error| format!("target-tokenizer-authentication: {error}"))?;
        let draft_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Draft06B, Cursor::new(&self.tokenizer))
                .map_err(|error| format!("draft-tokenizer-authentication: {error}"))?;
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
        .map_err(|error| format!("model-bundle-reconstruction: {error}"))?;
        validate_persisted_deployment(&prepacked, &descriptor.deployment, &self.deployment_bundle)?;
        let admission = seal_authenticated_bundle(prepacked)
            .map_err(|error| format!("model-bundle-reseal: {error}"))?;
        if admission.record().as_bytes().as_slice() != self.admission_record.as_slice() {
            return Err("model-admission-record: exact reseal mismatch".to_owned());
        }
        Ok(admission)
    }
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
) -> OwnerResult<()> {
    if prepacked.deployment() != expected {
        return Err("model-deployment: reconstructed deployment mismatch".to_owned());
    }
    let canonical = encode_canonical_deployment_bundle(prepacked.deployment())
        .map_err(|error| format!("model-deployment-encoding: {error}"))?;
    if canonical.as_bytes() != persisted {
        return Err("model-deployment: persisted canonical bytes mismatch".to_owned());
    }
    Ok(())
}

fn model_memory_plan(
    admission: AuthenticatedBundleAdmission,
) -> OwnerResult<ferric_build::AddresslessModelMemoryPlan> {
    let deployment = *admission.prepacked().deployment();
    let target_manifest = admission.prepacked().target_manifest().aggregate_id();
    let draft_manifest = admission.prepacked().draft_manifest().aggregate_id();
    let layout = build_authenticated_model_weight_layout(admission)
        .map_err(|error| format!("model-memory-layout: {error:?}"))?;
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
        ModelMemoryPlanOutcome::Rejected(failure) => Err(format!("model-memory-plan: {failure:?}")),
    }
}

fn load_model_inputs(root: &Path) -> OwnerResult<ModelInputBytesV1> {
    let snapshot = SecureDirectoryV1::open(root, "model-snapshot")?;
    snapshot.validate_exact_regular_file_roster(&MODEL_SNAPSHOT_FILES_V1, "model-snapshot")?;
    let model = ModelInputBytesV1 {
        admission_record: snapshot.read_exact(
            Path::new("bundle.admission.bin"),
            BUNDLE_ADMISSION_RECORD_BYTES as u64,
            "model-admission-record",
        )?,
        deployment_bundle: snapshot.read_exact(
            Path::new("deployment.bundle.bin"),
            CANONICAL_DEPLOYMENT_BUNDLE_BYTES as u64,
            "model-deployment-bundle",
        )?,
        draft_config: snapshot.read_exact(
            Path::new("draft.config.json"),
            QWEN3_DRAFT_CONFIG_BYTES,
            "draft-config",
        )?,
        draft_manifest: snapshot.read_exact(
            Path::new("draft.weights.manifest.bin"),
            u64::from(QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES),
            "draft-weight-manifest",
        )?,
        draft_tokenizer_metadata: snapshot.read_exact(
            Path::new("draft.tokenizer_config.json"),
            QWEN3_TOKENIZER_METADATA_BYTES,
            "draft-tokenizer-metadata",
        )?,
        draft_weights: snapshot
            .read_exact(
                Path::new("draft.weights.bin"),
                QWEN3_DRAFT_TENSOR_DATA_BYTES,
                "draft-prepacked-weights",
            )?
            .into_boxed_slice(),
        target_config: snapshot.read_exact(
            Path::new("target.config.json"),
            QWEN3_TARGET_CONFIG_BYTES,
            "target-config",
        )?,
        target_manifest: snapshot.read_exact(
            Path::new("target.weights.manifest.bin"),
            u64::from(QWEN3_TARGET_PREPACKED_MANIFEST_BYTES),
            "target-weight-manifest",
        )?,
        target_tokenizer_metadata: snapshot.read_exact(
            Path::new("target.tokenizer_config.json"),
            QWEN3_TOKENIZER_METADATA_BYTES,
            "target-tokenizer-metadata",
        )?,
        target_weights: snapshot
            .read_exact(
                Path::new("target.weights.bin"),
                QWEN3_TARGET_TENSOR_DATA_BYTES,
                "target-prepacked-weights",
            )?
            .into_boxed_slice(),
        tokenizer: snapshot.read_exact(
            Path::new("tokenizer.json"),
            QWEN3_TOKENIZER_BYTES,
            "shared-tokenizer",
        )?,
    };
    snapshot.validate_exact_regular_file_roster(&MODEL_SNAPSHOT_FILES_V1, "model-snapshot")?;
    Ok(model)
}

struct SecureDirectoryV1 {
    descriptor: OwnedFd,
}

struct SecureFileV1 {
    file: File,
    initial: Stat,
}

impl SecureDirectoryV1 {
    fn open(path: &Path, description: &str) -> OwnerResult<Self> {
        require_canonical_absolute_path(path, description)?;
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
        .map_err(|error| format!("{description}: secure directory open failed: {error}"))?;
        Ok(Self { descriptor })
    }

    fn open_file(&self, relative: &Path, description: &str) -> OwnerResult<SecureFileV1> {
        require_relative(relative, description)?;
        let descriptor = openat2(
            &self.descriptor,
            relative,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("{description}: secure file open failed: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("{description}: metadata unavailable: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
        {
            return Err(format!("{description}: regular single-link file required"));
        }
        Ok(SecureFileV1 {
            file: File::from(descriptor),
            initial,
        })
    }

    fn read_exact(
        &self,
        relative: &Path,
        expected_bytes: u64,
        description: &str,
    ) -> OwnerResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        let length = input.length(description)?;
        if u64::try_from(length).ok() != Some(expected_bytes) {
            return Err(format!("{description}: exact length rejected"));
        }
        input.read_snapshot(length, description)
    }

    fn validate_exact_regular_file_roster(
        &self,
        expected: &[SnapshotFileV1],
        description: &str,
    ) -> OwnerResult<()> {
        let expected_names = expected
            .iter()
            .map(|file| file.name.to_owned())
            .collect::<BTreeSet<_>>();
        let mut entries = Dir::read_from(&self.descriptor)
            .map_err(|error| format!("{description}: directory enumeration failed: {error}"))?;
        let mut actual = BTreeSet::new();
        while let Some(entry) = entries.read() {
            let entry =
                entry.map_err(|error| format!("{description}: directory entry failed: {error}"))?;
            let bytes = entry.file_name().to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            let name = std::str::from_utf8(bytes)
                .map_err(|_| format!("{description}: non-UTF-8 member rejected"))?;
            require_relative(Path::new(name), description)?;
            if !actual.insert(name.to_owned()) {
                return Err(format!("{description}: duplicate member rejected"));
            }
        }
        if actual != expected_names || expected_names.len() != expected.len() {
            return Err(format!("{description}: exact 11-file roster required"));
        }
        for expected_file in expected {
            let member = self.open_file(Path::new(expected_file.name), description)?;
            if u64::try_from(member.length(description)?).ok() != Some(expected_file.bytes) {
                return Err(format!(
                    "{description}: member length drifted: {}",
                    expected_file.name
                ));
            }
            member.validate_snapshot(description)?;
        }
        Ok(())
    }
}

impl SecureFileV1 {
    fn length(&self, description: &str) -> OwnerResult<usize> {
        usize::try_from(self.initial.st_size)
            .map_err(|_| format!("{description}: file length exceeds host usize"))
    }

    fn read_snapshot(&mut self, length: usize, description: &str) -> OwnerResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("{description}: read allocation failed"))?;
        let read = (&mut self.file)
            .take(u64::try_from(length).unwrap_or(u64::MAX).saturating_add(1))
            .read_to_end(&mut bytes);
        let snapshot = self.validate_snapshot(description);
        if let Err(error) = read {
            snapshot?;
            return Err(format!("{description}: exact read failed: {error}"));
        }
        snapshot?;
        if bytes.len() != length {
            return Err(format!("{description}: file changed during read"));
        }
        Ok(bytes)
    }

    fn validate_snapshot(&self, description: &str) -> OwnerResult<()> {
        let after = fstat(&self.file)
            .map_err(|error| format!("{description}: metadata recheck failed: {error}"))?;
        if !same_file_snapshot(&self.initial, &after) {
            return Err(format!("{description}: descriptor changed after admission"));
        }
        Ok(())
    }
}

fn read_secure_file(path: &Path, maximum: u64, description: &str) -> OwnerResult<Vec<u8>> {
    require_canonical_absolute_path(path, description)?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("{description}: parent absent"))?;
    let relative = path
        .file_name()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{description}: filename absent"))?;
    let root = SecureDirectoryV1::open(parent, description)?;
    let mut input = root.open_file(&relative, description)?;
    let length = input.length(description)?;
    if length == 0 || u64::try_from(length).unwrap_or(u64::MAX) > maximum {
        return Err(format!("{description}: bounded size rejected"));
    }
    input.read_snapshot(length, description)
}

struct SupervisorCapabilitiesV1 {
    begin_challenge: [u8; 32],
    compiler_current: OwnedFd,
    owner_plan_bytes: Vec<u8>,
    protected_verifier: OwnedFd,
}

fn admit_supervisor_capabilities(
    expected_owner_plan_sha256: [u8; 32],
) -> OwnerResult<SupervisorCapabilitiesV1> {
    // Duplicate the complete roster before consuming any canonical descriptor.
    // This leaves admission fail-atomic and never constructs an OwnedFd around
    // an unvalidated supervisor-provided integer.
    let compiler_current_probe = duplicate_inherited_fd(
        COMPILER_CURRENT_RECORD_FD_V1,
        "compiler-current-record-fd195",
    )?;
    let protected_verifier =
        duplicate_inherited_fd(PROTECTED_VERIFIER_FD_V1, "protected-verifier-fd196")?;
    let begin_challenge_fd =
        duplicate_inherited_fd(BEGIN_CHALLENGE_FD_V1, "begin-challenge-fd197")?;
    let owner_plan = duplicate_inherited_fd(OWNER_PLAN_FD_V1, "owner-plan-fd198")?;
    require_distinct_descriptors(&[
        ("compiler-current-record-fd195", &compiler_current_probe),
        ("protected-verifier-fd196", &protected_verifier),
        ("begin-challenge-fd197", &begin_challenge_fd),
        ("owner-plan-fd198", &owner_plan),
    ])?;
    require_seqpacket(&compiler_current_probe, "compiler-current-record-fd195")?;
    require_seqpacket(&protected_verifier, "protected-verifier-fd196")?;
    let begin_challenge = read_reserved_challenge(&begin_challenge_fd)?;
    let owner_plan_bytes = read_sealed_owner_plan(&owner_plan, expected_owner_plan_sha256)?;

    let compiler_current = take_canonical_slot(
        COMPILER_CURRENT_RECORD_FD_V1,
        &compiler_current_probe,
        "compiler-current-record-fd195",
    )?;
    consume_canonical_slot(
        PROTECTED_VERIFIER_FD_V1,
        &protected_verifier,
        "protected-verifier-fd196",
    )?;
    consume_canonical_slot(
        BEGIN_CHALLENGE_FD_V1,
        &begin_challenge_fd,
        "begin-challenge-fd197",
    )?;
    consume_canonical_slot(OWNER_PLAN_FD_V1, &owner_plan, "owner-plan-fd198")?;
    drop(compiler_current_probe);
    Ok(SupervisorCapabilitiesV1 {
        begin_challenge,
        compiler_current,
        owner_plan_bytes,
        protected_verifier,
    })
}

fn serve(expected_owner_plan_sha256: [u8; 32]) -> OwnerResult<()> {
    let SupervisorCapabilitiesV1 {
        begin_challenge: challenge_bytes,
        compiler_current,
        owner_plan_bytes,
        protected_verifier: verifier_fd,
    } = admit_supervisor_capabilities(expected_owner_plan_sha256)?;
    let owner_plan_sha256: [u8; 32] = Sha256::digest(&owner_plan_bytes).into();
    let plan = decode_owner_plan(&owner_plan_bytes)?;

    let bundle = HeldM1R33ServiceBundleV1::open(Path::new(&plan.service_plan_path))
        .map_err(|error| format!("service-plan-admission: {error}"))?;
    if bundle.plan_sha256() != plan.service_plan_sha256 {
        return Err("service-plan-sha256: bytes differ from owner plan".to_owned());
    }
    bundle
        .revalidate()
        .map_err(|error| format!("service-plan-revalidation: {error}"))?;
    let selector_bytes = read_bound_selector(&plan)?;
    let selector = decode_m1_worker_v3_selector_manifest_v2(&selector_bytes)
        .map_err(|error| format!("selector-manifest-admission: {error}"))?;

    let model = load_model_inputs(Path::new(&plan.model_snapshot_path))?;
    let runner_admission = model.authenticate()?;
    let catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("runner-plan-catalog: {error:?}"))?;
    let external = external_closure(&plan.closure, &catalog)?;
    let closure = build_preliminary_identity_closure(catalog, external)
        .map_err(|error| format!("runner-identity-closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(closure)
        .map_err(|error| format!("runner-declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("runner-publication: {error:?}"))?;
    let memory_plan = model_memory_plan(model.authenticate()?)?;
    let windows = resident_windows(&plan, owner_plan_sha256, &bundle)?;

    // No external verifier deadline starts until all large model files and all
    // deterministic addressless plans have been admitted.
    let verifier_identity = M1AllKernelsProtectedVerifierServiceIdentityV1::new(
        plan.protected_verifier.expected_uid,
        plan.protected_verifier.expected_gid,
    )
    .map_err(|error| format!("protected-verifier-identity: {error}"))?;
    let verifier_client = M1AllKernelsProtectedVerifierClientV2::admit_connected_path(
        verifier_fd,
        Path::new(&plan.protected_verifier.expected_path),
        verifier_identity,
        Duration::from_millis(plan.protected_verifier.timeout_ms),
    )
    .map_err(|failure| format!("protected-verifier-fd196: admission rejected: {failure:?}"))?;
    // SAFETY: The supervisor alone creates FD197 and promises the durable,
    // globally replay-excluding reservation contract. This owner verifies the
    // sealed one-use byte carrier but cannot manufacture that authority.
    let challenge = unsafe {
        M1AllKernelsProtectedVerifierBeginChallengeV2::from_durable_reservation(challenge_bytes)
    }
    .map_err(|error| format!("begin-challenge-fd197: reservation rejected: {error}"))?;
    let trust_policy = M1AllKernelsProtectedVerifierTrustPolicyV1::new(
        decode_bytes_32(&plan.protected_verifier.verifying_key_hex, false)?,
        decode_bytes_32(&plan.protected_verifier.verifier_measurement_sha256, true)?,
        decode_bytes_32(&plan.protected_verifier.checker_measurement_sha256, true)?,
    )
    .map_err(|error| format!("protected-verifier-trust-policy: {error}"))?;
    // The fe2 admission contract consumes canonical FD195 on success or
    // failure. Relinquish RAII custody only at that exact transfer boundary.
    let compiler_current_raw = compiler_current.into_raw_fd();
    assert_eq!(compiler_current_raw, COMPILER_CURRENT_RECORD_FD_V1);
    let current_auditor =
        InheritedWorkerV3CompilerCurrentRecordAuditorV1::admit_inherited_application_service()
            .map_err(|error| {
                format!(
                    "compiler-current-record-fd{COMPILER_CURRENT_RECORD_FD_V1}: admission rejected: {error}"
                )
            })?;
    // SAFETY: FD195, FD196, FD197, sealed FD198 policy, the key, and both
    // measurements are installed by the external supervisor. The exact
    // obligations remain explicit in the public constructor and in this
    // service's owner-plan contract.
    let protected = unsafe {
        M1AllKernelsProductionProtectedVerifierV1::new(
            verifier_client,
            challenge,
            trust_policy,
            current_auditor,
        )
    };
    let mut verifier = WorkerV3ProtectedRosterVerifierAdapterV1::new(protected);
    let runner = bind_m1_authenticated_physical_runner_from_selector_v1(
        selector,
        &mut verifier,
        publication,
    )
    .map_err(|failure| format!("authenticated-runner-bind: {failure:?}"))?;

    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("kfd-open: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("kfd-uapi-admission: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(plan.gpu_unique_id))
        .map_err(|error| format!("gfx942-device-bind: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|failure| format!("physical-model-memory: {failure:?}"))?;
    let backend = M1R33AuthenticatedProductionBackendV1::new_with_s1_k4_resident_windows(
        runner, memory, windows,
    )
    .map_err(|failure| format!("resident-backend-admission: {failure:?}"))?;
    let server =
        M1R33UnixServerV1::bind(&bundle).map_err(|error| format!("r33-listener-bind: {error}"))?;
    let mut coordinator = M1R33DaemonCoordinatorV1::new(&bundle, backend);
    let exchanges = (0..EXPECTED_SERVICE_EXCHANGES_V1).try_for_each(|ordinal| {
        bundle
            .revalidate()
            .map_err(|error| format!("service-input-revalidation-{ordinal}: {error}"))?;
        server
            .serve_one(&mut coordinator)
            .map_err(|error| format!("r33-exchange-{ordinal}: {error}"))
    });
    let (backend, terminal_proof, delivered_measurements) =
        match coordinator.into_completed_backend() {
            Ok((backend, proof)) => {
                let delivered = proof.successful_ordered_measurements();
                (backend, Some(proof), delivered)
            }
            Err(incomplete) => {
                let delivered = incomplete.successful_ordered_measurements();
                (incomplete.into_backend(), None, delivered)
            }
        };
    let close = backend.close();
    let queue_status = close.queue_status();
    if !close.retains_all_custody() {
        terminate_with_quarantined_custody(
            close,
            "resident-close: terminal custody contract rejected",
        );
    }
    if !close.permits_process_exit() {
        terminate_with_quarantined_custody(
            close,
            &format!("resident-close: native queue quarantined; queue_status={queue_status:?}"),
        );
    }
    drop(close);
    exchanges?;
    if queue_status.is_none() {
        return Err("resident-close: bounded lifecycle created no resident queue".to_owned());
    }
    let proof = terminal_proof.ok_or_else(|| {
        format!(
            "r33-terminal-proof: absent; successful_ordered_measurements={delivered_measurements} expected={M1_R33_WINDOWS_PER_START_V1}"
        )
    })?;
    if proof.successful_ordered_measurements() != M1_R33_WINDOWS_PER_START_V1
        || proof.server_start() != plan.server_start
        || proof.service_plan_sha256() != bundle.plan_sha256()
    {
        return Err("r33-terminal-proof: exact owner bindings rejected".to_owned());
    }
    Ok(())
}

fn duplicate_inherited_fd(raw: i32, name: &str) -> OwnerResult<OwnedFd> {
    // SAFETY: F_GETFD reads descriptor metadata without pointer arguments.
    // This owner is still single-threaded, so a successful result establishes
    // validity for the immediately following borrow and duplication.
    if unsafe { libc::fcntl(raw, libc::F_GETFD) } < 0 {
        return Err(format!(
            "{name}: required inherited capability absent: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: F_GETFD established that `raw` is live and the single-threaded
    // startup transaction retains the supervisor's canonical descriptor.
    let descriptor = unsafe { BorrowedFd::borrow_raw(raw) };
    let initial = fstat(descriptor)
        .map_err(|error| format!("{name}: initial metadata unavailable: {error}"))?;
    // SAFETY: F_DUPFD_CLOEXEC has no pointer arguments and duplicates the
    // validated live descriptor into a new, process-owned raw slot.
    let duplicate_raw = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate_raw < 0 {
        return Err(format!(
            "{name}: close-on-exec duplication failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: F_DUPFD_CLOEXEC returned this fresh descriptor to the process.
    let duplicate = unsafe { OwnedFd::from_raw_fd(duplicate_raw) };
    let copied = fstat(&duplicate)
        .map_err(|error| format!("{name}: duplicate metadata unavailable: {error}"))?;
    if !same_file_snapshot(&initial, &copied) {
        return Err(format!(
            "{name}: descriptor identity changed during duplication"
        ));
    }
    Ok(duplicate)
}

fn require_distinct_descriptors(descriptors: &[(&str, &OwnedFd)]) -> OwnerResult<()> {
    let metadata = descriptors
        .iter()
        .map(|(name, descriptor)| {
            fstat(*descriptor)
                .map(|stat| (*name, stat))
                .map_err(|error| format!("{name}: metadata unavailable: {error}"))
        })
        .collect::<OwnerResult<Vec<_>>>()?;
    for left in 0..metadata.len() {
        for right in left + 1..metadata.len() {
            if metadata[left].1.st_dev == metadata[right].1.st_dev
                && metadata[left].1.st_ino == metadata[right].1.st_ino
            {
                return Err(format!(
                    "supervisor-capabilities: {} and {} alias one kernel object",
                    metadata[left].0, metadata[right].0
                ));
            }
        }
    }
    Ok(())
}

fn require_seqpacket(descriptor: &OwnedFd, name: &str) -> OwnerResult<()> {
    let kind = socket_type(descriptor)
        .map_err(|error| format!("{name}: socket type unavailable: {error}"))?;
    if kind != SocketType::SEQPACKET {
        return Err(format!("{name}: connected Unix SOCK_SEQPACKET required"));
    }
    let local = getsockname(descriptor)
        .map_err(|error| format!("{name}: local socket identity unavailable: {error}"))?;
    let peer = getpeername(descriptor)
        .map_err(|error| format!("{name}: connected peer unavailable: {error}"))?;
    if local.address_family() != AddressFamily::UNIX
        || peer.map(|address| address.address_family()) != Some(AddressFamily::UNIX)
    {
        return Err(format!("{name}: connected Unix SOCK_SEQPACKET required"));
    }
    Ok(())
}

fn consume_canonical_slot(raw: i32, duplicate: &OwnedFd, name: &str) -> OwnerResult<()> {
    drop(take_canonical_slot(raw, duplicate, name)?);
    Ok(())
}

fn take_canonical_slot(raw: i32, duplicate: &OwnedFd, name: &str) -> OwnerResult<OwnedFd> {
    // SAFETY: The successful raw-FD preflight and duplicate above establish
    // that the supervisor transferred ownership of this exact canonical slot.
    let canonical = unsafe { OwnedFd::from_raw_fd(raw) };
    let canonical_stat = fstat(&canonical)
        .map_err(|error| format!("{name}: canonical metadata unavailable: {error}"))?;
    let duplicate_stat = fstat(duplicate)
        .map_err(|error| format!("{name}: duplicate metadata unavailable: {error}"))?;
    if !same_file_snapshot(&canonical_stat, &duplicate_stat) {
        return Err(format!("{name}: canonical slot changed before consumption"));
    }
    Ok(canonical)
}

fn read_reserved_challenge(descriptor: &OwnedFd) -> OwnerResult<[u8; 32]> {
    let stat = fstat(descriptor)
        .map_err(|error| format!("begin-challenge-fd197: metadata unavailable: {error}"))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || stat.st_size != 32 {
        return Err("begin-challenge-fd197: exact 32-byte regular memfd required".to_owned());
    }
    if stat.st_mode & 0o777 != 0o400 {
        return Err("begin-challenge-fd197: exact mode 0400 required".to_owned());
    }
    let expected = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
    let seals = rustix::fs::fcntl_get_seals(descriptor)
        .map_err(|error| format!("begin-challenge-fd197: memfd seals unavailable: {error}"))?;
    if seals != expected {
        return Err("begin-challenge-fd197: exact immutable seal set required".to_owned());
    }
    let mut bytes = [0_u8; 32];
    read_exact_at(descriptor, &mut bytes, "begin-challenge-fd197")?;
    let mut trailing = [0_u8; 1];
    if rustix::io::pread(descriptor, &mut trailing, 32)
        .map_err(|error| format!("begin-challenge-fd197: trailing read failed: {error}"))?
        != 0
    {
        return Err("begin-challenge-fd197: trailing bytes rejected".to_owned());
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Err("begin-challenge-fd197: all-zero reservation rejected".to_owned());
    }
    Ok(bytes)
}

fn read_sealed_owner_plan(descriptor: &OwnedFd, expected_sha256: [u8; 32]) -> OwnerResult<Vec<u8>> {
    let stat = fstat(descriptor)
        .map_err(|error| format!("owner-plan-fd198: metadata unavailable: {error}"))?;
    let length = usize::try_from(stat.st_size)
        .map_err(|_| "owner-plan-fd198: bounded size rejected".to_owned())?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || length == 0
        || u64::try_from(length).unwrap_or(u64::MAX) > MAX_OWNER_PLAN_BYTES_V1
    {
        return Err("owner-plan-fd198: bounded regular memfd required".to_owned());
    }
    if stat.st_mode & 0o777 != 0o400 {
        return Err("owner-plan-fd198: exact mode 0400 required".to_owned());
    }
    let expected_seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
    let seals = rustix::fs::fcntl_get_seals(descriptor)
        .map_err(|error| format!("owner-plan-fd198: memfd seals unavailable: {error}"))?;
    if seals != expected_seals {
        return Err("owner-plan-fd198: exact immutable seal set required".to_owned());
    }
    let mut bytes = vec![0_u8; length];
    read_exact_at(descriptor, &mut bytes, "owner-plan-fd198")?;
    if <[u8; 32]>::from(Sha256::digest(&bytes)) != expected_sha256 {
        return Err("owner-plan-fd198: supervisor digest mismatch".to_owned());
    }
    Ok(bytes)
}

fn read_exact_at(descriptor: &OwnedFd, bytes: &mut [u8], name: &str) -> OwnerResult<()> {
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let file_offset =
            u64::try_from(offset).map_err(|_| format!("{name}: read offset conversion failed"))?;
        let count = rustix::io::pread(descriptor, &mut bytes[offset..], file_offset)
            .map_err(|error| format!("{name}: exact read failed: {error}"))?;
        if count == 0 {
            return Err(format!("{name}: premature end of file"));
        }
        offset = offset
            .checked_add(count)
            .ok_or_else(|| format!("{name}: read offset overflow"))?;
    }
    Ok(())
}

fn terminate_with_quarantined_custody<T>(custody: T, diagnostic: &str) -> ! {
    eprintln!("FAIL-CLOSED: {diagnostic}; custody retained through process termination");
    let _custody = retain_quarantined_custody(custody);
    process::exit(1)
}

fn retain_quarantined_custody<T>(custody: T) -> ManuallyDrop<T> {
    ManuallyDrop::new(custody)
}

fn require_canonical_absolute_path(path: &Path, description: &str) -> OwnerResult<()> {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::RootDir))
        || !components.all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description}: canonical absolute path required"));
    }
    Ok(())
}

fn require_relative(path: &Path, description: &str) -> OwnerResult<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description}: safe relative path required"));
    }
    Ok(())
}

fn same_file_snapshot(initial: &Stat, after: &Stat) -> bool {
    initial.st_dev == after.st_dev
        && initial.st_ino == after.st_ino
        && initial.st_mode == after.st_mode
        && initial.st_nlink == after.st_nlink
        && initial.st_size == after.st_size
        && initial.st_mtime == after.st_mtime
        && initial.st_mtime_nsec == after.st_mtime_nsec
        && initial.st_ctime == after.st_ctime
        && initial.st_ctime_nsec == after.st_ctime_nsec
}

fn decode_identity(value: &str) -> OwnerResult<Identity> {
    Ok(Identity::new(decode_bytes_32(value, true)?))
}

fn decode_bytes_32(value: &str, reject_zero: bool) -> OwnerResult<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("lowercase 32-byte hex required".to_owned());
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    if reject_zero && bytes.iter().all(|byte| *byte == 0) {
        return Err("zero identity rejected".to_owned());
    }
    Ok(bytes)
}

fn hex_digit(byte: u8) -> OwnerResult<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err("lowercase hexadecimal digit required".to_owned()),
    }
}

fn domain_identity(domain: &[u8], fields: &[&[u8]]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    for field in fields {
        hash_field(&mut hasher, field);
    }
    Identity::new(hasher.finalize().into())
}

fn hash_field(hasher: &mut Sha256, field: &[u8]) {
    hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(field);
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(LOWER_HEX_V1[usize::from(byte >> 4)]));
        output.push(char::from(LOWER_HEX_V1[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::fs::MemfdFlags;
    use rustix::net::{SocketFlags, socketpair};
    use std::cell::Cell;
    use std::io::{Seek, SeekFrom, Write};
    use std::os::fd::AsRawFd;
    use std::rc::Rc;

    const IDENTITY: &str = "0101010101010101010101010101010101010101010101010101010101010101";
    const KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    fn plan() -> OwnerPlanV1 {
        OwnerPlanV1 {
            authority: OWNER_AUTHORITY_V1.to_owned(),
            closure: ClosurePlanV1 {
                compiler: IDENTITY.to_owned(),
                compiler_configuration: IDENTITY.to_owned(),
                executable_catalog: IDENTITY.to_owned(),
                fe2o3_source: IDENTITY.to_owned(),
                ferric_source: IDENTITY.to_owned(),
                kernel_abi_catalog: IDENTITY.to_owned(),
                kernel_proof_set: IDENTITY.to_owned(),
                qualification_protocol: IDENTITY.to_owned(),
                runtime_abi: IDENTITY.to_owned(),
                runtime_contract: IDENTITY.to_owned(),
                target_contract: IDENTITY.to_owned(),
                tcb_report: IDENTITY.to_owned(),
                validator_registry: IDENTITY.to_owned(),
            },
            diagnostic_ring_bytes: 4096,
            format: OWNER_PLAN_FORMAT_V1.to_owned(),
            gpu_unique_id: 1,
            model_snapshot_path: "/srv/ferric/model".to_owned(),
            protected_verifier: ProtectedVerifierPlanV1 {
                checker_measurement_sha256:
                    "0202020202020202020202020202020202020202020202020202020202020202".to_owned(),
                expected_gid: 1234,
                expected_path: "/run/ferric/verifier.sock".to_owned(),
                expected_uid: 1234,
                timeout_ms: 10_000,
                verifier_measurement_sha256: IDENTITY.to_owned(),
                verifying_key_hex: KEY.to_owned(),
            },
            queue_wait_timeout_ms: 1_000,
            selector_manifest_path: "/srv/ferric/selector.json".to_owned(),
            selector_manifest_sha256: IDENTITY.to_owned(),
            server_start: 0,
            service_plan_path: "/srv/ferric/service.json".to_owned(),
            service_plan_sha256: IDENTITY.to_owned(),
        }
    }

    #[test]
    fn exact_external_plan_is_accepted() {
        plan().validate().unwrap();
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let mut value = serde_json::to_value(plan()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("bypass".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<OwnerPlanV1>(value).is_err());
    }

    #[test]
    fn missing_or_weak_authority_inputs_are_diagnostic() {
        let mut value = plan();
        value.server_start = 3;
        assert!(value.validate().unwrap_err().contains("server-start"));
        value = plan();
        value.closure.executable_catalog = "00".repeat(32);
        assert!(value.validate().unwrap_err().contains("executable-catalog"));
        value = plan();
        value.protected_verifier.expected_uid = 0;
        assert!(value.validate().unwrap_err().contains("verifier-identity"));
    }

    #[test]
    fn path_and_hex_decoders_reject_aliases() {
        assert!(require_canonical_absolute_path(Path::new("relative"), "x").is_err());
        assert!(require_canonical_absolute_path(Path::new("/a/../b"), "x").is_err());
        assert!(decode_bytes_32(&"AA".repeat(32), true).is_err());
        assert!(decode_bytes_32(&"00".repeat(32), true).is_err());
    }

    fn sealed_memfd(name: &str, bytes: &[u8], seals: SealFlags) -> OwnedFd {
        let descriptor =
            rustix::fs::memfd_create(name, MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
                .unwrap();
        let mut file = File::from(descriptor);
        rustix::fs::fchmod(&file, Mode::RUSR).unwrap();
        file.write_all(bytes).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        rustix::fs::fcntl_add_seals(&file, seals).unwrap();
        file.into()
    }

    fn challenge_fd(bytes: &[u8], seals: SealFlags) -> OwnedFd {
        sealed_memfd("ferric-r33-owner-challenge-test", bytes, seals)
    }

    #[test]
    fn sealed_reserved_challenge_is_read_exactly_once() {
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        assert_eq!(
            read_reserved_challenge(&challenge_fd(&[7; 32], seals)).unwrap(),
            [7; 32]
        );
    }

    #[test]
    fn challenge_rejects_missing_seals_size_and_zero() {
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        assert!(read_reserved_challenge(&challenge_fd(&[7; 32], SealFlags::empty())).is_err());
        assert!(read_reserved_challenge(&challenge_fd(&[7; 31], seals)).is_err());
        assert!(read_reserved_challenge(&challenge_fd(&[0; 32], seals)).is_err());
    }

    #[test]
    fn sealed_supervisor_plan_requires_exact_digest() {
        let mut bytes = serde_json::to_vec_pretty(&plan()).unwrap();
        bytes.push(b'\n');
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        let descriptor = sealed_memfd("ferric-r33-owner-plan-test", &bytes, seals);
        assert_eq!(read_sealed_owner_plan(&descriptor, digest).unwrap(), bytes);
        assert!(read_sealed_owner_plan(&descriptor, [9; 32]).is_err());
        assert!(decode_owner_plan(&bytes).is_ok());
    }

    #[test]
    fn inherited_descriptor_preflight_rejects_absence_type_and_alias() {
        assert!(duplicate_inherited_fd(-1, "missing-fd").is_err());
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        let regular = challenge_fd(&[7; 32], seals);
        assert!(require_seqpacket(&regular, "current-record-fd").is_err());
        let (current, verifier) = socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        require_seqpacket(&current, "current-record-fd").unwrap();
        require_seqpacket(&verifier, "verifier-fd").unwrap();
        let alias = rustix::io::fcntl_dupfd_cloexec(&current, 0).unwrap();
        assert!(
            require_distinct_descriptors(&[("current-record-fd", &current), ("alias", &alias)])
                .unwrap_err()
                .contains("alias one kernel object")
        );
        let duplicate = duplicate_inherited_fd(current.as_raw_fd(), "current-record-fd").unwrap();
        assert_ne!(duplicate.as_raw_fd(), current.as_raw_fd());
        require_seqpacket(&duplicate, "current-record-duplicate").unwrap();
    }

    #[test]
    fn canonical_slot_is_owned_only_after_validated_duplication() {
        let (canonical, _peer) = socketpair(
            AddressFamily::UNIX,
            SocketType::SEQPACKET,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        let raw = canonical.into_raw_fd();
        let duplicate = duplicate_inherited_fd(raw, "test-canonical").unwrap();
        let canonical = take_canonical_slot(raw, &duplicate, "test-canonical").unwrap();
        assert_eq!(canonical.as_raw_fd(), raw);
        drop(canonical);
        require_seqpacket(&duplicate, "test-duplicate").unwrap();
    }

    #[test]
    fn invalid_selector_and_current_record_carriers_fail_before_authority() {
        assert!(decode_m1_worker_v3_selector_manifest_v2(b"{}\n").is_err());
        let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
        let wrong_current_record = challenge_fd(&[3; 32], seals);
        assert!(require_seqpacket(&wrong_current_record, "compiler-current-record-fd195").is_err());
    }

    #[test]
    fn quarantine_custody_does_not_drop_before_process_termination() {
        struct DropProbe(Rc<Cell<usize>>);

        impl Drop for DropProbe {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let drops = Rc::new(Cell::new(0));
        let mut retained = retain_quarantined_custody(DropProbe(Rc::clone(&drops)));
        assert_eq!(drops.get(), 0);
        // SAFETY: The test owns this ManuallyDrop and invokes its destructor once.
        unsafe { ManuallyDrop::drop(&mut retained) };
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn owner_source_requires_real_capability_chain() {
        let source = include_str!("main.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for required in [
            "InheritedWorkerV3CompilerCurrentRecordAuditorV1::admit_inherited_application_service",
            "M1AllKernelsProtectedVerifierClientV2::admit_connected_path",
            "M1AllKernelsProtectedVerifierBeginChallengeV2::from_durable_reservation",
            "M1AllKernelsProductionProtectedVerifierV1::new",
            "bind_m1_authenticated_physical_runner_from_selector_v1",
            "OpenedKfd::open_default",
            "initialize_m1_physical_runner_memory_v1",
            "new_with_s1_k4_resident_windows",
            "coordinator.into_completed_backend()",
            "terminate_with_quarantined_custody",
            "ManuallyDrop::new",
        ] {
            assert!(
                source.contains(required),
                "missing protected owner step: {required}"
            );
        }
        for forbidden in [
            "M1AllKernelsProtectedVerifierV1::new",
            "synthetic_for_test_only",
            "bind_engineering_structural",
            "reopen_m1_engineering",
        ] {
            assert!(!source.contains(forbidden), "forbidden bypass: {forbidden}");
        }
        assert_eq!(COMPILER_CURRENT_RECORD_FD_V1, 195);
        assert_eq!(PROTECTED_VERIFIER_FD_V1, 196);
        assert_eq!(BEGIN_CHALLENGE_FD_V1, 197);
        assert_eq!(OWNER_PLAN_FD_V1, 198);
    }
}
