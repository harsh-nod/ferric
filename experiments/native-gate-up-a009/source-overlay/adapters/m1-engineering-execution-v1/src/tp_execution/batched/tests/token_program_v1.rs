//! Actual driver with a recording transport, not kernel or native evidence.
#[path = "../../../token_program_abi_fixture.rs"]
mod abi_fixture;
#[path = "splitk_gate_up_capture_r1.rs"]
mod gate_up_capture;
use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
    QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};
use crate::tp_paged::{EngineeringTpPreparedBatchV1, EngineeringTpSequenceIdV1};

fn configured(
    pool: &EngineeringTpPagedPoolV1,
    selected: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    driver.inner.transports[0].token_program_supported = selected;
    driver
        .configure_ordered64_kv_copy_bindings_v1(
            Fp32ArgmaxBindingV11::recording(),
            QueryHoistBindingV14::recording(),
            WaveRmsNormBindingV15::recording(),
            PrefillKvCopyBindingV27::recording(),
            SplitAttentionBindingV21::recording(),
            GemvPrefetchBindingV20::recording(),
            C1KvCopyBindingV19::recording(),
            true,
        )
        .unwrap();
    driver
}

fn token_pool() -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new_wide32(
        wide_pool().scope(),
        EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap(),
    )
    .unwrap()
}

fn gate_up_configured(
    pool: &EngineeringTpPagedPoolV1,
    enabled: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
    use crate::tp_artifact::SplitKGateUpBindingR1;
    let mut driver = configured(pool, true);
    driver.inner.transports[0].prefill_program_supported = true;
    driver.inner.transports[0].prefill32_program_supported = true;
    driver
        .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true)
        .unwrap();
    let original = driver.inner.ranks[0].layers[0].weight(Qwen3TensorKind::GateProjection);
    let transposed = driver.inner.transports[0].reserve_readonly_capacity(4096 * 12_288, 2);
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    for layer in &mut driver.inner.ranks[0].layers {
        for (kind, weight) in &mut layer.weights {
            if matches!(
                kind,
                Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection
            ) {
                *weight = driver.inner.transports[0].reserve_readonly_capacity(4096 * 12_288, 2);
                let kn = driver.inner.transports[0].reserve_readonly_capacity(4096 * 12_288, 2);
                driver
                    .projection
                    .insert_synthetic_mfma_weight(weight.id, kn);
            }
        }
    }
    let scratch = allocate_tensor(&mut driver.inner.transports[0], 4 * 12_288, 4).unwrap();
    driver.splitk_gate_up_r1 = Some(super::super::splitk_gate_up_r1::Workspace::recording(
        scratch,
    ));
    driver
        .configure_splitk_gate_up_binding(SplitKGateUpBindingR1::recording(), enabled)
        .unwrap();
    driver.inner.transports[0].token_program_packets = if enabled { 724 } else { 652 };
    driver
}

fn bind_gate_up_rows(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    batch: &EngineeringTpPreparedBatchV1,
    decode: bool,
    published: bool,
) -> TpResult<()> {
    use crate::tp_scheduler::{TpBatchRowKindV1 as Kind, TpBatchRowV1, TpRequestIdV1};
    let rows = batch
        .rows()
        .iter()
        .enumerate()
        .map(|(i, row)| TpBatchRowV1 {
            request: TpRequestIdV1 {
                slot: 0,
                generation: 1,
            },
            token_id: row.token(),
            absolute_position: row.position(),
            kind: if decode {
                Kind::Decode
            } else if published && i == 31 {
                Kind::PrefillFinal
            } else {
                Kind::PrefillIntermediate
            },
        })
        .collect::<Vec<_>>();
    driver.bind_dispatch_rows(batch, &rows)
}

