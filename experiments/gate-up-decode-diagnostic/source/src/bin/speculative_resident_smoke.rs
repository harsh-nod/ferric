//! Explicit, nonqualifying repeated K4 smoke using the real structural resident controller.

use super::{
    ActiveDeviceKvCache, AdmittedEngineeringArtifactFacts, CompletionEpoch, DRAFT_DECODE,
    DRAFT_PREFILL, Engine, EngineeringStartupDiagnosticsV1, EngineeringStartupPhaseV1,
    M1FiniteSpeculativeQueueRolloverKvInputsV1, M1FullStepKvWorkspaceTablesV1,
    M1FullStepWorkspacePlans, M1ServingCompletionDispositionV1, M1ServingPhysicalQueueCustodyV1,
    M1ServingPlanV1, M1ServingRegistryV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativeGenerationPolicyV1, M1SpeculativeMemberControlV1, M1SpeculativeMemberSeedV1,
    QWEN3_END_OF_TEXT_TOKEN, Qwen3ModelRole, Qwen3PlanSelection, SmokeResult,
    SpecialTokenDecodePolicy, TARGET_PREFILL, TARGET_SPECULATIVE, TokenizerExecutionLimits, Value,
    Write, bind_m1_kv_workspace_table_v1, bytes_hex, elapsed_ns, fail_stop, identity_hex, json,
    lease_pages, monotonic_raw_ns, prefill_workspace_plans, program_strategy_label,
    require_or_abort, smoke_bootstrap, speculative_workspace_plans, step_inputs,
    validate_program_strategy_report, verification_name, workspace_plan,
};
use ferric_engine::{
    CheckedCompletionSemantics, M1PhysicalProgramStrategyV1, M1QueueWaitTimeoutV1,
    M1QueuedServingPhysicalInputProviderV1, M1ServingPhysicalRunnerOperationsV1,
    M1ServingPhysicalRunnerReadbackEvidenceV1, M1ServingQueuedFiniteSpeculativeRolloverV1,
    M1ServingQueuedFirstPublicationV1, M1ServingQueuedGenerationBindingV1,
    M1ServingQueuedGenerationInputV1, M1StructuralResidentCommittedRoundV1,
    M1StructuralResidentRoundInputV1,
};
use std::time::Instant;

const RESIDENT_STATUS: &str = "engineering-physical-speculative-resident-smoke-non-evidence";
const RESIDENT_NONCLAIM: &str = "Bounded authority-none structural Qwen execution using real registry, bridge, coordinator and GPU completions. The optional draft maintenance is executed only after genuine continuing full acceptance. This is not authenticated M1 execution, numerical qualification, production serving or a comparable benchmark.";
const MAX_OUTPUT_TOKENS: u32 = 32;

pub(super) fn parse_limit(value: &std::ffi::OsStr) -> SmokeResult<u32> {
    let value = value
        .to_str()
        .ok_or_else(|| "resident token limit must be UTF-8 decimal".to_owned())?;
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("resident token limit must be canonical decimal in 1..=32".to_owned());
    }
    let limit = value
        .parse::<u32>()
        .map_err(|_| "resident token limit overflows u32".to_owned())?;
    if !(1..=MAX_OUTPUT_TOKENS).contains(&limit) {
        return Err("resident token limit must be in 1..=32".to_owned());
    }
    Ok(limit)
}

fn catchup_workspace_plans() -> M1FullStepWorkspacePlans {
    let completion = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        ..DRAFT_DECODE
    };
    M1FullStepWorkspacePlans::draft_catchup(
        TARGET_SPECULATIVE,
        workspace_plan(DRAFT_DECODE, 0x41),
        workspace_plan(completion, 0x42),
    )
}

fn continuation_input() -> M1StructuralResidentRoundInputV1 {
    M1StructuralResidentRoundInputV1::new(
        speculative_workspace_plans(),
        speculative_workspace_plans(),
        catchup_workspace_plans(),
        catchup_workspace_plans(),
    )
}

