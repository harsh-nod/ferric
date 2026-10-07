//! Same-image routing and lifecycle checks; synthetic outputs are not numerical parity.

use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, QueryHoistBindingV14, WaveRmsNormBindingV15};

const MODES: [EngineeringTpWaveTargetModeV17; 4] = [
    EngineeringTpWaveTargetModeV17::Baseline,
    EngineeringTpWaveTargetModeV17::QueryHoist,
    EngineeringTpWaveTargetModeV17::RmsNorm,
    EngineeringTpWaveTargetModeV17::Combined,
];
const V14: &str = crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0];
const V15: &str = crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0];
const WAVE: &str = "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5";

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::wave_rmsnorm_v15::configured(pool);
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver
}

fn select(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    mode: EngineeringTpWaveTargetModeV17,
) -> TpResult<()> {
    driver.configure_ordered_c1_wave_target_bindings_v17(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        mode,
    )
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect()
}

#[test]
fn wave_target_v17_changes_only_selected_roots_and_preserves_all_dispatch_dependencies() {
    for rows in [1, 16, 17, 32] {
        for selected in [
            Vec::new(),
            vec![rows as usize - 1],
            (0..rows as usize).collect(),
        ] {
            let mut baseline = None;
            for mode in MODES {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                let before = allocations(&driver);
                select(&mut driver, mode).unwrap();
                assert_eq!(driver.attention_mode(), mode.attention());
                assert_eq!(driver.rmsnorm_mode(), mode.rmsnorm());
                assert_eq!(driver.layer_projection_mode(), "c1-wave");
                assert_eq!(driver.fp32_argmax_mode(), "wave-v11");
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                let transport = &driver.inner.transports[0];
                let mut commands = transport.commands.clone();
                let mut attention_count = 0;
                let mut norm_count = 0;
                for command in &mut commands {
                    if matches!(command.kernel, V14 | WAVE) {
                        assert_eq!(command.kernel == V14, mode.attention() == "query-hoist-v14");
                        attention_count += 1;
                        command.kernel = WAVE;
                    } else if command.kernel == V15
                        || command.kernel == RMSNORM && scalar(command, 6) == 4096
                    {
                        assert_eq!(command.kernel == V15, mode.rmsnorm() == "wave-v15");
                        norm_count += 1;
                        command.kernel = RMSNORM;
                    }
                }
                assert_eq!(attention_count, 36);
                assert_eq!(norm_count, 72 + usize::from(!selected.is_empty()));
                let mut events = transport.events.borrow().clone();
                for event in &mut events {
                    if let Event::Submit(_, kernel) | Event::Wait(_, kernel) = event {
                        if *kernel == V14 {
                            *kernel = WAVE;
                        }
                        if *kernel == V15 {
                            *kernel = RMSNORM;
                        }
                    }
                }
                let recorded = (
                    commands,
                    transport.reads.clone(),
                    transport.write_payloads.clone(),
                    transport.packet_preparations.clone(),
                    events,
                    output.choices.clone(),
                );
                if let Some(expected) = &baseline {
                    assert_eq!(
                        &recorded, expected,
                        "{mode:?}, rows {rows}, selected {selected:?}"
                    );
                } else {
                    baseline = Some(recorded);
                }
                assert_eq!(allocations(&driver), before);
                assert!(transport.pending.is_none() && transport.pending_ordered.is_none());
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
        }
    }
}

#[test]
fn wave_target_v17_rejects_every_missing_binding_and_incompatible_profile_atomically() {
    for mode in MODES {
        for mutation in 0..34 {
            let mut driver = configured(&wide_pool());
            match mutation {
                0 => driver.admitted_query_hoist_v14 = None,
                1 => {
                    let mut b = QueryHoistBindingV14::recording();
                    b.hsaco[0] ^= 1;
                    driver.admitted_query_hoist_v14 = Some(b);
                }
                2 => driver.admitted_wave_rmsnorm_v15 = None,
                3 => {
                    let mut b = WaveRmsNormBindingV15::recording();
                    b.hsaco[0] ^= 1;
                    driver.admitted_wave_rmsnorm_v15 = Some(b);
                }
                4 => driver.admitted_argmax_v11 = None,
                5 => {
                    let mut b = Fp32ArgmaxBindingV11::recording();
                    b.hsaco[0] ^= 1;
                    driver.admitted_argmax_v11 = Some(b);
                }
                6 => driver.query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
                7 => driver.wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording()),
                8 => driver.row_capacity = 16,
                9 => driver.inner.draft_v10 = true,
                10 => driver.fp32_logits = None,
                11 => driver.head_profile_configured = false,
                12 => driver.inner.large_kv = true,
                13 => driver.inner.sequences = Some(vec![Vec::new()]),
                14 => driver.inner.ordered_batches = Some(Vec::new()),
                15 => driver.wave_attention = false,
                16 => driver.prune_output_head = false,
                17 => {
                    driver.projection.mode =
                        super::super::super::EngineeringTpProjectionModeV3::Baseline;
                }
                18 => {
                    driver.projection.mode =
                        super::super::super::EngineeringTpProjectionModeV3::Wave;
                }
                19 => driver.last_batch = 1,
                20 => driver.completed_batches = 1,
                21 => driver.poisoned = true,
                22 => driver.inner.closed = true,
                23 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
                24 => driver.inner.transports.clear(),
                25 => driver.inner.ranks.clear(),
                26 => {
                    driver.inner.reduction =
                        super::super::super::reduction::ReductionWorkspace::Baseline;
                }
                27 => driver.inner.transports[0].ordered_supported = false,
                28 => driver
                    .inner
                    .ranks
                    .push(fixture(1, &wide_pool()).inner.ranks.pop().unwrap()),
                29 => driver
                    .inner
                    .transports
                    .push(fixture(1, &wide_pool()).inner.transports.pop().unwrap()),
                30 => driver.c1_wave_layers = true,
                31 => driver.fp32_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
                32 => {
                    driver.projection.mode =
                        super::super::super::EngineeringTpProjectionModeV3::Auto;
                }
                33 => driver.row_capacity = 1,
                _ => unreachable!(),
            }
            let state = |driver: &EngineeringTpBatchExecutionV2<Recording>| {
                (
                    driver.query_hoist_v14,
                    driver.wave_rmsnorm_v15,
                    driver.c1_wave_layers,
                    driver.fp32_argmax_v11,
                    driver.inner.ordered_batches.clone(),
                    driver
                        .inner
                        .transports
                        .iter()
                        .map(|t| {
                            (
                                t.buffers.len(),
                                t.commands.clone(),
                                t.reads.clone(),
                                t.writes.clone(),
                                t.events.borrow().clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            };
            let before = state(&driver);
            assert!(
                select(&mut driver, mode).is_err(),
                "{mode:?}, mutation {mutation}"
            );
            assert_eq!(state(&driver), before);
        }
    }
}

#[test]
fn wave_target_v17_selection_is_explicit_terminal_and_cannot_broaden_old_selectors() {
    for mode in MODES {
        let mut driver = configured(&wide_pool());
        assert_eq!(driver.attention_mode(), "wave");
        assert_eq!(driver.rmsnorm_mode(), "baseline");
        assert_eq!(driver.layer_projection_mode(), "mfma");
        assert!(
            driver
                .configure_ordered_c1_wave_rmsnorm_binding_v15(
                    Fp32ArgmaxBindingV11::recording(),
                    WaveRmsNormBindingV15::recording()
                )
                .is_err()
        );
        select(&mut driver, mode).unwrap();
        for other in MODES {
            assert!(select(&mut driver, other).is_err());
        }
        assert!(
            driver
                .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(
                    Fp32ArgmaxBindingV11::recording()
                )
                .is_err()
        );
        assert!(
            driver
                .configure_ordered_c1_wave_query_hoist_binding_v14(
                    Fp32ArgmaxBindingV11::recording(),
                    QueryHoistBindingV14::recording()
                )
                .is_err()
        );
        assert_eq!(driver.attention_mode(), mode.attention());
        assert_eq!(driver.rmsnorm_mode(), mode.rmsnorm());
        driver.close().unwrap();
    }
}

#[test]
fn wave_target_v17_failures_never_commit_and_quarantine_touched_pages() {
    for failure in [
        Failure::AttentionSubmit,
        Failure::AttentionWait,
        Failure::RmsNormSubmit(0),
        Failure::RmsNormWait(0),
        Failure::RmsNormSubmit(1),
        Failure::RmsNormWait(1),
        Failure::RmsNormSubmit(72),
        Failure::RmsNormWait(72),
        Failure::OrderedSubmit,
        Failure::OrderedWait,
        Failure::ArgmaxSubmit,
        Failure::ArgmaxWait,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, EngineeringTpWaveTargetModeV17::Combined).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(
            driver.execute_selected(&batch, &[0]).is_err(),
            "{failure:?}"
        );
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending.is_none());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert_eq!(pool.stats().quarantined_pages, 4);
        driver.close().unwrap();
    }
}
