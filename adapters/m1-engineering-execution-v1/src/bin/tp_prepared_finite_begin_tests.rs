#[test]
fn finite_parent_begin_acknowledgement_binds_exact_identity_budget_and_counts() {
    use scope::graph;
    for geometry in graph::GraphGeometry::ALL {
        let profile = graph::KernelProfile::WaveStackNormAttentionKvMlp;
        let mode = match geometry {
            graph::GraphGeometry::Short64 => PreparedMode::GraphOptions(
                graph::ExecutionMode::FiniteRequestAdmissionCache,
                profile,
                MetadataUploadMode::Batched,
            ),
            graph::GraphGeometry::Long2304 => PreparedMode::LongGraph(
                graph::ExecutionMode::FiniteRequestAdmissionCache,
                profile,
                MetadataUploadMode::Batched,
            ),
        };
        let original = graph::Response::RegisteredFinite {
            id: 7,
            plan_sha256: [3; 32],
            catalog_sha256: [4; 32],
            group_id: 0,
            geometry,
            budget: graph::FiniteBudget::for_geometry(geometry),
            full_boundaries: 1,
            steps: profile.steps() as u32,
            kernel_counts: profile.kernel_counts(),
        };
        for mutation in 0..11 {
            let mut response = original.clone();
            if let graph::Response::RegisteredFinite {
                id,
                plan_sha256,
                catalog_sha256,
                group_id,
                geometry,
                budget,
                full_boundaries,
                steps,
                kernel_counts,
            } = &mut response
            {
                match mutation {
                    1 => *id += 1,
                    2 => plan_sha256[0] ^= 1,
                    3 => catalog_sha256[0] ^= 1,
                    4 => *group_id = 1,
                    5 => {
                        *geometry = if *geometry == graph::GraphGeometry::Short64 {
                            graph::GraphGeometry::Long2304
                        } else {
                            graph::GraphGeometry::Short64
                        }
                    }
                    6 => budget.forwards -= 1,
                    7 => budget.lifetime_ms -= 1,
                    8 => *full_boundaries = 0,
                    9 => *steps -= 1,
                    10 => kernel_counts[1] -= 1,
                    _ => {}
                }
            }
            assert_eq!(
                validate_finite_registered(&response, 7, [3; 32], [4; 32], 0, mode).is_ok(),
                mutation == 0,
                "{mutation}"
            );
        }
        assert!(
            validate_finite_registered(
                &graph::Response::Closed { id: 7 },
                7,
                [3; 32],
                [4; 32],
                0,
                mode
            )
            .is_err()
        );
        let mut raw = serde_json::to_value(original).unwrap();
        raw["full_boundaries"] = serde_json::json!(true);
        assert!(serde_json::from_value::<graph::Response>(raw).is_err());
    }
}
