//! Host command-order evidence only; no dependent-packet GPU qualification.

use super::*;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver.configure_output_head_pruning(true).unwrap();
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.configure_head_precision_v8(true).unwrap();
    driver
}

#[test]
fn ordered_groups_preserve_every_bound_command_and_host_io() {
    for rows in [1, 16, 17, 32] {
        for selected in [Vec::new(), vec![rows - 1], (0..rows).collect()] {
            let mut recordings = Vec::new();
            for enabled in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                driver.configure_ordered_batches(enabled).unwrap();
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let selected = selected.iter().map(|&row| row as usize).collect::<Vec<_>>();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(
                    driver.dispatch_counts(),
                    vec![if selected.is_empty() { 613 } else { 616 }]
                );
                let transport = &driver.inner.transports[0];
                let groups = transport
                    .events
                    .borrow()
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(0, count) => Some(*count),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    groups,
                    if enabled {
                        [10, 5].repeat(36)
                    } else {
                        Vec::new()
                    }
                );
                assert!(transport.pending_ordered.is_none());
                assert!(
                    driver
                        .inner
                        .ordered_batches
                        .as_ref()
                        .is_none_or(Vec::is_empty)
                );
                recordings.push((
                    transport.commands.clone(),
                    transport.reads.clone(),
                    transport.writes.clone(),
                    result.choices.clone(),
                ));
                pool.commit_batch(&batch, result.completion).unwrap();
                assert!(driver.configure_ordered_batches(false).is_err());
                driver.close().unwrap();
            }
            assert_eq!(recordings[0], recordings[1]);
        }
    }
}

#[test]
fn ordered_failures_cannot_complete_or_resume_the_batch() {
    for failure in [Failure::OrderedSubmit, Failure::OrderedWait] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        driver.configure_ordered_batches(true).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 17);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        assert_eq!(driver.completed_batches, 0);
        assert_eq!(driver.dispatch_counts(), vec![1]);
        assert!(driver.poisoned);
        assert!(driver.execute(&batch).is_err());
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        driver.close().unwrap();
        assert!(driver.inner.closed);
    }
}

#[test]
fn ordered_configuration_is_explicit_and_freezes_incompatible_changes() {
    let pool = wide_pool();
    let mut incomplete = fixture(1, &pool);
    assert!(incomplete.configure_ordered_batches(true).is_err());
    let mut unsupported = configured(&pool);
    unsupported.inner.transports[0].ordered_supported = false;
    assert!(unsupported.configure_ordered_batches(true).is_err());
    let mut wave = fixture(1, &pool);
    wave.configure_wave_attention(true).unwrap();
    wave.configure_head_precision_v8(true).unwrap();
    assert!(wave.configure_ordered_batches(true).is_err());
    for legacy_sequence in [false, true] {
        let mut driver = configured(&pool);
        if legacy_sequence {
            driver.inner.sequences = Some(vec![Vec::new()]);
            assert!(driver.configure_ordered_batches(true).is_err());
        } else {
            driver.configure_ordered_batches(true).unwrap();
            assert!(driver.configure_dispatch_sequences(true).is_err());
            assert!(driver.configure_dispatch_sequences(false).is_err());
            assert!(driver.configure_ordered_batches(false).is_err());
            assert!(driver.configure_ordered_batches(true).is_err());
            assert!(driver.configure_wave_attention(true).is_err());
            assert!(driver.configure_output_head_pruning(false).is_err());
            assert!(
                driver
                    .configure_reduction(EngineeringTpReductionModeV3::HostStagedV1)
                    .is_err()
            );
            assert!(driver.configure_head_precision_v7(true).is_err());
            assert!(driver.configure_head_precision_v8(false).is_err());
        }
    }
    let mut large = configured(&pool);
    large.inner.large_kv = true;
    assert!(large.configure_ordered_batches(true).is_err());
}
