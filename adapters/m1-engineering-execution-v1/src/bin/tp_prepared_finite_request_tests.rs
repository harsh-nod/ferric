#[test]
fn finite_parent_ready_requires_exact_full_wave_batched_and_both_geometries() {
    use scope::graph;
    for geometry in graph::GraphGeometry::ALL {
        for profile in graph::KernelProfile::ALL {
            for metadata in graph::MetadataUploadMode::ALL {
                let mode = graph::ExecutionMode::FiniteRequestAdmissionCache;
                let selected = match geometry {
                    graph::GraphGeometry::Short64 => {
                        PreparedMode::GraphOptions(mode, profile, metadata)
                    }
                    graph::GraphGeometry::Long2304 => {
                        PreparedMode::LongGraph(mode, profile, metadata)
                    }
                };
                let response = match geometry {
                    graph::GraphGeometry::Short64 => {
                        scope::Reply::Graph(scope::graph_ready_fixture_with_options(
                            [1, 2],
                            30,
                            mode,
                            profile,
                            metadata,
                        ))
                    }
                    graph::GraphGeometry::Long2304 => scope::Reply::LongGraph(
                        scope::long_graph_ready_fixture([1, 2], 30, mode, profile, metadata),
                    ),
                };
                assert_eq!(
                    response.valid_ready(selected, [1, 2], 30),
                    profile == graph::KernelProfile::WaveStackNormAttentionKvMlp
                        && metadata == MetadataUploadMode::Batched
                );
            }
        }
    }
}

#[test]
fn finite_parent_requires_final_receipt_then_both_rank_close_and_clean_child_exit() {
    for geometry in GraphGeometry::ALL {
        let final_epoch = if geometry == GraphGeometry::Short64 {
            35
        } else {
            2302
        };
        for fault in [
            "normal",
            "close-eof",
            "close-id",
            "close-extra",
            "close-exit",
            "close-malformed",
            "close-truncated-prefix",
            "close-truncated-body",
            "reader-join",
        ] {
            let (mut connection, input) = graph_geometry_execute_fake(
                fault,
                GraphPolicy::FiniteRequestAdmissionCache,
                final_epoch,
                geometry,
            );
            let receipt = connection.execute_geometry_graph(&input).unwrap();
            if fault == "reader-join" {
                connection
                    .threads
                    .push(thread::spawn(|| panic!("injected IPC join failure")));
            }
            assert!(matches!(receipt.policy,
                ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphPolicyEvidenceV1::FiniteRequestAdmissionCache { provisional: false, .. }));
            let connection = Rc::new(RefCell::new(connection));
            let mut ranks: Vec<_> = (0..2)
                .map(|rank| PreparedWorker {
                    connection: Rc::clone(&connection),
                    rank,
                })
                .collect();
            ranks[0].close().unwrap();
            assert!(!connection.borrow().exited);
            let close = ranks[1].close();
            let mut published = 0;
            crate::publish_finite_after_close(
                &Ok(()),
                &close,
                &serde_json::json!({"generated_tokens":[12]}),
                |_| {
                    published += 1;
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(published, usize::from(fault == "normal"), "{fault}");
            assert_eq!(close.is_ok(), fault == "normal", "{fault}");
            assert_eq!(ranks[1].close().is_ok(), fault == "normal", "retry {fault}");
            assert!(connection.borrow().exited);
            assert_eq!(
                connection.borrow().phase,
                if fault == "normal" {
                    Phase::Closed
                } else {
                    Phase::Terminal
                }
            );
        }
        let (mut connection, input) = graph_geometry_execute_fake(
            "normal",
            GraphPolicy::FiniteRequestAdmissionCache,
            0,
            geometry,
        );
        let receipt = connection.execute_geometry_graph(&input).unwrap();
        assert!(matches!(receipt.policy,
            ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphPolicyEvidenceV1::FiniteRequestAdmissionCache { provisional: true, .. }));
        let connection = Rc::new(RefCell::new(connection));
        let mut ranks: Vec<_> = (0..2)
            .map(|rank| PreparedWorker {
                connection: Rc::clone(&connection),
                rank,
            })
            .collect();
        ranks[0].close().unwrap();
        assert!(ranks[1].close().is_err());
        assert_eq!(connection.borrow().phase, Phase::Terminal);
        assert!(connection.borrow().exited);
        // State-only cleanup-failure simulation; the child above is already reaped.
        connection.borrow_mut().exited = false;
        assert!(ranks[1].close().is_err());
        assert!(!connection.borrow().exited);
        connection.borrow_mut().exited = true;
    }
}

#[test]
fn finite_parent_rejects_bad_native_counters_without_committing_or_fallback() {
    for geometry in GraphGeometry::ALL {
        for fault in [
            "last-slot",
            "stale-read",
            "wrong-id",
            "fatal",
            "disconnect",
            "token-full",
            "token-graph",
            "token-boundaries",
            "token-reset",
            "token-loop",
            "token-old-policy",
        ] {
            let (mut connection, input) = graph_geometry_execute_fake(
                fault,
                GraphPolicy::FiniteRequestAdmissionCache,
                0,
                geometry,
            );
            let pages = connection.registration.as_ref().unwrap().pages.clone();
            assert!(
                connection.execute_geometry_graph(&input).is_err(),
                "{fault}"
            );
            assert_eq!(connection.phase, Phase::Terminal);
            assert!(connection.exited);
            assert_eq!(
                connection.registration.as_ref().unwrap().generation,
                input.generation
            );
            assert_eq!(connection.registration.as_ref().unwrap().pages, pages);
        }
    }
}
