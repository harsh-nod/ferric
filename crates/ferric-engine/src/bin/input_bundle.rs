use super::{
    aggregate_identity, build_authenticated_sequential_plan_catalog,
    build_preliminary_identity_closure, canonical_bytes, complete_closure,
    current_executable_sha256, dispatch_graph_identity_name, domain_identity, exact_object,
    expect_string, field, generate_qwen3_gfx942_runner_declaration, hex_identity, integer_field,
    kind_selection, load_model_inputs, parse_canonical, parse_cases, parse_closure_document,
    parse_environment_document, parse_identities, parse_input_tokens, parse_plan_document,
    parse_workload_document, reopen_persisted_m1_kernel_artifacts_v1, require_identity,
    require_relative, require_sha256, require_supported_capture, rows_for_kind, secure_parent,
    selection_json, sha256_array, sha256_hex, string_field, validate_plan_identities,
    validate_roster_document, CaptureResult, ClosureIdentities, DifferentialPlan, ModelInputBytes,
    PlanCase, SecureDirectory, StagingOutput, DECODE_CONTEXT_LENGTH, DIFFERENTIAL_KINDS,
    DIFFERENTIAL_NONCLAIM, ENVIRONMENT_FORMAT, M1_QUALIFICATION_TOKENS_PER_LANE, PLAN_FORMAT,
    ROSTER_FORMAT, TARGET, WORKLOAD_FORMAT,
};
use ferric_spec::{Qwen3ExecutionMode, Qwen3PlanSelection};
use rustix::fs::Dir;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::Path;

const BENCHMARK_INPUT_FORMAT: &str = "FERRIC-M1-BENCHMARK-INPUT-V1";
const ACCEPTANCE_POLICY_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1";
const ACCEPTANCE_POLICY_AUTHORITY: &str = "externally-admitted-differential-threshold-policy-only";
const ACCEPTANCE_POLICY_NONCLAIM: &str = "This artifact supplies plan-admitted differential thresholds only. It does not establish independent review, numerical correctness, hardware correctness, qualification authority, or close m1.r29.";
const INVOCATION_FORMAT: &str = "FERRIC-M1-QUALIFICATION-INVOCATIONS-V1";
const TOKEN_ID_DOMAIN: &[u8] = b"ferric.m1.qualification-token.v1";
const BASE_VOCABULARY_SIZE: u32 = 151_643;
const COMPLETION_WAIT_POLICY_ID: &str = "ferric-m1-completion-progress-wait-v2";
const MAX_CONSECUTIVE_SCANS_WITHOUT_PROGRESS: u32 = 8_192;
const MINIMUM_PENDING_SCAN_PAUSE_MICROS: u64 = 10_000;
const COMPLETION_WAIT_TIMEOUT_BASIS: &str = "paced-completion-signal-scans";
const TOTAL_SCAN_BOUND_RULE: &str = "(packet-count+1)*max-consecutive-scans-without-progress";
const EXACT_BUNDLE_FILE_COUNT: usize = 20;

const BENCHMARK_INPUT_PATH: &str = "benchmark-input.json";
const PLAN_PATH: &str = "plan.json";
const ROSTER_PATH: &str = "roster.json";
const CLOSURE_PATH: &str = "closure.json";
const ENVIRONMENT_PATH: &str = "environment.json";
const ACCEPTANCE_POLICY_PATH: &str = "acceptance-policy.json";

#[derive(Debug)]
struct CaseDocuments {
    cases: Vec<PlanCase>,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug)]
struct InputDocuments {
    files: BTreeMap<String, Vec<u8>>,
}

struct ReconstructedInputs {
    declaration: ferric_build::GeneratedRunnerDeclaration,
    deployment: ferric_spec::DeploymentBundle,
    model: ModelInputBytes,
}

pub(super) fn generate_inputs(arguments: &[OsString]) -> CaptureResult<()> {
    let [prepacked_root, artifact_root, closure_path, policy_path, reference_implementation_path, reference_protocol_path, gpu_unique_id, output] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture generate-inputs PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ACCEPTANCE-POLICY REFERENCE-IMPLEMENTATION REFERENCE-PROTOCOL GPU-ID OUTPUT".to_owned());
    };
    let gpu_unique_id = parse_gpu_unique_id(gpu_unique_id)?;
    let (closure_value, closure_bytes) =
        read_canonical_external(Path::new(closure_path), "qualification closure")?;
    let closure = parse_closure_document(&closure_value)?;
    let (policy_value, policy_bytes) =
        read_canonical_external(Path::new(policy_path), "differential acceptance policy")?;
    validate_acceptance_policy(&policy_value)?;
    let reference_implementation = measure_regular_file(
        Path::new(reference_implementation_path),
        "reference implementation",
    )?;
    let reference_protocol =
        measure_regular_file(Path::new(reference_protocol_path), "reference protocol")?;
    let executable = current_executable_sha256()?;
    let reconstructed = reconstruct_inputs(
        Path::new(prepacked_root),
        Path::new(artifact_root),
        &closure,
    )?;
    let documents = build_input_documents(
        &closure_bytes,
        &policy_bytes,
        gpu_unique_id,
        &executable,
        &reference_implementation,
        &reference_protocol,
        &reconstructed,
    )?;
    let plan = validate_protocol_documents(&documents, gpu_unique_id, &closure, &reconstructed)?;
    let invocation_map = invocation_map_bytes(
        Path::new(output),
        Path::new(prepacked_root),
        Path::new(artifact_root),
        gpu_unique_id,
        &plan,
    )?;
    publish_documents(&documents, Path::new(output))?;
    write_invocation_map(&invocation_map)
}

