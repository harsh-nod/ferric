#![forbid(unsafe_code)]

//! Authority-free paired-prefill to S1/K4 hardware diagnostic.

mod smoke_bootstrap;
mod speculative_resident_smoke;
mod startup_diagnostics;

use fe2o3_kfd::{DeviceSelector, GFX942_MAX_FIXED_DISPATCH_DATA_V1, OpenedKfd};
use ferric_build::{
    AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation,
    M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome, SpecialTokenDecodePolicy,
    TokenizerExecutionLimits, m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
};
use ferric_engine::{
    ActiveDeviceKvCache, DeviceKvCacheProjection, DeviceKvPageLease, Engine,
    M1CompletedDeviceKvMemberV1, M1CompletedStepOutcomeV1, M1DeviceKvCompletionDispositionV1,
    M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1,
    M1FiniteSpeculativeQueueRolloverKvInputsV1, M1FullStepKvWorkspaceTablesV1,
    M1FullStepWorkspacePlans, M1PartitionedModelMemoryKvPoolV1, M1PhysicalRunnerRecipeOutcomeV1,
    M1PhysicalRunnerV1, M1ReleasedDeviceKvMemberV1, M1ScheduledDispatchV1,
    M1ServingCompletionDispositionV1, M1ServingPhysicalOperationResultV1,
    M1ServingPhysicalOperationsV1, M1ServingPhysicalQueueCustodyV1, M1ServingPlanV1,
    M1ServingRegistryV1, M1ServingRolloverReasonV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativeGenerationPolicyV1, M1SpeculativeMemberControlV1, M1SpeculativeMemberSeedV1,
    M1SpeculativeVerificationChoiceV1, M1StepDispatchIntent, bind_m1_kv_workspace_table_v1,
    complete_m1_physical_step_v1, prepare_m1_finite_speculative_queue_rollover_v1,
    release_m1_completed_step_kv_pages_v1, reserve_m1_finite_speculative_queue_rollover_kv_v1,
    schedule_m1_finite_speculative_queue_rollover_v1,
};
use ferric_m1_engineering_execution_v1::{
    bind_engineering_structural_m1_physical_runner_v1, reopen_m1_engineering_aggregate_artifact_v1,
};
use ferric_spec::{
    Identity, M1_KV_PAGE_TOKENS, M1StepInputCandidate, M1StepInputValidationOutcome,
    QWEN3_END_OF_TEXT_TOKEN, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, ValidatedM1StepInputs, completion::CompletionEpoch,
    validate_m1_step_inputs,
};
use rustix::time::{ClockId, clock_gettime};
use serde_json::{Value, json};
use startup_diagnostics::{EngineeringStartupDiagnosticsV1, EngineeringStartupPhaseV1};
use std::ffi::OsString;
use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

type SmokeResult<T> = Result<T, String>;

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
const TARGET_SPECULATIVE: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Speculative,
    bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
};
const DRAFT_DECODE: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Decode,
    bucket: Qwen3PlanBucket::DecodeS1C8192,
};
const EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS: usize = 11;
const STATUS: &str = "engineering-physical-speculative-smoke-non-evidence";
const NONCLAIM: &str = "One authority-free structural run of a real paired Qwen prefill and one physical S1/K4 speculative graph. The exact engineering aggregate has authority none; authenticated A/B custody was not exercised. Independent diagnostic buffers were used to join choices to compact completion, but no compiler-origin, Worker V3, current-publication, numerical-correctness, qualification, production-serving, or benchmark-comparability claim is made.";

#[derive(Clone, Copy)]
struct EngineeringObservationFacts {
    manifest: Identity,
    hsaco: Identity,
    compiler_handoff: Identity,
    canonical_descriptor: Identity,
    program_catalog: Identity,
}