#[test]
fn splitk_gate_up_admission_retains_close_errors_and_closes_other_ranks() {
    use crate::tp_artifact::SplitKGateUpBindingR1;
    let pool = token_pool();
    let mut driver = configured(&pool, true);
    let mut transports = std::mem::take(&mut driver.inner.transports);
    let events = transports[0].events.clone();
    transports[0].failure = Some(Failure::Close);
    let error = super::super::splitk_gate_up_r1::admit_transports(
        &mut transports,
        SplitKGateUpBindingR1::recording(),
    )
    .unwrap_err();
    assert!(error.starts_with("split-K gate/up requires the explicit native32 transport"));
    assert!(error.contains("admission close rank 0: injected close failure"));
    transports[0].prefill_program_supported = true;
    transports[0].prefill32_program_supported = true;
    let error = super::super::splitk_gate_up_r1::admit_transports(
        &mut transports,
        SplitKGateUpBindingR1::recording(),
    )
    .unwrap_err();
    assert!(error.starts_with("recording image was not loaded"));
    assert!(error.contains("admission close rank 0: injected close failure"));
    let mut second = configured(&pool, true);
    let second_events = second.inner.transports[0].events.clone();
    let before = second_events.borrow().len();
    transports.append(&mut second.inner.transports);
    let error = super::super::splitk_gate_up_r1::admit_transports(
        &mut transports,
        SplitKGateUpBindingR1::recording(),
    )
    .unwrap_err();
    assert!(error.contains("admission close rank 0: injected close failure"));
    assert_eq!(second_events.borrow().len(), before + 1);
    assert_eq!(second_events.borrow().last(), Some(&Event::Close(0)));
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Close(_)))
    );
    transports[0].failure = None;
    let error = super::super::splitk_gate_up_r1::admit_transports(
        &mut transports,
        SplitKGateUpBindingR1::recording(),
    )
    .unwrap_err();
    assert!(!error.contains("admission close"));
    assert!(matches!(events.borrow().last(), Some(Event::Close(0))));
}

#[test]
fn splitk_gate_up_native724_preserves_prefill649_and_exact_decode_order() {
    use crate::tp_artifact::ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1 as ROOTS;
    for enabled in [false, true] {
        let mut pool = token_pool();
        let mut driver = gate_up_configured(&pool, enabled);
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 130], 0)
            .unwrap()
            .sequence();
        for first in (0..128).step_by(32) {
            let batch = reserve(&mut pool, sequence, first, first + 32);
            bind_gate_up_rows(&mut driver, &batch, false, first == 96).unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = driver
                .execute_selected(&batch, if first == 96 { &[31] } else { &[] })
                .unwrap();
            pool.commit_batch(&batch, output.completion).unwrap();
        }
        assert_eq!(driver.dispatch_counts(), [4 * 649 + 3]);
        assert!(
            driver.inner.transports[0]
                .commands
                .iter()
                .all(|c| !ROOTS.contains(&c.kernel))
        );
        for position in 128..130 {
            let batch = reserve(&mut pool, sequence, position, position + 1);
            bind_gate_up_rows(&mut driver, &batch, true, true).unwrap();
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, 1)
                    .unwrap(),
                [if enabled { 724 } else { 652 }]
            );
            pool.begin_submission(&batch).unwrap();
            let start = driver.inner.transports[0].commands.len();
            let output = driver.execute_selected(&batch, &[0]).unwrap();
            let commands = &driver.inner.transports[0].commands[start..];
            assert_eq!(commands.len(), if enabled { 724 } else { 652 });
            let selected = commands
                .iter()
                .filter(|c| ROOTS.contains(&c.kernel))
                .collect::<Vec<_>>();
            assert_eq!(selected.len(), if enabled { 144 } else { 0 });
            for group in selected.chunks_exact(4) {
                assert_eq!(
                    group.iter().map(|c| c.kernel).collect::<Vec<_>>(),
                    [ROOTS[0], ROOTS[1], ROOTS[0], ROOTS[1]]
                );
                assert_eq!(group[0].arguments[7], EngineeringTpArgumentV1::U32(4));
                assert_eq!(group[2].arguments[7], EngineeringTpArgumentV1::U32(5));
            }
            pool.commit_batch(&batch, output.completion).unwrap();
        }
        let events = driver.inner.transports[0].events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::TokenSubmit(_, 649)))
                .count(),
            4
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::TokenSubmit(_, 724)))
                .count(),
            if enabled { 2 } else { 0 }
        );
        drop(events);
        driver.close().unwrap();
    }
}

