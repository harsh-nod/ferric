#[cfg(test)]
mod resource_upper_bound_tests {
    use std::cell::Cell;

    use super::*;

    include!("guard_census_tests.rs");

    fn census() -> ProductionAnalysisInputCensusV1 {
        ProductionAnalysisInputCensusV1 {
            blocks: 5,
            operations: 24,
            operands: 41,
            results: 19,
            successors: 8,
            block_arguments: 7,
            attributes: 12,
            type_nodes: 31,
            identifier_bytes: 120,
            canonical_bytes: 900,
            max_operation_arity: 6,
            max_successor_arity: 2,
            pipeline_creates: 0,
            pipeline_events: 0,
            ranked_accesses: 0,
            workgroup_ranked_accesses: 0,
            allocation_effects: 0,
            collective_transpose_candidates: 0,
            ownership_contracts: 0,
            effect_refinement_contracts: 0,
            index_lt_branch_candidates: 0,
            memory_bounds_guard_candidates: 0,
            semantic_definitions: 0,
            semantic_refinement_contracts: 0,
            native_switch_verification_work: 0,
            native_switch_verification_scratch: 0,
        }
    }

    #[test]
    fn memory_bounds_bound_accepts_exact_limits_and_rejects_one_under() {
        let census = census();
        let exact = preflight_ranked_bounds_resource_upper_bound_v1(
            census,
            ProductionAnalysisResourceLimitsV1::new(usize::MAX, usize::MAX),
        )
        .unwrap();
        assert_eq!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                census,
                ProductionAnalysisResourceLimitsV1::new(
                    exact.work_upper_bound(),
                    exact.peak_storage_upper_bound(),
                ),
            ),
            Ok(exact)
        );
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                census,
                ProductionAnalysisResourceLimitsV1::new(
                    exact.work_upper_bound() - 1,
                    exact.peak_storage_upper_bound(),
                ),
            )
            .is_err()
        );
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                census,
                ProductionAnalysisResourceLimitsV1::new(
                    exact.work_upper_bound(),
                    exact.peak_storage_upper_bound() - 1,
                ),
            )
            .is_err()
        );
    }

    #[test]
    fn memory_bounds_bound_rejects_arithmetic_overflow_and_hard_caps() {
        let unlimited = ProductionAnalysisResourceLimitsV1::new(usize::MAX, usize::MAX);
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                ProductionAnalysisInputCensusV1 {
                    operands: usize::MAX,
                    results: 1,
                    ..ProductionAnalysisInputCensusV1::default()
                },
                unlimited,
            )
            .is_err()
        );
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                ProductionAnalysisInputCensusV1 {
                    blocks: MAX_RANKED_BOUNDS_BLOCKS + 1,
                    ..ProductionAnalysisInputCensusV1::default()
                },
                unlimited,
            )
            .is_err()
        );
    }

    #[test]
    fn memory_bounds_block_cap_accepts_legacy_measured_and_2048() {
        let unlimited = ProductionAnalysisResourceLimitsV1::new(usize::MAX, usize::MAX);
        for blocks in [1_024, 1_121, 2_048] {
            let census = ProductionAnalysisInputCensusV1 {
                blocks,
                operations: blocks,
                successors: blocks - 1,
                ..ProductionAnalysisInputCensusV1::default()
            };
            assert!(preflight_ranked_bounds_resource_upper_bound_v1(census, unlimited).is_ok());
        }
        assert_eq!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                ProductionAnalysisInputCensusV1 {
                    blocks: 2_049,
                    ..ProductionAnalysisInputCensusV1::default()
                },
                unlimited,
            ),
            Err(ranked_bounds_resource_error_v1("memory-bounds structural hard limit")),
        );
    }

    #[test]
    fn memory_bounds_block_cap_does_not_expand_other_limits() {
        assert_eq!(MAX_RANKED_BOUNDS_BLOCKS, 2_048);
        assert_eq!(MAX_RANKED_BOUNDS_EDGES, 2_048);
        assert_eq!(MAX_RANKED_BOUNDS_FACTS, 1_024);
        assert_eq!(MAX_RANKED_BOUNDS_OPERATIONS, 65_536);
        assert_eq!(MAX_RANKED_BOUNDS_FINDINGS, 4_096);
        assert_eq!(MAX_RANKED_BOUNDS_WORK_UNITS, 8_388_608);
        assert_eq!(MAX_RANKED_BOUNDS_STORAGE_ITEMS, 131_072);
        let unlimited = ProductionAnalysisResourceLimitsV1::new(usize::MAX, usize::MAX);
        let exact = ProductionAnalysisInputCensusV1 {
            blocks: 2_048,
            operations: 2_048,
            successors: 2_048,
            ..ProductionAnalysisInputCensusV1::default()
        };
        assert!(preflight_ranked_bounds_resource_upper_bound_v1(exact, unlimited).is_ok());
        for denied in [
            ProductionAnalysisInputCensusV1 { successors: 2_049, ..exact },
            ProductionAnalysisInputCensusV1 { operations: 65_537, ..exact },
        ] {
            assert_eq!(
                preflight_ranked_bounds_resource_upper_bound_v1(denied, unlimited),
                Err(ranked_bounds_resource_error_v1("memory-bounds structural hard limit")),
            );
        }
        let mut budget = RankedBoundsBudget::default();
        budget.reserve(RankedBoundsResource::Facts, 1_024).unwrap();
        assert_eq!(
            budget.reserve(RankedBoundsResource::Facts, 1),
            Err(RankedBoundsFindingV1::ResourceLimitExceeded {
                resource: "guard fact",
                limit: 1_024,
                actual: 1_025,
            }),
        );
        assert_eq!(budget.facts, 1_024);
    }

    #[test]
    fn memory_bounds_block_cap_preserves_independent_work_storage_refusals() {
        let mut budget = RankedBoundsBudget::default();
        budget.work(8_388_608).unwrap();
        assert_eq!(
            budget.work(1),
            Err(RankedBoundsFindingV1::ResourceLimitExceeded {
                resource: "analysis work unit",
                limit: 8_388_608,
                actual: 8_388_609,
            }),
        );
        assert_eq!(budget.work_units, 8_388_608);
        budget.storage(131_072).unwrap();
        assert_eq!(
            budget.storage(1),
            Err(RankedBoundsFindingV1::ResourceLimitExceeded {
                resource: "analysis storage item",
                limit: 131_072,
                actual: 131_073,
            }),
        );
        assert_eq!(budget.storage_items, 131_072);
        let dense = ProductionAnalysisInputCensusV1 {
            blocks: 2_048,
            operations: 2_048,
            successors: 2_048,
            memory_bounds_guard_candidates: 1_024,
            ..ProductionAnalysisInputCensusV1::default()
        };
        assert_eq!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                dense, ProductionAnalysisResourceLimitsV1::new(usize::MAX, usize::MAX),
            ),
            Err(ranked_bounds_resource_error_v1("memory-bounds work hard limit")),
        );
    }

    #[test]
    fn repeated_three_string_findings_have_a_literal_exact_boundary() {
        let census = ProductionAnalysisInputCensusV1 {
            blocks: 1,
            operations: 1,
            operands: 3,
            identifier_bytes: 1_024,
            memory_bounds_guard_candidates: 1,
            ..ProductionAnalysisInputCensusV1::default()
        };
        let findings = 4;
        let fact_words = 1;
        let operation_items = 3;
        let structural_items = 1 + 1 + 3 + 1_024;
        let diagnostic_bytes_per_finding = 3 * 64;
        let retained_per_finding = MAX_RANKED_MEMORY_RANK * 2 + 48 + diagnostic_bytes_per_finding;
        let exact_retained = (findings * retained_per_finding).max(64 + 2 * 1_024 + 256);
        let internal_storage = 9 + 1 + fact_words + findings;
        let exact_temporary =
            internal_storage + structural_items + MAX_RANKED_MEMORY_RANK * 6 + 2 * 1_024 + 128;
        let charged_work = 1 + operation_items + 5 + 4 + 1 + 3;
        let exact_work = structural_items * 3
            + charged_work
            + 3 * (fe2o3_kernel_analysis::MAX_PRESBURGER_WORK_UNITS_V1 + 1)
            + findings * diagnostic_bytes_per_finding * 4
            + 4 * 1_024
            + 1_024;
        let exact_peak = exact_retained + exact_temporary;

        let exact = preflight_ranked_bounds_resource_upper_bound_v1(
            census,
            ProductionAnalysisResourceLimitsV1::new(exact_work, exact_peak),
        )
        .expect("literal repeated-string envelope");
        assert_eq!(exact.work_upper_bound(), exact_work);
        assert_eq!(exact.retained_storage_upper_bound(), exact_retained);
        assert_eq!(exact.peak_storage_upper_bound(), exact_peak);
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                census,
                ProductionAnalysisResourceLimitsV1::new(exact_work - 1, exact_peak),
            )
            .is_err()
        );
        assert!(
            preflight_ranked_bounds_resource_upper_bound_v1(
                census,
                ProductionAnalysisResourceLimitsV1::new(exact_work, exact_peak - 1),
            )
            .is_err()
        );
    }

    #[test]
    fn rejected_4097th_finding_is_guarded_before_payload_construction() {
        let mut budget = RankedBoundsBudget {
            findings: MAX_RANKED_BOUNDS_FINDINGS,
            ..RankedBoundsBudget::default()
        };
        let mut findings = Vec::new();
        let constructed = Cell::new(false);
        let error = push_finding(&mut findings, &mut budget, || {
            constructed.set(true);
            RankedBoundsFindingV1::UnprovedBound {
                block: 0,
                operation: 0,
                access: AccessKindAttr::Read,
                view: "v".repeat(1_024),
                dimension: 0,
                index: "i".repeat(1_024),
                extent: "e".repeat(1_024),
            }
        })
        .unwrap_err();
        assert_eq!(
            error,
            RankedBoundsFindingV1::ResourceLimitExceeded {
                resource: "finding",
                limit: MAX_RANKED_BOUNDS_FINDINGS,
                actual: MAX_RANKED_BOUNDS_FINDINGS + 1,
            }
        );
        assert!(!constructed.get());
        assert!(findings.is_empty());
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;

    fn unproved_bound() -> RankedBoundsFindingV1 {
        RankedBoundsFindingV1::UnprovedBound {
            block: 0,
            operation: 0,
            access: AccessKindAttr::Read,
            view: "v0".to_owned(),
            dimension: 0,
            index: "i".to_owned(),
            extent: "n".to_owned(),
        }
    }

    fn static_out_of_bounds() -> RankedBoundsFindingV1 {
        RankedBoundsFindingV1::StaticOutOfBounds {
            block: 0,
            operation: 0,
            access: AccessKindAttr::Write,
            view: "v0".to_owned(),
            dimension: 0,
            index: 4,
            extent: 4,
        }
    }

    fn presburger_out_of_bounds() -> RankedBoundsFindingV1 {
        RankedBoundsFindingV1::PresburgerOutOfBounds {
            block: 0,
            operation: 0,
            access: AccessKindAttr::Write,
            view: "v0".to_owned(),
            dimension: 0,
            invocation: vec![3],
            index: 7,
            extent: 7,
        }
    }

    #[test]
    fn every_bounds_finding_has_the_shared_status() {
        let incomplete = [
            RankedBoundsFindingV1::StructuralVerificationFailed,
            RankedBoundsFindingV1::ResourceLimitExceeded {
                resource: "operation",
                limit: 1,
                actual: 2,
            },
            RankedBoundsFindingV1::UnreachableBlock { block: 1 },
            RankedBoundsFindingV1::UnsupportedTerminator {
                block: 0,
                operation: "test.terminator".to_owned(),
            },
            RankedBoundsFindingV1::UnsupportedOperation {
                block: 0,
                operation: 1,
                kind: "test.operation".to_owned(),
            },
            RankedBoundsFindingV1::SparseIndexAnalysisFailed {
                detail: "unresolved".to_owned(),
            },
            unproved_bound(),
        ];
        for finding in incomplete {
            assert_eq!(finding.status(), KernelCheckStatusV1::Incomplete);
        }
        for finding in [static_out_of_bounds(), presburger_out_of_bounds()] {
            assert_eq!(finding.status(), KernelCheckStatusV1::Rejected);
        }
    }

    #[test]
    fn rejected_bounds_finding_dominates_an_incomplete_finding() {
        let report = RankedBoundsReportV1 {
            findings: vec![unproved_bound(), static_out_of_bounds()],
        };
        assert_eq!(report.status(), KernelCheckStatusV1::Rejected);
        assert!(!report.is_clean());
        assert_eq!(
            RankedBoundsReportV1 { findings: vec![] }.status(),
            KernelCheckStatusV1::Clean
        );
    }
}