enum EngineeringIdentityInputV1 {
    #[allow(dead_code)]
    ExternalFile(PathBuf),
    DerivedEngineeringV1,
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[OsString]) -> SmokeResult<()> {
    let diagnostics = EngineeringStartupDiagnosticsV1::from_process_environment();
    let (base, resident_limit) = match arguments {
        [prepacked_root, observation_root, gpu_unique_id, prompt] =>
            ([prepacked_root, observation_root, gpu_unique_id, prompt], None),
        [prepacked_root, observation_root, gpu_unique_id, prompt, limit] =>
            ([prepacked_root, observation_root, gpu_unique_id, prompt], Some(speculative_resident_smoke::parse_limit(limit)?)),
        _ => return Err("usage: ferric-m1-engineering-speculative-smoke PREPACKED-SNAPSHOT ENGINEERING-OBSERVATION-DIRECTORY GPU-UNIQUE-ID RAW-PROMPT [RESIDENT-MAX-NEW-TOKENS:1..32]".to_owned()),
    };
    let [prepacked_root, observation_root, gpu_unique_id, prompt] = base;
    let deadline = std::time::Instant::now() + std::time::Duration::from_mins(15);
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let prompt = prompt
        .to_str()
        .ok_or_else(|| "RAW-PROMPT must be UTF-8".to_owned())?;

    let artifact = reopen_m1_engineering_aggregate_artifact_v1(Path::new(observation_root))
        .map_err(|error| format!("cannot admit engineering aggregate: {error}"))?;
    diagnostics.completed(EngineeringStartupPhaseV1::ArtifactAdmission);
    let facts = EngineeringObservationFacts {
        manifest: artifact.manifest_id(),
        hsaco: artifact.hsaco_id(),
        compiler_handoff: artifact.compiler_handoff_id(),
        canonical_descriptor: artifact.canonical_descriptor_id(),
        program_catalog: artifact.program_catalog_id(),
    };
    let bootstrap = smoke_bootstrap::prepare(
        Path::new(prepacked_root),
        &EngineeringIdentityInputV1::DerivedEngineeringV1,
        prompt,
        facts,
    )?;
    diagnostics.completed(EngineeringStartupPhaseV1::CpuModelBootstrapPreparation);
    let bound = bootstrap.bind(|publication| {
        bind_engineering_structural_m1_physical_runner_v1(artifact, publication)
            .map_err(|error| format!("cannot bind engineering physical runner: {error:?}"))
    })?;
    diagnostics.completed(EngineeringStartupPhaseV1::RunnerBind);
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    diagnostics.completed(EngineeringStartupPhaseV1::KfdBind);
    let initialized = bound.initialize_memory(checked)?;
    diagnostics.completed(EngineeringStartupPhaseV1::InitializeMemoryAllocationUpload);
    match resident_limit {
        Some(limit) => speculative_resident_smoke::execute_and_report(
            initialized,
            facts,
            &diagnostics,
            limit,
            deadline,
        ),
        None => execute_and_report(initialized, facts, &diagnostics),
    }
}