#[test]
fn splitk_gate_up_native_selection_rejects_reselection_and_unbound_execution() {
    use crate::tp_artifact::SplitKGateUpBindingR1;
    let mut pool = token_pool();
    let mut driver = gate_up_configured(&pool, true);
    assert!(
        driver
            .configure_splitk_gate_up_binding(SplitKGateUpBindingR1::recording(), true)
            .is_err()
    );
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 32], 0)
        .unwrap()
        .sequence();
    let batch = reserve(&mut pool, sequence, 0, 32);
    let commands = driver.inner.transports[0].commands.len();
    assert!(driver.execute_selected(&batch, &[31]).is_err());
    assert_eq!(commands, driver.inner.transports[0].commands.len());
    bind_gate_up_rows(&mut driver, &batch, false, true).unwrap();
    assert!(bind_gate_up_rows(&mut driver, &batch, false, true).is_err());
    assert!(driver.execute_selected(&batch, &[31]).is_err());
}

#[test]
fn splitk_gate_up_commands_select_distinct_layer_tag_weights_and_reject_aliases() {
    let pool = token_pool();
    let mut driver = gate_up_configured(&pool, true);
    for layer in 0..2 {
        for (tag, kind) in [
            (4, Qwen3TensorKind::GateProjection),
            (5, Qwen3TensorKind::UpProjection),
        ] {
            let original = 1_000_000 + layer * 10 + tag;
            let kn = Tensor {
                id: original + 100,
                elements: 4096 * 12_288,
                element_bytes: 2,
            };
            for (k, tensor) in &mut driver.inner.ranks[0].layers[layer as usize].weights {
                if *k == kind {
                    *tensor = Tensor { id: original, ..kn };
                }
            }
            driver.projection.insert_synthetic_mfma_weight(original, kn);
            let workspace = driver.splitk_gate_up_r1.as_ref().unwrap();
            let pair = workspace
                .commands(
                    &driver.inner.ranks[0],
                    &driver.projection,
                    layer as u32,
                    tag as u32,
                )
                .unwrap();
            assert_eq!(pair[0].arguments[1], kn.read());
        }
    }
    let scratch = driver.splitk_gate_up_r1.as_ref().unwrap().scratch;
    for invalid in [
        Tensor {
            id: 1_000_004,
            ..scratch
        },
        Tensor {
            id: 1_000_104,
            ..scratch
        },
        driver.inner.ranks[0].normalized,
        Tensor {
            elements: scratch.elements - 1,
            ..scratch
        },
    ] {
        driver.splitk_gate_up_r1.as_mut().unwrap().scratch = invalid;
        assert!(
            driver
                .splitk_gate_up_r1
                .as_ref()
                .unwrap()
                .commands(&driver.inner.ranks[0], &driver.projection, 0, 4)
                .is_err()
        );
    }
    driver.splitk_gate_up_r1.as_mut().unwrap().scratch = scratch;
    driver.projection = super::super::super::projection::ProjectionPolicy::default();
    assert!(
        driver
            .splitk_gate_up_r1
            .as_ref()
            .unwrap()
            .commands(&driver.inner.ranks[0], &driver.projection, 0, 4)
            .is_err()
    );
    assert!(driver.inner.transports[0].commands.is_empty());
}

