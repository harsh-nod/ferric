#[test]
fn closed_token_cli_requires_explicit_cache_and_batched_metadata_across_all_profiles() {
    for geometry in GraphGeometry::ALL {
        for profile in GraphKernels::ALL {
            let mut selected = dependency_options();
            if geometry == GraphGeometry::Long2304 {
                for (flag, value) in [("--context", "2304"), ("--pages", "144")] {
                    let index = selected.iter().position(|item| *item == flag).unwrap();
                    selected[index + 1] = value;
                }
            }
            selected.extend([
                "--runtime-tp2-queued-graph",
                "closed-token-admission-cache",
                "--runtime-cache-admission",
                "--runtime-tp2-metadata-uploads",
                "batched",
                "--tp2-graph-geometry",
                geometry.argument(),
                "--tp2-graph-kernel-profile",
                profile.argument(),
            ]);
            if profile.has_v22() {
                selected.extend(["--tp2-graph-argmax-artifact", "/v22"]);
            }
            if profile.has_v15() {
                selected.extend(["--tp2-graph-norm-artifact", "/v15"]);
            }
            if profile.split_attention() {
                selected.extend(["--tp2-graph-split-attention-artifact", "/split"]);
            }
            let parsed = args(&selected).unwrap();
            assert_eq!(
                parsed.prepared_graph,
                Some(GraphPolicy::ClosedTokenAdmissionCache)
            );
            assert_eq!(parsed.graph_kernel_profile, profile);
            assert_eq!(parsed.graph_geometry, geometry);
            assert_eq!(
                parsed.graph_metadata_uploads,
                Some(MetadataUploadMode::Batched)
            );
            assert!(parsed.runtime.cache_admission);
            assert!(
                !parsed.runtime.operational
                    && !parsed.runtime.profile
                    && !parsed.runtime.shared_full_currentness
            );
            assert!(parsed.host_timing.is_none());
            for (flag, length) in [
                ("--runtime-cache-admission", 1),
                ("--runtime-tp2-metadata-uploads", 2),
            ] {
                let mut missing = selected.clone();
                let index = missing.iter().position(|item| *item == flag).unwrap();
                missing.drain(index..index + length);
                assert!(args(&missing).is_err());
            }
            for (flag, value) in [
                ("--runtime-tp2-metadata-uploads", "separate-writes"),
                ("--runtime-tp2-queued-graph", "closed-token"),
            ] {
                let mut invalid = selected.clone();
                let index = invalid.iter().position(|item| *item == flag).unwrap();
                invalid[index + 1] = value;
                assert!(args(&invalid).is_err());
            }
            for incompatible in [
                vec!["--runtime-tp2-prepared"],
                vec!["--runtime-tp2-program-scope"],
                vec!["--host-timing", "/timing"],
                vec!["--runtime-profile"],
                vec!["--peer-shared-full-currentness"],
            ] {
                let mut invalid = selected.clone();
                invalid.extend(incompatible);
                assert!(args(&invalid).is_err());
            }
        }
    }
}

#[test]
fn closed_token_setup_is_explicit_and_does_not_change_old_policy_metadata() {
    let original = serde_json::json!({
        "runtime_tp2_prepared": { "gpu_overlap_claim": false, "joint_drain": true },
        "performance_profile": {},
    });
    for policy in GraphPolicy::PRE_FINITE {
        let mut setup = original.clone();
        record_closed_token_policy(&mut setup, policy);
        if !policy.closed_token() {
            assert_eq!(
                serde_json::to_vec(&setup).unwrap(),
                serde_json::to_vec(&original).unwrap()
            );
            continue;
        }
        let prepared = &setup["runtime_tp2_prepared"];
        assert_eq!(prepared["closed_token"], true);
        assert_eq!(prepared["observation_policy"], "TokenBoundariesV1");
        assert_eq!(prepared["native_token_api_calls_per_forward"], 1);
        for field in [
            "scoped_operation_observations",
            "provisional_outputs",
            "transient_currentness_equivalence_claim",
            "legacy_three_operation_timing_claim",
            "gpu_overlap_claim",
        ] {
            assert_eq!(prepared[field], false);
        }
        assert_eq!(
            setup["performance_profile"]["tp2_graph_observation_policy"],
            "TokenBoundariesV1"
        );
    }
}
