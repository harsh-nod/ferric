fn zero_comparison_filter_fixture_v1(
    operation: SemanticBinaryOpV1,
    literal: u128,
    reverse: bool,
    mutation: usize,
) -> SemanticFunctionDeclV1 {
    let tested_definition = typed_assignment(
        2,
        U64_TYPE,
        SemanticRvalueKindV1::Use(typed_operand(1, U64_TYPE)),
    );
    let candidate = match mutation {
        1 => 5,
        2 => 4,
        _ => 2,
    };
    let value = typed_operand(candidate, U64_TYPE);
    let literal = typed_constant(U64_TYPE, literal, 8);
    let (left, right) = if reverse {
        (literal, value)
    } else {
        (value, literal)
    };
    let mut comparison_statements = Vec::new();
    if mutation == 1 {
        comparison_statements.push(typed_assignment(
            5,
            U64_TYPE,
            SemanticRvalueKindV1::Use(typed_operand(2, U64_TYPE)),
        ));
    }
    comparison_statements.push(typed_assignment(
        3,
        BOOL_TYPE,
        SemanticRvalueKindV1::Binary {
            operation,
            left,
            right,
        },
    ));
    if mutation == 5 {
        comparison_statements.push(tested_definition.clone());
    }
    let mut blocks = vec![
        block(
            10,
            vec![],
            zero_switch(1, U64_TYPE, 1, if mutation == 4 { 2 } else { 6 }),
        ),
        block(
            11,
            if mutation == 5 {
                vec![]
            } else {
                vec![tested_definition]
            },
            SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 2)),
        ),
        block(
            12,
            comparison_statements,
            SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 3)),
        ),
        block(
            13,
            if mutation == 3 {
                vec![typed_assignment(
                    2,
                    U64_TYPE,
                    SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, 0, 8)),
                )]
            } else {
                vec![]
            },
            zero_switch(3, BOOL_TYPE, 4, 5),
        ),
        block(14, vec![], SemanticTerminatorKindV1::Return),
        block(15, vec![], SemanticTerminatorKindV1::Return),
    ];
    // The unrelated entry branch makes irrelevant dominance queries perform a
    // real CFG walk; it never reaches the comparison or its consuming switch.
    for index in 6..14 {
        blocks.push(block(
            10 + index as u8,
            vec![],
            SemanticTerminatorKindV1::Goto(cfg_edge(
                SemanticEdgeRoleV1::Goto,
                if index == 13 { 5 } else { index + 1 },
            )),
        ));
    }
    projection_function_with_locals(
        blocks,
        vec![
            local(30, SCALAR_TYPE, SemanticLocalRoleV1::Return),
            local(31, U64_TYPE, SemanticLocalRoleV1::Argument(0)),
            local(32, U64_TYPE, SemanticLocalRoleV1::Temporary),
            local(33, BOOL_TYPE, SemanticLocalRoleV1::Temporary),
            local(34, U64_TYPE, SemanticLocalRoleV1::Argument(1)),
            local(35, U64_TYPE, SemanticLocalRoleV1::Temporary),
        ],
    )
}

#[test]
fn zero_comparison_candidate_filter_matches_legacy_guard_results() {
    let types = assertion_proof_types();
    for operation in [
        SemanticBinaryOpV1::Equal,
        SemanticBinaryOpV1::NotEqual,
        SemanticBinaryOpV1::GreaterThan,
        SemanticBinaryOpV1::LessOrEqual,
        SemanticBinaryOpV1::LessThan,
        SemanticBinaryOpV1::GreaterOrEqual,
    ] {
        for reverse in [false, true] {
            for literal in [0, 1, 64] {
                for mutation in 0..6 {
                    let function =
                        zero_comparison_filter_fixture_v1(operation, literal, reverse, mutation);
                    let mut filtered = SemanticAssertProofsV1::new(&types, &function).unwrap();
                    let mut legacy = SemanticAssertProofsV1::new(&types, &function).unwrap();
                    let actual = filtered
                        .comparison_zero_excluding_target_with_filter_v1::<true>(3, 2, 3, 4, 5)
                        .unwrap();
                    let expected = legacy
                        .comparison_zero_excluding_target_with_filter_v1::<false>(3, 2, 3, 4, 5)
                        .unwrap();
                    assert_eq!(
                        actual, expected,
                        "op={operation:?}, reverse={reverse}, literal={literal}, mutation={mutation}"
                    );
                    if literal != 0 || mutation >= 2 {
                        assert_eq!(actual, None);
                    }
                }
            }
        }
    }
}

#[test]
fn zero_comparison_candidate_filter_avoids_unrelated_dominance_walks() {
    let types = assertion_proof_types();
    for (literal, mutation) in [(64, 0), (0, 2)] {
        let function = zero_comparison_filter_fixture_v1(
            SemanticBinaryOpV1::GreaterThan,
            literal,
            false,
            mutation,
        );
        let mut filtered = SemanticAssertProofsV1::new(&types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&types, &function).unwrap();
        assert_eq!(
            filtered
                .comparison_zero_excluding_target_with_filter_v1::<true>(3, 2, 3, 4, 5)
                .unwrap(),
            None
        );
        assert_eq!(
            legacy
                .comparison_zero_excluding_target_with_filter_v1::<false>(3, 2, 3, 4, 5)
                .unwrap(),
            None
        );
        assert!(filtered.work < legacy.work);
        assert!(!filtered.dominance.contains_key(&(1, 2)));
        assert_eq!(legacy.dominance.get(&(1, 2)), Some(&true));
        // The reaching definition of the condition is still authenticated.
        assert_eq!(filtered.dominance.get(&(2, 3)), Some(&true));
    }
}