#[test]
fn splitk_gate_up_scheduler_output_mismatch_sends_no_packets() {
    let mut pool = token_pool();
    let mut driver = gate_up_configured(&pool, true);
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 32], 0)
        .unwrap()
        .sequence();
    let batch = reserve(&mut pool, sequence, 0, 32);
    bind_gate_up_rows(&mut driver, &batch, false, true).unwrap();
    assert!(driver.execute_selected(&batch, &[]).is_err());
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(bind_gate_up_rows(&mut driver, &batch, true, true).is_err());
    assert!(driver.inner.transports[0].commands.is_empty());
}

#[test]
fn splitk_gate_up_rejects_misaligned_prefill_before_metadata() {
    let mut pool = token_pool();
    let mut driver = gate_up_configured(&pool, true);
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 33], 0)
        .unwrap()
        .sequence();
    let first = reserve(&mut pool, sequence, 0, 1);
    pool.begin_submission(&first).unwrap();
    pool.commit_batch(
        &first,
        crate::tp_paged::EngineeringTpBatchCompletionV1::after_all_ranks(&first),
    )
    .unwrap();
    let batch = reserve(&mut pool, sequence, 1, 33);
    let writes = driver.inner.transports[0].write_payloads.len();
    assert!(bind_gate_up_rows(&mut driver, &batch, false, false).is_err());
    assert_eq!(writes, driver.inner.transports[0].write_payloads.len());
    assert!(driver.inner.transports[0].commands.is_empty());
}

#[test]
fn native_prefill649_matches_existing_page_pair_commands_and_preserves_decode() {
    let mut reference = None;
    for program in [false, true] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, program);
        driver.inner.transports[0].prefill_program_supported = program;
        driver.inner.transports[0].prefill32_program_supported = program;
        driver
            .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true)
            .unwrap();
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
        for first in (0..128).step_by(32) {
            let batch = reserve(&mut pool, sequence, first, first + 32);
            pool.begin_submission(&batch).unwrap();
            let selected = if first == 96 { &[31][..] } else { &[][..] };
            let output = driver.execute_selected(&batch, selected).unwrap();
            pool.commit_batch(&batch, output.completion).unwrap();
        }
        assert_eq!(driver.dispatch_counts(), [4 * 649 + 3]);
        let batch = reserve(&mut pool, sequence, 128, 129);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let observed = (
            driver.inner.transports[0].commands.clone(),
            driver.inner.transports[0].write_payloads.clone(),
            output.choices.clone(),
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
        );
        if let Some(expected) = &reference {
            assert_eq!(&observed, expected);
        } else {
            reference = Some(observed);
        }
        let events = driver.inner.transports[0].events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 649)))
                .count(),
            if program { 4 } else { 0 }
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 652)))
                .count(),
            usize::from(program)
        );
        drop(events);
        assert_eq!(driver.dispatch_counts(), [4 * 649 + 3 + 652]);
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn native_prefill649_failure_preserves_committed_state() {
    for failure in [Failure::OrderedSubmit, Failure::OrderedWait] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = true;
        driver.inner.transports[0].prefill32_program_supported = true;
        driver
            .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true)
            .unwrap();
        let initial = driver.inner.collective;
        driver.inner.transports[0].failure = Some(failure);
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 32], 0)
            .unwrap()
            .sequence();
        let batch = reserve(&mut pool, sequence, 0, 32);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[31]).is_err());
        assert_eq!(driver.dispatch_counts(), [0]);
        assert_eq!(driver.completed_batches(), 0);
        assert_eq!(driver.inner.collective, initial);
        assert!(driver.poisoned && driver.inner.packed_c1.is_none());
    }
}

#[test]
fn native_prefill649_profile_drift_refuses_before_metadata() {
    for mutation in 0..6 {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = true;
        driver.inner.transports[0].prefill32_program_supported = true;
        driver
            .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true)
            .unwrap();
        match mutation {
            0 => driver.c1_kv_copy_v19 = None,
            1 => driver.prefill32_pages_v1 = Some(false),
            2 => driver.prefill_kv_copy_v28 = Some(false),
            3 => driver.partial_gemv_v28 = Some(true),
            4 => driver.prefill16_ordered_v1 = Some(false),
            _ => driver.prune_output_head = false,
        }
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 32], 0)
            .unwrap()
            .sequence();
        let batch = reserve(&mut pool, sequence, 0, 32);
        let writes = driver.inner.transports[0].write_payloads.len();
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[31]).is_err());
        assert_eq!(driver.dispatch_counts(), [0]);
        assert_eq!(driver.inner.transports[0].write_payloads.len(), writes);
    }
}

