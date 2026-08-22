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
    bind_m1_kv_workspace_table_v1, bind_m1_physical_runner_v1, complete_m1_physical_step_v1,
    initialize_m1_physical_runner_memory_v1, release_m1_completed_step_kv_pages_v1,
    reopen_persisted_m1_kernel_artifacts_v1, ActiveDeviceKvCache,
    CompletionWireSemanticExpectation, Engine, M1CompletedStepOutcomeV1,
    M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1, M1FullStepKvWorkspaceTablesV1,
    M1FullStepWorkspacePlans, M1ObservedQualificationOutputV1, M1PhysicalRunnerRecipeOutcomeV1,
    M1QualificationObservationFailureCustodyV1, M1StepDispatchIntent,
};
use ferric_spec::{
    validate_m1_step_inputs, EngineLimits, Identity, M1StepInputCandidate,
    M1StepInputValidationOutcome, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, StepPlan, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS, QWEN3_VOCABULARY_SIZE,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, FileType, Mode, OFlags,
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

const PLAN_FORMAT: &str = "FERRIC-M1-BENCHMARK-PLAN-V1";
const ROSTER_FORMAT: &str = "FERRIC-M1-QUALIFICATION-ROSTER-V1";
const WORKLOAD_FORMAT: &str = "FERRIC-M1-QUALIFICATION-WORKLOAD-V1";
const CLOSURE_FORMAT: &str = "FERRIC-M1-QUALIFICATION-CLOSURE-V1";
const ENVIRONMENT_FORMAT: &str = "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1";
const OUTPUT_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1";
const TRANSCRIPT_FORMAT: &str = "FERRIC-M1-QUALIFICATION-CAPTURE-V1";
const TARGET: &str = "gfx942:xnack-";
const DIFFERENTIAL_NONCLAIM: &str = "Structural acceptance authenticates externally collected target-only differential records only. It does not validate a logit tolerance, prove token equality, establish numerical or hardware correctness, qualify performance, or close m1.r29.";
const MAX_DOCUMENT_BYTES: usize = 8 * 1_024 * 1_024;
const MAX_POLLS: u64 = 100_000_000;
const METADATA_BYTES: u64 = 64 * 1_024;
const BF16_BYTES: u64 = 2;
const DECODE_CONTEXT_LENGTH: u32 = 8_191;
const DECODE_PRIMING_UNAVAILABLE: &str = "canonical c8192 decode capture requires 8191 authenticated initialized target-KV tokens per lane, but Ferric M1 has no chunked-prefill continuation beyond PrefillS1T2048's 2048-token context capacity, no prefill-to-decode PhysicalKvState/queue selection transition, and no final-only qualification-capture rearm; refusing an uninitialized decode capture";

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

type CaptureResult<T> = Result<T, String>;

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
    max_polls: u32,
    selection: Qwen3PlanSelection,
}

#[derive(Debug)]
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
}

struct StagingOutput {
    parent: OwnedFd,
    staging: OwnedFd,
    staging_name: OsString,
    output_name: OsString,
    files: Vec<OsString>,
    armed: bool,
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
                    return Ok(Self {
                        parent,
                        staging,
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
        self.files.push(name.clone());
        writer(&mut file)
            .map_err(|error| format!("cannot write staged output {}: {error}", name.display()))?;
        file.sync_all()
            .map_err(|error| format!("cannot sync staged output {}: {error}", name.display()))?;
        Ok(())
    }

    fn publish(mut self) -> CaptureResult<()> {
        fsync(&self.staging).map_err(|error| format!("cannot sync staging directory: {error}"))?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| format!("cannot publish output without replacement: {error}"))?;
        self.armed = false;
        if let Err(error) = fsync(&self.parent) {
            eprintln!("WARN: output bundle is visible but parent sync failed: {error}");
        }
        Ok(())
    }
}