fn execute_and_report(
    initialized: smoke_bootstrap::InitializedSmokeBootstrapV1,
    facts: EngineeringObservationFacts,
    diagnostics: &EngineeringStartupDiagnosticsV1,
) -> SmokeResult<()> {
    let smoke_bootstrap::InitializedSmokeBootstrapV1 {
        runner,
        memory,
        tokenizer,
        prompt_tokens,
    } = initialized;
    if prompt_tokens.is_empty() || prompt_tokens.len() > 128 {
        return Err(format!(
            "raw prompt must encode to 1..=128 tokens, got {}",
            prompt_tokens.len()
        ));
    }
    let raw_prompt_token_count = prompt_tokens.len();
    let raw_prompt_tokens = prompt_tokens.clone();
    let mut padded_prompt = prompt_tokens;
    padded_prompt.resize(128, QWEN3_END_OF_TEXT_TOKEN);
    let physical_prompt_tokens = padded_prompt.clone();

    let mut engine = require_or_abort(Engine::<1>::new(512, 256, 8_192), "construct engine");
    let request = require_or_abort(engine.admit(), "admit request");
    require_or_abort(
        engine.append_tentative(request, 1),
        "make paired-prefill request ready",
    );
    let Some(scheduled) = require_or_abort(engine.dispatch_m1_ready(), "dispatch paired prefill")
    else {
        fail_stop("dispatch paired prefill", &"no ready request")
    };
    let epoch = scheduled.epoch();
    let target_inputs = step_inputs(
        &runner,
        request,
        epoch,
        TARGET_PREFILL,
        padded_prompt.clone(),
        (0..128).collect(),
        128,
        0,
    );
    let draft_inputs = step_inputs(
        &runner,
        request,
        epoch,
        DRAFT_PREFILL,
        padded_prompt,
        (0..128).collect(),
        128,
        0,
    );

    let mut memory = memory;
    let mut cache = require_or_abort(
        ActiveDeviceKvCache::new(memory.device(), request, TARGET_PREFILL, DRAFT_PREFILL),
        "construct paired-prefill KV cache",
    );
    let initial_projection = cache.projection();
    let page_count = 128_u32.div_ceil(M1_KV_PAGE_TOKENS);
    let target_pages = lease_pages(
        &mut memory,
        request,
        Qwen3ModelRole::Target8B,
        0..page_count,
    );
    let draft_pages = lease_pages(
        &mut memory,
        request,
        Qwen3ModelRole::Draft06B,
        0..page_count,
    );
    let target_pending = require_or_abort(
        cache.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            128,
            epoch,
            target_pages,
        ),
        "reserve target paired-prefill KV write",
    );
    let draft_pending = require_or_abort(
        cache.reserve_step_write(
            request,
            Qwen3ModelRole::Draft06B,
            0,
            128,
            epoch,
            draft_pages,
        ),
        "reserve draft paired-prefill KV write",
    );
    let target_table = require_or_abort(
        bind_m1_kv_workspace_table_v1(target_inputs, vec![target_pending]),
        "bind target paired-prefill KV table",
    );
    let draft_table = require_or_abort(
        bind_m1_kv_workspace_table_v1(draft_inputs, vec![draft_pending]),
        "bind draft paired-prefill KV table",
    );
    let target_rollover_page = require_or_abort(
        memory.lease_page(request, Qwen3ModelRole::Target8B, page_count),
        "lease target rollover page",
    );
    let draft_rollover_page = require_or_abort(
        memory.lease_page(request, Qwen3ModelRole::Draft06B, page_count),
        "lease draft rollover page",
    );

    let recipe = match runner.derive_step_recipe(
        M1StepDispatchIntent::PairedPrefill(TARGET_PREFILL),
        prefill_workspace_plans(),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            fail_stop("derive paired-prefill physical recipe", &error)
        }
    };
    let tables = M1FullStepKvWorkspaceTablesV1::PairedPrefill {
        draft: draft_table,
        target: target_table,
    };
    let prepared = require_or_abort(
        runner.prepare_scheduled_workspaces(scheduled, prefill_workspace_plans(), tables),
        "prepare paired-prefill workspaces",
    );
    let mut allocated = require_or_abort(
        runner.allocate_scheduled_workspaces(memory, prepared),
        "allocate paired-prefill workspaces",
    );
    require_or_abort(
        allocated.reserve_s1_k4_rollover_output(),
        "reserve S1/K4 rollover output",
    );
    let completion = require_or_abort(
        allocated.allocate_completion_output(TARGET_PREFILL),
        "allocate paired-prefill completion output",
    );
    let completion = require_or_abort(
        allocated.enable_direct_diagnostic_choices_capture(completion),
        "attach independent paired-prefill choice capture",
    );
    let prefill_queue_data_allocations = allocated.partitioned_memory().retained_allocation_count();
    if prefill_queue_data_allocations != EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS
        || prefill_queue_data_allocations > GFX942_MAX_FIXED_DISPATCH_DATA_V1
    {
        fail_stop(
            "preflight paired-prefill fixed-dispatch data allocation roster",
            &prefill_queue_data_allocations,
        );
    }
    let _ = writeln!(
        std::io::stderr().lock(),
        "PREFLIGHT: paired_prefill_queue_data_allocations={prefill_queue_data_allocations} maximum={GFX942_MAX_FIXED_DISPATCH_DATA_V1}"
    );
    let prefill_started_ns = require_or_abort(monotonic_raw_ns(), "read prefill start clock");
    let published = require_or_abort(
        runner.publish_first_step(&mut engine, 1 << 20, allocated, recipe, completion),
        "publish paired-prefill queue",
    );
    let completed = require_or_abort(published.wait_for(120_000), "wait for paired prefill");
    let recycled = require_or_abort(completed.recycle(), "recycle paired-prefill signals");
    let observed = require_or_abort(
        recycled.observe_completion(),
        "copy paired-prefill compact completion",
    );
    let diagnostic = require_or_abort(
        observed.observe_direct_diagnostic_choices(),
        "copy independent paired-prefill target choice",
    );
    let first_token = match diagnostic.choices().choices() {
        [token] => *token,
        choices => fail_stop("validate paired-prefill choice roster", &choices),
    };
    let prefill_choice_sha256 = match diagnostic.choices().raw_sha256() {
        [digest] => *digest,
        digests => fail_stop("validate paired-prefill choice digest roster", &digests),
    };
    let prefill_dispatch_generation = diagnostic.choices().dispatch_generation();
    let joined = require_or_abort(
        diagnostic.check_completion(),
        "join compact prefill completion to independent target choice",
    );
    let (readback, _direct_choices) = joined.into_parts();
    let completion = complete_m1_physical_step_v1(
        &mut engine,
        readback,
        M1DeviceKvCompletionRosterV1::new(vec![M1DeviceKvCompletionMemberV1::continuing(cache)]),
    );
    let completed = match completion {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        other => fail_stop("complete paired-prefill Engine and KV state", &other),
    };
    let released = require_or_abort(
        release_m1_completed_step_kv_pages_v1(completed),
        "release paired-prefill retired pages",
    );
    let prefill_projection = match released.members() {
        [M1ReleasedDeviceKvMemberV1::Active(cache)] => cache.projection(),
        members => fail_stop("validate released paired-prefill roster", &members),
    };
    let prefill_duration_ns = require_or_abort(
        elapsed_ns(prefill_started_ns),
        "measure paired-prefill duration",
    );
    let prior = require_or_abort(
        M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL),
        "construct paired-prefill serving plan",
    );
    let next = require_or_abort(
        M1ServingPlanV1::new(TARGET_SPECULATIVE, DRAFT_DECODE),
        "construct S1/K4 serving plan",
    );
    let rollover_batch = build_rollover_fixture_batch(prior, next, request);
    let rollover_epoch = rollover_batch.epoch();
    require_or_abort(
        engine.append_tentative(request, 5),
        "reserve logical S1/K4 completion span",
    );
    let target_inputs = step_inputs(
        &runner,
        request,
        rollover_epoch,
        TARGET_SPECULATIVE,
        vec![first_token, 0, 0, 0, 0],
        (128..133).collect(),
        5,
        128,
    );
    let draft_inputs = step_inputs(
        &runner,
        request,
        rollover_epoch,
        DRAFT_DECODE,
        vec![first_token],
        vec![128],
        1,
        128,
    );
    let rollover_inputs = M1FiniteSpeculativeQueueRolloverKvInputsV1::new(
        draft_inputs,
        target_inputs,
        vec![draft_rollover_page],
        vec![target_rollover_page],
    );
    let rollover_recipe = match runner.derive_step_recipe(
        M1StepDispatchIntent::SpeculativeRound(TARGET_SPECULATIVE),
        speculative_workspace_plans(),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            fail_stop("derive S1/K4 physical recipe", &error)
        }
    };
    let speculative_started_ns = require_or_abort(monotonic_raw_ns(), "read S1/K4 start clock");
    let scheduled = require_or_abort(
        schedule_m1_finite_speculative_queue_rollover_v1(&mut engine, released, &rollover_batch),
        "schedule finite S1/K4 queue rollover",
    );
    let pre_speculative_projection = scheduled.selected_cache();
    let reserved = require_or_abort(
        reserve_m1_finite_speculative_queue_rollover_kv_v1(&mut engine, scheduled, rollover_inputs),
        "reserve finite S1/K4 KV writes",
    );
    let prepared = require_or_abort(
        prepare_m1_finite_speculative_queue_rollover_v1(
            &mut engine,
            reserved,
            runner.logical_runner(),
            speculative_workspace_plans(),
        ),
        "prepare finite S1/K4 workspaces",
    );
    let published = require_or_abort(
        runner.submit_finite_speculative_rollover(&mut engine, 1 << 20, prepared, rollover_recipe),
        "publish finite S1/K4 queue rollover",
    );
    let completed = require_or_abort(
        published.wait_for(120_000, &mut engine),
        "wait for finite S1/K4 completion",
    );
    let recycled = require_or_abort(
        completed.recycle(&mut engine),
        "recycle finite S1/K4 signals",
    );
    let diagnostic = require_or_abort(
        recycled.read_and_check_speculative_k4_diagnostic_completion(),
        "copy and join independent finite S1/K4 choices",
    );
    let draft_choices = *diagnostic.choices().draft_choices();
    let target_choices = *diagnostic.choices().target_choices();
    let draft_choices_sha256 = *diagnostic.choices().draft_sha256();
    let target_choices_sha256 = *diagnostic.choices().target_sha256();
    let speculative_dispatch_generation = diagnostic.choices().dispatch_generation();
    let checked = diagnostic.checked();
    let record = match checked.records() {
        [record] => record,
        records => fail_stop("validate finite S1/K4 compact roster", &records),
    };
    let wire = record.record();
    let accepted_draft_tokens = wire.accepted_draft_tokens;
    let emitted_token_count = usize::from(wire.emitted_token_count);
    let emitted_tokens = match wire.emitted_tokens.get(..emitted_token_count) {
        Some(tokens) => tokens.to_vec(),
        None => fail_stop("validate finite S1/K4 emitted token count", &wire),
    };
    let correction_or_bonus = match emitted_tokens.get(usize::from(accepted_draft_tokens)) {
        Some(token) => *token,
        None => fail_stop("validate finite S1/K4 correction or bonus", &wire),
    };
    let policy = require_or_abort(
        M1SpeculativeGenerationPolicyV1::new(1, &[]),
        "construct one-round speculative policy",
    );
    let seed = M1SpeculativeMemberSeedV1::new(request, first_token, 128, 128, policy);
    let mut coordinator = require_or_abort(
        M1SpeculativeGenerationLoopV1::new(TARGET_SPECULATIVE, &[seed]),
        "construct one-round speculative coordinator",
    );
    let retirement_binding = require_or_abort(
        coordinator.bind_round(0, rollover_epoch, &[request]),
        "bind speculative retirement preflight",
    );
    let controls = [M1SpeculativeMemberControlV1::continuing(request)];
    let retirement_preflight = require_or_abort(
        coordinator.preflight_checked_round(retirement_binding, checked, &controls),
        "preflight checked S1/K4 logical settlement",
    );
    if !matches!(
        retirement_preflight.members(),
        [member] if member.physical_disposition() == M1DeviceKvCompletionDispositionV1::Retire
    ) {
        fail_stop(
            "validate one-round output-limit retirement preflight",
            &retirement_preflight,
        );
    }
    drop(retirement_preflight);
    let speculative_compact_sha256 = *checked.raw_sha256();
    let (readback, _speculative_choices) = diagnostic.into_parts();
    require_or_abort(engine.retire(request), "retire one-round S1/K4 request");
    let physical = require_or_abort(
        readback.complete(&mut engine, vec![M1DeviceKvCompletionDispositionV1::Retire]),
        "complete finite S1/K4 Engine and KV state",
    );
    let physical_completed = match physical.outcome() {
        M1CompletedStepOutcomeV1::Completed(completed) => completed,
        other => fail_stop("validate finite S1/K4 physical completion", &other),
    };
    let post_speculative_projection = match physical_completed.members() {
        [M1CompletedDeviceKvMemberV1::Active(cache)] => cache.projection(),
        [M1CompletedDeviceKvMemberV1::Quiescent(cache)] => cache.projection(),
        members => fail_stop("validate finite S1/K4 completed roster", &members),
    };
    let binding = require_or_abort(
        coordinator.bind_round(0, rollover_epoch, &[request]),
        "bind one-round speculative coordinator",
    );
    let logical = require_or_abort(
        coordinator.complete_checked_round(binding, physical_completed.checked(), &controls),
        "commit checked S1/K4 logical settlement after physical completion",
    );
    let logical_member = match logical.members() {
        [member] => *member,
        members => fail_stop("validate S1/K4 logical outcome roster", &members),
    };
    if logical_member.physical_disposition() != M1DeviceKvCompletionDispositionV1::Retire {
        fail_stop(
            "validate one-round output-limit retirement",
            &logical_member.physical_disposition(),
        );
    }
    let released = match physical.release_completed() {
        ferric_engine::M1RearmedRoundReleaseOutcomeV1::Released(released) => released,
        other => fail_stop("release finite S1/K4 retired pages", &other),
    };
    let _shutdown = require_or_abort(
        released.shutdown_all_terminal_queue(&mut engine),
        "destroy all-terminal finite S1/K4 queue",
    );
    let speculative_duration_ns = require_or_abort(
        elapsed_ns(speculative_started_ns),
        "measure finite S1/K4 duration",
    );
    diagnostics.completed(EngineeringStartupPhaseV1::ControllerExecution);
    let Some(physical_total_ns) = prefill_duration_ns.checked_add(speculative_duration_ns) else {
        fail_stop(
            "sum paired-prefill and speculative durations",
            &(prefill_duration_ns, speculative_duration_ns),
        )
    };

    let published_tokens = logical_member.published().tokens().to_vec();
    let text_bytes = tokenizer
        .decode_to_bytes(
            &published_tokens,
            TokenizerExecutionLimits::m1(),
            SpecialTokenDecodePolicy::Skip,
        )
        .map_err(|error| format!("cannot decode speculative tokens: {error}"))?;
    let text = String::from_utf8_lossy(&text_bytes).into_owned();
    let plan_id = |selection| {
        require_or_abort(
            runner.logical_runner().plan(selection),
            "resolve exact generated plan identity",
        )
        .plan_id
    };
    let report = json!({
        "schema": "ferric.m1-engineering-speculative-smoke-observation.v1",
        "status": STATUS,
        "nonclaim": NONCLAIM,
        "authority": "none",
        "artifact_authority": "none",
        "benchmark_comparable": false,
        "authenticated_AB_exercised": false,
        "compiler_origin_authenticated": false,
        "current_publication_selected": false,
        "worker_v3_authenticated": false,
        "hardware_completion_observed": true,
        "target": "gfx942:xnack-",
        "gpu_unique_id": post_speculative_projection.device.gpu_unique_id(),
        "identities": {
            "observation_manifest_sha256": identity_hex(facts.manifest),
            "hsaco_sha256": identity_hex(facts.hsaco),
            "compiler_handoff_sha256": identity_hex(facts.compiler_handoff),
            "canonical_descriptor_sha256": identity_hex(facts.canonical_descriptor),
            "observation_program_catalog_sha256": identity_hex(facts.program_catalog),
            "admitted_artifact_manifest_sha256": identity_hex(runner.kernel_artifact_manifest_id()),
            "admitted_program_catalog_sha256": identity_hex(runner.program_catalog_id()),
            "model_bundle_sha256": identity_hex(runner.logical_runner().bundle_id()),
            "target_prepacked_sha256": identity_hex(runner.logical_runner().target_prepacked_id()),
            "draft_prepacked_sha256": identity_hex(runner.logical_runner().draft_prepacked_id()),
            "plan_catalog_sha256": identity_hex(runner.logical_runner().plan_catalog_id()),
            "generated_runner_declaration_sha256": identity_hex(runner.declaration_id()),
            "generated_kernel_catalog_sha256": identity_hex(runner.kernel_catalog_id()),
        },
        "plans": {
            "target_prefill": {"selection": selection_name(TARGET_PREFILL), "sha256": identity_hex(plan_id(TARGET_PREFILL))},
            "draft_prefill": {"selection": selection_name(DRAFT_PREFILL), "sha256": identity_hex(plan_id(DRAFT_PREFILL))},
            "target_speculative": {"selection": selection_name(TARGET_SPECULATIVE), "sha256": identity_hex(plan_id(TARGET_SPECULATIVE))},
            "draft_decode": {"selection": selection_name(DRAFT_DECODE), "sha256": identity_hex(plan_id(DRAFT_DECODE))},
        },
        "prompt": {
            "raw_token_count": raw_prompt_token_count,
            "raw_token_ids": raw_prompt_tokens,
            "physical_active_token_count": 128,
            "physical_token_ids": physical_prompt_tokens,
            "suffix_fill_token_id": QWEN3_END_OF_TEXT_TOKEN,
            "suffix_fill_count": 128 - raw_prompt_token_count,
            "suffix_fill_semantics": "active-token-fill-not-attention-mask-padding",
        },
        "paired_prefill": {
            "epoch": epoch.value(),
            "first_token_id": first_token,
            "dispatch_generation": prefill_dispatch_generation,
            "independent_choice_sha256": bytes_hex(&prefill_choice_sha256),
            "queue_data_allocation_count": prefill_queue_data_allocations,
            "queue_data_allocation_maximum": GFX942_MAX_FIXED_DISPATCH_DATA_V1,
            "queue_data_allocation_portfolio": "4-model-memory+2-paired-workspaces+3-k4-successor-output+1-prefill-compact+1-prefill-direct-choice",
            "kv_before": projection_json(&initial_projection),
            "kv_after": projection_json(&prefill_projection),
        },
        "speculative_k4": {
            "epoch": rollover_epoch.value(),
            "dispatch_generation": speculative_dispatch_generation,
            "draft_choices": draft_choices,
            "target_choices": target_choices,
            "draft_choices_sha256": bytes_hex(&draft_choices_sha256),
            "target_choices_sha256": bytes_hex(&target_choices_sha256),
            "compact_completion_sha256": bytes_hex(&speculative_compact_sha256),
            "accepted_draft_tokens": accepted_draft_tokens,
            "emitted_token_ids": emitted_tokens,
            "correction_or_bonus_token_id": correction_or_bonus,
            "k4_round_published_token_ids": published_tokens,
            "published_token_scope": "speculative-k4-round-excludes-prefill-anchor",
            "verification_kind": verification_name(logical_member.verification_choice()),
            "target_commit_end": logical_member.target_settlement().commit_end(),
            "target_rollback_tokens": logical_member.target_settlement().rollback_tokens(),
            "draft_commit_end": logical_member.draft_settlement().commit_end(),
            "draft_rollback_tokens": logical_member.draft_settlement().rollback_tokens(),
            "kv_before": projection_json(&pre_speculative_projection),
            "kv_after": projection_json(&post_speculative_projection),
        },
        "decoded_published_text": text,
        "decoded_published_bytes_hex": bytes_hex(&text_bytes),
        "timing": {
            "clock": "monotonic-raw-nanoseconds",
            "scope": "single-process-engineering-hardware-smoke-nonbenchmark",
            "paired_prefill_publish_through_release_ns": prefill_duration_ns,
            "speculative_rollover_schedule_through_shutdown_ns": speculative_duration_ns,
            "physical_total_ns": physical_total_ns,
        },
        "registry_rollover_descriptor": "structural-host-fixture-only-no-token-or-completion-oracle",
    });
    validate_report(&report)?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize speculative smoke report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot terminate speculative smoke report: {error}"))?;
    Ok(())
}