pub(super) fn validate_inputs(arguments: &[OsString]) -> CaptureResult<()> {
    let [prepacked_root, artifact_root, closure_path, policy_path, reference_implementation_path, reference_protocol_path, gpu_unique_id, input_bundle] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture validate-inputs PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE ACCEPTANCE-POLICY REFERENCE-IMPLEMENTATION REFERENCE-PROTOCOL GPU-ID INPUT-BUNDLE".to_owned());
    };
    let gpu_unique_id = parse_gpu_unique_id(gpu_unique_id)?;
    require_published_roster(Path::new(input_bundle))?;
    let (closure_value, closure_bytes) =
        read_canonical_external(Path::new(closure_path), "qualification closure")?;
    let closure = parse_closure_document(&closure_value)?;
    let (policy_value, policy_bytes) =
        read_canonical_external(Path::new(policy_path), "differential acceptance policy")?;
    validate_acceptance_policy(&policy_value)?;
    let reference_implementation = measure_regular_file(
        Path::new(reference_implementation_path),
        "reference implementation",
    )?;
    let reference_protocol =
        measure_regular_file(Path::new(reference_protocol_path), "reference protocol")?;
    let executable = current_executable_sha256()?;
    let reconstructed = reconstruct_inputs(
        Path::new(prepacked_root),
        Path::new(artifact_root),
        &closure,
    )?;
    let expected = build_input_documents(
        &closure_bytes,
        &policy_bytes,
        gpu_unique_id,
        &executable,
        &reference_implementation,
        &reference_protocol,
        &reconstructed,
    )?;
    compare_published_documents(Path::new(input_bundle), &expected)?;
    let plan = validate_protocol_documents(&expected, gpu_unique_id, &closure, &reconstructed)?;
    invocation_map_bytes(
        Path::new(input_bundle),
        Path::new(prepacked_root),
        Path::new(artifact_root),
        gpu_unique_id,
        &plan,
    )?;
    println!("status=VALIDATED");
    println!("plan_sha256={}", plan.sha256());
    Ok(())
}

fn parse_gpu_unique_id(value: &OsStr) -> CaptureResult<u64> {
    let value = value
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    if value == 0 {
        return Err("GPU unique ID must be nonzero".to_owned());
    }
    Ok(value)
}

fn read_canonical_external(path: &Path, description: &str) -> CaptureResult<(Value, Vec<u8>)> {
    let (root, relative) = secure_parent(path, description)?;
    root.read_canonical(&relative, description)
}

fn measure_regular_file(path: &Path, description: &str) -> CaptureResult<String> {
    let (root, relative) = secure_parent(path, description)?;
    root.open_file(&relative, description)?
        .sha256_snapshot(description)
}

fn reconstruct_inputs(
    prepacked_root: &Path,
    artifact_root: &Path,
    closure: &ClosureIdentities,
) -> CaptureResult<ReconstructedInputs> {
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(artifact_root)
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog = artifacts.program_catalog_id();
    let snapshot = SecureDirectory::open(prepacked_root, "prepacked snapshot root")?;
    let model = load_model_inputs(&snapshot)?;
    let runner_admission = model.authenticate()?;
    let deployment = *runner_admission.prepacked().deployment();
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(closure, &plan_catalog, executable_catalog)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    Ok(ReconstructedInputs {
        declaration,
        deployment,
        model,
    })
}

fn build_input_documents(
    closure_bytes: &[u8],
    policy_bytes: &[u8],
    gpu_unique_id: u64,
    executable: &str,
    reference_implementation: &str,
    reference_protocol: &str,
    reconstructed: &ReconstructedInputs,
) -> CaptureResult<InputDocuments> {
    let cases = build_case_documents()?;
    let environment_bytes = canonical_bytes(&json!({
        "format": ENVIRONMENT_FORMAT,
        "gpu_unique_id": gpu_unique_id,
        "target": TARGET,
    }))?;
    let roster_bytes = canonical_bytes(&json!({
        "cases": case_values(&cases.cases),
        "format": ROSTER_FORMAT,
        "suite": "differential",
    }))?;
    let mut identities = derived_identities(
        &cases.cases,
        closure_bytes,
        policy_bytes,
        &environment_bytes,
        executable,
        reference_implementation,
        reference_protocol,
        reconstructed,
    )?;
    identities.insert("workload-roster".to_owned(), sha256_hex(&roster_bytes));
    assemble_input_documents(
        cases,
        closure_bytes,
        policy_bytes,
        environment_bytes,
        roster_bytes,
        identities,
    )
}

fn assemble_input_documents(
    cases: CaseDocuments,
    closure_bytes: &[u8],
    policy_bytes: &[u8],
    environment_bytes: Vec<u8>,
    roster_bytes: Vec<u8>,
    identities: BTreeMap<String, String>,
) -> CaptureResult<InputDocuments> {
    let benchmark_input = json!({
        "cases": case_values(&cases.cases),
        "format": BENCHMARK_INPUT_FORMAT,
        "identities": identities,
        "suite": "differential",
        "target": TARGET,
    });
    let benchmark_input_bytes = canonical_bytes(&benchmark_input)?;
    let plan_bytes = canonical_bytes(&json!({
        "authority": "benchmark-run-plan-only",
        "cases": case_values(&cases.cases),
        "format": PLAN_FORMAT,
        "identities": field(
            benchmark_input.as_object().ok_or_else(|| "generated benchmark input must be an object".to_owned())?,
            "identities",
        )?,
        "input_sha256": sha256_hex(&benchmark_input_bytes),
        "milestone": "M1",
        "nonclaim": DIFFERENTIAL_NONCLAIM,
        "obligation_id": "m1.r29",
        "path_id": "differential-bench",
        "source_path": "benches/m1/differential.rs",
        "suite": "differential",
        "target": TARGET,
    }))?;
    let mut files = cases.files;
    files.insert(BENCHMARK_INPUT_PATH.to_owned(), benchmark_input_bytes);
    files.insert(PLAN_PATH.to_owned(), plan_bytes);
    files.insert(ROSTER_PATH.to_owned(), roster_bytes);
    files.insert(CLOSURE_PATH.to_owned(), closure_bytes.to_vec());
    files.insert(ENVIRONMENT_PATH.to_owned(), environment_bytes);
    files.insert(ACCEPTANCE_POLICY_PATH.to_owned(), policy_bytes.to_vec());
    if files.len() != EXACT_BUNDLE_FILE_COUNT {
        return Err("generated qualification input file roster drifted".to_owned());
    }
    Ok(InputDocuments { files })
}