impl Drop for StagingOutput {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for name in &self.files {
            let _ = unlinkat(&self.staging, name.as_os_str(), AtFlags::empty());
        }
        let _ = unlinkat(
            &self.parent,
            self.staging_name.as_os_str(),
            AtFlags::REMOVEDIR,
        );
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
    let [plan_path, roster_path, case_id, workload_path, source_root, prepacked_root, artifact_root, closure_path, environment_path, gpu_unique_id, output] =
        arguments.as_slice()
    else {
        return Err("usage: ferric-m1-qualification-capture PLAN ROSTER CASE-ID WORKLOAD MODEL-SOURCE PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ENVIRONMENT GPU-UNIQUE-ID OUTPUT-BUNDLE".to_owned());
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

    let capture = execute_capture(&runner, memory, &workload, input_tokens, gpu_unique_id)?;
    let runner_declaration = runner.declaration_id();
    let kernel_manifest = runner.kernel_artifact_manifest_id();
    let transcript = capture_transcript(
        &plan,
        &case,
        &workload,
        &capture,
        CaptureIdentities {
            gpu_unique_id,
            runner_declaration,
            kernel_manifest,
            program_catalog: executable_catalog_id,
        },
    )?;
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
    dispatch_generation: u64,
    logits: Vec<u8>,
    logits_row_sha256: Vec<[u8; 32]>,
    tokens: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
struct CaptureIdentities {
    gpu_unique_id: u64,
    runner_declaration: Identity,
    kernel_manifest: Identity,
    program_catalog: Identity,
}

fn execute_capture(
    runner: &ferric_engine::M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    workload: &Workload,
    input_tokens: Vec<u32>,
    _gpu_unique_id: u64,
) -> CaptureResult<CapturedOutput> {
    let selection = workload.selection;
    let draft_selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: selection.mode,
        bucket: selection.bucket,
    };
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "workload selection has no admitted dimensions".to_owned())?;
    let mut engine = Engine::<32>::new(512, 256, 8_192)
        .map_err(|error| format!("cannot construct M1 engine: {error:?}"))?;
    let mut requests = Vec::with_capacity(workload.lanes.len());
    for lane in &workload.lanes {
        let request = engine
            .admit()
            .map_err(|error| format!("cannot admit workload lane: {error:?}"))?;
        engine
            .append_tentative(request, lane.active_length)
            .map_err(|error| format!("cannot make workload lane schedulable: {error:?}"))?;
        requests.push(request);
    }
    let scheduled = engine
        .dispatch_m1_ready()
        .map_err(|error| format!("cannot schedule workload: {error:?}"))?
        .ok_or_else(|| "workload produced no schedulable batch".to_owned())?;
    if scheduled.member_count() != workload.lanes.len() {
        return Err("scheduler live roster differs from workload lanes".to_owned());
    }
    let mut plans = Vec::with_capacity(requests.len());
    for request in &requests {
        plans.push(
            runner
                .logical_runner()
                .bind_step_plan(*request, scheduled.epoch(), selection)
                .map_err(|error| format!("cannot bind workload step plan: {error:?}"))?,
        );
    }
    let inputs = validated_inputs(workload, &plans, input_tokens, dimensions.active_tokens)?;
    let active_lengths = inputs.active_lengths().to_vec();
    let context_lengths = inputs.context_lengths().to_vec();
    let mut caches = Vec::with_capacity(requests.len());
    let mut reservations = Vec::with_capacity(requests.len());
    for (lane, request) in requests.iter().copied().enumerate() {
        let mut cache =
            ActiveDeviceKvCache::new(memory.device(), request, selection, draft_selection)
                .map_err(|error| format!("cannot create lane {lane} device KV cache: {error:?}"))?;
        let pages = qualification_kv_page_count(context_lengths[lane], active_lengths[lane])
            .map_err(|error| format!("lane {lane} {error}"))?;
        let mut leases = Vec::with_capacity(pages as usize);
        for page in 0..pages {
            leases.push(
                memory
                    .lease_page(request, Qwen3ModelRole::Target8B, page)
                    .map_err(|error| format!("cannot lease lane {lane} page {page}: {error:?}"))?,
            );
        }
        let pending = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                context_lengths[lane],
                active_lengths[lane],
                scheduled.epoch(),
                leases,
            )
            .map_err(|error| format!("cannot reserve lane {lane} KV write: {error:?}"))?;
        caches.push(cache);
        reservations.push(pending);
    }
    let table = bind_m1_kv_workspace_table_v1(inputs, reservations)
        .map_err(|error| format!("cannot bind target KV workspace: {error:?}"))?;
    let tables = M1FullStepKvWorkspaceTablesV1::TargetOnly { target: table };
    let workspace_plan = workload_workspace_plan(selection, sha256_array(&workload.bytes))?;
    let prepared = runner
        .prepare_scheduled_workspaces(
            scheduled,
            M1FullStepWorkspacePlans::target_only(workspace_plan),
            tables,
        )
        .map_err(|error| format!("cannot prepare scheduled workspace: {error:?}"))?;
    let completion = memory
        .allocate_completion_output(selection)
        .map_err(|error| format!("cannot allocate compact completion: {error}"))?;
    let completion = memory
        .enable_qualification_logits_capture(completion)
        .map_err(|error| format!("cannot enable qualification logits: {error:?}"))?;
    let allocated = runner
        .allocate_scheduled_workspaces(memory, prepared)
        .map_err(|error| format!("cannot allocate scheduled workspaces: {error:?}"))?;
    let recipe = match runner.derive_step_recipe(
        M1StepDispatchIntent::TargetOnly(selection),
        M1FullStepWorkspacePlans::target_only(workload_workspace_plan(
            selection,
            sha256_array(&workload.bytes),
        )?),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            return Err(format!("cannot derive physical recipe: {error:?}"));
        }
    };
    let published = runner
        .publish_first_step(&mut engine, 1 << 20, allocated, recipe, completion)
        .map_err(|error| format!("cannot publish qualification step: {error:?}"))?;
    let completed = match published.wait(workload.max_polls) {
        Ok(completed) => completed,
        Err(error) => {
            return Err(format!(
                "qualification queue wait entered terminal quarantine: {error:?}"
            ));
        }
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(error) => {
            return Err(format!(
                "qualification queue recycle entered terminal quarantine: {error:?}"
            ));
        }
    };
    let device_id = recycled.custody().device().device_id();
    let observed = match recycled.observe_qualification_completion() {
        Ok(observed) => observed,
        Err(failure) => return Err(release_qualification_failure(*failure)),
    };
    let (output, choices) = match copy_capture_candidate(&observed, device_id, workload.lanes.len())
    {
        Ok(candidate) => candidate,
        Err(error) => return Err(release_observed_after_error(observed, error)),
    };
    for request in &requests {
        if let Err(error) = engine.retire(*request) {
            return Err(release_observed_after_error(
                observed,
                format!("cannot retire captured request before completion: {error:?}"),
            ));
        }
    }
    let expectations = choices
        .iter()
        .copied()
        .map(|choice| CompletionWireSemanticExpectation::DirectFinalRow { choice })
        .collect::<Vec<_>>();
    let qualified = match observed.check_completion(&expectations) {
        Ok(qualified) => qualified,
        Err(failure) => {
            let (error, observed) = failure.into_parts();
            return Err(release_observed_after_error(
                observed,
                format!("qualification semantic completion join failed: {error}"),
            ));
        }
    };
    let (completed, evidence) = qualified.into_parts();
    drop(evidence);
    let roster = M1DeviceKvCompletionRosterV1::new(
        caches
            .into_iter()
            .map(M1DeviceKvCompletionMemberV1::retiring)
            .collect(),
    );
    let completed = match complete_m1_physical_step_v1(&mut engine, completed, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        M1CompletedStepOutcomeV1::Rejected(rejected) => {
            let (error, readback, roster) = rejected.into_parts();
            let (queue, checked, completion, reservations) = readback.into_parts();
            let release = queue.destroy_and_release();
            drop((checked, completion, reservations, roster));
            return Err(format!(
                "qualification completion was rejected: {error:?}; {}",
                release_result("rejected completion queue", release)
            ));
        }
        M1CompletedStepOutcomeV1::Poisoned(poisoned) => {
            return Err(format!(
                "qualification completion entered terminal quarantine: {:?}",
                poisoned.error()
            ));
        }
    };
    let released = match release_m1_completed_step_kv_pages_v1(completed) {
        Ok(released) => released,
        Err(failure) => {
            let (error, completed) = (*failure).into_parts();
            let (queue, checked, members, emitted_counts) = completed.into_parts();
            let release = queue.destroy_and_release();
            drop((checked, members, emitted_counts));
            return Err(format!(
                "qualification KV page release failed: {error:?}; {}",
                release_result("page-release queue", release)
            ));
        }
    };
    let teardown = released
        .destroy_queue_and_retain_step()
        .map_err(|failure| format!("qualification final queue teardown failed: {failure:?}"))?;
    if teardown.members().len() != workload.lanes.len()
        || teardown
            .members()
            .iter()
            .any(|member| matches!(member, ferric_engine::M1ReleasedDeviceKvMemberV1::Active(_)))
    {
        return Err("qualification teardown retained a nonterminal KV member".to_owned());
    }
    Ok(output)
}