#[test]
fn native_prefill613_preserves_all_commands_buffers_outputs_and_decode() {
    let mut reference = None;
    for enabled in [false, true] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = enabled;
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
        prefill(&mut driver, &mut pool, sequence, 0, 128);
        let transport = &driver.inner.transports[0];
        assert_eq!(driver.dispatch_counts(), [4907]);
        let events = transport.events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 613)))
                .count(),
            if enabled { 8 } else { 0 }
        );
        drop(events);
        let batch = reserve(&mut pool, sequence, 128, 129);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let observed = (
            transport.commands.clone(),
            transport.write_payloads.clone(),
            transport.reads.clone(),
            output.choices.clone(),
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
        );
        if let Some(expected) = &reference {
            assert_eq!(&observed, expected);
        } else {
            reference = Some(observed);
        }
        assert_eq!(driver.dispatch_counts(), [5559]);
        assert_eq!(
            transport
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 652)))
                .count(),
            1
        );
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn native_prefill613_failure_does_not_commit_collective_or_batch() {
    for failure in [Failure::OrderedSubmit, Failure::OrderedWait] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = true;
        driver.inner.transports[0].failure = Some(failure);
        let initial = driver.inner.collective;
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 16], 0)
            .unwrap()
            .sequence();
        let batch = reserve(&mut pool, sequence, 0, 16);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[15]).is_err());
        assert_eq!(driver.dispatch_counts(), [0]);
        assert_eq!(driver.completed_batches(), 0);
        assert_eq!(driver.inner.collective, initial);
        assert!(driver.poisoned && driver.inner.packed_c1.is_none());
        assert!(driver.execute_selected(&batch, &[15]).is_err());
    }
}

#[test]
fn native_prefill613_valid_ineligible_batches_keep_ordinary_fallback() {
    for rows in [1, 15, 17, 32] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = true;
        let sequence = pool
            .open_sequence(pool.scope(), &vec![1; rows], 0)
            .unwrap()
            .sequence();
        let batch = reserve(&mut pool, sequence, 0, rows as u32);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[rows - 1]).unwrap();
        assert!(
            !driver.inner.transports[0]
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::TokenSubmit(_, 613)))
        );
        pool.commit_batch(&batch, output.completion).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn native_prefill613_profile_drift_rejects_before_metadata_or_publication() {
    for mutation in 0..5 {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        driver.inner.transports[0].prefill_program_supported = true;
        match mutation {
            0 => driver.c1_kv_copy_v19 = None,
            1 => driver.admitted_c1_kv_copy_v19 = None,
            2 => driver.prefill_kv_copy_v28 = Some(false),
            3 => driver.prefill16_ordered_v1 = Some(true),
            _ => driver.prune_output_head = false,
        }
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 16], 0)
            .unwrap()
            .sequence();
        let batch = reserve(&mut pool, sequence, 0, 16);
        let writes = driver.inner.transports[0].write_payloads.len();
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[15]).is_err());
        assert_eq!(driver.dispatch_counts(), [0]);
        assert_eq!(driver.inner.transports[0].write_payloads.len(), writes);
    }
}

fn reserve(
    pool: &mut EngineeringTpPagedPoolV1,
    sequence: EngineeringTpSequenceIdV1,
    first: u32,
    end: u32,
) -> EngineeringTpPreparedBatchV1 {
    let rows = (first..end)
        .map(|position| EngineeringTpPageRowV1 {
            sequence,
            token: 1,
            position,
        })
        .collect::<Vec<_>>();
    pool.reserve_batch(&rows).unwrap()
}