#[allow(clippy::too_many_arguments)]
fn derived_identities(
    cases: &[PlanCase],
    closure_bytes: &[u8],
    policy_bytes: &[u8],
    environment_bytes: &[u8],
    executable: &str,
    reference_implementation: &str,
    reference_protocol: &str,
    reconstructed: &ReconstructedInputs,
) -> CaptureResult<BTreeMap<String, String>> {
    require_sha256(executable)?;
    require_sha256(reference_implementation)?;
    require_sha256(reference_protocol)?;
    let closure_value = parse_canonical(closure_bytes, "qualification closure")?;
    let closure = parse_closure_document(&closure_value)?;
    let deployment = &reconstructed.deployment;
    let declaration = &reconstructed.declaration;
    let model = &reconstructed.model;
    let config = aggregate_identity(
        b"ferric.m1.deployment-configs.v1",
        &[
            deployment.target_model.config.config_id,
            deployment.draft_model.config.config_id,
        ],
    );
    let tokenizer = aggregate_identity(
        b"ferric.m1.deployment-tokenizers.v1",
        &[
            deployment.target_model.tokenizer.tokenizer_id,
            deployment.target_model.tokenizer.vocabulary_id,
            deployment.draft_model.tokenizer.tokenizer_id,
            deployment.draft_model.tokenizer.vocabulary_id,
        ],
    );
    let weights = domain_identity(
        b"ferric.m1.deployment-prepacked-weights.v1",
        &[
            &sha256_array(&model.target_manifest),
            &sha256_array(&model.draft_manifest),
            &sha256_array(&model.target_weights),
            &sha256_array(&model.draft_weights),
        ],
    );
    let mut identities = BTreeMap::from([
        ("benchmark-executable".to_owned(), executable.to_owned()),
        (
            "benchmark-protocol".to_owned(),
            hex_identity(closure.qualification_protocol),
        ),
        ("config".to_owned(), hex_identity(config)),
        (
            "dispatch-graph".to_owned(),
            hex_identity(declaration.plan_catalog_id()),
        ),
        ("environment".to_owned(), sha256_hex(environment_bytes)),
        (
            "fe2o3-source-closure".to_owned(),
            hex_identity(closure.fe2o3_source),
        ),
        (
            "ferric-source-closure".to_owned(),
            hex_identity(closure.ferric_source),
        ),
        (
            "generated-plan".to_owned(),
            hex_identity(declaration.declaration_id()),
        ),
        ("model".to_owned(), hex_identity(deployment.bundle_id)),
        (
            "schedule-catalog".to_owned(),
            hex_identity(declaration.kernel_catalog_id()),
        ),
        ("tokenizer".to_owned(), hex_identity(tokenizer)),
        ("weights".to_owned(), hex_identity(weights)),
        (
            "differential-acceptance-policy".to_owned(),
            sha256_hex(policy_bytes),
        ),
        (
            "reference-implementation".to_owned(),
            reference_implementation.to_owned(),
        ),
        (
            "reference-protocol".to_owned(),
            reference_protocol.to_owned(),
        ),
    ]);
    for case in cases {
        let selection = kind_selection(&case.kind)?;
        let selected = declaration
            .plans()
            .iter()
            .find(|candidate| candidate.selection == selection)
            .ok_or_else(|| "generated declaration lacks differential workload plan".to_owned())?;
        identities.insert(
            dispatch_graph_identity_name(&case.kind)?.to_owned(),
            hex_identity(selected.plan_id),
        );
    }
    Ok(identities)
}