fn copy_capture_candidate(
    observed: &M1ObservedQualificationOutputV1,
    device_id: Identity,
    expected_lanes: usize,
) -> CaptureResult<(CapturedOutput, Vec<u32>)> {
    let compact = observed.compact();
    let records = compact.records();
    if records.len() != expected_lanes {
        return Err("compact live record count differs from workload lanes".to_owned());
    }
    let evidence = observed.evidence();
    if evidence.logits().rows().len() != records.len() {
        return Err("captured logits row count differs from compact records".to_owned());
    }
    let mut choices = Vec::with_capacity(records.len());
    let mut tokens = Vec::with_capacity(records.len() * 4);
    for (lane, record) in records.iter().enumerate() {
        if record.record().emitted_token_count != 1 || record.accepted_draft_tokens() != 0 {
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
    for (lane, row) in evidence.logits().rows().iter().enumerate() {
        if row.lane() != lane || row.raw_bytes().len() != row_bytes {
            return Err(format!("captured logits row {lane} geometry drifted"));
        }
        let choice = lowest_id_finite_bf16_argmax(row.raw_bytes(), lane)?;
        choices.push(choice);
        tokens.extend_from_slice(&choice.to_le_bytes());
        logits.extend_from_slice(row.raw_bytes());
        logits_row_sha256.push(*row.raw_sha256());
    }
    let output = CapturedOutput {
        compact_sha256: *evidence.compact_raw_sha256(),
        device_id,
        dispatch_generation: compact.dispatch_generation(),
        logits,
        logits_row_sha256,
        tokens,
    };
    Ok((output, choices))
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

fn release_qualification_failure(
    failure: ferric_engine::M1QualificationObservationFailureV1,
) -> String {
    let (error, custody) = failure.into_parts();
    let release = match custody {
        M1QualificationObservationFailureCustodyV1::Recycled(queue) => queue.destroy_and_release(),
        M1QualificationObservationFailureCustodyV1::CompactRejected(output) => {
            output.destroy_and_release()
        }
        M1QualificationObservationFailureCustodyV1::Observed {
            completion,
            partial_logits,
        } => {
            drop(partial_logits);
            completion.destroy_and_release()
        }
    };
    format!(
        "qualification observation failed: {error}; {}",
        release_result("failed observation queue", release)
    )
}

fn release_observed_after_error(
    observed: M1ObservedQualificationOutputV1,
    error: String,
) -> String {
    let release = observed.destroy_and_release();
    format!(
        "{error}; {}",
        release_result("observed qualification queue", release)
    )
}

fn release_result<T, E: core::fmt::Debug>(description: &str, result: Result<T, E>) -> String {
    match result {
        Ok(_) => format!("{description} released"),
        Err(error) => format!("{description} entered terminal release quarantine: {error:?}"),
    }
}

fn require_supported_capture(workload: &Workload) -> CaptureResult<()> {
    if workload.selection.mode == Qwen3ExecutionMode::Decode {
        Err(DECODE_PRIMING_UNAVAILABLE.to_owned())
    } else {
        Ok(())
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
    let object = exact_object(
        &value,
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
    require_sha256(string_field(object, "input_sha256")?)?;
    let identities = parse_identities(field(object, "identities")?)?;
    let cases = parse_cases(field(object, "cases")?)?;
    Ok(DifferentialPlan {
        bytes,
        cases,
        identities,
    })
}

fn parse_identities(value: &Value) -> CaptureResult<BTreeMap<String, String>> {
    let object = value
        .as_object()
        .ok_or_else(|| "benchmark identities must be an object".to_owned())?;
    let mut expected = COMMON_IDENTITIES.to_vec();
    expected.extend(["reference-implementation", "reference-protocol"]);
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
    require_identity(
        plan.identity("workload-roster")?,
        &sha256_hex(&bytes),
        "workload roster",
    )?;
    let object = exact_object(&value, &["cases", "format", "suite"], "workload roster")?;
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
    require_identity(
        &case.workload_sha256,
        &sha256_hex(&bytes),
        "qualification workload",
    )?;
    let object = exact_object(
        &value,
        &[
            "case_id",
            "format",
            "input",
            "kind",
            "lanes",
            "max_polls",
            "selection",
        ],
        "qualification workload",
    )?;
    expect_string(object, "format", WORKLOAD_FORMAT)?;
    expect_string(object, "case_id", &case.id)?;
    expect_string(object, "kind", &case.kind)?;
    let selection = kind_selection(&case.kind)?;
    validate_selection(field(object, "selection")?, selection)?;
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "qualification selection is not admitted".to_owned())?;
    let lanes = parse_lanes(field(object, "lanes")?, selection, dimensions.sequences)?;
    let max_polls = integer_field(object, "max_polls")?;
    if max_polls == 0 || max_polls > MAX_POLLS {
        return Err("qualification max_polls is outside 1..=100000000".to_owned());
    }
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
        max_polls: u32::try_from(max_polls)
            .map_err(|_| "qualification max_polls does not fit u32".to_owned())?,
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
    if values.len() != expected as usize {
        return Err("qualification lane count differs from selected bucket".to_owned());
    }
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "qualification selection has no dimensions".to_owned())?;
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
        match selection.mode {
            Qwen3ExecutionMode::Prefill => {
                if active != dimensions.active_tokens || context != 0 {
                    return Err(format!(
                        "lane {lane} canonical prefill geometry requires the full declared active width at empty context"
                    ));
                }
            }
            Qwen3ExecutionMode::Decode => {
                if active != 1 || context != DECODE_CONTEXT_LENGTH {
                    return Err(format!(
                        "lane {lane} canonical c8192 decode geometry requires one active token after exactly 8191 committed context tokens"
                    ));
                }
            }
            Qwen3ExecutionMode::Speculative => {
                return Err("qualification capture accepts target-only modes only".to_owned());
            }
        }
        lanes.push(LaneInput {
            active_length: active,
            context_length: context,
        });
    }
    Ok(lanes)
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
    let bytes = root.read_exact(
        &workload.input_path,
        workload.input_bytes,
        "qualification token payload",
    )?;
    let actual = sha256_hex(&bytes);
    require_identity(&workload.input_sha256, &actual, "workload input payload")?;
    require_identity(&case.input_sha256, &actual, "benchmark case input")?;
    let mut tokens = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let token = u32::from_le_bytes(chunk.try_into().expect("exact four-byte chunk"));
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
    let (root, relative) = secure_parent(path, "qualification closure")?;
    let (value, _) = root.read_canonical(&relative, "qualification closure")?;
    let object = exact_object(
        &value,
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
    let object = exact_object(
        &value,
        &["format", "gpu_unique_id", "target"],
        "qualification environment",
    )?;
    expect_string(object, "format", ENVIRONMENT_FORMAT)?;
    expect_string(object, "target", TARGET)?;
    if integer_field(object, "gpu_unique_id")? != gpu_unique_id {
        return Err("environment GPU unique ID differs from the selected device".to_owned());
    }
    Ok(bytes)
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
    let selected = declaration
        .plans()
        .iter()
        .find(|candidate| candidate.selection == kind_selection(&case.kind).unwrap())
        .ok_or_else(|| "generated declaration lacks selected workload plan".to_owned())?;
    require_identity(
        plan.identity("dispatch-graph")?,
        &hex_identity(selected.plan_id),
        "selected dispatch graph",
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
    canonical_bytes(&json!({
        "authority": "observed-target-only-qualification-capture",
        "case_id": case.id,
        "compact_sha256": hex_bytes(&capture.compact_sha256),
        "device_identity_sha256": hex_identity(capture.device_id),
        "dispatch_generation": capture.dispatch_generation,
        "format": TRANSCRIPT_FORMAT,
        "gpu_unique_id": identities.gpu_unique_id,
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

    struct BareOutput(PathBuf);

    impl BareOutput {
        fn new() -> Self {
            let nonce = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
            let path = PathBuf::from(format!(
                ".ferric-m1-qualification-capture-bare-test.{}.{nonce}",
                std::process::id()
            ));
            assert!(!path.exists());
            Self(path)
        }
    }

    impl Drop for BareOutput {
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
            "max_polls": 20_000_000,
            "selection": selection_json(kind_selection(kind).unwrap()),
        })
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

        row[4 * 2..4 * 2 + 2].copy_from_slice(&0x8000_u16.to_le_bytes());
        row[7 * 2..7 * 2 + 2].copy_from_slice(&0_u16.to_le_bytes());
        assert_eq!(lowest_id_finite_bf16_argmax(&row, 0).unwrap(), 4);

        row[9 * 2..9 * 2 + 2].copy_from_slice(&0x7fc0_u16.to_le_bytes());
        assert!(lowest_id_finite_bf16_argmax(&row, 0).is_err());
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
        for kind in DIFFERENTIAL_KINDS {
            let selection = kind_selection(kind).unwrap();
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
                "format",
                "input",
                "kind",
                "lanes",
                "max_polls",
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
                "format",
                "input",
                "kind",
                "lanes",
                "max_polls",
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
            max_polls: 1,
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
    fn decode_final_step_uses_token_after_authenticated_context() {
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
            max_polls: 1,
            selection,
        };
        let plan = StepPlan::new(
            RequestId::new(0, 1),
            ferric_spec::completion::CompletionEpoch::new(1),
            Identity::new([7; 32]),
            selection,
        );
        let mut tokens = vec![3; 8_192];
        tokens[8_191] = 17;
        let inputs = validated_inputs(&workload, &[plan], tokens, 1).unwrap();
        assert_eq!(inputs.token_ids(), &[17]);
        assert_eq!(inputs.position_ids(), &[DECODE_CONTEXT_LENGTH]);
        assert_eq!(
            require_supported_capture(&workload),
            Err(DECODE_PRIMING_UNAVAILABLE.to_owned())
        );
    }

    #[test]
    fn bare_input_and_output_paths_use_current_directory() {
        let bare_workload = Path::new("workload.json");
        let workload_parent = bare_workload
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        assert_eq!(workload_parent, Path::new("."));

        let output = BareOutput::new();
        let mut staging = StagingOutput::create(&output.0).unwrap();
        staging.write("payload", b"captured\n").unwrap();
        staging.publish().unwrap();
        assert_eq!(fs::read(output.0.join("payload")).unwrap(), b"captured\n");
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
    fn producer_manifest_has_exact_payload_contract() {
        let identities = COMMON_IDENTITIES
            .iter()
            .chain([&"reference-implementation", &"reference-protocol"])
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
