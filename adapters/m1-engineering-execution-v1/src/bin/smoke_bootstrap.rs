use ferric_build::{
    AuthenticatedBundleAdmission, AuthenticatedDeploymentAssets, AuthenticatedModelAssets,
    AuthenticatedTokenizer, BUNDLE_ADMISSION_RECORD_BYTES, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    DRAFT_REPOSITORY, DRAFT_REVISION, DeclaredDeviceAllocation, ExternalIdentityClosureInputs,
    ModelMemoryAllocationSet, ModelMemoryPlanOutcome, PrepackedDeploymentBundle,
    PublishedRunnerDeclaration, QWEN3_DRAFT_CONFIG_BYTES, QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES,
    QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
    QWEN3_TARGET_CONFIG_BYTES, QWEN3_TARGET_PREPACKED_MANIFEST_BYTES,
    QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TOKENIZER_BYTES, QWEN3_TOKENIZER_METADATA_BYTES,
    SpecialTokenEncodePolicy, TARGET_REPOSITORY, TARGET_REVISION, TokenizerExecutionLimits,
    authenticate_qwen3_tokenizer, build_authenticated_model_weight_layout,
    build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
    build_prepacked_deployment_bundle, decode_bundle_admission_record,
    encode_canonical_deployment_bundle, expected_preliminary_kernel_catalog_identity,
    expected_qwen3_gfx942_runner_source_identity, generate_qwen3_gfx942_runner_declaration,
    plan_authenticated_model_memory, publish_qwen3_gfx942_runner_declaration, qwen3_kv_arena_bytes,
    reopen_persisted_qwen3_weights, seal_authenticated_bundle,
};
use ferric_engine::{M1PartitionedModelMemoryKvPoolV1, M1PhysicalRunnerV1};
use ferric_spec::{EngineLimits, Identity, Qwen3ModelRole};
use rustix::fd::OwnedFd;
use rustix::fs::{CWD, Dir, FileType, Mode, OFlags, ResolveFlags, Stat, fstat, openat2};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::thread;

use super::{EngineeringIdentityInputV1, EngineeringObservationFacts};

type SmokeResult<T> = Result<T, String>;

const CLOSURE_FORMAT: &str = "FERRIC-M1-QUALIFICATION-CLOSURE-V1";
const MAX_DOCUMENT_BYTES: u64 = 8 * 1_024 * 1_024;
const DERIVED_ENGINEERING_IDENTITY_DOMAIN_V1: &[u8] =
    b"ferric.m1.engineering-derived-external-identity.v1";
const DERIVED_ENGINEERING_COMPONENT_LABELS_V1: [&[u8]; 12] = [
    b"ferric-source-observation",
    b"fe2o3-source-observation",
    b"compiler-observation",
    b"compiler-configuration-observation",
    b"target-contract-observation",
    b"kernel-proof-set-observation",
    b"kernel-abi-catalog-observation",
    b"runtime-contract-observation",
    b"runtime-abi-observation",
    b"validator-roster-observation",
    b"protocol-observation",
    b"tcb-report-observation",
];

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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
struct EngineeringModelPlanCoordinatesV1 {
    admission_record: Identity,
    model_bundle: Identity,
    target_prepacked: Identity,
    draft_prepacked: Identity,
    plan_catalog: Identity,
}

pub(crate) struct SmokeBootstrapV1 {
    pub(crate) publication: PublishedRunnerDeclaration,
    memory_plan: ferric_build::AddresslessModelMemoryPlan,
    target_weights: Box<[u8]>,
    draft_weights: Box<[u8]>,
    pub(crate) tokenizer: AuthenticatedTokenizer,
    pub(crate) prompt_tokens: Vec<u32>,
}

