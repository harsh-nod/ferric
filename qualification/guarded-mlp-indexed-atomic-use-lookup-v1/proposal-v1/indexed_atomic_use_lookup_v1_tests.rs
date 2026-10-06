// Synthetic ordered inventories test lookup equivalence, not source authority.
fn indexed_atomic_use_lookup_seed_v1() -> IndexedAtomicUseV1 {
    let (types, function) = indexed_atomic_fixture_v1(true, false);
    let (checks, allocations) = indexed_atomic_inventory_fixture_v1(&types, &function);
    authenticated_atomic_allocations_v1(&types, &function, &checks, &allocations)
        .unwrap()
        .indexed_uses[0]
        .clone()
}

fn indexed_atomic_use_lookup_fixture_v1(
    seed: &IndexedAtomicUseV1,
    sites: &[(usize, usize)],
) -> AuthenticatedAtomicAllocationsV1 {
    assert!(sites.windows(2).all(|pair| pair[0] <= pair[1]));
    AuthenticatedAtomicAllocationsV1 {
        indexed_uses: sites
            .iter()
            .map(|&(block, statement)| IndexedAtomicUseV1 {
                block,
                statement,
                ..seed.clone()
            })
            .collect(),
        ..Default::default()
    }
}

fn indexed_atomic_use_lookup_bound_for_test_v1(mut len: usize) -> usize {
    let mut comparisons = 0;
    while len != 0 {
        comparisons += 1;
        len /= 2;
    }
    comparisons
}

fn indexed_atomic_use_lookup_query_for_test_v1(
    inventory: &AuthenticatedAtomicAllocationsV1,
    block: usize,
    statement: usize,
) {
    let expected = inventory
        .indexed_uses
        .iter()
        .find(|usage| usage.block == block && usage.statement == statement);
    let before = inventory.work.get();
    let actual = inventory.statement_use(block, statement).unwrap();
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert!(std::ptr::eq(actual, expected)),
        (None, None) => {}
        _ => panic!("binary lookup changed the exact first matching use"),
    }
    assert_eq!(
        inventory.work.get() - before,
        indexed_atomic_use_lookup_bound_for_test_v1(inventory.indexed_uses.len())
    );
}

#[test]
fn indexed_atomic_use_lookup_matches_all_small_sorted_sites() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    let mut queries = 0;
    for mask in 0_u32..256 {
        let sites = (0..8)
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| (bit / 4, (bit % 4) * 2))
            .collect::<Vec<_>>();
        let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
        for block in 0..3 {
            for statement in 0..8 {
                indexed_atomic_use_lookup_query_for_test_v1(&inventory, block, statement);
                queries += 1;
            }
        }
        indexed_atomic_use_lookup_query_for_test_v1(&inventory, usize::MAX, usize::MAX);
        queries += 1;
        assert_eq!(
            inventory.work.get(),
            25 * indexed_atomic_use_lookup_bound_for_test_v1(sites.len())
        );
    }
    assert_eq!(queries, 6400);
}

#[test]
fn indexed_atomic_use_lookup_handles_boundary_lengths() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    for len in [
        0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 511, 512, 552,
        1023, 1024, 2047, 2048, 65536, MAX_PROJECTED_CAPABILITY_STATE_ENTRIES_V1 / 3,
    ] {
        let sites = (0..len)
            .map(|index| (index / 8, (index % 8) * 2))
            .collect::<Vec<_>>();
        let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
        let last = sites.last().copied().unwrap_or((0, 0));
        for (block, statement) in [
            (0, 0),
            (0, 1),
            last,
            (last.0, last.1 + 1),
            (last.0 + 1, 0),
            (0, usize::MAX),
            (usize::MAX, usize::MAX),
        ] {
            indexed_atomic_use_lookup_query_for_test_v1(&inventory, block, statement);
        }
        assert_eq!(
            inventory.work.get(),
            7 * indexed_atomic_use_lookup_bound_for_test_v1(len)
        );
        assert_eq!(inventory.indexed_uses.len(), len);
    }
}

#[test]
fn indexed_atomic_use_lookup_preserves_first_duplicate_and_extreme_sites() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    for sites in [
        vec![(0, 0)],
        vec![(usize::MAX, usize::MAX)],
        vec![(0, 0); 9],
        vec![(0, 0), (0, 0), (0, usize::MAX), (1, 0), (1, 0)],
        vec![
            (usize::MAX - 1, usize::MAX),
            (usize::MAX, 0),
            (usize::MAX, usize::MAX),
            (usize::MAX, usize::MAX),
        ],
    ] {
        let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
        for (block, statement) in sites.iter().copied().chain([
            (0, 0),
            (0, 1),
            (1, 0),
            (usize::MAX - 1, 0),
            (usize::MAX, usize::MAX - 1),
            (usize::MAX, usize::MAX),
        ]) {
            indexed_atomic_use_lookup_query_for_test_v1(&inventory, block, statement);
        }
        assert_eq!(
            inventory
                .indexed_uses
                .iter()
                .map(|usage| (usage.block, usage.statement))
                .collect::<Vec<_>>(),
            sites
        );
    }
}