fn build_case_documents() -> CaptureResult<CaseDocuments> {
    let mut cases = Vec::with_capacity(DIFFERENTIAL_KINDS.len());
    let mut files = BTreeMap::new();
    for kind in DIFFERENTIAL_KINDS {
        let selection = kind_selection(kind)?;
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .ok_or_else(|| "generated qualification selection has no dimensions".to_owned())?;
        let token_bytes = deterministic_token_bytes(kind, selection)?;
        let token_file = token_path(kind);
        let case_id = format!("{kind}.001");
        let (active_length, context_length) = match selection.mode {
            Qwen3ExecutionMode::Decode => (1, DECODE_CONTEXT_LENGTH),
            Qwen3ExecutionMode::Prefill => (dimensions.active_tokens, 0),
            Qwen3ExecutionMode::Speculative => {
                return Err("generated qualification case must be target-only".to_owned());
            }
        };
        let lanes = (0..dimensions.sequences)
            .map(|_| {
                json!({
                    "active_length": active_length,
                    "context_length": context_length,
                })
            })
            .collect::<Vec<_>>();
        let workload_bytes = canonical_bytes(&json!({
            "case_id": case_id,
            "completion_wait_policy": {
                "id": COMPLETION_WAIT_POLICY_ID,
                "max_consecutive_scans_without_progress": MAX_CONSECUTIVE_SCANS_WITHOUT_PROGRESS,
                "minimum_pending_scan_pause_micros": MINIMUM_PENDING_SCAN_PAUSE_MICROS,
                "timeout_basis": COMPLETION_WAIT_TIMEOUT_BASIS,
                "total_scan_bound_rule": TOTAL_SCAN_BOUND_RULE,
            },
            "format": WORKLOAD_FORMAT,
            "input": {
                "bytes": token_bytes.len(),
                "encoding": "u32-le",
                "path": token_file,
                "sha256": sha256_hex(&token_bytes),
            },
            "kind": kind,
            "lanes": lanes,
            "selection": selection_json(selection),
        }))?;
        cases.push(PlanCase {
            id: format!("{kind}.001"),
            input_sha256: sha256_hex(&token_bytes),
            kind: (*kind).to_owned(),
            workload_sha256: sha256_hex(&workload_bytes),
        });
        files.insert(token_file, token_bytes);
        files.insert(workload_path(kind), workload_bytes);
    }
    Ok(CaseDocuments { cases, files })
}

fn deterministic_token_bytes(kind: &str, selection: Qwen3PlanSelection) -> CaptureResult<Vec<u8>> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or_else(|| "qualification selection has no dimensions".to_owned())?;
    let tokens_per_lane = match selection.mode {
        Qwen3ExecutionMode::Decode => M1_QUALIFICATION_TOKENS_PER_LANE,
        Qwen3ExecutionMode::Prefill => dimensions.active_tokens,
        Qwen3ExecutionMode::Speculative => {
            return Err("qualification input generator accepts target-only modes".to_owned());
        }
    };
    let token_count = usize::try_from(dimensions.sequences)
        .ok()
        .and_then(|sequences| sequences.checked_mul(usize::try_from(tokens_per_lane).ok()?))
        .ok_or_else(|| "generated qualification token count overflowed".to_owned())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(token_count.saturating_mul(4))
        .map_err(|_| "cannot reserve generated qualification tokens".to_owned())?;
    for lane in 0..dimensions.sequences {
        for ordinal in 0..tokens_per_lane {
            bytes.extend_from_slice(&deterministic_token(kind, lane, ordinal).to_le_bytes());
        }
    }
    Ok(bytes)
}

fn deterministic_token(kind: &str, lane: u32, ordinal: u32) -> u32 {
    let lane = lane.to_le_bytes();
    let ordinal = ordinal.to_le_bytes();
    let identity = domain_identity(TOKEN_ID_DOMAIN, &[kind.as_bytes(), &lane, &ordinal]);
    let &[byte_0, byte_1, byte_2, byte_3, ..] = identity.as_bytes();
    u32::from_le_bytes([byte_0, byte_1, byte_2, byte_3]) % BASE_VOCABULARY_SIZE
}

fn case_values(cases: &[PlanCase]) -> Vec<Value> {
    cases
        .iter()
        .map(|case| {
            json!({
                "id": case.id,
                "input_sha256": case.input_sha256,
                "kind": case.kind,
                "workload_sha256": case.workload_sha256,
            })
        })
        .collect()
}

fn workload_path(kind: &str) -> String {
    format!("{kind}.001.workload.json")
}

fn token_path(kind: &str) -> String {
    format!("{kind}.001.tokens.u32le")
}

fn validate_protocol_documents(
    documents: &InputDocuments,
    gpu_unique_id: u64,
    closure: &ClosureIdentities,
    reconstructed: &ReconstructedInputs,
) -> CaptureResult<DifferentialPlan> {
    if documents.files.len() != EXACT_BUNDLE_FILE_COUNT {
        return Err("qualification input bundle file roster drifted".to_owned());
    }
    let plan_bytes = document_bytes(documents, PLAN_PATH)?.to_vec();
    let plan_value = parse_canonical(&plan_bytes, "benchmark plan")?;
    let plan = parse_plan_document(&plan_value, plan_bytes)?;
    let benchmark_input_bytes = document_bytes(documents, BENCHMARK_INPUT_PATH)?;
    let benchmark_input = parse_canonical(benchmark_input_bytes, "benchmark input")?;
    validate_benchmark_input(&benchmark_input, benchmark_input_bytes, &plan)?;
    let roster_bytes = document_bytes(documents, ROSTER_PATH)?;
    let roster = parse_canonical(roster_bytes, "workload roster")?;
    validate_roster_document(&roster, roster_bytes, &plan)?;
    let closure_value = parse_canonical(
        document_bytes(documents, CLOSURE_PATH)?,
        "qualification closure",
    )?;
    if parse_closure_document(&closure_value)? != *closure {
        return Err("bundled qualification closure identity roster drifted".to_owned());
    }
    let environment_bytes = document_bytes(documents, ENVIRONMENT_PATH)?;
    let environment = parse_canonical(environment_bytes, "qualification environment")?;
    if parse_environment_document(&environment)? != gpu_unique_id {
        return Err("bundled environment GPU unique ID drifted".to_owned());
    }
    require_identity(
        plan.identity("environment")?,
        &sha256_hex(environment_bytes),
        "environment",
    )?;
    let policy_bytes = document_bytes(documents, ACCEPTANCE_POLICY_PATH)?;
    let policy = parse_canonical(policy_bytes, "differential acceptance policy")?;
    validate_acceptance_policy(&policy)?;
    require_identity(
        plan.identity("differential-acceptance-policy")?,
        &sha256_hex(policy_bytes),
        "differential acceptance policy",
    )?;
    for case in &plan.cases {
        let workload_bytes = document_bytes(documents, &workload_path(&case.kind))?.to_vec();
        let workload_value = parse_canonical(&workload_bytes, "qualification workload")?;
        let workload = parse_workload_document(&workload_value, workload_bytes, case)?;
        let tokens = document_bytes(documents, &token_path(&case.kind))?;
        parse_input_tokens(tokens, &workload, case)?;
        require_supported_capture(&workload)?;
        validate_plan_identities(
            &plan,
            case,
            closure,
            &reconstructed.declaration,
            &reconstructed.deployment,
            &reconstructed.model,
        )?;
    }
    Ok(plan)
}