fn prefill(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    pool: &mut EngineeringTpPagedPoolV1,
    sequence: EngineeringTpSequenceIdV1,
    first: u32,
    end: u32,
) {
    // Every prefix row executes the actual driver before its pool completion commits.
    for start in (first..end).step_by(16) {
        let stop = (start + 16).min(end);
        let rows = (start..stop)
            .map(|position| EngineeringTpPageRowV1 {
                sequence,
                token: 1,
                position,
            })
            .collect::<Vec<_>>();
        let batch = pool.reserve_batch(&rows).unwrap();
        pool.begin_submission(&batch).unwrap();
        let selected = if stop == end {
            vec![rows.len() - 1]
        } else {
            Vec::new()
        };
        let output = driver.execute_selected(&batch, &selected).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
    }
}

#[test]
fn full_token_matches_default_commands_and_commits_only_one_aggregate() {
    let mut reference = None;
    for selected in [false, true] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, selected);
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
        prefill(&mut driver, &mut pool, sequence, 0, 128);
        assert!(
            !driver.inner.transports[0]
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::TokenSubmit(..)))
        );
        driver.inner.transports[0].events.borrow_mut().clear();
        let batch = reserve(&mut pool, sequence, 128, 129);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let actual = (
            transport.commands.clone(),
            transport.write_payloads.clone(),
            transport.reads.clone(),
            output.choices.clone(),
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            driver.dispatch_counts(),
            driver.completed_batches,
        );
        if let Some(expected) = &reference {
            assert_eq!(&actual, expected);
        } else {
            reference = Some(actual);
        }
        let events = transport.events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 652)))
                .count(),
            usize::from(selected)
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenWait(_, 652)))
                .count(),
            usize::from(selected)
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::OrderedSubmit(..)))
                .count(),
            if selected { 0 } else { 11 }
        );
        drop(events);
        pool.commit_batch(&batch, output.completion).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn aggregate_submit_or_completion_failure_never_commits_output_or_host_cursor() {
    for failure in [Failure::OrderedSubmit, Failure::OrderedWait] {
        let mut pool = token_pool();
        let mut driver = configured(&pool, true);
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
        prefill(&mut driver, &mut pool, sequence, 0, 128);
        driver.inner.transports[0].failure = Some(failure);
        let writes_before = driver.inner.transports[0].write_payloads.len();
        let reads_before = driver.inner.transports[0].reads.len();
        let before = (
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            driver.dispatch_counts(),
            driver.completed_batches,
        );
        let batch = reserve(&mut pool, sequence, 128, 129);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(
            (
                driver.inner.collective,
                driver.inner.ranks[0].hidden.id,
                driver.dispatch_counts(),
                driver.completed_batches
            ),
            before
        );
        assert!(driver.poisoned && driver.inner.packed_c1.is_none());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert_eq!(driver.inner.transports[0].reads.len(), reads_before);
        // Five metadata writes already happened. Their effects are not rolled back.
        assert_eq!(
            driver.inner.transports[0].write_payloads.len(),
            writes_before + 5
        );
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        driver.inner.transports[0].failure = None;
        driver.close().unwrap();
    }
}

#[test]
fn ordinary_bootstrap_and_fallback_cover_127_to_128_and_256_to_257() {
    let mut pool = token_pool();
    let mut driver = configured(&pool, true);
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 257], 0)
        .unwrap()
        .sequence();
    prefill(&mut driver, &mut pool, sequence, 0, 126);
    for (position, token) in [(126, false), (127, true), (255, true), (256, false)] {
        if position == 255 {
            prefill(&mut driver, &mut pool, sequence, 128, 255);
        }
        let transport = &driver.inner.transports[0];
        transport.events.borrow_mut().clear();
        let commands_before = transport.commands.len();
        let batch = reserve(&mut pool, sequence, position, position + 1);
        let expected = driver
            .expected_dispatch_counts_for_batch(&batch, 1)
            .unwrap()[0];
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let events = transport.events.borrow();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenSubmit(_, 652)))
                .count(),
            usize::from(token)
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::TokenWait(_, 652)))
                .count(),
            usize::from(token)
        );
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, Event::OrderedSubmit(..))),
            !token
        );
        assert_eq!(
            transport.commands.len() - commands_before,
            usize::try_from(expected).unwrap()
        );
        assert_eq!(expected, if token { 652 } else { 616 });
        drop(events);
        pool.commit_batch(&batch, output.completion).unwrap();
    }
    assert_eq!(pool.committed_position(sequence).unwrap(), 257);
    pool.check_invariants().unwrap();
    driver.close().unwrap();
}