pub(crate) fn prepare(
    prepacked_root: &Path,
    identity_input: &EngineeringIdentityInputV1,
    prompt: &str,
    observation: EngineeringObservationFacts,
) -> SmokeResult<SmokeBootstrapV1> {
    let external_file = match identity_input {
        EngineeringIdentityInputV1::ExternalFile(path) => Some(load_closure(path)?),
        EngineeringIdentityInputV1::DerivedEngineeringV1 => None,
    };
    let snapshot = SecureDirectory::open(prepacked_root, "prepacked snapshot root")?;
    let model = load_model_inputs(&snapshot)?;
    let tokenizer =
        authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&model.tokenizer))
            .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
    let prompt_tokens = tokenizer
        .encode(
            prompt,
            TokenizerExecutionLimits::m1(),
            SpecialTokenEncodePolicy::Reject,
        )
        .map_err(|error| format!("cannot encode raw prompt: {error}"))?;

    let (publication, memory_admission) = authenticate_model_inputs_in_parallel_v1(
        |_| model.authenticate(),
        |runner_admission| {
            let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
                .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
            let external = match external_file {
                Some(closure) => {
                    complete_closure(&closure, &plan_catalog, observation.program_catalog)?
                }
                None => derive_engineering_external_identity_inputs_v1(&plan_catalog, observation)?,
            };
            let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
                .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
            let declaration =
                generate_qwen3_gfx942_runner_declaration(identity_closure).map_err(|error| {
                    format!("cannot generate authenticated runner declaration: {error:?}")
                })?;
            publish_qwen3_gfx942_runner_declaration(declaration)
                .map_err(|error| format!("cannot publish runner declaration: {error:?}"))
        },
    )?;
    let memory_plan = model_memory_plan(memory_admission)?;
    Ok(SmokeBootstrapV1 {
        publication,
        memory_plan,
        target_weights: model.target_weights,
        draft_weights: model.draft_weights,
        tokenizer,
        prompt_tokens,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelAuthenticationPurposeV1 {
    Runner,
    Memory,
}

fn authenticate_model_inputs_in_parallel_v1<T, R>(
    authenticate: impl Fn(ModelAuthenticationPurposeV1) -> SmokeResult<T> + Sync,
    build_runner: impl FnOnce(T) -> SmokeResult<R>,
) -> SmokeResult<(R, T)>
where
    T: Send,
{
    thread::scope(|scope| {
        let memory = thread::Builder::new()
            .name("ferric-smoke-memory-authentication".to_owned())
            .spawn_scoped(scope, || authenticate(ModelAuthenticationPurposeV1::Memory))
            .map_err(|_| "cannot start memory model authentication worker".to_owned())?;
        let runner_result =
            authenticate(ModelAuthenticationPurposeV1::Runner).and_then(build_runner);
        let memory_result = match memory.join() {
            Ok(result) => result,
            Err(_) => Err("memory model authentication worker panicked".to_owned()),
        };

        match runner_result {
            Ok(runner) => memory_result.map(|memory| (runner, memory)),
            Err(error) => Err(error),
        }
    })
}

fn derive_engineering_external_identity_inputs_v1(
    catalog: &ferric_build::SequentialPlanCatalog,
    observation: EngineeringObservationFacts,
) -> SmokeResult<ExternalIdentityClosureInputs> {
    let admission_record = catalog
        .admission_record()
        .ok_or_else(|| "engineering model plan has no admission record".to_owned())?
        .record_id();
    let model = EngineeringModelPlanCoordinatesV1 {
        admission_record,
        model_bundle: catalog.deployment().bundle_id,
        target_prepacked: Identity::new(catalog.prepacked().target_manifest().aggregate_id()),
        draft_prepacked: Identity::new(catalog.prepacked().draft_manifest().aggregate_id()),
        plan_catalog: catalog.catalog_id(),
    };
    let component = |label| derive_engineering_component_identity_v1(label, observation, model);
    let [
        ferric_source,
        fe2o3_source,
        compiler,
        compiler_configuration,
        target_contract,
        kernel_proof_set,
        kernel_abi_catalog,
        runtime_contract,
        runtime_abi,
        validator_registry,
        protocol,
        tcb_report,
    ] = DERIVED_ENGINEERING_COMPONENT_LABELS_V1;
    let mut external = ExternalIdentityClosureInputs {
        ferric_source: component(ferric_source),
        fe2o3_source: component(fe2o3_source),
        compiler: component(compiler),
        compiler_configuration: component(compiler_configuration),
        target_contract: component(target_contract),
        kernel_catalog: domain_identity(
            b"ferric.m1.engineering-pending-kernel-catalog.v1",
            &[catalog.catalog_id().as_bytes()],
        ),
        kernel_proof_set: component(kernel_proof_set),
        kernel_abi_catalog: component(kernel_abi_catalog),
        executable_catalog: observation.program_catalog,
        runtime_contract: component(runtime_contract),
        runtime_abi: component(runtime_abi),
        generated_runner: expected_qwen3_gfx942_runner_source_identity(),
        validator_registry: component(validator_registry),
        qualification_protocol: component(protocol),
        tcb_report: component(tcb_report),
    };
    external.kernel_catalog = expected_preliminary_kernel_catalog_identity(catalog, &external)
        .map_err(|error| format!("cannot derive engineering kernel catalog identity: {error:?}"))?;
    Ok(external)
}

fn derive_engineering_component_identity_v1(
    component: &[u8],
    observation: EngineeringObservationFacts,
    model: EngineeringModelPlanCoordinatesV1,
) -> Identity {
    domain_identity(
        DERIVED_ENGINEERING_IDENTITY_DOMAIN_V1,
        &[
            component,
            observation.manifest.as_bytes(),
            observation.hsaco.as_bytes(),
            observation.compiler_handoff.as_bytes(),
            observation.canonical_descriptor.as_bytes(),
            observation.program_catalog.as_bytes(),
            model.admission_record.as_bytes(),
            model.model_bundle.as_bytes(),
            model.target_prepacked.as_bytes(),
            model.draft_prepacked.as_bytes(),
            model.plan_catalog.as_bytes(),
        ],
    )
}

impl SmokeBootstrapV1 {
    pub(crate) fn bind(
        self,
        bind_publication: impl FnOnce(PublishedRunnerDeclaration) -> SmokeResult<M1PhysicalRunnerV1>,
    ) -> SmokeResult<BoundSmokeBootstrapV1> {
        let runner = bind_publication(self.publication)?;
        Ok(BoundSmokeBootstrapV1 {
            runner,
            memory_plan: self.memory_plan,
            target_weights: self.target_weights,
            draft_weights: self.draft_weights,
            tokenizer: self.tokenizer,
            prompt_tokens: self.prompt_tokens,
        })
    }
}

pub(crate) struct BoundSmokeBootstrapV1 {
    runner: M1PhysicalRunnerV1,
    memory_plan: ferric_build::AddresslessModelMemoryPlan,
    target_weights: Box<[u8]>,
    draft_weights: Box<[u8]>,
    tokenizer: AuthenticatedTokenizer,
    prompt_tokens: Vec<u32>,
}

impl BoundSmokeBootstrapV1 {
    pub(crate) fn initialize_memory(
        self,
        checked: fe2o3_kfd::CheckedGfx942XnackMinusDevice,
    ) -> SmokeResult<InitializedSmokeBootstrapV1> {
        let memory = ferric_engine::initialize_m1_physical_runner_memory_v1(
            checked,
            self.memory_plan,
            self.target_weights,
            self.draft_weights,
        )
        .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;
        Ok(InitializedSmokeBootstrapV1 {
            runner: self.runner,
            memory,
            tokenizer: self.tokenizer,
            prompt_tokens: self.prompt_tokens,
        })
    }
}

pub(crate) struct InitializedSmokeBootstrapV1 {
    pub(crate) runner: M1PhysicalRunnerV1,
    pub(crate) memory: M1PartitionedModelMemoryKvPoolV1,
    pub(crate) tokenizer: AuthenticatedTokenizer,
    pub(crate) prompt_tokens: Vec<u32>,
}

struct ModelInputBytes {
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

impl ModelInputBytes {
    fn authenticate(&self) -> SmokeResult<AuthenticatedBundleAdmission> {
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
        let target_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&self.tokenizer))
                .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
        let draft_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Draft06B, Cursor::new(&self.tokenizer))
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

fn load_model_inputs(snapshot: &SecureDirectory) -> SmokeResult<ModelInputBytes> {
    snapshot
        .validate_exact_regular_file_roster(&MODEL_SNAPSHOT_FILES_V1, "prepacked model snapshot")?;
    let model = ModelInputBytes {
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
        draft_config: snapshot.read_exact(
            Path::new("draft.config.json"),
            QWEN3_DRAFT_CONFIG_BYTES,
            "draft config",
        )?,
        draft_manifest: snapshot.read_exact(
            Path::new("draft.weights.manifest.bin"),
            u64::from(QWEN3_DRAFT_PREPACKED_MANIFEST_BYTES),
            "draft weight manifest",
        )?,
        draft_tokenizer_metadata: snapshot.read_exact(
            Path::new("draft.tokenizer_config.json"),
            QWEN3_TOKENIZER_METADATA_BYTES,
            "draft tokenizer metadata",
        )?,
        draft_weights: snapshot
            .read_exact(
                Path::new("draft.weights.bin"),
                QWEN3_DRAFT_TENSOR_DATA_BYTES,
                "draft prepacked weights",
            )?
            .into_boxed_slice(),
        target_config: snapshot.read_exact(
            Path::new("target.config.json"),
            QWEN3_TARGET_CONFIG_BYTES,
            "target config",
        )?,
        target_manifest: snapshot.read_exact(
            Path::new("target.weights.manifest.bin"),
            u64::from(QWEN3_TARGET_PREPACKED_MANIFEST_BYTES),
            "target weight manifest",
        )?,
        target_tokenizer_metadata: snapshot.read_exact(
            Path::new("target.tokenizer_config.json"),
            QWEN3_TOKENIZER_METADATA_BYTES,
            "target tokenizer metadata",
        )?,
        target_weights: snapshot
            .read_exact(
                Path::new("target.weights.bin"),
                QWEN3_TARGET_TENSOR_DATA_BYTES,
                "target prepacked weights",
            )?
            .into_boxed_slice(),
        tokenizer: snapshot.read_exact(
            Path::new("tokenizer.json"),
            QWEN3_TOKENIZER_BYTES,
            "shared target and draft tokenizer",
        )?,
    };
    snapshot
        .validate_exact_regular_file_roster(&MODEL_SNAPSHOT_FILES_V1, "prepacked model snapshot")?;
    Ok(model)
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
) -> SmokeResult<()> {
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
) -> SmokeResult<ferric_build::AddresslessModelMemoryPlan> {
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

fn load_closure(path: &Path) -> SmokeResult<ClosureIdentities> {
    let (root, relative) = secure_parent(path, "qualification closure")?;
    let bytes = root.read_bounded(&relative, MAX_DOCUMENT_BYTES, "qualification closure")?;
    let value = parse_canonical(&bytes, "qualification closure")?;
    parse_closure_document(&value)
}

fn parse_closure_document(value: &Value) -> SmokeResult<ClosureIdentities> {
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
) -> SmokeResult<ExternalIdentityClosureInputs> {
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

struct SecureDirectory {
    descriptor: OwnedFd,
}

struct SecureFile {
    file: File,
    initial: Stat,
}

impl SecureDirectory {
    fn open(path: &Path, description: &str) -> SmokeResult<Self> {
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
    ) -> SmokeResult<Vec<u8>> {
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
    ) -> SmokeResult<Vec<u8>> {
        let mut input = self.open_file(relative, description)?;
        let length = input.length(description)?;
        if u64::try_from(length).ok() != Some(expected_bytes) {
            return Err(format!("{description} length drifted"));
        }
        input.read_exact_snapshot(length, description)
    }

    fn validate_exact_regular_file_roster(
        &self,
        expected: &[SnapshotFileV1],
        description: &str,
    ) -> SmokeResult<()> {
        let expected_names = expected
            .iter()
            .map(|file| file.name.to_owned())
            .collect::<BTreeSet<_>>();
        if expected_names.len() != expected.len() {
            return Err(format!(
                "{description} expected roster contains a duplicate"
            ));
        }
        if self.directory_roster(description)? != expected_names {
            return Err(format!(
                "{description} must contain exactly the admitted regular-file roster"
            ));
        }
        for file in expected {
            let member_description = format!("{description} member {}", file.name);
            let member = self.open_file(Path::new(file.name), &member_description)?;
            if u64::try_from(member.length(&member_description)?).ok() != Some(file.bytes) {
                return Err(format!("{member_description} length drifted"));
            }
            member.validate_snapshot(&member_description)?;
        }
        Ok(())
    }

    fn directory_roster(&self, description: &str) -> SmokeResult<BTreeSet<String>> {
        let mut entries = Dir::read_from(&self.descriptor)
            .map_err(|error| format!("cannot enumerate {description}: {error}"))?;
        let mut names = BTreeSet::new();
        while let Some(entry) = entries.read() {
            let entry =
                entry.map_err(|error| format!("cannot enumerate {description}: {error}"))?;
            let bytes = entry.file_name().to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            if !bytes.is_ascii() {
                return Err(format!("{description} filename must be ASCII"));
            }
            let name = std::str::from_utf8(bytes)
                .map_err(|_| format!("{description} filename must be UTF-8"))?;
            require_relative(Path::new(name), &format!("{description} member"))?;
            if !names.insert(name.to_owned()) {
                return Err(format!("{description} contains a duplicate filename"));
            }
        }
        Ok(names)
    }

    fn open_file(&self, relative: &Path, description: &str) -> SmokeResult<SecureFile> {
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
    fn length(&self, description: &str) -> SmokeResult<usize> {
        usize::try_from(self.initial.st_size)
            .map_err(|_| format!("{description} is too large for this host"))
    }

    fn read_exact_snapshot(&mut self, length: usize, description: &str) -> SmokeResult<Vec<u8>> {
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

    fn validate_snapshot(&self, description: &str) -> SmokeResult<()> {
        let final_stat = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if !same_file_snapshot(&self.initial, &final_stat) {
            return Err(format!("{description} changed while being read"));
        }
        Ok(())
    }
}

fn secure_parent(path: &Path, description: &str) -> SmokeResult<(SecureDirectory, PathBuf)> {
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

fn require_relative(path: &Path, description: &str) -> SmokeResult<()> {
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

fn parse_canonical(bytes: &[u8], description: &str) -> SmokeResult<Value> {
    if !bytes.is_ascii() {
        return Err(format!("{description} must be ASCII JSON"));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse {description}: {error}"))?;
    let mut canonical = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("cannot serialize canonical JSON: {error}"))?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(format!("{description} is not canonical JSON"));
    }
    Ok(value)
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> SmokeResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!("{description} field roster drifted"));
    }
    Ok(object)
}

fn string_field<'a>(object: &'a Map<String, Value>, name: &str) -> SmokeResult<&'a str> {
    object
        .get(name)
        .ok_or_else(|| format!("required field is absent: {name}"))?
        .as_str()
        .ok_or_else(|| format!("field {name} must be a string"))
}

fn identity_field(object: &Map<String, Value>, name: &str) -> SmokeResult<Identity> {
    decode_identity(string_field(object, name)?)
}

fn expect_string(object: &Map<String, Value>, name: &str, expected: &str) -> SmokeResult<()> {
    if string_field(object, name)? != expected {
        return Err(format!("field {name} has an unexpected value"));
    }
    Ok(())
}

fn decode_identity(value: &str) -> SmokeResult<Identity> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == b'0')
    {
        return Err("invalid lowercase SHA-256 identity".to_owned());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    fn observation() -> EngineeringObservationFacts {
        EngineeringObservationFacts {
            manifest: identity(1),
            hsaco: identity(2),
            compiler_handoff: identity(3),
            canonical_descriptor: identity(4),
            program_catalog: identity(5),
        }
    }

    fn model() -> EngineeringModelPlanCoordinatesV1 {
        EngineeringModelPlanCoordinatesV1 {
            admission_record: identity(6),
            model_bundle: identity(7),
            target_prepacked: identity(8),
            draft_prepacked: identity(9),
            plan_catalog: identity(10),
        }
    }

    #[test]
    fn derived_engineering_components_are_deterministic_nonzero_and_distinct() {
        let observation = observation();
        let model = model();
        let first = DERIVED_ENGINEERING_COMPONENT_LABELS_V1
            .map(|label| derive_engineering_component_identity_v1(label, observation, model));
        let second = DERIVED_ENGINEERING_COMPONENT_LABELS_V1
            .map(|label| derive_engineering_component_identity_v1(label, observation, model));
        assert_eq!(first, second);
        assert!(first.iter().all(Identity::is_present));
        assert_eq!(
            first
                .iter()
                .map(|identity| *identity.as_bytes())
                .collect::<BTreeSet<_>>()
                .len(),
            DERIVED_ENGINEERING_COMPONENT_LABELS_V1.len()
        );
        assert!(first.iter().all(|identity| {
            *identity != observation.program_catalog
                && *identity != expected_qwen3_gfx942_runner_source_identity()
        }));
    }

    #[test]
    fn every_admitted_observation_and_model_plan_coordinate_rekeys_derivation() {
        let observation = observation();
        let model = model();
        let label = DERIVED_ENGINEERING_COMPONENT_LABELS_V1[0];
        let expected = derive_engineering_component_identity_v1(label, observation, model);

        for coordinate in 0..10 {
            let mut changed_observation = observation;
            let mut changed_model = model;
            let replacement = identity(32 + coordinate);
            match coordinate {
                0 => changed_observation.manifest = replacement,
                1 => changed_observation.hsaco = replacement,
                2 => changed_observation.compiler_handoff = replacement,
                3 => changed_observation.canonical_descriptor = replacement,
                4 => changed_observation.program_catalog = replacement,
                5 => changed_model.admission_record = replacement,
                6 => changed_model.model_bundle = replacement,
                7 => changed_model.target_prepacked = replacement,
                8 => changed_model.draft_prepacked = replacement,
                9 => changed_model.plan_catalog = replacement,
                _ => return,
            }
            assert_ne!(
                derive_engineering_component_identity_v1(label, changed_observation, changed_model),
                expected,
                "coordinate {coordinate} did not rekey the derived identity"
            );
        }
    }

    #[test]
    fn runner_and_memory_authentication_each_run_once_and_overlap() {
        let rendezvous = Barrier::new(2);
        let runner_calls = AtomicUsize::new(0);
        let memory_calls = AtomicUsize::new(0);
        let (runner, memory) = authenticate_model_inputs_in_parallel_v1(
            |purpose| {
                match purpose {
                    ModelAuthenticationPurposeV1::Runner => {
                        runner_calls.fetch_add(1, Ordering::SeqCst);
                    }
                    ModelAuthenticationPurposeV1::Memory => {
                        memory_calls.fetch_add(1, Ordering::SeqCst);
                    }
                }
                rendezvous.wait();
                Ok(purpose)
            },
            Ok,
        )
        .expect("both independent admissions complete");

        assert_eq!(runner, ModelAuthenticationPurposeV1::Runner);
        assert_eq!(memory, ModelAuthenticationPurposeV1::Memory);
        assert_eq!(runner_calls.load(Ordering::SeqCst), 1);
        assert_eq!(memory_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn runner_authentication_error_has_deterministic_precedence() {
        let error = authenticate_model_inputs_in_parallel_v1(
            |purpose| -> SmokeResult<ModelAuthenticationPurposeV1> {
                match purpose {
                    ModelAuthenticationPurposeV1::Runner => Err("runner authentication".to_owned()),
                    ModelAuthenticationPurposeV1::Memory => Err("memory authentication".to_owned()),
                }
            },
            Ok,
        )
        .expect_err("both admissions fail");
        assert_eq!(error, "runner authentication");
    }

    #[test]
    fn runner_pipeline_error_precedes_memory_authentication_error() {
        let error = authenticate_model_inputs_in_parallel_v1(
            |purpose| match purpose {
                ModelAuthenticationPurposeV1::Runner => Ok(purpose),
                ModelAuthenticationPurposeV1::Memory => Err("memory authentication".to_owned()),
            },
            |_| Err::<(), _>("runner pipeline".to_owned()),
        )
        .expect_err("runner construction and memory admission fail");
        assert_eq!(error, "runner pipeline");
    }

    #[test]
    fn memory_authentication_panic_fails_closed_without_payload_disclosure() {
        let error = authenticate_model_inputs_in_parallel_v1(
            |purpose| match purpose {
                ModelAuthenticationPurposeV1::Runner => Ok(purpose),
                ModelAuthenticationPurposeV1::Memory => {
                    panic!("untrusted worker panic payload")
                }
            },
            Ok,
        )
        .expect_err("memory worker panic must reject startup");
        assert_eq!(error, "memory model authentication worker panicked");
        assert!(!error.contains("untrusted worker panic payload"));
    }
}