fn fail_stop(stage: &str, diagnostic: &dyn Debug) -> ! {
    let _ = writeln!(
        std::io::stderr().lock(),
        "FAIL-STOP: {stage}: {diagnostic:?}"
    );
    std::process::abort()
}

fn require_or_abort<T, E: Debug>(result: Result<T, E>, stage: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => fail_stop(stage, &error),
    }
}

#[allow(clippy::too_many_arguments)]
fn step_inputs(
    runner: &M1PhysicalRunnerV1,
    request: RequestId,
    epoch: CompletionEpoch,
    selection: Qwen3PlanSelection,
    tokens: Vec<u32>,
    positions: Vec<u32>,
    active_length: u32,
    context_length: u32,
) -> ValidatedM1StepInputs {
    let plan = require_or_abort(
        runner
            .logical_runner()
            .bind_step_plan(request, epoch, selection),
        "bind logical step plan",
    );
    let candidate = M1StepInputCandidate::new(
        selection,
        vec![Some(plan)],
        tokens,
        positions,
        vec![active_length],
        vec![context_length],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => inputs,
        M1StepInputValidationOutcome::Rejected(failure) => {
            fail_stop("validate physical step inputs", &failure)
        }
    }
}

fn lease_pages(
    memory: &mut M1PartitionedModelMemoryKvPoolV1,
    request: RequestId,
    role: Qwen3ModelRole,
    indices: std::ops::Range<u32>,
) -> Vec<DeviceKvPageLease> {
    indices
        .map(|index| {
            require_or_abort(
                memory.lease_page(request, role, index),
                "lease paired-prefill KV page",
            )
        })
        .collect()
}