#[test]
fn token_capable_transport_cannot_select_narrow_packed_policy() {
    let mut pool = token_pool();
    let mut driver = configured(&pool, true);
    driver.inner.ordered_batch_width = OrderedBatchWidth::Packets16;
    let sequence = pool
        .open_sequence(pool.scope(), &[1], 0)
        .unwrap()
        .sequence();
    let batch = reserve(&mut pool, sequence, 0, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert!(driver.inner.packed_c1.is_none());
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(
        !driver.inner.transports[0]
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::TokenSubmit(..)))
    );
    driver.close().unwrap();
}

#[test]
#[ignore = "explicit FERRIC_TOKEN_ABI_GRAPH_OUTPUT; CPU recording capture, no GPU"]
fn capture_actual_driver_652_graphs_across_physical_page_boundary() {
    capture_driver_graphs(false);
}

#[test]
#[ignore = "explicit FERRIC_TOKEN_ABI_GRAPH_OUTPUT; CPU prefill recording capture, no GPU"]
fn capture_actual_driver_613_graphs_across_pages_and_final_rotation() {
    capture_driver_graphs(true);
}

fn capture_driver_graphs(prefill_program: bool) {
    capture_driver_graphs_width(if prefill_program { 16 } else { 0 });
}

#[test]
#[ignore = "requires fresh FERRIC_TOKEN_ABI_GRAPH_OUTPUT; recording transport, CPU only"]
fn capture_actual_driver_prefill649_graphs() {
    capture_driver_graphs_width(32);
}

