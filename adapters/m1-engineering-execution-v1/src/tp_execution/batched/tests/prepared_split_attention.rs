//! Split graph recording and lifecycle tests, with no device backing or execution.
use super::*;
use crate::tp_execution::{
    EngineeringTp2GraphGeometryV1 as Geometry, EngineeringTp2GraphKernelProfileV1 as Kernels,
    EngineeringTp2GraphPolicyV1 as Policy,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn split_pool(geometry: Geometry) -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(geometry.context_tokens(), 32, geometry.pages(), 100)
            .unwrap(),
    )
    .unwrap()
}

fn ordinary(program: &Program, index: usize, expected_rank: u32) -> &EngineeringTpDispatchV1 {
    let Step::Rank { rank, dispatch } = &program.steps[index] else {
        panic!("rank step")
    };
    assert_eq!(*rank, expected_rank);
    dispatch
}

#[test]
fn split_recording_preserves_every_other_step_and_all_resident_layouts() {
    let roots = crate::tp_artifact::ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1;
    for geometry in Geometry::ALL {
        let mut driver = builder_for_pool(&split_pool(geometry));
        let control = driver
            .record_graph_geometry_fixture(Kernels::WaveStackNormAttentionKvMlp, geometry)
            .unwrap();
        let catalog = driver.inner.transports[0].catalog.clone();
        let count = catalog.borrow().allocations.len();
        let transpose_bytes = driver.projection.bytes;
        let candidate = driver.record_split_geometry_fixture(geometry).unwrap();
        assert_eq!(candidate.steps.len(), 1085);
        assert_eq!(catalog.borrow().allocations.len(), count + 2);
        assert_eq!(driver.projection.bytes, transpose_bytes);
        validate_extents(&candidate, &catalog.borrow().allocations);
        let scratch = driver.split_attention_scratch.unwrap();
        for (rank, tensor) in scratch.iter().enumerate() {
            let allocation = catalog.borrow().allocations[count + rank];
            assert_eq!(allocation.id, tensor.id);
            assert_eq!(allocation.rank, u32::try_from(rank).unwrap());
            assert!(!allocation.peer_readable);
            assert_eq!(
                allocation.bytes,
                usize::try_from(geometry.attention_scratch_values() * 4).unwrap()
            );
        }
        let mut restored = candidate.clone();
        for layer in (0..36).rev() {
            let first = 2 + 30 * layer + 16;
            let old = 2 + 28 * layer + 16;
            for (rank, tensor) in scratch.iter().enumerate() {
                let rank_id = u32::try_from(rank).unwrap();
                let partial = ordinary(&candidate, first + rank, rank_id);
                let merge = ordinary(&candidate, first + 2 + rank, rank_id);
                let original = ordinary(&control, old + rank, rank_id);
                assert_eq!(partial.kernel, roots[0]);
                assert_eq!(merge.kernel, roots[1]);
                assert_eq!(partial.grid_workgroups, 16 * geometry.attention_splits());
                assert_eq!(merge.grid_workgroups, 16);
                assert_eq!(partial.arguments[..5], original.arguments[..5]);
                assert_eq!(partial.arguments[5], tensor.write());
                assert_eq!(partial.arguments[6..], original.arguments[6..]);
                assert_eq!(
                    merge.arguments,
                    vec![
                        tensor.read(),
                        original.arguments[3].clone(),
                        original.arguments[5].clone(),
                        EngineeringTpArgumentV1::U32(1),
                        EngineeringTpArgumentV1::U32(2),
                        EngineeringTpArgumentV1::U32(geometry.pages()),
                        EngineeringTpArgumentV1::U32(1)
                    ]
                );
            }
            restored.steps.splice(
                first..first + 4,
                control.steps[old..old + 2].iter().cloned(),
            );
        }
        assert_eq!(restored, control);
        assert_eq!(
            driver.record_split_geometry_fixture(geometry).unwrap(),
            candidate
        );
        assert_eq!(catalog.borrow().allocations.len(), count + 2);
        assert_eq!(driver.completed_batches, 0);
        assert_eq!(driver.dispatch_counts(), [0, 0]);
    }
}