fn prefill_workspace_plans() -> M1FullStepWorkspacePlans {
    M1FullStepWorkspacePlans::paired_prefill(
        workspace_plan(DRAFT_PREFILL, 0x31),
        workspace_plan(TARGET_PREFILL, 0x32),
    )
}

fn speculative_workspace_plans() -> M1FullStepWorkspacePlans {
    M1FullStepWorkspacePlans::speculative_round(
        workspace_plan(DRAFT_DECODE, 0x41),
        workspace_plan(TARGET_SPECULATIVE, 0x42),
    )
}

struct FixtureOperations {
    engine: Engine<1>,
}

impl FixtureOperations {
    fn new(request: RequestId) -> Self {
        let mut engine = require_or_abort(Engine::<1>::new(512, 256, 8_192), "fixture engine");
        let admitted = require_or_abort(engine.admit(), "fixture request admission");
        if admitted != request {
            fail_stop("fixture request identity", &(request, admitted));
        }
        require_or_abort(
            engine.append_tentative(admitted, 1),
            "fixture request readiness",
        );
        Self { engine }
    }
}

impl M1ServingPhysicalOperationsV1 for FixtureOperations {
    type Quiescent = ();
    type Published = M1ScheduledDispatchV1;
    type Readback = ();
    type Error = &'static str;
    type TerminalCustody = ();