fn validate_benchmark_input(
    value: &Value,
    bytes: &[u8],
    plan: &DifferentialPlan,
) -> CaptureResult<()> {
    require_identity(
        &plan.input_sha256,
        &sha256_hex(bytes),
        "benchmark plan input",
    )?;
    let object = exact_object(
        value,
        &["cases", "format", "identities", "suite", "target"],
        "benchmark input",
    )?;
    expect_string(object, "format", BENCHMARK_INPUT_FORMAT)?;
    expect_string(object, "suite", "differential")?;
    expect_string(object, "target", TARGET)?;
    if parse_cases(field(object, "cases")?)? != plan.cases {
        return Err("benchmark input cases differ from the generated plan".to_owned());
    }
    if parse_identities(field(object, "identities")?)? != plan.identities {
        return Err("benchmark input identities differ from the generated plan".to_owned());
    }
    Ok(())
}

fn document_bytes<'a>(documents: &'a InputDocuments, path: &str) -> CaptureResult<&'a [u8]> {
    documents
        .files
        .get(path)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("qualification input document is absent: {path}"))
}

fn publish_documents(documents: &InputDocuments, output: &Path) -> CaptureResult<()> {
    let mut staging = StagingOutput::create(output)?;
    for (path, bytes) in &documents.files {
        staging.write(path, bytes)?;
    }
    staging.publish()
}

fn compare_published_documents(path: &Path, expected: &InputDocuments) -> CaptureResult<()> {
    let root = SecureDirectory::open(path, "qualification input bundle")?;
    let actual_roster = directory_roster(&root)?;
    let expected_roster = expected_file_roster();
    if actual_roster != expected_roster {
        return Err("qualification input bundle has a missing or trailing file".to_owned());
    }
    for (name, expected_bytes) in &expected.files {
        let actual = root.read_exact(
            Path::new(name),
            u64::try_from(expected_bytes.len())
                .map_err(|_| "expected qualification input size does not fit u64".to_owned())?,
            "qualification input bundle member",
        )?;
        if actual != *expected_bytes {
            return Err(format!("qualification input bundle member drifted: {name}"));
        }
    }
    Ok(())
}

fn require_published_roster(path: &Path) -> CaptureResult<()> {
    let root = SecureDirectory::open(path, "qualification input bundle")?;
    if directory_roster(&root)? != expected_file_roster() {
        return Err("qualification input bundle has a missing or trailing file".to_owned());
    }
    Ok(())
}

fn expected_file_roster() -> BTreeSet<String> {
    let mut paths = BTreeSet::from([
        BENCHMARK_INPUT_PATH.to_owned(),
        PLAN_PATH.to_owned(),
        ROSTER_PATH.to_owned(),
        CLOSURE_PATH.to_owned(),
        ENVIRONMENT_PATH.to_owned(),
        ACCEPTANCE_POLICY_PATH.to_owned(),
    ]);
    for kind in DIFFERENTIAL_KINDS {
        paths.insert(workload_path(kind));
        paths.insert(token_path(kind));
    }
    paths
}

fn directory_roster(root: &SecureDirectory) -> CaptureResult<BTreeSet<String>> {
    let mut directory = Dir::read_from(&root.descriptor)
        .map_err(|error| format!("cannot enumerate qualification input bundle: {error}"))?;
    let mut names = BTreeSet::new();
    while let Some(entry) = directory.read() {
        let entry = entry
            .map_err(|error| format!("cannot enumerate qualification input bundle: {error}"))?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        if !name.is_ascii() {
            return Err("qualification input bundle filename must be ASCII".to_owned());
        }
        let name = std::str::from_utf8(name)
            .map_err(|_| "qualification input bundle filename must be UTF-8".to_owned())?;
        require_relative(Path::new(name), "qualification input bundle member")?;
        if !names.insert(name.to_owned()) {
            return Err("qualification input bundle has a duplicate filename".to_owned());
        }
    }
    Ok(names)
}

