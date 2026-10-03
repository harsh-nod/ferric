//! Actual recorder geometry and allocation metadata, without backing weights or GPU work.
use super::*;
use crate::tp_execution::{
    EngineeringTp2GraphGeometryV1 as Geometry, EngineeringTp2GraphKernelProfileV1 as Kernels,
    EngineeringTp2GraphPolicyV1 as Policy,
};

fn geometry_pool(geometry: Geometry) -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(geometry.context_tokens(), 32, geometry.pages(), 100)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn graph_context2304_recorder_binds_exact_kv_table_and_scalar_extents_for_every_profile() {
    for geometry in Geometry::ALL {
        let driver = builder_for_pool(&geometry_pool(geometry));
        let pages = usize::try_from(geometry.pages()).unwrap();
        for profile in Kernels::ALL
            .into_iter()
            .filter(|profile| !profile.split_attention())
        {
            let program = driver
                .record_graph_geometry_fixture(profile, geometry)
                .unwrap();
            assert_eq!(program.steps.len(), 1013);
            validate_extents(
                &program,
                &driver.inner.transports[0].catalog.borrow().allocations,
            );
            for layer in 0..36 {
                for rank in 0..2 {
                    let first = 2 + layer * 28;
                    let Step::Rank {
                        dispatch: append, ..
                    } = &program.steps[first + 14 + rank]
                    else {
                        panic!()
                    };
                    let Step::Rank {
                        dispatch: attention,
                        ..
                    } = &program.steps[first + 16 + rank]
                    else {
                        panic!()
                    };
                    assert_eq!(
                        append.arguments[8..],
                        [EngineeringTpArgumentV1::U32(geometry.pages()); 2]
                    );
                    assert_eq!(
                        attention.arguments[8..10],
                        [EngineeringTpArgumentV1::U32(geometry.pages()); 2]
                    );
                    let state = &driver.inner.ranks[rank];
                    assert_eq!(state.layers[layer].k_cache.elements, pages * 16 * 512);
                    assert_eq!(state.layers[layer].v_cache.elements, pages * 16 * 512);
                    assert_eq!(driver.page_tables[rank].elements, 16 * pages);
                    assert_eq!(attention.arguments[1], state.layers[layer].k_cache.read());
                    assert_eq!(attention.arguments[2], state.layers[layer].v_cache.read());
                }
            }
            if geometry == Geometry::Short64 {
                assert_eq!(
                    program,
                    driver.record_graph_kernel_profile_fixture(profile).unwrap()
                );
            }
        }
        assert!(driver.prepared_peer.is_none());
        assert_eq!(driver.dispatch_counts(), [0, 0]);
    }
}

#[test]
fn graph_context2304_registration_rejects_wrong_geometry_and_short_wrappers_before_seal() {
    for geometry in Geometry::ALL {
        let pool = geometry_pool(geometry);
        let mut driver = builder_for_pool(&pool);
        let other = if geometry == Geometry::Short64 {
            Geometry::Long2304
        } else {
            Geometry::Short64
        };
        assert!(
            driver
                .configure_prepared_peer_graph_geometry(
                    Policy::TransactionFences,
                    Kernels::Baseline,
                    other,
                    None,
                    None
                )
                .is_err()
        );
        assert!(driver.prepared_peer.is_none());
        assert!(!driver.poisoned);
        assert!(
            driver.inner.transports[0]
                .catalog
                .borrow()
                .registered
                .is_none()
        );
        if geometry == Geometry::Long2304 {
            assert!(driver.configure_prepared_peer().is_err());
            assert!(
                driver
                    .configure_prepared_peer_graph(Policy::TransactionFences)
                    .is_err()
            );
        }
        driver
            .configure_prepared_peer_graph_geometry(
                Policy::TransactionFences,
                Kernels::Baseline,
                geometry,
                None,
                None,
            )
            .unwrap();
        assert!(driver.prepared_peer.is_some());
        assert!(
            driver
                .configure_prepared_peer_graph_geometry(
                    Policy::TransactionFences,
                    Kernels::Baseline,
                    geometry,
                    None,
                    None
                )
                .is_err()
        );
    }
}

#[test]
fn graph_context2304_packet_budget_reuse_requires_the_sealed_long_graph() {
    let per_rank = 688;
    for geometry in Geometry::ALL {
        for policy in Policy::PRE_FINITE {
            let mut driver = builder_for_pool(&geometry_pool(geometry));
            assert!(
                driver
                    .packet_budget_for_batch(2303, per_rank, false)
                    .is_err()
            );
            assert_eq!(
                driver.packet_budget_for_batch(1, per_rank, false).unwrap(),
                per_rank
            );
            assert_eq!(
                driver
                    .packet_budget_for_batch(2303, per_rank, true)
                    .unwrap(),
                per_rank
            );
            driver
                .configure_prepared_peer_graph_geometry(
                    policy,
                    Kernels::Baseline,
                    geometry,
                    None,
                    None,
                )
                .unwrap();
            for next in [1, 173, 174, 2303] {
                let result = driver.packet_budget_for_batch(next, per_rank, false);
                if geometry == Geometry::Long2304 {
                    assert_eq!(result.unwrap(), 759);
                } else {
                    assert_eq!(result.is_ok(), next * per_rank <= 131_072);
                }
            }
            if geometry == Geometry::Short64 {
                assert!(
                    driver
                        .packet_budget_for_batch(u64::MAX, per_rank, false)
                        .is_err()
                );
            }
            assert_eq!(driver.completed_batches, 0);
            assert_eq!(driver.dispatch_counts(), [0, 0]);
        }
    }
}