fn round_observation(committed: &M1StructuralResidentCommittedRoundV1) -> Value {
    let [member] = committed.outcome().members() else {
        fail_stop("resident committed singleton roster", committed)
    };
    let Some(M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(choices)) =
        committed.diagnostic_history().evidence().last()
    else {
        fail_stop("resident independent choice custody", committed)
    };
    json!({
        "round": committed.outcome().completed_round(),
        "epoch": committed.outcome().completed_epoch().value(),
        "dispatch_generation": choices.dispatch_generation(),
        "draft_choices": choices.draft_choices(),
        "target_choices": choices.target_choices(),
        "draft_choices_sha256": bytes_hex(choices.draft_sha256()),
        "target_choices_sha256": bytes_hex(choices.target_sha256()),
        "published_token_ids": member.published().tokens(),
        "verification_kind": verification_name(member.verification_choice()),
        "target_commit_end": member.target_settlement().commit_end(),
        "draft_commit_end": member.draft_settlement().commit_end(),
        "requires_draft_catchup": committed.requires_draft_catchup(),
        "completed_draft_catchups": committed.completed_draft_catchup_count(),
        "terminal": committed.outcome().next_active_roster().is_empty(),
    })
}

pub(super) fn execute_and_report(
    initialized: smoke_bootstrap::InitializedSmokeBootstrapV1,
    facts: AdmittedEngineeringArtifactFacts,
    diagnostics: &EngineeringStartupDiagnosticsV1,
    limit: u32,
    deadline: Instant,
) -> SmokeResult<()> {
    let AdmittedEngineeringArtifactFacts {
        observation: facts,
        program_strategy,
    } = facts;
    let smoke_bootstrap::InitializedSmokeBootstrapV1 {
        runner,
        mut memory,
        tokenizer,
        prompt_tokens,
    } = initialized;
    if prompt_tokens.is_empty()
        || prompt_tokens.len() > 128
        || !(1..=MAX_OUTPUT_TOKENS).contains(&limit)
    {
        return Err("resident prompt/limit is outside the bounded singleton envelope".to_owned());
    }
    if Instant::now() >= deadline {
        return Err("resident deadline expired before queue preparation".to_owned());
    }
    let raw_prompt_tokens = prompt_tokens.clone();
    let mut physical_prompt_tokens = prompt_tokens;
    physical_prompt_tokens.resize(128, QWEN3_END_OF_TEXT_TOKEN);
    let mut engine = require_or_abort(
        Engine::<1>::new(512, 256, 8_192),
        "construct resident engine",
    );
    let request = require_or_abort(engine.admit(), "admit resident request");
    require_or_abort(
        engine.append_tentative(request, 1),
        "reserve logical prefill anchor",
    );
    let prefill = require_or_abort(
        M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL),
        "bind resident prefill plan",
    );
    let speculative = require_or_abort(
        M1ServingPlanV1::new(TARGET_SPECULATIVE, DRAFT_DECODE),
        "bind resident speculative plan",
    );
    let mut registry = require_or_abort(
        M1ServingRegistryV1::<1>::new(),
        "construct actual resident registry",
    );
    require_or_abort(
        registry.admit(request, prefill),
        "admit actual resident registry member",
    );
    let prefill_batch = require_or_abort(registry.plan_next(), "plan actual paired prefill")
        .expect("fresh admitted registry is ready");
    let prefill_epoch = prefill_batch.epoch();
    let mut cache = require_or_abort(
        ActiveDeviceKvCache::new(memory.device(), request, TARGET_PREFILL, DRAFT_PREFILL),
        "construct resident KV owner",
    );
    let initial_projection = cache.projection();
    let target_inputs = step_inputs(
        &runner,
        request,
        prefill_epoch,
        TARGET_PREFILL,
        physical_prompt_tokens.clone(),
        (0..128).collect(),
        128,
        0,
    );
    let draft_inputs = step_inputs(
        &runner,
        request,
        prefill_epoch,
        DRAFT_PREFILL,
        physical_prompt_tokens.clone(),
        (0..128).collect(),
        128,
        0,
    );
    let target_pages = lease_pages(&mut memory, request, Qwen3ModelRole::Target8B, 0..8);
    let draft_pages = lease_pages(&mut memory, request, Qwen3ModelRole::Draft06B, 0..8);
    let target_pending = require_or_abort(
        cache.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            128,
            prefill_epoch,
            target_pages,
        ),
        "reserve resident target prefill",
    );
    let draft_pending = require_or_abort(
        cache.reserve_step_write(
            request,
            Qwen3ModelRole::Draft06B,
            0,
            128,
            prefill_epoch,
            draft_pages,
        ),
        "reserve resident draft prefill",
    );
    let target_table = require_or_abort(
        bind_m1_kv_workspace_table_v1(target_inputs, vec![target_pending]),
        "bind resident target table",
    );
    let draft_table = require_or_abort(
        bind_m1_kv_workspace_table_v1(draft_inputs, vec![draft_pending]),
        "bind resident draft table",
    );
    let first_target_page = require_or_abort(
        memory.lease_page(request, Qwen3ModelRole::Target8B, 8),
        "lease initial target successor page",
    );
    let first_draft_page = require_or_abort(
        memory.lease_page(request, Qwen3ModelRole::Draft06B, 8),
        "lease initial draft successor page",
    );
    let first = M1ServingQueuedGenerationInputV1::first_publication(
        M1ServingQueuedFirstPublicationV1::new_with_finite_speculative_successor(
            M1ServingQueuedGenerationBindingV1::new(
                prefill,
                vec![request].into_boxed_slice(),
                prefill_epoch,
            ),
            speculative,
            memory,
            M1FullStepKvWorkspaceTablesV1::PairedPrefill {
                draft: draft_table,
                target: target_table,
            },
            prefill_workspace_plans(),
            prefill_workspace_plans(),
            vec![cache],
        ),
    );
    let provider = M1QueuedServingPhysicalInputProviderV1::from_ordered_inputs(vec![first]);
    let mut operations = require_or_abort(
        M1ServingPhysicalRunnerOperationsV1::new(
            &runner,
            &mut engine,
            provider,
            1 << 20,
            M1QueueWaitTimeoutV1::new(120_000).expect("bounded queue timeout"),
        ),
        "construct real structural resident operations",
    );
    let reservation = require_or_abort(
        registry.reserve_publication(prefill_batch),
        "reserve real prefill publication",
    );
    let started = require_or_abort(monotonic_raw_ns(), "read resident start clock");
    if Instant::now() >= deadline {
        fail_stop("resident deadline before prefill publication", &reservation);
    }
    let published = require_or_abort(
        M1ServingPhysicalQueueCustodyV1::Vacant.publish(
            reservation,
            &mut registry,
            &mut operations,
        ),
        "publish real resident prefill",
    );
    let readback = require_or_abort(
        operations.read_structural_resident_round(published, deadline),
        "read real resident prefill",
    );
    let first_token = match readback.checked(&operations).records() {
        [record] => match record.semantics() {
            CheckedCompletionSemantics::DirectFinalRow { token } => token,
            other => fail_stop("resident prefill direct semantics", &other),
        },
        records => fail_stop("resident prefill singleton receipt", &records),
    };
    let first_speculative_epoch = CompletionEpoch::new(
        prefill_epoch
            .value()
            .checked_add(1)
            .expect("fresh prefill successor epoch"),
    );
    let target_inputs = step_inputs(
        &runner,
        request,
        first_speculative_epoch,
        TARGET_SPECULATIVE,
        vec![first_token, 0, 0, 0, 0],
        (128..133).collect(),
        5,
        128,
    );
    let draft_inputs = step_inputs(
        &runner,
        request,
        first_speculative_epoch,
        DRAFT_DECODE,
        vec![first_token],
        vec![128],
        1,
        128,
    );
    let rollover = M1ServingQueuedFiniteSpeculativeRolloverV1::new(
        M1ServingQueuedGenerationBindingV1::new(
            speculative,
            vec![request].into_boxed_slice(),
            first_speculative_epoch,
        ),
        M1FiniteSpeculativeQueueRolloverKvInputsV1::new(
            draft_inputs,
            target_inputs,
            vec![first_draft_page],
            vec![first_target_page],
        ),
        speculative_workspace_plans(),
        speculative_workspace_plans(),
    );
    require_or_abort(
        operations
            .try_enqueue_finite_speculative_rollover_after_readback(&readback, Box::new(rollover)),
        "bind actual prefill anchor into first speculative input",
    );
    let (_, physical) = require_or_abort(
        readback.complete_exact(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative)],
            &mut operations,
        ),
        "commit real prefill and registry transition",
    );
    let first_batch = require_or_abort(registry.plan_next(), "plan actual first speculative round")
        .expect("continuing singleton remains ready");
    require_or_abort(
        operations.reserve_structural_first_speculative_span(&physical, &first_batch),
        "reserve first speculative span from joined prefill owner",
    );
    let reservation = require_or_abort(
        registry.reserve_publication(first_batch),
        "reserve actual first speculative publication",
    );
    if Instant::now() >= deadline {
        fail_stop(
            "resident deadline before first speculative publication",
            &physical,
        );
    }
    let mut published = require_or_abort(
        physical.publish(reservation, &mut registry, &mut operations),
        "publish actual first speculative rollover",
    );
    let policy = require_or_abort(
        M1SpeculativeGenerationPolicyV1::new(limit, &[]),
        "construct bounded resident generation policy",
    );
    let seed = M1SpeculativeMemberSeedV1::new(request, first_token, 128, 128, policy);
    let mut coordinator = require_or_abort(
        M1SpeculativeGenerationLoopV1::new(TARGET_SPECULATIVE, &[seed]),
        "construct actual resident coordinator",
    );
    let mut rounds = Vec::with_capacity(limit as usize);
    let mut tokens = Vec::with_capacity(limit as usize);
    let mut round = 0_u64;
    let (completed_catchups, shutdown) = loop {
        if round >= u64::from(limit) {
            fail_stop("resident round budget", &published);
        }
        let epoch = published.epoch();
        let readback = require_or_abort(
            operations.read_structural_resident_round(published, deadline),
            "read actual resident speculative completion",
        );
        let binding = require_or_abort(
            coordinator.bind_round(round, epoch, &[request]),
            "bind exact resident round and physical epoch",
        );
        let permit = require_or_abort(
            coordinator.preflight_checked_round(
                binding,
                readback.checked(&operations),
                &[M1SpeculativeMemberControlV1::continuing(request)],
            ),
            "preflight actual resident model choices",
        );
        let committed = require_or_abort(
            operations.commit_structural_resident_round(
                readback,
                &mut registry,
                &mut coordinator,
                permit,
            ),
            "commit actual resident round and pending maintenance",
        );
        let [member] = committed.outcome().members() else {
            fail_stop("resident committed output roster", &committed);
        };
        tokens.extend_from_slice(member.published().tokens());
        rounds.push(round_observation(&committed));
        if tokens.len() > limit as usize {
            fail_stop("resident output cap", &committed);
        }
        if committed.outcome().next_active_roster().is_empty() {
            let completed_catchups = committed.completed_draft_catchup_count();
            let (_, stopped) = require_or_abort(
                operations.finish_structural_resident_round(committed),
                "destroy actual all-terminal resident queue",
            );
            break (completed_catchups, stopped);
        }
        let (next, _) = require_or_abort(
            operations.continue_structural_resident_round(
                committed,
                &mut registry,
                &mut coordinator,
                continuation_input(),
                deadline,
            ),
            "continue actual speculative owner through optional 425-packet draft maintenance",
        );
        published = next;
        round += 1;
    };
    drop(operations);
    if engine.is_faulted() {
        fail_stop("healthy resident terminal engine", &shutdown);
    }
    let elapsed = require_or_abort(elapsed_ns(started), "measure bounded resident observation");
    diagnostics.completed(EngineeringStartupPhaseV1::ControllerExecution);
    let bytes = tokenizer
        .decode_to_bytes(
            &tokens,
            TokenizerExecutionLimits::m1(),
            SpecialTokenDecodePolicy::Skip,
        )
        .map_err(|error| format!("cannot decode actual resident tokens: {error}"))?;
    let report = json!({
        "schema": "ferric.m1-engineering-speculative-resident-smoke-observation.v1",
        "status": RESIDENT_STATUS, "nonclaim": RESIDENT_NONCLAIM,
        "authority": "none", "artifact_authority": "none", "benchmark_comparable": false,
        "authenticated_AB_exercised": false, "compiler_origin_authenticated": false,
        "current_publication_selected": false, "worker_v3_authenticated": false,
        "hardware_completion_observed": true, "native_queue_destroyed": true,
        "target": "gfx942:xnack-", "gpu_unique_id": initial_projection.device.gpu_unique_id(),
        "program_strategy": program_strategy_label(program_strategy),
        "program_count": program_strategy.program_count(),
        "registry_path": "actual-structural-registry-bridge-coordinator",
        "maximum_new_tokens": limit, "prefill_anchor": first_token,
        "published_token_scope": "speculative-rounds-exclude-prefill-anchor",
        "published_token_ids": tokens, "rounds": rounds, "completed_draft_catchups": completed_catchups,
        "prompt": { "raw_token_ids": raw_prompt_tokens, "physical_token_ids": physical_prompt_tokens,
            "suffix_fill_semantics": "active-token-fill-not-attention-mask-padding" },
        "decoded_published_text": String::from_utf8_lossy(&bytes),
        "decoded_published_bytes_hex": bytes_hex(&bytes),
        "identities": {
            "observation_manifest_sha256": identity_hex(facts.manifest), "hsaco_sha256": identity_hex(facts.hsaco),
            "compiler_handoff_sha256": identity_hex(facts.compiler_handoff), "canonical_descriptor_sha256": identity_hex(facts.canonical_descriptor),
            "observation_program_catalog_sha256": identity_hex(facts.program_catalog),
            "model_bundle_sha256": identity_hex(runner.logical_runner().bundle_id()),
            "target_prepacked_sha256": identity_hex(runner.logical_runner().target_prepacked_id()),
            "draft_prepacked_sha256": identity_hex(runner.logical_runner().draft_prepacked_id()),
            "plan_catalog_sha256": identity_hex(runner.logical_runner().plan_catalog_id()),
            "generated_runner_declaration_sha256": identity_hex(runner.declaration_id()),
        },
        "timing": { "scope": "single-process-structural-observation-not-benchmark", "prefill-through-native-teardown_ns": elapsed },
    });
    validate_resident_report(&report, program_strategy)?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize resident report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot terminate resident report: {error}"))?;
    Ok(())
}