fn validate_acceptance_policy(value: &Value) -> CaptureResult<()> {
    let policy = exact_object(
        value,
        &[
            "authority",
            "cases",
            "finite_logits_required",
            "format",
            "logit_metric",
            "nonclaim",
            "obligation_id",
            "path_id",
            "suite",
            "target",
            "token_metric",
            "token_selection",
        ],
        "differential acceptance policy",
    )?;
    expect_string(policy, "authority", ACCEPTANCE_POLICY_AUTHORITY)?;
    expect_string(policy, "format", ACCEPTANCE_POLICY_FORMAT)?;
    expect_string(
        policy,
        "logit_metric",
        "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
    )?;
    expect_string(policy, "nonclaim", ACCEPTANCE_POLICY_NONCLAIM)?;
    expect_string(policy, "obligation_id", "m1.r29")?;
    expect_string(policy, "path_id", "differential-bench")?;
    expect_string(policy, "suite", "differential")?;
    expect_string(policy, "target", TARGET)?;
    expect_string(
        policy,
        "token_metric",
        "ferric-reference-greedy-token-mismatch-count",
    )?;
    expect_string(policy, "token_selection", "lowest-token-id-bf16-argmax")?;
    if field(policy, "finite_logits_required")?.as_bool() != Some(true) {
        return Err("differential acceptance policy must require finite logits".to_owned());
    }
    let cases = field(policy, "cases")?
        .as_array()
        .ok_or_else(|| "differential acceptance policy cases must be an array".to_owned())?;
    if cases.len() != DIFFERENTIAL_KINDS.len() {
        return Err("differential acceptance policy must cover seven cases".to_owned());
    }
    let mut prior: Option<&str> = None;
    let mut kinds = BTreeSet::new();
    for case in cases {
        let case = exact_object(
            case,
            &[
                "kind",
                "maximum_logit_ulp_error",
                "maximum_token_mismatches",
            ],
            "differential acceptance policy case",
        )?;
        let kind = string_field(case, "kind")?;
        if prior.is_some_and(|previous| previous >= kind) {
            return Err("differential acceptance policy cases must be sorted".to_owned());
        }
        prior = Some(kind);
        if !DIFFERENTIAL_KINDS.contains(&kind) || !kinds.insert(kind) {
            return Err("differential acceptance policy case roster drifted".to_owned());
        }
        integer_field(case, "maximum_logit_ulp_error")?;
        if integer_field(case, "maximum_token_mismatches")? > rows_for_kind(kind)? {
            return Err("differential token mismatch threshold exceeds row count".to_owned());
        }
    }
    if kinds != DIFFERENTIAL_KINDS.iter().copied().collect() {
        return Err("differential acceptance policy case roster drifted".to_owned());
    }
    Ok(())
}

fn invocation_map_bytes(
    bundle: &Path,
    prepacked_root: &Path,
    artifact_root: &Path,
    gpu_unique_id: u64,
    plan: &DifferentialPlan,
) -> CaptureResult<Vec<u8>> {
    let plan_path = bundle.join(PLAN_PATH);
    let roster_path = bundle.join(ROSTER_PATH);
    let closure_path = bundle.join(CLOSURE_PATH);
    let environment_path = bundle.join(ENVIRONMENT_PATH);
    let invocations = plan
        .cases
        .iter()
        .map(|case| {
            let output = bundle.with_file_name(format!("{}.capture.bundle", case.kind));
            Ok(json!({
                "arguments": [
                    path_string(&plan_path)?,
                    path_string(&roster_path)?,
                    case.id,
                    path_string(&bundle.join(workload_path(&case.kind)))?,
                    path_string(prepacked_root)?,
                    path_string(artifact_root)?,
                    path_string(&closure_path)?,
                    path_string(&environment_path)?,
                    gpu_unique_id.to_string(),
                    path_string(&output)?,
                ],
                "case_id": case.id,
                "kind": case.kind,
            }))
        })
        .collect::<CaptureResult<Vec<_>>>()?;
    canonical_bytes(&json!({
        "command": "ferric-m1-qualification-capture",
        "format": INVOCATION_FORMAT,
        "invocations": invocations,
        "plan_sha256": plan.sha256(),
    }))
}

fn write_invocation_map(bytes: &[u8]) -> CaptureResult<()> {
    std::io::stdout()
        .write_all(bytes)
        .map_err(|error| format!("cannot write qualification invocation map: {error}"))
}

