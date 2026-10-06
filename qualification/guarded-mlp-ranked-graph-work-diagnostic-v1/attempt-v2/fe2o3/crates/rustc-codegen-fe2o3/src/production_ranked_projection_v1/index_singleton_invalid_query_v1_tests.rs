fn terminal_singleton_proofs_v1<'a>(
    fixture: &SingletonIndexFixtureV1,
    function: &'a SemanticFunctionDeclV1,
    types: &'a [SemanticTypeDeclV1],
    upper: u64,
    stop_invalid: bool,
) -> SemanticAssertProofsV1<'a> {
    let mut proof = index_singleton_switch_proofs_v1(
        types,
        &fixture.callables,
        function,
        &fixture.indices(),
        Some(upper),
    )
    .unwrap()
    .unwrap();
    proof.index_singletons.as_mut().unwrap().stop_invalid_query = stop_invalid;
    proof
}

fn terminal_singleton_invalid_tail_fixture_v1() -> SingletonIndexFixtureV1 {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    let mut statements = fixture.blocks[3].statements().to_vec();
    // The left operand has no definition. The right operand is a genuine
    // seeded expression, but visiting it cannot rescue this invalid query.
    statements[1] = typed_assignment(
        6,
        BOOL_TYPE,
        SemanticRvalueKindV1::Binary {
            operation: SemanticBinaryOpV1::Equal,
            left: typed_operand(7, U64_TYPE),
            right: typed_operand(5, U64_TYPE),
        },
    );
    fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
    fixture
}

#[test]
fn index_singleton_terminal_invalid_preserves_all_valid_query_results() {
    for cutoff in [0, 1, 64, 128, 129] {
        let fixture = SingletonIndexFixtureV1::new(cutoff);
        let function = fixture.function();
        for upper in [1, 64, 128, 256] {
            let mut fast =
                terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, upper, true);
            let mut legacy =
                terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, upper, false);
            for (block, local) in [(2, 4), (3, 6), (2, 4), (3, 6)] {
                let operand = typed_operand(local, BOOL_TYPE);
                assert_eq!(
                    fast.index_singleton_switch_value_v1(&operand, block)
                        .unwrap(),
                    legacy
                        .index_singleton_switch_value_v1(&operand, block)
                        .unwrap(),
                    "cutoff={cutoff}, upper={upper}, block={block}",
                );
                let fast_context = fast.index_singletons.as_ref().unwrap();
                let legacy_context = legacy.index_singletons.as_ref().unwrap();
                assert!(!fast_context.invalid && !legacy_context.invalid);
                assert_eq!(fast_context.used_seed, legacy_context.used_seed);
                assert_eq!(fast.work, legacy.work);
            }
        }
    }
}

#[test]
fn index_singleton_terminal_invalid_keeps_seed_discovered_only_by_guard() {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    fixture.blocks[2] = block(
        232,
        vec![
            typed_assignment(
                5,
                U64_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, 63, 8)),
            ),
            typed_assignment(
                4,
                BOOL_TYPE,
                SemanticRvalueKindV1::Binary {
                    operation: SemanticBinaryOpV1::LessThan,
                    left: typed_operand(5, U64_TYPE),
                    right: typed_operand(3, U64_TYPE),
                },
            ),
        ],
        zero_switch(4, BOOL_TYPE, 4, 3),
    );
    fixture.blocks[3] = block(
        233,
        vec![typed_assignment(
            6,
            BOOL_TYPE,
            SemanticRvalueKindV1::Binary {
                operation: SemanticBinaryOpV1::LessThan,
                left: typed_operand(5, U64_TYPE),
                right: typed_constant(U64_TYPE, 64, 8),
            },
        )],
        zero_switch(6, BOOL_TYPE, 4, 5),
    );
    let function = fixture.function();
    for stop_invalid in [false, true] {
        let mut proof =
            terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, stop_invalid);
        assert_eq!(
            proof
                .index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3)
                .unwrap(),
            Some(1),
        );
        let context = proof.index_singletons.as_ref().unwrap();
        assert!(context.used_seed && !context.invalid);
        assert!(context.local_decisions.contains_key(&(3, 2, 1)));
    }
}

