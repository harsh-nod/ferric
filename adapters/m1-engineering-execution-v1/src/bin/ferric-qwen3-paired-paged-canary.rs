//! Bounded paired native K4 observation, not speculative serving or M1 authority.

#![recursion_limit = "256"]

mod paired_paged_canary_contract;
mod tp_worker;

use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::batched::{
    EngineeringTpBatchExecutionV2, EngineeringTpBatchOutputV2, EngineeringTpDraftBatchExecutionV10,
};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
};
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::speculative::{
    EngineeringTpSpeculativeKvV1, EngineeringTpSpeculativePhaseV1,
};
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1,
    EngineeringTpPoolScopeV1, EngineeringTpPreparedBatchV1, EngineeringTpSequenceIdV1,
};
use ferric_spec::{Identity, Qwen3ModelRole};
use paired_paged_canary_contract::{
    CHUNK, CONTEXT, K, Options, PAGES, PREFILL_BATCHES, PREFIX_TOKENS, PROMPT_TOKENS, PacketBudget,
    ROUNDS, Reference, SOURCE_REFERENCE_SHA256, bootstrap_index, digest, finish_pair, hash_file,
    hex,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tp_worker::Worker;

fn emit(value: &Value) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

fn packet_count(counts: &[u64]) -> Result<u64, String> {
    match counts {
        [count] => Ok(*count),
        _ => Err("exact TP1 packet counter required".into()),
    }
}

fn input_rows(rows: &[EngineeringTpPageRowV1]) -> Vec<Value> {
    rows.iter()
        .map(|row| json!({"token":row.token,"position":row.position}))
        .collect()
}

fn artifact_identity(artifact: &EngineeringTpArtifactV1) -> Value {
    json!({"hsaco":hex(artifact.hsaco_id().as_bytes()),
        "manifest":hex(artifact.manifest_id().as_bytes()),
        "handoff":hex(artifact.handoff_id().as_bytes())})
}

trait PrefillDriver {
    fn execute_prefill(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> Result<EngineeringTpBatchOutputV2, String>;
    fn counts(&self) -> Vec<u64>;
    fn batches(&self) -> u64;
}

impl<R: EngineeringTpRankTransportV1> PrefillDriver for EngineeringTpBatchExecutionV2<R> {
    fn execute_prefill(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> Result<EngineeringTpBatchOutputV2, String> {
        self.execute_selected(batch, &[])
    }
    fn counts(&self) -> Vec<u64> {
        self.dispatch_counts()
    }
    fn batches(&self) -> u64 {
        self.completed_batches()
    }
}

impl<R: EngineeringTpRankTransportV1> PrefillDriver for EngineeringTpDraftBatchExecutionV10<R> {
    fn execute_prefill(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> Result<EngineeringTpBatchOutputV2, String> {
        self.execute_selected(batch, &[])
    }
    fn counts(&self) -> Vec<u64> {
        self.dispatch_counts()
    }
    fn batches(&self) -> u64 {
        self.completed_batches()
    }
}

fn prefill<D: PrefillDriver>(
    driver: &mut D,
    pool: &mut EngineeringTpPagedPoolV1,
    prompt: &[u32],
    expected_packets: u64,
) -> Result<(EngineeringTpSequenceIdV1, Vec<Value>), String> {
    if prompt.len() != PROMPT_TOKENS
        || !pool.is_empty()
        || packet_count(&driver.counts())? != 0
        || driver.batches() != 0
    {
        return Err("prefill requires fresh independent driver/pool".into());
    }
    let hit = pool
        .open_sequence(pool.scope(), prompt, 0)
        .map_err(|e| format!("open: {e:?}"))?;
    if hit.hit_tokens() != 0 || hit.hit_pages() != 0 {
        return Err("paired canary forbids prefix reuse".into());
    }
    let sequence = hit.sequence();
    let mut steps = Vec::with_capacity(PREFILL_BATCHES);
    for (ordinal, tokens) in prompt[..PREFIX_TOKENS].chunks(CHUNK).enumerate() {
        let start = pool
            .committed_position(sequence)
            .map_err(|e| format!("cursor: {e:?}"))?;
        let rows = tokens
            .iter()
            .enumerate()
            .map(|(index, &token)| {
                Ok(EngineeringTpPageRowV1 {
                    sequence,
                    token,
                    position: start
                        .checked_add(u32::try_from(index).map_err(|_| "row index")?)
                        .ok_or("position overflow")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let batch = pool
            .reserve_batch(&rows)
            .map_err(|e| format!("prefill reserve: {e:?}"))?;
        let before = packet_count(&driver.counts())?;
        let expected_batches = u64::try_from(ordinal + 1).map_err(|_| "prefill ordinal")?;
        pool.begin_submission(&batch)
            .map_err(|e| format!("prefill submission: {e:?}"))?;
        let observed = match driver.execute_prefill(&batch) {
            Ok(value) => value,
            Err(error) => {
                let quarantine = pool.quarantine_batch(&batch);
                return Err(format!(
                    "prefill execution: {error}; quarantine: {quarantine:?}"
                ));
            }
        };
        let completed = match packet_count(&driver.counts()) {
            Ok(value) => value,
            Err(error) => {
                let quarantine = pool.quarantine_batch(&batch);
                return Err(format!(
                    "prefill counter: {error}; quarantine: {quarantine:?}"
                ));
            }
        };
        if !observed.choices.is_empty()
            || completed.checked_sub(before) != Some(expected_packets)
            || driver.batches() != expected_batches
        {
            let quarantine = pool.quarantine_batch(&batch);
            return Err(format!(
                "prefill output/completion count; quarantine: {quarantine:?}"
            ));
        }
        if let Err(error) = pool.commit_batch(&batch, observed.completion) {
            let quarantine = pool.quarantine_batch(&batch);
            return Err(format!(
                "prefill commit: {error:?}; quarantine: {quarantine:?}"
            ));
        }
        steps.push(json!({"batch_id":batch.id(),"inputs":input_rows(&rows),
            "selected_rows":[],"completed_packets":completed}));
    }
    if pool
        .committed_position(sequence)
        .map_err(|e| format!("prefill cursor: {e:?}"))?
        != u32::try_from(PREFIX_TOKENS).map_err(|_| "prefix length")?
    {
        return Err("exact127 resident prefill inputs required".into());
    }
    pool.check_invariants()
        .map_err(|e| format!("prefill pool: {e:?}"))?;
    Ok((sequence, steps))
}

fn run_round<R: EngineeringTpRankTransportV1>(
    ordinal: usize,
    target: &mut EngineeringTpBatchExecutionV2<R>,
    draft: &mut EngineeringTpDraftBatchExecutionV10<R>,
    owner: &mut EngineeringTpSpeculativeKvV1,
    budget: PacketBudget,
) -> Result<(Value, Vec<u32>, bool), String> {
    let anchor = owner.next_anchor();
    let epoch = owner.next_epoch().value();
    owner
        .reserve_proposals()
        .map_err(|e| format!("reserve proposals: {e:?}"))?;
    let mut proposals = Vec::with_capacity(K);
    for index in 0..K {
        let before = packet_count(&draft.dispatch_counts())?;
        let work = owner
            .proposal_work()
            .map_err(|e| format!("proposal work: {e:?}"))?;
        if usize::from(work.ordinal()) != index {
            return Err("proposal ordinal drift".into());
        }
        let result = draft.execute_speculative_proposal(work)?;
        let completed = packet_count(&draft.dispatch_counts())?;
        if completed.checked_sub(before) != Some(budget.draft_proposal)
            || result.completion_epoch().value() != epoch
        {
            return Err("proposal completion/epoch/packet drift".into());
        }
        proposals.push(
            json!({"ordinal":result.ordinal(),"pool_identity":result.pool_identity(),
            "batch_id":result.batch_id(),"epoch":result.completion_epoch().value(),
            "request":{"slot":result.request().slot(),"generation":result.request().generation()},
            "input":{"token":result.input().token,"position":result.input().position},
            "choice":result.choice(),"completed_packets":completed}),
        );
        owner
            .record_proposal(result)
            .map_err(|e| format!("record proposal: {e:?}"))?;
    }
    let before = packet_count(&target.dispatch_counts())?;
    let work = owner
        .target_work()
        .map_err(|e| format!("target work: {e:?}"))?;
    let result = target.execute_speculative_target(work)?;
    let completed = packet_count(&target.dispatch_counts())?;
    if completed.checked_sub(before) != Some(budget.target_verify)
        || result.completion_epoch().value() != epoch
        || result.choices().len() != K + 1
    {
        return Err("target verification completion/epoch/packet drift".into());
    }
    let target_before = result
        .inputs()
        .first()
        .ok_or("empty target inputs")?
        .position;
    let verification = json!({"pool_identity":result.pool_identity(),"batch_id":result.batch_id(),
        "epoch":result.completion_epoch().value(),
        "request":{"slot":result.request().slot(),"generation":result.request().generation()},
        "inputs":input_rows(result.inputs()),"selected_rows":result.output_rows(),
        "choices":result.choices(),"completed_packets":completed});
    owner
        .record_target(result)
        .map_err(|e| format!("record target: {e:?}"))?;
    let receipt = owner.settle().map_err(|e| format!("settlement: {e:?}"))?;
    let accepted = receipt.greedy_commit().accepted_draft_tokens();
    let emitted = receipt.greedy_commit().emitted_tokens().to_vec();
    let catch_up_required = receipt.draft_catch_up_required();
    if accepted > K || emitted.len() != accepted + 1 || catch_up_required != (accepted == K) {
        return Err("settlement cardinality drift".into());
    }
    let catch_up = if catch_up_required {
        owner
            .reserve_draft_catch_up()
            .map_err(|e| format!("catch-up reserve: {e:?}"))?;
        let before = packet_count(&draft.dispatch_counts())?;
        let work = owner
            .draft_work()
            .map_err(|e| format!("catch-up work: {e:?}"))?;
        if !work.is_catch_up() {
            return Err("catch-up purpose drift".into());
        }
        let result = draft.execute_speculative_draft(work)?;
        let completed = packet_count(&draft.dispatch_counts())?;
        if completed.checked_sub(before) != Some(budget.draft_catch_up) || !result.is_catch_up() {
            return Err("catch-up completion/packet drift".into());
        }
        let trace = json!({"pool_identity":result.pool_identity(),"batch_id":result.batch_id(),
            "epoch":result.completion_epoch().value(),"inputs":input_rows(result.inputs()),
            "selected_rows":[],"completed_packets":completed});
        owner
            .record_draft(result)
            .map_err(|e| format!("record catch-up: {e:?}"))?;
        Some(trace)
    } else {
        None
    };
    let trace = json!({"schema":"FerricPairedPagedK4RoundV1","authority":"none","performance_qualified":false,
        "ordinal":ordinal,"epoch":epoch,"anchor":anchor,"cursor_before":target_before,
        "proposals":proposals,"target_verification":verification,"accepted_draft_tokens":accepted,
        "emitted_tokens":emitted,"correction_or_bonus":receipt.greedy_commit().target_correction_or_bonus(),
        "target_cursor_after_settlement":receipt.target_cursor(),
        "draft_cursor_after_settlement":receipt.draft_cursor(),"catch_up":catch_up,
        "next_epoch":owner.next_epoch().value(),"next_anchor":owner.next_anchor()});
    Ok((trace, emitted, catch_up_required))
}

#[allow(clippy::too_many_arguments)]
fn run_canary<R: EngineeringTpRankTransportV1>(
    target: &mut EngineeringTpBatchExecutionV2<R>,
    draft: &mut EngineeringTpDraftBatchExecutionV10<R>,
    mut target_pool: EngineeringTpPagedPoolV1,
    mut draft_pool: EngineeringTpPagedPoolV1,
    model: &EngineeringQwenModelV1,
    reference: &Reference,
    plan_id: Identity,
    budget: PacketBudget,
) -> Result<bool, String> {
    let (target_sequence, target_steps) = prefill(
        target,
        &mut target_pool,
        &reference.prompt_token_ids,
        budget.target_prefill,
    )?;
    let (draft_sequence, draft_steps) = prefill(
        draft,
        &mut draft_pool,
        &reference.prompt_token_ids,
        budget.draft_prefill,
    )?;
    emit(
        &json!({"schema":"FerricPairedPagedK4PrefillV1","authority":"none","performance_qualified":false,
        "committed_tokens":PREFIX_TOKENS,"anchor":reference.prompt_token_ids[PREFIX_TOKENS],
        "target_pool_identity":target_pool.identity(),"draft_pool_identity":draft_pool.identity(),
        "target_steps":target_steps,"draft_steps":draft_steps}),
    )?;
    let index = bootstrap_index(reference.prompt_token_ids[PREFIX_TOKENS], plan_id)?;
    let mut owner = EngineeringTpSpeculativeKvV1::new(
        target_pool,
        target_sequence,
        draft_pool,
        draft_sequence,
        &index,
    )
    .map_err(|e| format!("paired owner: {e:?}"))?;
    let observed = (|| {
        let mut generated = Vec::with_capacity(ROUNDS * (K + 1));
        let mut catch_ups = 0_u64;
        for ordinal in 0..ROUNDS {
            let (trace, emitted, catch_up) = run_round(ordinal, target, draft, &mut owner, budget)?;
            generated.extend_from_slice(&emitted);
            catch_ups += u64::from(catch_up);
            for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
                let pool = owner.pool(role);
                pool.check_invariants()
                    .map_err(|e| format!("settled pool: {e:?}"))?;
                if pool.stats().cached_pages != 0 || pool.stats().quarantined_pages != 0 {
                    return Err("settled role retained cached/quarantined pages".into());
                }
            }
            if owner.phase() != EngineeringTpSpeculativePhaseV1::Ready {
                return Err("round did not finish required catch-up".into());
            }
            emit(&trace)?;
        }
        let target_cursor = owner
            .pool(Qwen3ModelRole::Target8B)
            .committed_position(target_sequence)
            .map_err(|e| format!("target cursor: {e:?}"))?;
        let draft_cursor = owner
            .pool(Qwen3ModelRole::Draft06B)
            .committed_position(draft_sequence)
            .map_err(|e| format!("draft cursor: {e:?}"))?;
        let target_packets = packet_count(&target.dispatch_counts())?;
        let draft_packets = packet_count(&draft.dispatch_counts())?;
        let draft_expected = u64::try_from(PREFILL_BATCHES).map_err(|_| "prefill count")?
            * budget.draft_prefill
            + u64::try_from(ROUNDS * K).map_err(|_| "proposal count")? * budget.draft_proposal
            + catch_ups * budget.draft_catch_up;
        if target_cursor != draft_cursor
            || target_cursor
                != u32::try_from(PREFIX_TOKENS + generated.len()).map_err(|_| "final cursor")?
            || target_packets != budget.target_total
            || draft_packets != draft_expected
            || draft_packets > budget.draft_total_max
            || target.completed_batches()
                != u64::try_from(PREFILL_BATCHES + ROUNDS).map_err(|_| "target batches")?
            || draft.completed_batches()
                != u64::try_from(PREFILL_BATCHES + ROUNDS * K).map_err(|_| "draft batches")?
                    + catch_ups
        {
            return Err("paired final cursor/batch/packet budget mismatch".into());
        }
        let bytes = model.decode(&generated)?;
        let parity = reference.matches(&generated, &bytes);
        let pool_state = |role| {
            let pool = owner.pool(role);
            let stats = pool.stats();
            json!({"identity":pool.identity(),"free_pages":stats.free_pages,
                "retained_pages":stats.retained_pages,"cached_pages":stats.cached_pages,
                "quarantined_pages":stats.quarantined_pages})
        };
        emit(
            &json!({"schema":"FerricPairedPagedK4ObservationV1","authority":"none","performance_qualified":false,
            "rounds":ROUNDS,"generated_tokens":generated,"generated_utf8_hex":hex(&bytes),
            "reference_passed":parity,"catch_up_count":catch_ups,"native_catch_up_exercised":catch_ups > 0,
            "target_cursor":target_cursor,"draft_cursor":draft_cursor,
            "target_packets":target_packets,"draft_packets":draft_packets,
            "target_pool":pool_state(Qwen3ModelRole::Target8B),"draft_pool":pool_state(Qwen3ModelRole::Draft06B),
            "pool_retirement":"resident until both owned drivers close; no mutable pool extraction"}),
        )?;
        Ok::<_, String>(parity)
    })();
    observed.map_err(|error| {
        let abort = owner.abort_unsubmitted();
        let quarantine = if abort.is_err() {
            Some(owner.fail_submitted())
        } else {
            None
        };
        format!("{error}; unsubmitted abort: {abort:?}; submitted quarantine: {quarantine:?}")
    })
}

fn checked_worker(options: &Options, artifact: &EngineeringTpArtifactV1) -> Result<Worker, String> {
    let mut worker =
        Worker::spawn_with_options(&options.worker, options.device, artifact, options.runtime)?;
    let running = hash_file(&PathBuf::from(format!("/proc/{}/exe", worker.pid())));
    match running {
        Ok(hash) if hash == options.worker_sha256 => Ok(worker),
        other => {
            let close = worker.close();
            Err(format!(
                "running worker identity: {other:?}; close: {close:?}"
            ))
        }
    }
}

fn run(options: &Options) -> Result<(), String> {
    let reference = Reference::open(options)?;
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&options.worker)? != options.worker_sha256 {
        return Err("worker SHA-256 differs".into());
    }
    let target_artifact = EngineeringTpArtifactV1::open_batch32(
        &options.target_artifact,
        &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
        true,
    )
    .map_err(|e| e.to_string())?;
    let head_artifact = EngineeringTpArtifactV1::open_fp32_head32(
        &options.target_head_artifact,
        &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
    )
    .map_err(|e| e.to_string())?;
    let draft_artifact = EngineeringTpArtifactV1::open_draft32(&options.draft_artifact)
        .map_err(|e| e.to_string())?;
    let model = EngineeringQwenModelV1::open_with_draft(&options.source)?;
    let draft_model = model.draft().ok_or("authenticated draft not retained")?;
    let mut session = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let limits = EngineeringTpPagedLimitsV1::new(CONTEXT, 1, PAGES, 100)
        .map_err(|e| format!("limits: {e:?}"))?;
    let target_pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *model.bundle_id().as_bytes(),
            session,
        },
        limits,
    )
    .map_err(|e| format!("target pool: {e:?}"))?;
    let draft_pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *draft_model.config().model_id.as_bytes(),
            session,
        },
        limits,
    )
    .map_err(|e| format!("draft pool: {e:?}"))?;
    let plan_record = json!({"schema":"FerricPairedPagedK4EngineeringPlanV1","authority":"none",
        "controller_sha256":controller_sha256,"worker_sha256":options.worker_sha256,
        "source_reference_sha256":SOURCE_REFERENCE_SHA256,"reference_sha256":options.reference_sha256,
        "model_bundle_id":hex(model.bundle_id().as_bytes()),"target_model_id":hex(model.config().model_id.as_bytes()),
        "draft_model_id":hex(draft_model.config().model_id.as_bytes()),
        "target_artifact":artifact_identity(&target_artifact),"target_head_artifact":artifact_identity(&head_artifact),
        "draft_artifact":artifact_identity(&draft_artifact),"k":K,"rounds":ROUNDS,
        "context":CONTEXT,"pages_per_role":PAGES,"prefill_chunk":CHUNK});
    let plan_bytes = serde_json::to_vec(&plan_record).map_err(|e| e.to_string())?;
    let plan_id = Identity::new(Sha256::digest(&plan_bytes).into());
    let mut target_worker = checked_worker(options, &target_artifact)?;
    if let Err(error) = target_worker.load_additional_artifact(&head_artifact) {
        let close = target_worker.close();
        return Err(format!(
            "target head load: {error}; target close: {close:?}"
        ));
    }
    let mut draft_worker = match checked_worker(options, &draft_artifact) {
        Ok(worker) => worker,
        Err(error) => {
            let close = target_worker.close();
            return Err(format!(
                "draft worker setup: {error}; target close: {close:?}"
            ));
        }
    };
    let pids = [target_worker.pid(), draft_worker.pid()];
    let mut target = match EngineeringTpBatchExecutionV2::new_wide32(
        vec![target_worker],
        model.config(),
        model.target_weights(),
        model.layout(),
        &target_pool,
    ) {
        Ok(driver) => driver,
        Err(error) => {
            let close = draft_worker.close();
            return Err(format!(
                "target driver setup: {error}; draft close: {close:?}"
            ));
        }
    };
    let configured = (|| {
        target.configure_output_head_pruning(true)?;
        target.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)?;
        target.configure_projection(
            EngineeringTpProjectionModeV3::Mfma,
            model.target_weights(),
            model.layout(),
        )?;
        target.configure_head_precision_v8(true)
    })();
    if let Err(error) = configured {
        let target_close = target.close();
        let draft_close = draft_worker.close();
        return Err(format!(
            "target profile: {error}; target close: {target_close:?}; draft close: {draft_close:?}"
        ));
    }
    let mut draft = match EngineeringTpDraftBatchExecutionV10::new(
        vec![draft_worker],
        draft_model.config(),
        draft_model.weights(),
        draft_model.layout(),
        &draft_pool,
        &draft_artifact,
        EngineeringTpProjectionModeV3::Baseline,
    ) {
        Ok(driver) => driver,
        Err(error) => {
            let close = target.close();
            return Err(format!(
                "draft driver setup: {error}; target close: {close:?}"
            ));
        }
    };
    let observed = (|| {
        let budget = PacketBudget::new(
            &target.expected_dispatch_counts(0),
            &target.expected_dispatch_counts(K + 1),
            &draft.expected_dispatch_counts(0),
            &draft.expected_dispatch_counts(1),
        )?;
        emit(
            &json!({"schema":"FerricPairedPagedK4SetupV1","authority":"none","performance_qualified":false,
            "engineering_plan_id":hex(plan_id.as_bytes()),"engineering_plan_record":plan_record,
            "worker_pids":{"target":pids[0],"draft":pids[1]},"device_unique_id":options.device,
            "session":hex(&session),"target_pool_identity":target_pool.identity(),"draft_pool_identity":draft_pool.identity(),
            "prompt_token_ids":reference.prompt_token_ids,"initial_committed_tokens":PREFIX_TOKENS,
            "initial_anchor":reference.prompt_token_ids[PREFIX_TOKENS],"row_capacity":32,
            "target_projection":"mfma","draft_projection":"baseline","attention":"baseline",
            "target_head":"fp32-v8","draft_head":"fp32-v10","collective":"device-tp1-v3",
            "prefix_cache":false,"runtime_cache_admission":true,"runtime_operational":true,"runtime_rollover":true,
            "runtime_ordered_batches":false,"runtime_sequences":false,"wave_attention":false,
            "target_payload_bytes":model.target_weights().len(),"draft_payload_bytes":draft_model.weights().len(),
            "draft_weights_sha256":digest(draft_model.weights()),"target_transposed_bytes":target.transposed_weight_bytes(),
            "draft_transposed_bytes":draft.transposed_weight_bytes(),"target_kv_payload_bytes":limits.target_kv_payload_bytes().map_err(|e| format!("target KV bytes: {e:?}"))?,
            "draft_kv_payload_bytes":u64::from(PAGES)*16*28*2*1024*2,
            "fp32_workspace_bytes_per_role":19_447_808_u64,"packet_budget":budget,
            "numerical_status":"two genuine K4 rounds compared with frozen target prefix; catch-up coverage conditional"}),
        )?;
        run_canary(
            &mut target,
            &mut draft,
            target_pool,
            draft_pool,
            &model,
            &reference,
            plan_id,
            budget,
        )
    })();
    // Both close calls run even after an execution error or the first close failure.
    let target_close = target.close();
    let draft_close = draft.close();
    let closed_record = emit(
        &json!({"schema":"FerricPairedPagedK4ClosedV1","authority":"none","performance_qualified":false,
        "execution_completed":observed.is_ok(),"reference_passed":observed.as_ref().ok(),
        "target_worker_exited":target_close.is_ok(),"draft_worker_exited":draft_close.is_ok(),
        "worker_pids":{"target":pids[0],"draft":pids[1]},"target_packets":target.dispatch_counts(),
        "draft_packets":draft.dispatch_counts(),"target_completed_batches":target.completed_batches(),
        "draft_completed_batches":draft.completed_batches()}),
    );
    let finished = finish_pair(observed, target_close, draft_close);
    match (finished, closed_record) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), closed) => Err(format!("{error}; closed record: {closed:?}")),
        (Ok(()), Err(error)) => Err(error),
    }
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Paired paged canary rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