    fn scheduled_dispatch<'a>(&self, custody: &'a Self::Published) -> &'a M1ScheduledDispatchV1 {
        custody
    }

    fn fresh_launch(
        &mut self,
        batch: &ferric_engine::M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), (), Self::Error> {
        self.engine
            .dispatch_m1_exact_ready(batch.epoch(), batch.requests())
            .map_err(
                |_| ferric_engine::M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "fixture dispatch rejected",
                    custody: (),
                },
            )
    }

    fn same_shape_rearm(
        &mut self,
        custody: (),
        _batch: &ferric_engine::M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), (), Self::Error> {
        Err(
            ferric_engine::M1ServingPhysicalOperationFailureV1::Retryable {
                source: "fixture same-shape path is outside scope",
                custody,
            },
        )
    }

    fn quiescent_rollover(
        &mut self,
        custody: (),
        _prior: M1ServingPlanV1,
        _next: M1ServingPlanV1,
        _reason: M1ServingRolloverReasonV1,
        _batch: &ferric_engine::M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), (), Self::Error> {
        Err(
            ferric_engine::M1ServingPhysicalOperationFailureV1::Retryable {
                source: "fixture rollover path is outside scope",
                custody,
            },
        )
    }

    fn quiescent_new_window(
        &mut self,
        custody: (),
        _prior: M1ServingPlanV1,
        _next: M1ServingPlanV1,
        _batch: &ferric_engine::M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), (), Self::Error> {
        Err(
            ferric_engine::M1ServingPhysicalOperationFailureV1::Retryable {
                source: "fixture new-window path is outside scope",
                custody,
            },
        )
    }

    fn read_published(
        &mut self,
        _custody: Self::Published,
        _epoch: CompletionEpoch,
        _batch: &ferric_engine::M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<(), Self::Published, (), Self::Error> {
        Ok(())
    }

    fn checked_completion<'a>(
        &self,
        _custody: &'a (),
    ) -> &'a ferric_engine::M1CheckedCompletionOutputV1 {
        fail_stop("fixture checked completion", &"outside fixture scope")
    }

    fn settle_readback(
        &mut self,
        _custody: (),
        _dispositions: Vec<ferric_engine::M1DeviceKvCompletionDispositionV1>,
    ) -> M1ServingPhysicalOperationResultV1<(), (), (), Self::Error> {
        Ok(())
    }
}