#[test]
fn split_transform_rejects_role_root_extent_and_context_drift() {
    for geometry in Geometry::ALL {
        let mut driver = builder_for_pool(&split_pool(geometry));
        driver.record_split_geometry_fixture(geometry).unwrap();
        let control = driver
            .record_graph_geometry_fixture(Kernels::WaveStackNormAttentionKvMlp, geometry)
            .unwrap();
        for mutation in 0..6 {
            let mut changed = control.clone();
            let Step::Rank { rank, dispatch } = &mut changed.steps[18] else {
                panic!()
            };
            match mutation {
                0 => *rank = 1,
                1 => dispatch.kernel = "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2",
                2 => dispatch.grid_workgroups = 1,
                3 => dispatch.arguments[10] = EngineeringTpArgumentV1::U32(2),
                4 => dispatch.arguments.swap(1, 2),
                _ => {
                    dispatch.arguments.pop();
                }
            }
            let saved = changed.clone();
            assert!(
                driver
                    .record_split_attention(&mut changed, geometry)
                    .is_err()
            );
            assert_eq!(changed, saved);
        }
        let mut wrong_scratch = control.clone();
        driver.split_attention_scratch.as_mut().unwrap()[1].elements -= 1;
        assert!(
            driver
                .record_split_attention(&mut wrong_scratch, geometry)
                .is_err()
        );
        assert_eq!(wrong_scratch, control);
    }
}

#[test]
fn split_allocation_and_registration_error_or_unwind_is_terminal() {
    for geometry in Geometry::ALL {
        for policy in Policy::PRE_FINITE {
            for stage in 0..3 {
                for unwind in [false, true] {
                    let mut driver = builder_for_pool(&split_pool(geometry));
                    let catalog = driver.inner.transports[0].catalog.clone();
                    let count = catalog.borrow().allocations.len();
                    if stage < 2 {
                        catalog.borrow_mut().allocation_fault = Some((count + stage, unwind));
                    } else {
                        catalog.borrow_mut().registration_fault = Some(unwind);
                    }
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        driver.configure_split_geometry_fixture(policy, geometry)
                    }));
                    if unwind {
                        assert!(result.is_err());
                    } else {
                        assert!(result.unwrap().is_err());
                    }
                    assert!(driver.poisoned);
                    assert!(driver.prepared_peer.is_none());
                    assert_eq!(driver.completed_batches, 0);
                    assert_eq!(driver.dispatch_counts(), [0, 0]);
                    assert!(
                        driver
                            .configure_split_geometry_fixture(policy, geometry)
                            .is_err()
                    );
                    assert!(driver.configure_prepared_peer_graph(policy).is_err());
                    driver.inner.close().unwrap();
                }
            }
        }
    }
}

#[test]
fn split_registration_is_once_and_long_ring_bound_uses_selected_counts() {
    for geometry in Geometry::ALL {
        for policy in Policy::PRE_FINITE {
            let mut driver = builder_for_pool(&split_pool(geometry));
            driver
                .configure_split_geometry_fixture(policy, geometry)
                .unwrap();
            assert_eq!(
                driver.inner.transports[0]
                    .catalog
                    .borrow()
                    .registered
                    .as_ref()
                    .unwrap()
                    .steps
                    .len(),
                1085
            );
            assert!(!driver.poisoned);
            assert_eq!(driver.expected_dispatch_counts(0), [652, 649]);
            assert_eq!(driver.expected_dispatch_counts(1), [652, 649]);
            assert!(
                driver
                    .configure_split_geometry_fixture(policy, geometry)
                    .is_err()
            );
            assert_eq!(driver.packet_budget_for_batch(1, 795, false).unwrap(), 795);
            assert_eq!(
                driver.packet_budget_for_batch(2303, 795, false).is_ok(),
                geometry == Geometry::Long2304
            );
            assert_eq!(driver.completed_batches, 0);
            assert_eq!(driver.dispatch_counts(), [0, 0]);
        }
    }
}

#[test]
fn split_execution_error_preserves_all_cursors_and_disallows_retry() {
    for geometry in Geometry::ALL {
        let mut pool = split_pool(geometry);
        let mut driver = builder_for_pool(&pool);
        driver
            .configure_split_geometry_fixture(Policy::TransactionFences, geometry)
            .unwrap();
        let cursor = driver.inner.collective.expected();
        let hidden = driver.inner.hidden.clone();
        let registered = fixture_json(&driver);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert!(driver.poisoned && driver.inner.closed);
        assert_eq!(driver.inner.collective.expected(), cursor);
        assert_eq!(driver.inner.hidden, hidden);
        assert_eq!(driver.dispatch_counts(), [0, 0]);
        assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
        assert_eq!(fixture_json(&driver), registered);
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert!(
            driver
                .configure_split_geometry_fixture(Policy::TransactionFences, geometry)
                .is_err()
        );
    }
}