fn validate_resident_report(
    report: &Value,
    admitted_strategy: M1PhysicalProgramStrategyV1,
) -> SmokeResult<()> {
    validate_program_strategy_report(report, admitted_strategy)?;
    for (field, expected) in [
        ("authority", json!("none")),
        ("artifact_authority", json!("none")),
        ("benchmark_comparable", json!(false)),
        ("authenticated_AB_exercised", json!(false)),
        ("compiler_origin_authenticated", json!(false)),
        ("current_publication_selected", json!(false)),
        ("worker_v3_authenticated", json!(false)),
        ("hardware_completion_observed", json!(true)),
        ("native_queue_destroyed", json!(true)),
        ("status", json!(RESIDENT_STATUS)),
        ("nonclaim", json!(RESIDENT_NONCLAIM)),
        (
            "registry_path",
            json!("actual-structural-registry-bridge-coordinator"),
        ),
    ] {
        if report.get(field) != Some(&expected) {
            return Err(format!("resident report claim drift at {field}"));
        }
    }
    let limit = report
        .get("maximum_new_tokens")
        .and_then(Value::as_u64)
        .filter(|limit| (1..=u64::from(MAX_OUTPUT_TOKENS)).contains(limit))
        .ok_or_else(|| "resident report limit drift".to_owned())?;
    let tokens = report
        .get("published_token_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "resident report lacks output tokens".to_owned())?;
    let rounds = report
        .get("rounds")
        .and_then(Value::as_array)
        .ok_or_else(|| "resident report lacks rounds".to_owned())?;
    if tokens.is_empty()
        || tokens.len() as u64 > limit
        || rounds.is_empty()
        || rounds.len() as u64 > limit
        || rounds.last().and_then(|round| round.get("terminal")) != Some(&json!(true))
    {
        return Err("resident bounded terminal output drift".to_owned());
    }
    for round in rounds {
        if round
            .get("draft_choices")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(4)
            || round
                .get("target_choices")
                .and_then(Value::as_array)
                .map(Vec::len)
                != Some(5)
        {
            return Err("resident K4 diagnostic shape drift".to_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::Qwen3ExecutionMode;

    #[test]
    fn resident_report_is_bound_to_admitted_strategy_and_count() {
        for (admitted, label, count) in [
            (
                M1PhysicalProgramStrategyV1::LegacyScalar12,
                "LegacyScalar12",
                12,
            ),
            (
                M1PhysicalProgramStrategyV1::AttributedMfma13,
                "AttributedMfma13",
                13,
            ),
        ] {
            assert_eq!(program_strategy_label(admitted), label);
            assert_eq!(admitted.program_count(), count);
            let other = match admitted {
                M1PhysicalProgramStrategyV1::LegacyScalar12 => {
                    M1PhysicalProgramStrategyV1::AttributedMfma13
                }
                M1PhysicalProgramStrategyV1::AttributedMfma13 => {
                    M1PhysicalProgramStrategyV1::LegacyScalar12
                }
            };
            let report = json!({
                "authority": "none", "artifact_authority": "none", "benchmark_comparable": false,
                "authenticated_AB_exercised": false, "compiler_origin_authenticated": false,
                "current_publication_selected": false, "worker_v3_authenticated": false,
                "hardware_completion_observed": true, "native_queue_destroyed": true,
                "status": RESIDENT_STATUS, "nonclaim": RESIDENT_NONCLAIM,
                "registry_path": "actual-structural-registry-bridge-coordinator",
                "program_strategy": label, "program_count": count,
                "maximum_new_tokens": 1, "published_token_ids": [1],
                "rounds": [{"terminal": true, "draft_choices": [1, 2, 3, 4], "target_choices": [1, 2, 3, 4, 5]}],
            });
            assert!(validate_resident_report(&report, admitted).is_ok());
            assert!(validate_resident_report(&report, other).is_err());
            let mut wrong_count = report.clone();
            wrong_count["program_count"] = json!(other.program_count());
            assert!(validate_resident_report(&wrong_count, admitted).is_err());
            let mut wrong_label = report.clone();
            wrong_label["program_strategy"] = json!(program_strategy_label(other));
            assert!(validate_resident_report(&wrong_label, admitted).is_err());
            let mut missing = report;
            missing.as_object_mut().unwrap().remove("program_count");
            assert!(validate_resident_report(&missing, admitted).is_err());
        }
    }

    #[test]
    fn resident_cli_limit_is_bounded_and_canonical() {
        for value in ["1", "4", "32"] {
            assert!(parse_limit(value.as_ref()).is_ok());
        }
        for value in [
            "",
            "0",
            "00",
            "01",
            "33",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "null",
            "4294967296",
        ] {
            assert!(parse_limit(value.as_ref()).is_err(), "{value}");
        }
    }

    #[test]
    fn resident_plan_pair_keeps_original_speculative_allocation_identity() {
        let spec = speculative_workspace_plans();
        let catchup = catchup_workspace_plans();
        let M1FullStepWorkspacePlans::SpeculativeRound {
            draft_decode,
            target_speculative,
        } = spec
        else {
            panic!("speculative plans");
        };
        let M1FullStepWorkspacePlans::DraftCatchup {
            parent,
            draft_decode: catchup_draft,
            completion,
        } = catchup
        else {
            panic!("catchup plans");
        };
        assert_eq!(parent, TARGET_SPECULATIVE);
        assert_eq!(
            draft_decode.allocation().allocation_id(),
            catchup_draft.allocation().allocation_id()
        );
        assert_eq!(
            target_speculative.allocation().allocation_id(),
            completion.allocation().allocation_id()
        );
        assert_eq!(completion.selection().role, Qwen3ModelRole::Target8B);
        assert_eq!(completion.selection().mode, Qwen3ExecutionMode::Decode);
    }
}