fn path_string(path: &Path) -> CaptureResult<&str> {
    path.to_str()
        .ok_or_else(|| "qualification invocation path must be UTF-8".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        COMMON_IDENTITIES, DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES, DIFFERENTIAL_IDENTITIES,
    };
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Component, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-input-bundle-test.{}.{nonce}",
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

    fn policy_bytes() -> Vec<u8> {
        canonical_bytes(&json!({
            "authority": ACCEPTANCE_POLICY_AUTHORITY,
            "cases": DIFFERENTIAL_KINDS.iter().map(|kind| json!({
                "kind": kind,
                "maximum_logit_ulp_error": 0,
                "maximum_token_mismatches": 0,
            })).collect::<Vec<_>>(),
            "finite_logits_required": true,
            "format": ACCEPTANCE_POLICY_FORMAT,
            "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
            "nonclaim": ACCEPTANCE_POLICY_NONCLAIM,
            "obligation_id": "m1.r29",
            "path_id": "differential-bench",
            "suite": "differential",
            "target": TARGET,
            "token_metric": "ferric-reference-greedy-token-mismatch-count",
            "token_selection": "lowest-token-id-bf16-argmax",
        }))
        .unwrap()
    }

    fn fixture_documents() -> InputDocuments {
        let cases = build_case_documents().unwrap();
        let roster_bytes = canonical_bytes(&json!({
            "cases": case_values(&cases.cases),
            "format": ROSTER_FORMAT,
            "suite": "differential",
        }))
        .unwrap();
        let environment_bytes = canonical_bytes(&json!({
            "format": ENVIRONMENT_FORMAT,
            "gpu_unique_id": 7,
            "target": TARGET,
        }))
        .unwrap();
        let mut identity_names = COMMON_IDENTITIES.to_vec();
        identity_names.extend_from_slice(DIFFERENTIAL_IDENTITIES);
        identity_names.extend(
            DIFFERENTIAL_DISPATCH_GRAPH_IDENTITIES
                .iter()
                .map(|(_, name)| *name),
        );
        let mut identities = identity_names
            .into_iter()
            .map(|name| (name.to_owned(), sha256_hex(name.as_bytes())))
            .collect::<BTreeMap<_, _>>();
        identities.insert("workload-roster".to_owned(), sha256_hex(&roster_bytes));
        assemble_input_documents(
            cases,
            &canonical_bytes(&json!({"closure": "fixture"})).unwrap(),
            &policy_bytes(),
            environment_bytes,
            roster_bytes,
            identities,
        )
        .unwrap()
    }

    #[test]
    fn deterministic_tokens_cover_exact_lane_major_payloads_without_special_ids() {
        let first = build_case_documents().unwrap();
        let second = build_case_documents().unwrap();
        assert_eq!(first.files, second.files);
        assert_eq!(first.cases, second.cases);
        assert_eq!(
            workload_path("decode-s8-c8192"),
            "decode-s8-c8192.001.workload.json"
        );
        assert_eq!(
            token_path("decode-s8-c8192"),
            "decode-s8-c8192.001.tokens.u32le"
        );
        let lane = 0_u32.to_le_bytes();
        let ordinal = 0_u32.to_le_bytes();
        assert_eq!(
            hex_identity(domain_identity(
                TOKEN_ID_DOMAIN,
                &[b"decode-s8-c8192", &lane, &ordinal]
            )),
            "e14f68764882f4cc2b0d48ea4dca777cb2f17b41e84aa8254b281f113a22652e"
        );
        let total = first
            .files
            .iter()
            .filter(|(path, _)| path.ends_with(".tokens.u32le"))
            .map(|(_, bytes)| bytes.len())
            .sum::<usize>();
        assert_eq!(total, 1_358_336);
        for (path, bytes) in &first.files {
            if !path.ends_with(".tokens.u32le") {
                continue;
            }
            for token in bytes.chunks_exact(4) {
                let token = u32::from_le_bytes(token.try_into().unwrap());
                assert!(token < BASE_VOCABULARY_SIZE);
            }
        }
        let decode = first.files.get("decode-s8-c8192.001.tokens.u32le").unwrap();
        assert_eq!(decode.len(), 8 * 8_192 * 4);
        assert_eq!(
            sha256_hex(decode),
            "6676d88d0e2ebbb1a0f945a91f5cb5e16f4089f10a49d7a53b15b31fcd59ae5f"
        );
        assert_eq!(
            &decode[..4],
            &deterministic_token("decode-s8-c8192", 0, 0).to_le_bytes()
        );
        assert_eq!(
            &decode[8_192 * 4..8_192 * 4 + 4],
            &deterministic_token("decode-s8-c8192", 1, 0).to_le_bytes()
        );
    }

    #[test]
    fn generated_benchmark_plan_and_capture_documents_share_exact_schemas() {
        let first = fixture_documents();
        let second = fixture_documents();
        assert_eq!(first.files, second.files);
        assert_eq!(first.files.len(), EXACT_BUNDLE_FILE_COUNT);

        let plan_bytes = document_bytes(&first, PLAN_PATH).unwrap().to_vec();
        let plan_value = parse_canonical(&plan_bytes, "plan").unwrap();
        let plan = parse_plan_document(&plan_value, plan_bytes).unwrap();
        let input_bytes = document_bytes(&first, BENCHMARK_INPUT_PATH).unwrap();
        let input = parse_canonical(input_bytes, "benchmark input").unwrap();
        validate_benchmark_input(&input, input_bytes, &plan).unwrap();
        let roster_bytes = document_bytes(&first, ROSTER_PATH).unwrap();
        let roster = parse_canonical(roster_bytes, "roster").unwrap();
        validate_roster_document(&roster, roster_bytes, &plan).unwrap();

        let invocation_bytes = invocation_map_bytes(
            Path::new("inputs.bundle"),
            Path::new("prepacked"),
            Path::new("kernel-artifacts"),
            7,
            &plan,
        )
        .unwrap();
        let invocation = parse_canonical(&invocation_bytes, "invocation map").unwrap();
        assert_eq!(invocation["format"], INVOCATION_FORMAT);
        assert_eq!(invocation["invocations"].as_array().unwrap().len(), 7);
        assert_eq!(
            invocation["invocations"][0]["arguments"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            invocation["invocations"][0]["arguments"][0],
            "inputs.bundle/plan.json"
        );
        assert_eq!(
            invocation["invocations"][0]["arguments"][3],
            "inputs.bundle/decode-s1-c8192.001.workload.json"
        );
        assert_eq!(invocation["invocations"][0]["arguments"][4], "prepacked");
        assert_eq!(invocation["plan_sha256"], plan.sha256());

        for case in &plan.cases {
            let workload_bytes = document_bytes(&first, &workload_path(&case.kind))
                .unwrap()
                .to_vec();
            let workload_value = parse_canonical(&workload_bytes, "workload").unwrap();
            assert_eq!(
                workload_value["completion_wait_policy"],
                json!({
                    "id": COMPLETION_WAIT_POLICY_ID,
                    "max_consecutive_scans_without_progress": MAX_CONSECUTIVE_SCANS_WITHOUT_PROGRESS,
                    "minimum_pending_scan_pause_micros": MINIMUM_PENDING_SCAN_PAUSE_MICROS,
                    "timeout_basis": COMPLETION_WAIT_TIMEOUT_BASIS,
                    "total_scan_bound_rule": TOTAL_SCAN_BOUND_RULE,
                })
            );
            assert!(workload_value.get("max_polls").is_none());
            let workload = parse_workload_document(&workload_value, workload_bytes, case).unwrap();
            let token_bytes = document_bytes(&first, &token_path(&case.kind)).unwrap();
            parse_input_tokens(token_bytes, &workload, case).unwrap();

            let mut trailing = token_bytes.to_vec();
            trailing.extend_from_slice(&0_u32.to_le_bytes());
            assert!(parse_input_tokens(&trailing, &workload, case).is_err());
        }
    }

    #[test]
    fn acceptance_policy_parser_matches_benchmark_contract() {
        let bytes = policy_bytes();
        let value = parse_canonical(&bytes, "policy").unwrap();
        validate_acceptance_policy(&value).unwrap();
        let mut trailing = value.clone();
        trailing["unexpected"] = Value::Bool(true);
        assert!(validate_acceptance_policy(&trailing).is_err());
        let mut missing = value.clone();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(validate_acceptance_policy(&missing).is_err());
    }

    #[test]
    fn exact_bundle_comparison_rejects_missing_trailing_substituted_and_symlinked_files() {
        let cases = build_case_documents().unwrap();
        let mut files = cases.files;
        for (path, bytes) in [
            (BENCHMARK_INPUT_PATH, b"input\n".as_slice()),
            (PLAN_PATH, b"plan\n".as_slice()),
            (ROSTER_PATH, b"roster\n".as_slice()),
            (CLOSURE_PATH, b"closure\n".as_slice()),
            (ENVIRONMENT_PATH, b"environment\n".as_slice()),
            (ACCEPTANCE_POLICY_PATH, b"policy\n".as_slice()),
        ] {
            files.insert(path.to_owned(), bytes.to_vec());
        }
        let documents = InputDocuments { files };
        assert_eq!(documents.files.len(), EXACT_BUNDLE_FILE_COUNT);

        let temporary = TestDirectory::new();
        let exact = temporary.0.join("exact.bundle");
        publish_documents(&documents, &exact).unwrap();
        compare_published_documents(&exact, &documents).unwrap();
        assert!(publish_documents(&documents, &exact).is_err());

        let missing = temporary.0.join("missing.bundle");
        publish_documents(&documents, &missing).unwrap();
        fs::remove_file(missing.join(PLAN_PATH)).unwrap();
        assert!(compare_published_documents(&missing, &documents).is_err());

        let trailing = temporary.0.join("trailing.bundle");
        publish_documents(&documents, &trailing).unwrap();
        fs::write(trailing.join("unexpected"), b"trailing").unwrap();
        assert!(compare_published_documents(&trailing, &documents).is_err());

        let substituted = temporary.0.join("substituted.bundle");
        publish_documents(&documents, &substituted).unwrap();
        fs::write(substituted.join(PLAN_PATH), b"same\n").unwrap();
        assert!(compare_published_documents(&substituted, &documents).is_err());

        let symlinked = temporary.0.join("symlinked.bundle");
        publish_documents(&documents, &symlinked).unwrap();
        fs::remove_file(symlinked.join(PLAN_PATH)).unwrap();
        symlink(exact.join(PLAN_PATH), symlinked.join(PLAN_PATH)).unwrap();
        assert!(compare_published_documents(&symlinked, &documents).is_err());
    }

    #[test]
    fn measured_reference_requires_nonempty_regular_single_link_snapshot() {
        let temporary = TestDirectory::new();
        let reference = temporary.0.join("reference.bin");
        fs::write(&reference, b"reference implementation\n").unwrap();
        assert_eq!(
            measure_regular_file(&reference, "reference").unwrap(),
            sha256_hex(b"reference implementation\n")
        );

        let empty = temporary.0.join("empty.bin");
        fs::write(&empty, b"").unwrap();
        assert!(measure_regular_file(&empty, "empty reference").is_err());

        let linked = temporary.0.join("linked.bin");
        fs::hard_link(&reference, &linked).unwrap();
        assert!(measure_regular_file(&reference, "linked reference").is_err());
        fs::remove_file(&linked).unwrap();

        let alias = temporary.0.join("alias.bin");
        symlink(&reference, &alias).unwrap();
        assert!(measure_regular_file(&alias, "symlinked reference").is_err());
    }

    #[test]
    fn exact_file_roster_is_fixed_at_twenty_flat_members() {
        let cases = build_case_documents().unwrap();
        assert_eq!(cases.files.len(), 14);
        let expected = expected_file_roster();
        assert_eq!(
            expected,
            BTreeSet::from([
                "acceptance-policy.json".to_owned(),
                "benchmark-input.json".to_owned(),
                "closure.json".to_owned(),
                "decode-s1-c8192.001.tokens.u32le".to_owned(),
                "decode-s1-c8192.001.workload.json".to_owned(),
                "decode-s32-c8192.001.tokens.u32le".to_owned(),
                "decode-s32-c8192.001.workload.json".to_owned(),
                "decode-s8-c8192.001.tokens.u32le".to_owned(),
                "decode-s8-c8192.001.workload.json".to_owned(),
                "environment.json".to_owned(),
                "plan.json".to_owned(),
                "prefill-s1-t128.001.tokens.u32le".to_owned(),
                "prefill-s1-t128.001.workload.json".to_owned(),
                "prefill-s1-t2048.001.tokens.u32le".to_owned(),
                "prefill-s1-t2048.001.workload.json".to_owned(),
                "prefill-s1-t512.001.tokens.u32le".to_owned(),
                "prefill-s1-t512.001.workload.json".to_owned(),
                "prefill-s8-t128.001.tokens.u32le".to_owned(),
                "prefill-s8-t128.001.workload.json".to_owned(),
                "roster.json".to_owned(),
            ])
        );
        assert_eq!(expected.len(), EXACT_BUNDLE_FILE_COUNT);
        assert!(expected.iter().all(|path| {
            Path::new(path)
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        }));
    }
}