fn build_rollover_fixture_batch(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    request: RequestId,
) -> ferric_engine::M1ServingBatchPlanV1 {
    let mut registry = require_or_abort(M1ServingRegistryV1::<1>::new(), "fixture registry");
    require_or_abort(
        registry.admit(request, prior),
        "fixture registry request admission",
    );
    let Some(prefill) = require_or_abort(registry.plan_next(), "fixture prefill planning") else {
        fail_stop("fixture prefill planning", &"no ready batch")
    };
    let epoch = prefill.epoch();
    let reservation = require_or_abort(
        registry.reserve_publication(prefill),
        "fixture publication reservation",
    );
    let mut operations = FixtureOperations::new(request);
    let published = require_or_abort(
        M1ServingPhysicalQueueCustodyV1::Vacant.publish(
            reservation,
            &mut registry,
            &mut operations,
        ),
        "fixture publication bridge",
    );
    let readback = require_or_abort(
        published.read_physical(epoch, &mut operations),
        "fixture readback bridge",
    );
    let dispositions = [M1ServingCompletionDispositionV1::Continue(next)];
    let _ = require_or_abort(
        readback.complete_exact(&mut registry, &dispositions, &mut operations),
        "fixture completion bridge",
    );
    let Some(rollover) = require_or_abort(registry.plan_next(), "fixture rollover planning") else {
        fail_stop("fixture rollover planning", &"no ready batch")
    };
    rollover
}