#[test]
fn indexed_atomic_use_lookup_charges_exact_bound_before_query() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    for len in [1, 3, 552] {
        let sites = (0..len).map(|index| (index, 0)).collect::<Vec<_>>();
        let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
        let amount = indexed_atomic_use_lookup_bound_for_test_v1(len);
        for query in [(0, 0), (0, 1), (usize::MAX, usize::MAX)] {
            inventory.work.set(MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - amount);
            indexed_atomic_use_lookup_query_for_test_v1(&inventory, query.0, query.1);
            assert_eq!(inventory.work.get(), MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
            let error = inventory.statement_use(query.0, query.1).unwrap_err();
            assert!(matches!(
                error,
                ProductionRankedProjectionErrorV1::GraphWorkLimit(ref diagnostic)
                    if diagnostic.reason == GraphWorkFailureV1::AboveLimit
                        && diagnostic.before == MAX_PROJECTED_LOOP_GRAPH_WORK_V1
                        && diagnostic.amount == amount
                        && diagnostic.sum == Some(MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + amount)
            ));
            assert_eq!(inventory.work.get(), MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
            let before = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - amount + 1;
            inventory.work.set(before);
            assert!(inventory.statement_use(query.0, query.1).is_err());
            assert_eq!(inventory.work.get(), before);
        }
    }
    let empty = indexed_atomic_use_lookup_fixture_v1(&seed, &[]);
    empty.work.set(MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    assert!(empty.statement_use(0, 0).unwrap().is_none());
    assert_eq!(empty.work.get(), MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
}

#[test]
fn indexed_atomic_use_lookup_failed_charges_do_not_commit_work() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    for (len, reason, amount, sum) in [
        (1, GraphWorkFailureV1::Overflow, 1, None),
        (3, GraphWorkFailureV1::Overflow, 2, None),
        (0, GraphWorkFailureV1::AboveLimit, 0, Some(usize::MAX)),
    ] {
        let sites = (0..len).map(|index| (index, 0)).collect::<Vec<_>>();
        let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
        inventory.work.set(usize::MAX);
        let error = inventory.statement_use(0, 0).unwrap_err();
        assert!(matches!(
            error,
            ProductionRankedProjectionErrorV1::GraphWorkLimit(ref diagnostic)
                if diagnostic.reason == reason && diagnostic.before == usize::MAX
                    && diagnostic.amount == amount && diagnostic.sum == sum
        ));
        assert_eq!(inventory.work.get(), usize::MAX);
        assert_eq!(inventory.indexed_uses.len(), len);
    }
}

#[test]
fn indexed_atomic_use_lookup_bounds_repeated_measured_size_queries() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    let sites = (0..552).map(|index| (index / 8, index % 8)).collect::<Vec<_>>();
    let inventory = indexed_atomic_use_lookup_fixture_v1(&seed, &sites);
    assert_eq!(indexed_atomic_use_lookup_bound_for_test_v1(552), 10);
    for index in 0..8192 {
        indexed_atomic_use_lookup_query_for_test_v1(&inventory, index / 8, index % 8);
    }
    assert_eq!(inventory.work.get(), 81920);
    assert!(552 * 8192 > MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    assert_eq!(inventory.indexed_uses.len(), 552);
    assert!(inventory.indexed_locals.is_empty());
    assert!(inventory.indexed_borrows.is_empty());
    assert!(inventory.coherent.is_empty());
    assert!(inventory.atomic_only_roots.is_empty());
}

#[test]
fn indexed_atomic_use_lookup_keeps_independent_inventories() {
    let seed = indexed_atomic_use_lookup_seed_v1();
    let first = indexed_atomic_use_lookup_fixture_v1(&seed, &[(0, 0), (0, 2), (1, 0)]);
    let second = indexed_atomic_use_lookup_fixture_v1(&seed, &[(0, 1), (1, 1), (2, 1), (3, 1)]);
    let cloned = first.clone();
    indexed_atomic_use_lookup_query_for_test_v1(&first, 0, 2);
    assert_eq!(first.work.get(), 2);
    assert_eq!(second.work.get(), 0);
    assert_eq!(cloned.work.get(), 0);
    for query in [(0, 0), (0, 1), (1, 0), (3, 1), (usize::MAX, usize::MAX)] {
        indexed_atomic_use_lookup_query_for_test_v1(&second, query.0, query.1);
        indexed_atomic_use_lookup_query_for_test_v1(&cloned, query.0, query.1);
    }
    assert_eq!(first.work.get(), 2);
    assert_eq!(second.work.get(), 15);
    assert_eq!(cloned.work.get(), 10);
}

#[test]
fn indexed_atomic_use_lookup_real_inventory_keeps_exact_statement_binding() {
    let (types, function) = indexed_atomic_fixture_v1(true, false);
    let (checks, allocations) = indexed_atomic_inventory_fixture_v1(&types, &function);
    let inventory =
        authenticated_atomic_allocations_v1(&types, &function, &checks, &allocations).unwrap();
    assert_eq!(inventory.indexed_uses.len(), 2);
    assert!(inventory.indexed_uses.windows(2).all(|pair| {
        (pair[0].block, pair[0].statement) < (pair[1].block, pair[1].statement)
    }));
    for (block, body) in function.blocks().iter().enumerate() {
        for (position, record) in body.statements().iter().enumerate() {
            indexed_atomic_use_lookup_query_for_test_v1(&inventory, block, position);
            inventory.validate_statement(block, position, record).unwrap();
        }
    }
    inventory
        .validate_statement(usize::MAX, usize::MAX, &statement(SemanticStatementKindV1::Nop))
        .unwrap();
    for (index, usage) in inventory.indexed_uses.iter().enumerate() {
        let record = &function.blocks()[usage.block].statements()[usage.statement];
        assert!(matches!(
            inventory.validate_statement(
                usage.block,
                usage.statement,
                &statement(SemanticStatementKindV1::Nop),
            ),
            Err(ProductionRankedProjectionErrorV1::Unsupported(
                "an indexed atomic use lost its exact semantic statement"
            ))
        ));
        for changed in 0..4 {
            let mut mutant = inventory.clone();
            let expected = &mut mutant.indexed_uses[index];
            match changed {
                0 => {
                    expected.address = SemanticPlaceV1::new(
                        SemanticLocalIdV1::from_index(u32::MAX),
                        vec![],
                        expected.address.ty(),
                    )
                    .unwrap();
                }
                1 => {
                    expected.access = if expected.access == AccessKindAttr::AtomicRead {
                        AccessKindAttr::AtomicWrite
                    } else {
                        AccessKindAttr::AtomicRead
                    };
                }
                2 => {
                    expected.atomic = SemanticAtomicAccessV1::new(
                        if expected.atomic.ordering() == SemanticAtomicOrderingV1::Relaxed {
                            SemanticAtomicOrderingV1::Acquire
                        } else {
                            SemanticAtomicOrderingV1::Relaxed
                        },
                        expected.atomic.scope(),
                    );
                }
                3 => {
                    expected.atomic = SemanticAtomicAccessV1::new(
                        expected.atomic.ordering(),
                        SemanticAtomicScopeV1::Agent,
                    );
                }
                _ => unreachable!(),
            }
            assert!(matches!(
                mutant.validate_statement(usage.block, usage.statement, record),
                Err(ProductionRankedProjectionErrorV1::Unsupported(
                    "an indexed atomic statement changed its exact address, kind, ordering, or scope"
                ))
            ));
        }
    }
}

#[test]
fn indexed_atomic_use_lookup_real_inventory_keeps_atomic_refusals() {
    for (marker, escape) in [(false, false), (true, true)] {
        let (types, function) = indexed_atomic_fixture_v1(marker, escape);
        let (checks, allocations) = indexed_atomic_inventory_fixture_v1(&types, &function);
        let inventory =
            authenticated_atomic_allocations_v1(&types, &function, &checks, &allocations).unwrap();
        assert!(inventory.indexed_uses.iter().all(|usage| usage.block != 1));
        for position in 0..function.blocks()[1].statements().len() {
            indexed_atomic_use_lookup_query_for_test_v1(&inventory, 1, position);
        }
    }
    let (types, function) = indexed_atomic_fixture_v1(true, false);
    let (checks, allocations) = indexed_atomic_inventory_fixture_v1(&types, &function);
    let missing = authenticated_atomic_allocations_v1(&types, &function, &[], &allocations).unwrap();
    assert!(missing.indexed_uses.iter().all(|usage| usage.block != 1));
    let mut duplicate = checks.clone();
    duplicate.push(checks[0]);
    assert!(
        authenticated_atomic_allocations_v1(&types, &function, &duplicate, &allocations).is_err()
    );
    let mut statements = function.blocks()[1].statements().to_vec();
    statements.push(statement(SemanticStatementKindV1::Assign(
        SemanticAssignmentV1::new(
            SemanticPlaceV1::new(
                SemanticLocalIdV1::from_index(11),
                vec![],
                SemanticTypeIdV1::from_index(6),
            )
            .unwrap(),
            SemanticRvalueV1::new(
                SemanticTypeIdV1::from_index(6),
                SemanticRvalueKindV1::Cast {
                    kind: SemanticCastKindV1::Pointer,
                    operand: SemanticOperandV1::Copy(
                        SemanticPlaceV1::new(
                            SemanticLocalIdV1::from_index(7),
                            vec![],
                            SemanticTypeIdV1::from_index(7),
                        )
                        .unwrap(),
                    ),
                },
            ),
        ),
    )));
    let escaped = indexed_atomic_replace_block_v1(&function, 1, statements);
    assert!(authenticated_atomic_allocations_v1(&types, &escaped, &checks, &allocations).is_err());
}