#[test]
fn index_singleton_terminal_invalid_skips_tail_and_resets_for_next_query() {
    let fixture = terminal_singleton_invalid_tail_fixture_v1();
    let function = fixture.function();
    let mut fast = terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, true);
    let mut legacy = terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, false);
    let fast_before = fast.work;
    let legacy_before = legacy.work;
    for proof in [&mut fast, &mut legacy] {
        assert_eq!(
            proof
                .index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3)
                .unwrap(),
            None,
        );
        assert!(proof.index_singletons.as_ref().unwrap().invalid);
    }
    assert!(fast.work - fast_before < legacy.work - legacy_before);
    let fast_context = fast.index_singletons.as_ref().unwrap();
    let legacy_context = legacy.index_singletons.as_ref().unwrap();
    assert!(!fast_context.used_seed);
    assert!(legacy_context.used_seed);
    assert!(!fast_context.local_decisions.contains_key(&(5, 3, 1)));
    assert!(legacy_context.local_decisions.contains_key(&(5, 3, 1)));
    // The rejected query left unfinished local frames, not shared traversal
    // state. Its flags must not poison the next genuine seeded query.
    for proof in [&mut fast, &mut legacy] {
        assert_eq!(
            proof
                .index_singleton_switch_value_v1(&typed_operand(4, BOOL_TYPE), 2)
                .unwrap(),
            Some(0),
        );
        let context = proof.index_singletons.as_ref().unwrap();
        assert!(context.used_seed && !context.invalid);
    }
}

#[test]
fn index_singleton_terminal_invalid_drops_cyclic_visiting_state() {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    let mut statements = fixture.blocks[3].statements().to_vec();
    statements[0] = typed_assignment(
        5,
        U64_TYPE,
        SemanticRvalueKindV1::Binary {
            operation: SemanticBinaryOpV1::Add,
            left: typed_operand(5, U64_TYPE),
            right: typed_constant(U64_TYPE, 1, 8),
        },
    );
    fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
    let function = fixture.function();
    for stop_invalid in [false, true] {
        let mut proof =
            terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, stop_invalid);
        assert_eq!(
            proof
                .index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3)
                .unwrap(),
            None,
        );
        assert!(proof.index_singletons.as_ref().unwrap().invalid);
        assert_eq!(
            proof
                .index_singleton_switch_value_v1(&typed_operand(4, BOOL_TYPE), 2)
                .unwrap(),
            Some(0),
        );
        assert!(!proof.index_singletons.as_ref().unwrap().invalid);
    }
}

#[test]
fn index_singleton_terminal_invalid_preserves_prefix_errors_and_meters_to_refusal() {
    let fixture = terminal_singleton_invalid_tail_fixture_v1();
    let function = fixture.function();
    for stop_invalid in [false, true] {
        let mut proof =
            terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, stop_invalid);
        assert!(matches!(
            proof.index_singleton_switch_value_v1(&typed_constant(BOOL_TYPE, 2, 1), 3),
            Err(ProductionRankedProjectionErrorV1::Unsupported(
                "an unsigned scalar constant exceeds its semantic type"
            )),
        ));
        assert!(!proof.index_singletons.as_ref().unwrap().invalid);
        proof.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
        assert!(matches!(
            proof.index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3),
            Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
        ));
        assert!(!proof.index_singletons.as_ref().unwrap().invalid);
    }
    let mut measured = terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, true);
    let before = measured.work;
    assert_eq!(
        measured
            .index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3)
            .unwrap(),
        None,
    );
    let cost = measured.work - before;
    assert!(cost > 0);
    for short in [false, true] {
        let mut proof =
            terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, true);
        proof.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost + usize::from(short);
        let result = proof.index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3);
        if short {
            assert!(matches!(
                result,
                Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
            ));
        } else {
            assert_eq!(result.unwrap(), None);
            assert!(proof.index_singletons.as_ref().unwrap().invalid);
            assert_eq!(proof.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
        }
    }
    // The old traversal exhausts this same budget only after the query is
    // already invalid. Converting that discarded tail into None is intended.
    let mut legacy = terminal_singleton_proofs_v1(&fixture, &function, &fixture.types, 128, false);
    legacy.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost;
    assert!(matches!(
        legacy.index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
    ));
    assert!(legacy.index_singletons.as_ref().unwrap().invalid);
}