fn workspace_plan(
    selection: Qwen3PlanSelection,
    identity_byte: u8,
) -> AddresslessM1StepWorkspacePlan {
    let requirements = require_or_abort(
        m1_step_workspace_requirements(selection),
        "derive workspace requirements",
    );
    let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
        selection,
        DeclaredM1StepWorkspaceAllocation::new(
            Identity::new([identity_byte; 32]),
            requirements.allocation_byte_len(),
            requirements.allocation_alignment(),
        ),
        requirements.ranges().to_vec().into_boxed_slice(),
    ));
    match plan_addressless_m1_step_workspace(selection, available) {
        M1StepWorkspacePlanOutcome::Planned(plan) => plan,
        M1StepWorkspacePlanOutcome::Rejected(failure) => {
            fail_stop("plan exact step workspace", &failure)
        }
    }
}

fn monotonic_raw_ns() -> SmokeResult<u128> {
    let timestamp = clock_gettime(ClockId::MonotonicRaw);
    let seconds = u128::try_from(timestamp.tv_sec)
        .map_err(|_| "monotonic-raw clock returned negative seconds".to_owned())?;
    let nanoseconds = u128::try_from(timestamp.tv_nsec)
        .map_err(|_| "monotonic-raw clock returned negative nanoseconds".to_owned())?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or_else(|| "monotonic-raw clock conversion overflowed".to_owned())
}

fn elapsed_ns(started_ns: u128) -> SmokeResult<u64> {
    let elapsed = monotonic_raw_ns()?
        .checked_sub(started_ns)
        .ok_or_else(|| "monotonic-raw clock moved backwards".to_owned())?;
    u64::try_from(elapsed).map_err(|_| "physical duration does not fit u64".to_owned())
}

fn identity_hex(identity: Identity) -> String {
    bytes_hex(identity.as_bytes())
}

fn bytes_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn selection_name(selection: Qwen3PlanSelection) -> String {
    format!(
        "{:?}/{:?}/{:?}",
        selection.role, selection.mode, selection.bucket
    )
}

const fn verification_name(choice: M1SpeculativeVerificationChoiceV1) -> &'static str {
    match choice {
        M1SpeculativeVerificationChoiceV1::Correction { .. } => "correction",
        M1SpeculativeVerificationChoiceV1::Bonus { .. } => "bonus",
    }
}

fn projection_json(projection: &DeviceKvCacheProjection) -> Value {
    json!({
        "device_sha256": identity_hex(projection.device.device_id()),
        "node_id": projection.device.node_id(),
        "kfd_gpu_id": projection.device.kfd_gpu_id(),
        "gpu_unique_id": projection.device.gpu_unique_id(),
        "request_slot": projection.request.slot(),
        "request_generation": projection.request.generation(),
        "target": {
            "lifecycle": format!("{:?}", projection.target.lifecycle),
            "resident_tokens": projection.target.resident_tokens,
            "committed_tokens": projection.target.committed_tokens,
            "arena_allocation_sha256": projection.target_arena_allocation_id.map(identity_hex),
            "active_pages": projection.target_active_pages,
            "retired_pages": projection.target_retired_pages,
            "quiescent_retired_pages": projection.target_quiescent_retired_pages,
            "write_pending": projection.target_write_pending,
            "qualification_future_pages": projection.target_qualification_future_pages,
        },
        "draft": {
            "lifecycle": format!("{:?}", projection.draft.lifecycle),
            "resident_tokens": projection.draft.resident_tokens,
            "committed_tokens": projection.draft.committed_tokens,
            "arena_allocation_sha256": projection.draft_arena_allocation_id.map(identity_hex),
            "active_pages": projection.draft_active_pages,
            "retired_pages": projection.draft_retired_pages,
            "quiescent_retired_pages": projection.draft_quiescent_retired_pages,
            "write_pending": projection.draft_write_pending,
        },
    })
}

fn validate_report(report: &Value) -> SmokeResult<()> {
    let object = report
        .as_object()
        .ok_or_else(|| "speculative smoke report is not an object".to_owned())?;
    for (field, expected) in [
        ("authority", json!("none")),
        ("artifact_authority", json!("none")),
        ("benchmark_comparable", json!(false)),
        ("authenticated_AB_exercised", json!(false)),
        ("compiler_origin_authenticated", json!(false)),
        ("current_publication_selected", json!(false)),
        ("worker_v3_authenticated", json!(false)),
        ("hardware_completion_observed", json!(true)),
        ("status", json!(STATUS)),
        ("nonclaim", json!(NONCLAIM)),
    ] {
        if object.get(field) != Some(&expected) {
            return Err(format!("speculative smoke report claim drifted at {field}"));
        }
    }
    if object.get("registry_rollover_descriptor")
        != Some(&json!(
            "structural-host-fixture-only-no-token-or-completion-oracle"
        ))
    {
        return Err("registry rollover provenance drifted".to_owned());
    }
    let speculative = object
        .get("speculative_k4")
        .and_then(Value::as_object)
        .ok_or_else(|| "speculative smoke report lacks S1/K4 facts".to_owned())?;
    if speculative
        .get("draft_choices")
        .and_then(Value::as_array)
        .map(Vec::len)
        != Some(4)
        || speculative
            .get("target_choices")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(5)
    {
        return Err("speculative diagnostic choice shape drifted".to_owned());
    }
    Ok(())
}
