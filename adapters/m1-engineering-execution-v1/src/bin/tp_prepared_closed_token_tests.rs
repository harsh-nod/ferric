// CPU protocol inputs only; native token evidence is never synthesized in production.

#[test]
fn closed_token_parent_launch_and_ready_require_batched_metadata_for_every_geometry_profile() {
    use scope::graph;
    let execution = graph::ExecutionMode::ClosedTokenAdmissionCache;
    assert!(validate_metadata_option(PreparedMode::Graph(execution)).is_err());
    for geometry in graph::GraphGeometry::ALL {
        for profile in graph::KernelProfile::ALL {
            for metadata in graph::MetadataUploadMode::ALL {
                let selected = match geometry {
                    graph::GraphGeometry::Short64 => {
                        PreparedMode::GraphOptions(execution, profile, metadata)
                    }
                    graph::GraphGeometry::Long2304 => {
                        PreparedMode::LongGraph(execution, profile, metadata)
                    }
                };
                assert_eq!(
                    selected.graph_policy(),
                    Some(GraphPolicy::ClosedTokenAdmissionCache)
                );
                assert_eq!(
                    validate_metadata_option(selected).is_ok(),
                    metadata == MetadataUploadMode::Batched
                );
                assert!(validate_admission_option(selected, true).is_ok());
                assert!(validate_admission_option(selected, false).is_err());
                let ready = match geometry {
                    graph::GraphGeometry::Short64 => scope::graph_ready_fixture_with_options(
                        [1, 2],
                        30,
                        execution,
                        profile,
                        metadata,
                    ),
                    graph::GraphGeometry::Long2304 => {
                        scope::long_graph_ready_fixture([1, 2], 30, execution, profile, metadata)
                            .response
                    }
                };
                let valid = |response| match geometry {
                    graph::GraphGeometry::Short64 => {
                        scope::Reply::Graph(response).valid_ready(selected, [1, 2], 30)
                    }
                    graph::GraphGeometry::Long2304 => {
                        scope::Reply::LongGraph(graph::LongResponse { geometry, response })
                            .valid_ready(selected, [1, 2], 30)
                    }
                };
                assert_eq!(
                    valid(ready.clone()),
                    metadata == MetadataUploadMode::Batched
                );
                for field in [
                    "currentness",
                    "native_source_sha256",
                    "profile",
                    "authority",
                ] {
                    let mut raw = serde_json::to_value(&ready).unwrap();
                    raw[field] = "substituted".into();
                    assert!(!valid(serde_json::from_value(raw).unwrap()));
                }
                for field in [
                    "native_program_deadline_ms",
                    "native_queued_deadline_ms",
                    "control_allocation_flags",
                ] {
                    let mut raw = serde_json::to_value(&ready).unwrap();
                    raw[field] = serde_json::json!(0);
                    assert!(!valid(serde_json::from_value(raw).unwrap()));
                }
            }
        }
    }
}

#[test]
fn closed_token_parent_wire_mapping_copies_actual_counters_without_legacy_conversion() {
    use ferric_m1_engineering_execution_v1::tp_execution::EngineeringTp2GraphPolicyEvidenceV1 as Evidence;
    for loops in [9_111, 10_631, 12_151] {
        let (program, input, mut wire) =
            graph_receipt_fixture(16, GraphPolicy::ClosedTokenAdmissionCache);
        wire.policy = scope::graph::PolicyEvidence::ClosedTokenAdmissionCache {
            full_boundaries: 2,
            graph_operational_boundaries: 9,
            token_boundaries: 4,
            reset_only_rounds: 23,
            loop_checks: loops,
        };
        let typed = scope::validate_graph_receipt(
            wire.clone(),
            &program,
            &input,
            [1, 2],
            50,
            GraphPolicy::ClosedTokenAdmissionCache,
        )
        .unwrap();
        assert_eq!(
            typed.policy,
            Evidence::ClosedTokenAdmissionCache {
                full_boundaries: 2,
                graph_operational_boundaries: 9,
                token_boundaries: 4,
                reset_only_rounds: 23,
                loop_checks: loops,
            }
        );
        for wrong in GraphPolicy::PRE_FINITE
            .into_iter()
            .filter(|policy| !policy.closed_token())
        {
            assert!(
                scope::validate_graph_receipt(wire.clone(), &program, &input, [1, 2], 50, wrong)
                    .is_err()
            );
        }
        let mut raw = serde_json::to_value(wire.policy).unwrap();
        raw["token_boundaries"] = serde_json::json!(true);
        assert!(serde_json::from_value::<scope::graph::PolicyEvidence>(raw).is_err());
    }
}

#[test]
fn closed_token_parent_ipc_failure_is_terminal_without_page_or_generation_commit() {
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
            "token-provisional",
            "token-request",
            "token-old-policy",
        ] {
            let (mut connection, input) = graph_geometry_execute_fake(
                fault,
                GraphPolicy::ClosedTokenAdmissionCache,
                16,
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
            assert!(connection.execute_geometry_graph(&input).is_err());
        }
    }
}