fn capture_driver_graphs_width(width: u32) {
    let prefill_program = width != 0;
    use abi_fixture::{Access, Argument, Dispatch, Graph, Snapshot};
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let path =
        std::env::var_os("FERRIC_TOKEN_ABI_GRAPH_OUTPUT").expect("fresh explicit fixture output");
    let path = std::path::Path::new(&path);
    assert!(path.is_absolute() && !path.exists());
    assert_eq!(
        path.parent().unwrap().canonicalize().unwrap(),
        path.parent().unwrap()
    );
    let mut pool = token_pool();
    let mut driver = configured(&pool, true);
    driver.inner.transports[0].prefill_program_supported = prefill_program;
    if width == 32 {
        driver.inner.transports[0].prefill32_program_supported = true;
        driver
            .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true)
            .unwrap();
    }
    let sequence = pool
        .open_sequence(pool.scope(), &[1; 145], 0)
        .unwrap()
        .sequence();
    if !prefill_program {
        prefill(&mut driver, &mut pool, sequence, 0, 143);
    }
    let mut graphs = Vec::new();
    let mut capacities = None;
    let mut pages = Vec::new();
    let positions = if prefill_program {
        (0..128).step_by(width as usize).collect::<Vec<_>>()
    } else {
        vec![143, 144]
    };
    for position in positions {
        driver.inner.transports[0].events.borrow_mut().clear();
        let start = driver.inner.transports[0].commands.len();
        let batch = reserve(
            &mut pool,
            sequence,
            position,
            position + if prefill_program { width } else { 1 },
        );
        pool.begin_submission(&batch).unwrap();
        let selected: &[usize] = if !prefill_program {
            &[0]
        } else if position + width == 128 {
            if width == 32 { &[31] } else { &[15] }
        } else {
            &[]
        };
        let output = driver.execute_selected(&batch, selected).unwrap();
        let transport = &driver.inner.transports[0];
        let commands = &transport.commands[start..];
        let count = if width == 32 {
            649
        } else if prefill_program {
            613
        } else {
            652
        };
        assert_eq!(
            commands.len(),
            count
                + if prefill_program && position + width == 128 {
                    3
                } else {
                    0
                }
        );
        let commands = &commands[..count];
        assert_eq!(
            transport
                .events
                .borrow()
                .iter()
                .filter(
                    |event| matches!(event, Event::TokenSubmit(_, packets) if *packets == count)
                )
                .count(),
            1
        );
        let buffers = transport
            .buffers
            .iter()
            .map(|(&id, data)| (id, data.len()))
            .collect::<Vec<_>>();
        if let Some(previous) = &capacities {
            assert_eq!(previous, &buffers);
        } else {
            capacities = Some(buffers);
        }
        let copies = commands
            .iter()
            .filter(|command| {
                command.kernel
                    == if prefill_program {
                        crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0]
                    } else {
                        crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0]
                    }
            })
            .collect::<Vec<_>>();
        assert_eq!(copies.len(), if width == 32 { 72 } else { 36 });
        let page = scalar(copies[0], 5);
        if width == 32 {
            let terminal = position + width == 128;
            for pair in copies.chunks_exact(2) {
                assert_eq!(scalar(pair[0], 4), position + if terminal { 16 } else { 0 });
                assert_eq!(scalar(pair[1], 4), position + if terminal { 0 } else { 16 });
                assert_ne!(scalar(pair[0], 5), scalar(pair[1], 5));
                assert_eq!(scalar(pair[0], 7), u32::from(terminal));
                assert_eq!(scalar(pair[1], 7), 0);
            }
        } else {
            assert!(
                copies
                    .iter()
                    .all(|command| scalar(command, 4) == position && scalar(command, 5) == page)
            );
        }
        pages.push(page);
        if !prefill_program || [0, width, 128 - width].contains(&position) {
            graphs.push(Graph {
                position,
                commands: commands
                    .iter()
                    .map(|command| Dispatch {
                        kernel: command.kernel.into(),
                        grid_workgroups: command.grid_workgroups,
                        workgroup_size: command.workgroup_size,
                        arguments: command
                            .arguments
                            .iter()
                            .map(|argument| match *argument {
                                EngineeringTpArgumentV1::Buffer {
                                    id,
                                    offset,
                                    elements,
                                    element_bytes,
                                    access,
                                } => Argument::Buffer {
                                    id,
                                    offset,
                                    elements,
                                    element_bytes,
                                    access: match access {
                                        EngineeringTpBufferAccessV1::Read => Access::Read,
                                        EngineeringTpBufferAccessV1::Write => Access::Write,
                                        EngineeringTpBufferAccessV1::ReadWrite => Access::ReadWrite,
                                    },
                                },
                                EngineeringTpArgumentV1::U32(value) => Argument::U32 { value },
                                EngineeringTpArgumentV1::F32(value) => Argument::F32 {
                                    bits: value.to_bits(),
                                },
                            })
                            .collect(),
                    })
                    .collect(),
            });
        }
        pool.commit_batch(&batch, output.completion).unwrap();
    }
    assert_ne!(pages[0], pages[1]);
    assert_eq!(
        pool.committed_position(sequence).unwrap(),
        if prefill_program { 128 } else { 145 }
    );
    pool.check_invariants().unwrap();
    driver.close().unwrap();
    let snapshot = Snapshot {
        schema: abi_fixture::SCHEMA.into(),
        buffers: capacities.unwrap(),
        graphs,
    };
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    assert!(bytes.len() <= abi_fixture::MAX_BYTES);
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    output.write_all(&bytes).unwrap();
    output.sync_all().unwrap();
}