#[test]
fn zero_comparison_candidate_filter_keeps_relevant_guard_stability_checks() {
    let types = assertion_proof_types();
    for mutation in [0, 1, 3, 4, 5] {
        let function =
            zero_comparison_filter_fixture_v1(SemanticBinaryOpV1::GreaterThan, 0, false, mutation);
        let mut proofs = SemanticAssertProofsV1::new(&types, &function).unwrap();
        assert_eq!(
            proofs
                .comparison_zero_excluding_target(3, 2, 3, 4, 5)
                .unwrap(),
            if mutation <= 1 { Some(5) } else { None },
        );
        if mutation <= 1 {
            assert_eq!(proofs.dominance.get(&(1, 2)), Some(&true));
        } else if mutation == 4 {
            assert_eq!(proofs.dominance.get(&(1, 2)), Some(&false));
        }
    }
}

#[test]
fn zero_comparison_candidate_filter_preserves_exact_and_short_work_refusal() {
    let types = assertion_proof_types();
    let function = zero_comparison_filter_fixture_v1(SemanticBinaryOpV1::GreaterThan, 0, false, 0);
    let mut measured = SemanticAssertProofsV1::new(&types, &function).unwrap();
    let before = measured.work;
    assert_eq!(
        measured
            .comparison_zero_excluding_target(3, 2, 3, 4, 5)
            .unwrap(),
        Some(5)
    );
    let cost = measured.work - before;
    assert!(cost > 0);
    for short in [false, true] {
        let mut proofs = SemanticAssertProofsV1::new(&types, &function).unwrap();
        proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost + usize::from(short);
        let result = proofs.comparison_zero_excluding_target(3, 2, 3, 4, 5);
        if short {
            assert!(matches!(
                result,
                Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
            ));
        } else {
            assert_eq!(result.unwrap(), Some(5));
            assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
        }
    }
}

#[test]
fn zero_comparison_candidate_filter_rejects_non_comparison_conditions() {
    let types = assertion_proof_types();
    for value in [
        SemanticRvalueKindV1::Use(typed_constant(BOOL_TYPE, 1, 1)),
        SemanticRvalueKindV1::Binary {
            operation: SemanticBinaryOpV1::BitAnd,
            left: typed_constant(BOOL_TYPE, 0, 1),
            right: typed_constant(BOOL_TYPE, 1, 1),
        },
    ] {
        let original =
            zero_comparison_filter_fixture_v1(SemanticBinaryOpV1::GreaterThan, 0, false, 0);
        let mut blocks = original.blocks().to_vec();
        blocks[2] = block(
            12,
            vec![typed_assignment(3, BOOL_TYPE, value)],
            SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 3)),
        );
        let function = projection_function_with_locals(blocks, original.locals().to_vec());
        let mut filtered = SemanticAssertProofsV1::new(&types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&types, &function).unwrap();
        assert_eq!(
            filtered
                .comparison_zero_excluding_target_with_filter_v1::<true>(3, 2, 3, 4, 5)
                .unwrap(),
            None
        );
        assert_eq!(
            legacy
                .comparison_zero_excluding_target_with_filter_v1::<false>(3, 2, 3, 4, 5)
                .unwrap(),
            None
        );
        assert!(filtered.work < legacy.work);
        assert!(!filtered.dominance.contains_key(&(1, 2)));
    }
    for operation in [SemanticBinaryOpV1::Add, SemanticBinaryOpV1::BitAnd] {
        for reverse in [false, true] {
            let original =
                zero_comparison_filter_fixture_v1(SemanticBinaryOpV1::GreaterThan, 0, false, 0);
            let mut blocks = original.blocks().to_vec();
            let mut locals = original.locals().to_vec();
            locals[3] = local(33, U64_TYPE, SemanticLocalRoleV1::Temporary);
            let value = typed_operand(2, U64_TYPE);
            let zero = typed_constant(U64_TYPE, 0, 8);
            let (left, right) = if reverse {
                (zero, value)
            } else {
                (value, zero)
            };
            blocks[2] = block(
                12,
                vec![typed_assignment(
                    3,
                    U64_TYPE,
                    SemanticRvalueKindV1::Binary {
                        operation,
                        left,
                        right,
                    },
                )],
                SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 3)),
            );
            blocks[3] = block(13, vec![], zero_switch(3, U64_TYPE, 4, 5));
            let function = projection_function_with_locals(blocks, locals);
            let mut filtered = SemanticAssertProofsV1::new(&types, &function).unwrap();
            let mut legacy = SemanticAssertProofsV1::new(&types, &function).unwrap();
            assert_eq!(
                filtered
                    .comparison_zero_excluding_target_with_filter_v1::<true>(3, 2, 3, 4, 5)
                    .unwrap(),
                None
            );
            assert_eq!(
                legacy
                    .comparison_zero_excluding_target_with_filter_v1::<false>(3, 2, 3, 4, 5)
                    .unwrap(),
                None
            );
            assert!(filtered.work < legacy.work);
            assert!(!filtered.dominance.contains_key(&(1, 2)));
        }
    }
}
