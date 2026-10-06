// Component projection tests, not source admission or host launch authority.
struct SingletonIndexFixtureV1 {
    types: Vec<SemanticTypeDeclV1>,
    callables: Vec<SemanticCallableDeclV1>,
    locals: Vec<SemanticLocalDeclV1>,
    blocks: Vec<SemanticBasicBlockV1>,
}

impl SingletonIndexFixtureV1 {
    fn new(cutoff: u64) -> Self {
        let mut types = assertion_proof_types();
        let witness = SemanticTypeIdV1::from_index(types.len() as u32);
        types.push(SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(220)),
            SemanticLayoutIdentityV1::from_sha256(bytes(220)),
            SemanticTypeLayoutV1::new(Some(8), 8).unwrap(),
            SemanticTypeShapeV1::Aggregate(SemanticAggregateTypeV1::new(vec![U64_TYPE]).unwrap()),
        ));
        let reference = SemanticTypeIdV1::from_index(types.len() as u32);
        types.push(SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(221)),
            SemanticLayoutIdentityV1::from_sha256(bytes(221)),
            SemanticTypeLayoutV1::new(Some(8), 8).unwrap(),
            SemanticTypeShapeV1::Pointer(
                SemanticPointerTypeV1::new_with_kind(
                    witness,
                    SemanticPointerKindV1::Reference,
                    SemanticMutabilityV1::Immutable,
                    0,
                    64,
                    SemanticPointerMetadataV1::None,
                )
                .unwrap(),
            ),
        ));
        let call = |callee, arguments, destination, ty, target| {
            SemanticTerminatorKindV1::Call(
                SemanticDirectCallV1::new_callable(
                    SemanticCallableIdV1::from_index(callee),
                    arguments,
                    Some(SemanticCallDestinationV1::new(
                        typed_place(destination, ty),
                        cfg_edge(SemanticEdgeRoleV1::CallReturn, target),
                    )),
                    SemanticUnwindActionV1::Unreachable,
                )
                .unwrap(),
            )
        };
        let callables = vec![
            compiler_intrinsic_callable(SemanticCompilerIntrinsicOperationV1::ThreadIndex1d {
                index_witness: witness,
                raw_index: U64_TYPE,
            }),
            compiler_intrinsic_callable(SemanticCompilerIntrinsicOperationV1::ThreadIndexGet {
                index_witness: witness,
                raw_index: U64_TYPE,
            }),
        ];
        let locals = [
            SCALAR_TYPE,
            witness,
            reference,
            U64_TYPE,
            BOOL_TYPE,
            U64_TYPE,
            BOOL_TYPE,
            U64_TYPE,
            U64_POINTER_TYPE,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, ty)| {
            local(
                220 + i as u8,
                ty,
                if i == 0 {
                    SemanticLocalRoleV1::Return
                } else {
                    SemanticLocalRoleV1::Temporary
                },
            )
        })
        .collect();
        let blocks = vec![
            block(230, vec![], call(0, vec![], 1, witness, 1)),
            block(
                231,
                vec![typed_assignment(
                    2,
                    reference,
                    SemanticRvalueKindV1::Borrow {
                        kind: SemanticBorrowKindV1::Shared,
                        place: typed_place(1, witness),
                    },
                )],
                call(1, vec![typed_operand(2, reference)], 3, U64_TYPE, 2),
            ),
            block(
                232,
                vec![typed_assignment(
                    4,
                    BOOL_TYPE,
                    SemanticRvalueKindV1::Binary {
                        operation: SemanticBinaryOpV1::GreaterOrEqual,
                        left: typed_operand(3, U64_TYPE),
                        right: typed_constant(U64_TYPE, cutoff.into(), 8),
                    },
                )],
                zero_switch(4, BOOL_TYPE, 3, 4),
            ),
            block(
                233,
                vec![
                    typed_assignment(
                        5,
                        U64_TYPE,
                        SemanticRvalueKindV1::Binary {
                            operation: SemanticBinaryOpV1::Remainder,
                            left: typed_operand(3, U64_TYPE),
                            right: typed_constant(U64_TYPE, 64, 8),
                        },
                    ),
                    typed_assignment(
                        6,
                        BOOL_TYPE,
                        SemanticRvalueKindV1::Binary {
                            operation: SemanticBinaryOpV1::GreaterOrEqual,
                            left: typed_operand(5, U64_TYPE),
                            right: typed_constant(U64_TYPE, 64, 8),
                        },
                    ),
                ],
                zero_switch(6, BOOL_TYPE, 4, 5),
            ),
            block(234, vec![], SemanticTerminatorKindV1::Return),
            block(235, vec![], SemanticTerminatorKindV1::Return),
        ];
        Self {
            types,
            callables,
            locals,
            blocks,
        }
    }

    fn function(&self) -> SemanticFunctionDeclV1 {
        projection_function_with_locals(self.blocks.clone(), self.locals.clone())
    }

    fn indices(&self) -> Vec<Option<ProjectedDisjointIndexV1>> {
        let mut indices = vec![None; self.locals.len()];
        for local in [1, 2, 3] {
            indices[local] = Some(ProjectedDisjointIndexV1 {
                value: ProductionRankedValueV1::Local(ProductionRankedValueIdV1::new(0)),
                mapping: SemanticDisjointIndexSpaceV1::Index1d,
                precondition: None,
                availability: None,
            });
        }
        indices
    }

    fn query(&self, upper: Option<u64>, block: usize, local: u32) -> Option<u64> {
        let function = self.function();
        index_singleton_switch_proofs_v1(
            &self.types,
            &self.callables,
            &function,
            &self.indices(),
            upper,
        )
        .unwrap()
        .and_then(|mut proofs| {
            proofs
                .index_singleton_switch_value_v1(&typed_operand(local, BOOL_TYPE), block)
                .unwrap()
        })
    }
}

#[test]
fn index_singleton_switch_projects_global_and_lane_guards() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let mut operations = Vec::new();
    let mut next = 0;
    let mut text = String::new();
    let projected = project_intrinsic_contracts_with_wave_v1(
        SemanticFunctionIdV1::from_index(0),
        &fixture.callables,
        &DefinedCallableEmptyEffectSummariesV1 {
            decisions: Box::new([]),
        },
        &fixture.types,
        &function,
        Some(128),
        &constant_locals(&function).unwrap(),
        &mut operations,
        &mut next,
        &mut text,
        None,
        None,
    )
    .unwrap();
    for (site, success, failure) in [(2, 3, 4), (3, 4, 5)] {
        let switch = projected.deterministic_switches[site].as_ref().unwrap();
        assert!(switch.lane_uniform);
        assert_eq!(switch.targets.len(), 1);
        assert_eq!(switch.targets[0].0, 0);
        assert_eq!(switch.targets[0].2, success);
        assert_eq!(switch.otherwise, failure);
        assert!(operations.iter().any(|operation| matches!(operation,
            ProductionRankedOperationV1::IndexConstant { result, value: 0 }
                if switch.discriminant == ProductionRankedValueV1::Local(*result))));
    }
    assert_eq!(
        operations
            .iter()
            .filter(|operation| matches!(
                operation,
                ProductionRankedOperationV1::InvocationIndex {
                    dimension: 0,
                    launch_extent: 0,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(!operations.iter().any(|operation| matches!(
        operation,
        ProductionRankedOperationV1::DeterministicJoin { .. }
    )));
    assert_eq!(fixture.query(Some(128), 2, 4), Some(0));
    assert_eq!(fixture.query(Some(128), 3, 6), Some(0));
}

#[test]
fn index_singleton_switch_requires_a_bounded_singleton_domain() {
    let fixture = SingletonIndexFixtureV1::new(128);
    for upper in [None, Some(0), Some(129), Some(u64::MAX)] {
        assert_eq!(fixture.query(upper, 2, 4), None, "upper={upper:?}");
    }
    assert_eq!(
        SingletonIndexFixtureV1::new(127).query(Some(128), 2, 4),
        None
    );
    assert_eq!(fixture.query(Some(64), 2, 4), Some(0));
    let function = fixture.function();
    let constants = constant_locals(&function).unwrap();
    let mut slots = vec![None; fixture.locals.len()];
    let mut argument = 1;
    let mut operations = vec![ProductionRankedOperationV1::InvocationIndex {
        result: ProductionRankedValueIdV1::new(0),
        dimension: 0,
        launch_extent: 0,
    }];
    let mut next = 1;
    let switches = project_deterministic_scalar_switches_with_index_bounds_v1(
        &fixture.types,
        &fixture.callables,
        &DefinedCallableEmptyEffectSummariesV1 {
            decisions: Box::new([]),
        },
        &function,
        &constants,
        &local_definition_counts(&function),
        &vec![None; fixture.locals.len()],
        &fixture.indices(),
        &vec![None; fixture.locals.len()],
        &vec![None; fixture.locals.len()],
        &mut slots,
        &mut argument,
        &mut operations,
        &mut next,
        None,
    )
    .unwrap();
    assert!(operations.iter().any(|operation| matches!(operation,
        ProductionRankedOperationV1::DeterministicJoin { result, .. }
            if switches[2].as_ref().unwrap().discriminant == ProductionRankedValueV1::Local(*result))));
}

#[test]
fn index_singleton_switch_requires_exact_unconditioned_custody() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let (option_function, producers) = option_dominance_chain(1);
    let dominance = SemanticOptionDominanceV1::analyze(&option_function, &producers).unwrap();
    let availability = CapabilityAvailabilityV1::Option(
        dominance.availability(producers[0].option_local()).unwrap(),
    );
    for mutation in 0..10 {
        let mut indices = fixture.indices();
        let index = indices[3].as_mut().unwrap();
        match mutation {
            0 => {
                index.mapping = SemanticDisjointIndexSpaceV1::BlockedIndex1d {
                    lanes_per_block: 64,
                    elements_per_lane: 2,
                }
            }
            1 => index.precondition = Some((index.value, index.value)),
            2 => index.value = ProductionRankedValueV1::Argument(0),
            3 => indices[2] = None,
            4 => indices[3] = None,
            5 => index.availability = Some(availability),
            6 => {
                indices[2].as_mut().unwrap().mapping =
                    SemanticDisjointIndexSpaceV1::BlockedIndex1d {
                        lanes_per_block: 64,
                        elements_per_lane: 2,
                    }
            }
            7 => {
                let receiver = indices[2].as_mut().unwrap();
                receiver.precondition = Some((receiver.value, receiver.value));
            }
            8 => indices[2].as_mut().unwrap().availability = Some(availability),
            9 => indices[2].as_mut().unwrap().value = ProductionRankedValueV1::Argument(0),
            _ => unreachable!(),
        }
        assert!(
            index_singleton_switch_proofs_v1(
                &fixture.types,
                &fixture.callables,
                &function,
                &indices,
                Some(128)
            )
            .unwrap()
            .is_none(),
            "mutation={mutation}"
        );
    }
    let mut mismatched = SingletonIndexFixtureV1::new(128);
    mismatched.callables[1] =
        compiler_intrinsic_callable(SemanticCompilerIntrinsicOperationV1::ThreadIndexGet {
            index_witness: U64_TYPE,
            raw_index: U64_TYPE,
        });
    assert_eq!(mismatched.query(Some(128), 2, 4), None);
    mismatched.callables[1] =
        compiler_intrinsic_callable(SemanticCompilerIntrinsicOperationV1::ThreadIndexGet {
            index_witness: U64_TYPE,
            raw_index: SCALAR_TYPE,
        });
    assert_eq!(mismatched.query(Some(128), 2, 4), None);
}

#[test]
fn index_singleton_switch_rejects_stale_and_exposed_values() {
    for mutation in 0..7 {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        let mut statements = fixture.blocks[2].statements().to_vec();
        let kill = match mutation {
            0 => statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(3),
            )),
            1 => statement(SemanticStatementKindV1::StorageLive(
                SemanticLocalIdV1::from_index(3),
            )),
            2 => typed_assignment(
                7,
                U64_TYPE,
                SemanticRvalueKindV1::Use(SemanticOperandV1::Move(typed_place(3, U64_TYPE))),
            ),
            3 => typed_assignment(
                3,
                U64_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, 0, 8)),
            ),
            4 => typed_assignment(
                8,
                U64_POINTER_TYPE,
                SemanticRvalueKindV1::Borrow {
                    kind: SemanticBorrowKindV1::Mutable,
                    place: typed_place(3, U64_TYPE),
                },
            ),
            5 => statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(4),
            )),
            6 => statement(SemanticStatementKindV1::StorageLive(
                SemanticLocalIdV1::from_index(4),
            )),
            _ => unreachable!(),
        };
        if mutation >= 5 {
            statements.push(kill)
        } else {
            statements.insert(0, kill)
        }
        fixture.blocks[2] = block(232, statements, zero_switch(4, BOOL_TYPE, 3, 4));
        assert_eq!(fixture.query(Some(128), 2, 4), None, "mutation={mutation}");
    }
    // One branch reaches the comparison without executing the issuing call.
    let mut bypass = SingletonIndexFixtureV1::new(128);
    bypass.blocks[0] = block(230, vec![], zero_switch(0, SCALAR_TYPE, 1, 2));
    assert_eq!(bypass.query(Some(128), 2, 4), None);

    // The invariant raw index may be read repeatedly; a later lifetime kill on
    // the backedge must still invalidate the earlier-looking use.
    for kill in [false, true] {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        fixture.blocks[4] = block(
            234,
            if kill {
                vec![statement(SemanticStatementKindV1::StorageDead(
                    SemanticLocalIdV1::from_index(3),
                ))]
            } else {
                vec![]
            },
            SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 2)),
        );
        assert_eq!(
            fixture.query(Some(128), 2, 4),
            if kill { None } else { Some(0) }
        );
    }

    // The model also consumes Move operands in non-call terminators. Neither
    // a switch nor an assertion-message move may survive a use-loop backedge.
    for assertion in [false, true] {
        for moved in [false, true] {
            let mut fixture = SingletonIndexFixtureV1::new(128);
            let operand = if moved {
                SemanticOperandV1::Move(typed_place(3, U64_TYPE))
            } else {
                typed_operand(3, U64_TYPE)
            };
            let terminator = if assertion {
                SemanticTerminatorKindV1::Assert {
                    condition: typed_constant(BOOL_TYPE, 1, 1),
                    expected: true,
                    message: SemanticAssertMessageV1::DivisionByZero(operand),
                    target: cfg_edge(SemanticEdgeRoleV1::AssertSuccess, 2),
                    unwind: SemanticUnwindActionV1::Unreachable,
                }
            } else {
                SemanticTerminatorKindV1::SwitchInt {
                    discriminant: operand,
                    targets: SemanticSwitchTargetsV1::new(
                        vec![SemanticSwitchTargetV1::new(
                            0,
                            cfg_edge(SemanticEdgeRoleV1::SwitchValue, 2),
                        )],
                        cfg_edge(SemanticEdgeRoleV1::SwitchOtherwise, 5),
                    )
                    .unwrap(),
                }
            };
            fixture.blocks[4] = block(234, vec![], terminator);
            assert_eq!(
                fixture.query(Some(128), 2, 4),
                if moved { None } else { Some(0) },
                "assertion={assertion}, moved={moved}"
            );
        }
    }
}

#[test]
fn index_singleton_switch_charges_shared_range_and_freshness_work() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let indices = fixture.indices();
    let make = || {
        index_singleton_switch_proofs_v1(
            &fixture.types,
            &fixture.callables,
            &function,
            &indices,
            Some(128),
        )
        .unwrap()
        .unwrap()
    };
    let mut measured = make();
    let before = measured.work;
    assert_eq!(
        measured
            .index_singleton_switch_value_v1(&typed_operand(4, BOOL_TYPE), 2)
            .unwrap(),
        Some(0)
    );
    let cost = measured.work - before;
    assert!(cost > function.blocks().len());
    let first = measured.work;
    assert_eq!(
        measured
            .index_singleton_switch_value_v1(&typed_operand(6, BOOL_TYPE), 3)
            .unwrap(),
        Some(0)
    );
    assert!(measured.work > first);
    for short in [false, true] {
        let mut proofs = make();
        proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost + usize::from(short);
        let result = proofs.index_singleton_switch_value_v1(&typed_operand(4, BOOL_TYPE), 2);
        if short {
            assert!(matches!(
                result,
                Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
            ));
        } else {
            assert_eq!(result.unwrap(), Some(0));
            assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
        }
    }
}

fn singleton_local_cache_proofs_v1<'a>(
    fixture: &'a SingletonIndexFixtureV1,
    function: &'a SemanticFunctionDeclV1,
    cached: bool,
) -> SemanticAssertProofsV1<'a> {
    let mut proofs = index_singleton_switch_proofs_v1(
        &fixture.types,
        &fixture.callables,
        function,
        &fixture.indices(),
        Some(128),
    )
    .unwrap()
    .unwrap();
    proofs
        .index_singletons
        .as_mut()
        .unwrap()
        .cache_local_decisions = cached;
    proofs
}

#[test]
fn index_singleton_local_cache_matches_uncached_ranges_and_flags() {
    for mutation in 0..5 {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        match mutation {
            0 => {}
            1 => {
                let mut statements = fixture.blocks[2].statements().to_vec();
                statements.insert(
                    0,
                    statement(SemanticStatementKindV1::StorageDead(
                        SemanticLocalIdV1::from_index(3),
                    )),
                );
                fixture.blocks[2] = block(232, statements, zero_switch(4, BOOL_TYPE, 3, 4));
            }
            2 => {
                fixture.blocks[4] = block(
                    234,
                    vec![statement(SemanticStatementKindV1::StorageLive(
                        SemanticLocalIdV1::from_index(3),
                    ))],
                    SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 2)),
                );
            }
            3 => {
                let mut statements = fixture.blocks[3].statements().to_vec();
                statements[0] = typed_assignment(
                    5,
                    U64_TYPE,
                    SemanticRvalueKindV1::Use(typed_operand(5, U64_TYPE)),
                );
                fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
            }
            4 => {
                let mut statements = fixture.blocks[3].statements().to_vec();
                statements.push(typed_assignment(
                    8,
                    U64_POINTER_TYPE,
                    SemanticRvalueKindV1::Borrow {
                        kind: SemanticBorrowKindV1::Mutable,
                        place: typed_place(5, U64_TYPE),
                    },
                ));
                fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
            }
            _ => unreachable!(),
        }
        let function = fixture.function();
        let mut cached = singleton_local_cache_proofs_v1(&fixture, &function, true);
        let mut uncached = singleton_local_cache_proofs_v1(&fixture, &function, false);
        for (block, local) in [(2, 4), (3, 6), (3, 6), (2, 4)] {
            let operand = typed_operand(local, BOOL_TYPE);
            let actual = cached
                .index_singleton_switch_value_v1(&operand, block)
                .unwrap();
            let expected = uncached
                .index_singleton_switch_value_v1(&operand, block)
                .unwrap();
            assert_eq!(actual, expected, "mutation={mutation}, block={block}");
            let actual_flags = cached.index_singletons.as_ref().unwrap();
            let expected_flags = uncached.index_singletons.as_ref().unwrap();
            assert_eq!(actual_flags.used_seed, expected_flags.used_seed);
            assert_eq!(actual_flags.invalid, expected_flags.invalid);
            if mutation == 0 {
                assert_eq!(actual, Some(0));
            } else if mutation <= 2 || block == 3 {
                assert_eq!(actual, None);
            }
        }
        if mutation == 0 {
            assert!(cached.work < uncached.work);
        }
    }
}

#[test]
fn index_singleton_local_cache_keys_include_local_block_and_statement() {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    let mut statements = fixture.blocks[2].statements().to_vec();
    statements.push(statement(SemanticStatementKindV1::StorageDead(
        SemanticLocalIdV1::from_index(3),
    )));
    fixture.blocks[2] = block(232, statements, zero_switch(4, BOOL_TYPE, 3, 4));
    let function = fixture.function();
    let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
    let seed = IndexSingletonLocalV1::Seed(UnsignedRangeProofV1 {
        minimum: 0,
        maximum: 127,
    });
    for (local, block, statement, expected) in [
        (3, 2, 1, seed),
        (4, 2, 1, IndexSingletonLocalV1::Assignment),
        (3, 2, 2, IndexSingletonLocalV1::Unavailable),
        (3, 3, 1, IndexSingletonLocalV1::Unavailable),
        (3, 2, 1, seed),
    ] {
        assert_eq!(
            proofs
                .index_singleton_local_v1(local, ScalarAssignmentSiteV1 { block, statement })
                .unwrap(),
            expected,
        );
    }
    let keys: HashSet<_> = proofs
        .index_singletons
        .as_ref()
        .unwrap()
        .local_decisions
        .keys()
        .copied()
        .collect();
    assert_eq!(
        keys,
        HashSet::from([(3, 2, 1), (4, 2, 1), (3, 2, 2), (3, 3, 1)])
    );
}

#[test]
fn index_singleton_local_cache_replays_flags_without_clearing_prior_effects() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
    let site = ScalarAssignmentSiteV1 {
        block: 2,
        statement: 1,
    };
    assert!(matches!(
        proofs.index_singleton_local_v1(3, site).unwrap(),
        IndexSingletonLocalV1::Seed(_)
    ));
    assert_eq!(
        proofs.index_singleton_local_v1(7, site).unwrap(),
        IndexSingletonLocalV1::Unavailable
    );
    {
        let context = proofs.index_singletons.as_mut().unwrap();
        context.used_seed = false;
        context.invalid = true;
    }
    assert!(matches!(
        proofs.index_singleton_local_v1(3, site).unwrap(),
        IndexSingletonLocalV1::Seed(_)
    ));
    assert!(proofs.index_singletons.as_ref().unwrap().used_seed);
    assert!(proofs.index_singletons.as_ref().unwrap().invalid);
    proofs.index_singletons.as_mut().unwrap().invalid = false;
    assert_eq!(
        proofs.index_singleton_local_v1(7, site).unwrap(),
        IndexSingletonLocalV1::Unavailable
    );
    assert!(proofs.index_singletons.as_ref().unwrap().used_seed);
    assert!(proofs.index_singletons.as_ref().unwrap().invalid);
}

#[test]
fn index_singleton_local_cache_has_bounded_storage_and_preserves_prior_entries() {
    let mut cache = HashMap::new();
    let decision = IndexSingletonLocalDecisionV1 {
        result: IndexSingletonLocalV1::Assignment,
        used_seed: false,
        invalid: false,
    };
    assert!(insert_index_singleton_local_decision_v1(&mut cache, (3, 2, 0), decision, 0).is_err());
    assert!(cache.is_empty());
    insert_index_singleton_local_decision_v1(&mut cache, (3, 2, 0), decision, 1).unwrap();
    insert_index_singleton_local_decision_v1(&mut cache, (3, 2, 0), decision, 1).unwrap();
    let before = cache.clone();
    assert!(matches!(
        insert_index_singleton_local_decision_v1(&mut cache, (3, 2, 1), decision, 1),
        Err(ProductionRankedProjectionErrorV1::Unsupported(
            "index singleton local proof cache exceeds the bounded entry limit"
        ))
    ));
    assert_eq!(cache, before);
}

#[test]
fn index_singleton_local_cache_never_publishes_a_work_exhausted_decision() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 2,
        statement: 1,
    };
    let mut measured = singleton_local_cache_proofs_v1(&fixture, &function, true);
    let before = measured.work;
    let expected = measured.index_singleton_local_v1(3, site).unwrap();
    let cost = measured.work - before;
    for short in [false, true] {
        let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
        proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost + usize::from(short);
        let result = proofs.index_singleton_local_v1(3, site);
        if short {
            assert!(matches!(
                result,
                Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
            ));
            let context = proofs.index_singletons.as_ref().unwrap();
            assert!(context.local_decisions.is_empty());
            assert!(!context.used_seed);
            assert!(!context.invalid);
        } else {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
            assert_eq!(
                proofs
                    .index_singletons
                    .as_ref()
                    .unwrap()
                    .local_decisions
                    .len(),
                1
            );
        }
    }
    // A warm hit is still charged before its result or flag effects are visible.
    measured.index_singletons.as_mut().unwrap().used_seed = false;
    measured.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
    assert!(measured.index_singleton_local_v1(3, site).is_err());
    assert!(!measured.index_singletons.as_ref().unwrap().used_seed);
    measured.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - 1;
    assert_eq!(
        measured.index_singleton_local_v1(3, site).unwrap(),
        expected
    );
    assert_eq!(measured.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
}

#[test]
fn index_singleton_local_cache_reuses_a_nested_site_across_distinct_switches() {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    fixture
        .locals
        .push(local(229, BOOL_TYPE, SemanticLocalRoleV1::Temporary));
    // Both comparisons read the remainder assigned in block 3. Its raw-index
    // operand is reconstructed at (block 3, statement 0), irrespective of which
    // later switch requested the range. The block-2 failure bypasses both uses.
    fixture.blocks[2] = block(
        232,
        fixture.blocks[2].statements().to_vec(),
        zero_switch(4, BOOL_TYPE, 3, 5),
    );
    fixture.blocks[4] = block(
        234,
        vec![typed_assignment(
            9,
            BOOL_TYPE,
            SemanticRvalueKindV1::Binary {
                operation: SemanticBinaryOpV1::GreaterOrEqual,
                left: typed_operand(5, U64_TYPE),
                right: typed_constant(U64_TYPE, 64, 8),
            },
        )],
        zero_switch(9, BOOL_TYPE, 5, 6),
    );
    fixture
        .blocks
        .push(block(236, vec![], SemanticTerminatorKindV1::Return));

    let function = fixture.function();
    let mut cached = singleton_local_cache_proofs_v1(&fixture, &function, true);
    let mut uncached = singleton_local_cache_proofs_v1(&fixture, &function, false);
    let cached_start = cached.work;
    let uncached_start = uncached.work;
    // Each switch is queried exactly once, matching the production projection
    // pass; no repeated top-level query is needed to exercise this cache hit.
    for (block, local) in [(3, 6), (4, 9)] {
        let operand = typed_operand(local, BOOL_TYPE);
        let actual = cached
            .index_singleton_switch_value_v1(&operand, block)
            .unwrap();
        let expected = uncached
            .index_singleton_switch_value_v1(&operand, block)
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual, Some(0));
        let actual_context = cached.index_singletons.as_ref().unwrap();
        let expected_context = uncached.index_singletons.as_ref().unwrap();
        assert_eq!(actual_context.used_seed, expected_context.used_seed);
        assert_eq!(actual_context.invalid, expected_context.invalid);
        assert!(actual_context.used_seed);
        assert!(!actual_context.invalid);
        assert_eq!(
            actual_context.local_decisions.get(&(3, 3, 0)),
            Some(&IndexSingletonLocalDecisionV1 {
                result: IndexSingletonLocalV1::Seed(UnsignedRangeProofV1 {
                    minimum: 0,
                    maximum: 127,
                }),
                used_seed: true,
                invalid: false,
            }),
        );
    }
    assert!(cached.work - cached_start < uncached.work - uncached_start);
    assert!(
        uncached
            .index_singletons
            .as_ref()
            .unwrap()
            .local_decisions
            .is_empty()
    );
}

#[test]
fn index_singleton_ineligible_locals_skip_only_unused_graph_precharge() {
    for mutation in 0..4 {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        let mut statements = fixture.blocks[3].statements().to_vec();
        let queried_local = if mutation == 0 { 7 } else { 5 };
        match mutation {
            0 => {}
            1 => {
                statements[0] = typed_assignment(
                    5,
                    U64_TYPE,
                    SemanticRvalueKindV1::Cast {
                        kind: SemanticCastKindV1::Integer,
                        operand: typed_operand(3, U64_TYPE),
                    },
                );
            }
            2 => {
                // Malformed private helper fixture, not admitted semantic MIR.
                statements[0] = typed_assignment(
                    5,
                    U64_TYPE,
                    SemanticRvalueKindV1::Use(typed_constant(BOOL_TYPE, 1, 1)),
                );
            }
            3 => statements.push(typed_assignment(
                5,
                U64_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, 0, 8)),
            )),
            _ => unreachable!(),
        }
        let site = ScalarAssignmentSiteV1 {
            block: 3,
            statement: statements.len(),
        };
        fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
        let function = fixture.function();
        let mut direct = singleton_local_cache_proofs_v1(&fixture, &function, true);
        let before = direct.work;
        assert_eq!(
            direct
                .index_singleton_local_uncached_v1(queried_local, site)
                .unwrap(),
            IndexSingletonLocalDecisionV1 {
                result: IndexSingletonLocalV1::Unavailable,
                used_seed: false,
                invalid: true,
            },
            "mutation={mutation}",
        );
        assert_eq!(direct.work, before);
        assert!(direct.dominance.is_empty());
        for cached in [false, true] {
            let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, cached);
            proofs.index_singletons.as_mut().unwrap().used_seed = true;
            let before = proofs.work;
            assert_eq!(
                proofs
                    .index_singleton_local_v1(queried_local, site)
                    .unwrap(),
                IndexSingletonLocalV1::Unavailable,
            );
            assert_eq!(proofs.work - before, if cached { 2 } else { 0 });
            let context = proofs.index_singletons.as_ref().unwrap();
            assert!(context.used_seed);
            assert!(context.invalid);
            assert_eq!(context.local_decisions.len(), usize::from(cached));
        }
    }
}

#[test]
fn index_singleton_non_assignment_inventory_keeps_its_distinct_refusal_flags() {
    let mut fixture = SingletonIndexFixtureV1::new(128);
    let mut statements = fixture.blocks[3].statements().to_vec();
    let marker = statements.len();
    statements.push(statement(SemanticStatementKindV1::StorageLive(
        SemanticLocalIdV1::from_index(7),
    )));
    fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: marker + 1,
    };
    let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
    // Deliberately inconsistent private inventory exercises the pre-existing
    // non-Assign refusal. This does not construct a checked source owner.
    proofs.assignments[7] = Some(ScalarAssignmentSiteV1 {
        block: 3,
        statement: marker,
    });
    let before = proofs.work;
    assert_eq!(
        proofs.index_singleton_local_uncached_v1(7, site).unwrap(),
        IndexSingletonLocalDecisionV1 {
            result: IndexSingletonLocalV1::Unavailable,
            used_seed: false,
            invalid: false,
        },
    );
    assert_eq!(proofs.work, before);
    assert_eq!(
        proofs.index_singleton_local_v1(7, site).unwrap(),
        IndexSingletonLocalV1::Unavailable
    );
    let context = proofs.index_singletons.as_ref().unwrap();
    assert!(!context.used_seed && !context.invalid);
    assert_eq!(context.local_decisions.len(), 1);
}

#[test]
fn index_singleton_eligible_seed_and_cross_block_assignment_keep_the_full_precharge() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: 2,
    };
    let graph_precharge = function.blocks().len() * 4;
    for (local, site) in [
        (3, site),
        (
            5,
            ScalarAssignmentSiteV1 {
                block: 4,
                statement: 0,
            },
        ),
    ] {
        let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
        proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - graph_precharge + 1;
        assert!(matches!(
            proofs.index_singleton_local_uncached_v1(local, site),
            Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
        ));
        assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1);
        assert!(proofs.dominance.is_empty());
        let context = proofs.index_singletons.as_ref().unwrap();
        assert!(!context.used_seed && !context.invalid);
        assert!(context.local_decisions.is_empty());
    }
}

#[test]
fn index_singleton_same_block_freshness_matches_graph_oracle_for_all_small_graphs() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    // Every directed graph on blocks3..5, including self-loops, disconnected
    // vertices and cycles. These are private graph-oracle queries, not admission.
    for edges in 0u16..512 {
        let mut current = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut successors = vec![Vec::new(); function.blocks().len()];
        let mut predecessors = successors.clone();
        for from in 0..3 {
            for to in 0..3 {
                if edges & (1 << (from * 3 + to)) != 0 {
                    successors[from + 3].push(to + 3);
                    predecessors[to + 3].push(from + 3);
                }
            }
        }
        for proof in [&mut current, &mut legacy] {
            proof.graph.successors = successors.clone();
            proof.graph.predecessors = predecessors.clone();
        }
        let site = ScalarAssignmentSiteV1 {
            block: 3,
            statement: 2,
        };
        assert_eq!(
            current.index_singleton_fresh_v1(5, 3, 1, 3, site).unwrap(),
            legacy
                .index_singleton_fresh_graph_v1(5, 3, 1, 3, site)
                .unwrap(),
            "graph={edges}"
        );
        assert!(current.work < legacy.work);
    }
}

#[test]
fn index_singleton_same_block_freshness_keeps_every_in_slice_kill() {
    for kind in 0..5 {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        let kill = match kind {
            0 => statement(SemanticStatementKindV1::StorageLive(
                SemanticLocalIdV1::from_index(5),
            )),
            1 => statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(5),
            )),
            2 => typed_assignment(
                7,
                U64_TYPE,
                SemanticRvalueKindV1::Use(SemanticOperandV1::Move(typed_place(5, U64_TYPE))),
            ),
            3 => typed_assignment(
                5,
                U64_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, 0, 8)),
            ),
            4 => statement(SemanticStatementKindV1::Deinitialize(typed_place(
                5, U64_TYPE,
            ))),
            _ => unreachable!(),
        };
        let mut statements = fixture.blocks[3].statements().to_vec();
        statements.insert(1, kill);
        fixture.blocks[3] = block(233, statements, zero_switch(6, BOOL_TYPE, 4, 5));
        let function = fixture.function();
        let site = ScalarAssignmentSiteV1 {
            block: 3,
            statement: 3,
        };
        let mut current = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert!(
            !current.index_singleton_fresh_v1(5, 3, 1, 3, site).unwrap(),
            "kill={kind}"
        );
        assert!(
            !legacy
                .index_singleton_fresh_graph_v1(5, 3, 1, 3, site)
                .unwrap()
        );
    }
}

#[test]
fn index_singleton_same_block_freshness_distinguishes_assignment_refresh_from_seed() {
    for killed in [3, 5] {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        fixture.blocks[4] = block(
            234,
            vec![statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(killed),
            ))],
            SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 3)),
        );
        let function = fixture.function();
        let site = ScalarAssignmentSiteV1 {
            block: 3,
            statement: 2,
        };
        let mut current = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert!(current.index_singleton_fresh_v1(5, 3, 1, 3, site).unwrap());
        assert!(
            legacy
                .index_singleton_fresh_graph_v1(5, 3, 1, 3, site)
                .unwrap()
        );
        assert_eq!(
            fixture.query(Some(128), 3, 6),
            if killed == 3 { None } else { Some(0) }
        );
    }
    let mut fixture = SingletonIndexFixtureV1::new(128);
    let mut statements = fixture.blocks[3].statements().to_vec();
    statements.insert(
        0,
        statement(SemanticStatementKindV1::StorageDead(
            SemanticLocalIdV1::from_index(5),
        )),
    );
    statements.push(typed_assignment(
        7,
        U64_TYPE,
        SemanticRvalueKindV1::Use(SemanticOperandV1::Move(typed_place(5, U64_TYPE))),
    ));
    fixture.blocks[3] = block(
        233,
        statements,
        SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 3)),
    );
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: 3,
    };
    let mut current = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    assert!(current.index_singleton_fresh_v1(5, 3, 2, 3, site).unwrap());
    assert!(
        legacy
            .index_singleton_fresh_graph_v1(5, 3, 2, 3, site)
            .unwrap()
    );
}

#[test]
fn index_singleton_same_block_freshness_refuses_invalid_slices_and_use_before_definition() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    for (local, block, start, end) in [
        (5, 3, 2, 1),
        (5, 3, 1, 3),
        (5, 6, 0, 0),
        (9, 3, 0, 1),
        (5, 3, usize::MAX, usize::MAX),
    ] {
        let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert!(
            !proof
                .index_singleton_fresh_v1(
                    local,
                    block,
                    start,
                    block,
                    ScalarAssignmentSiteV1 {
                        block,
                        statement: end
                    }
                )
                .unwrap()
        );
        assert_eq!(proof.work, 1);
    }
    let mut proof = singleton_local_cache_proofs_v1(&fixture, &function, false);
    let before = proof.work;
    assert_eq!(
        proof
            .index_singleton_local_uncached_v1(
                5,
                ScalarAssignmentSiteV1 {
                    block: 3,
                    statement: 0
                }
            )
            .unwrap(),
        IndexSingletonLocalDecisionV1 {
            result: IndexSingletonLocalV1::Unavailable,
            used_seed: false,
            invalid: true
        }
    );
    assert_eq!(proof.work, before);
}

#[test]
fn index_singleton_same_block_freshness_preserves_cached_flags_and_exact_work_refusal() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: 2,
    };
    for cached in [false, true] {
        let mut measured = singleton_local_cache_proofs_v1(&fixture, &function, cached);
        let before = measured.work;
        assert_eq!(
            measured.index_singleton_local_v1(5, site).unwrap(),
            IndexSingletonLocalV1::Assignment
        );
        assert_eq!(
            measured
                .index_singletons
                .as_ref()
                .unwrap()
                .freshness_local_calls,
            1
        );
        assert_eq!(
            measured
                .index_singletons
                .as_ref()
                .unwrap()
                .freshness_graph_calls,
            0
        );
        let cost = measured.work - before;
        assert!(cost > 0 && cost < function.blocks().len() * 4);
        for short in [false, true] {
            let mut proof = singleton_local_cache_proofs_v1(&fixture, &function, cached);
            proof.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - cost + usize::from(short);
            let result = proof.index_singleton_local_v1(5, site);
            let context = proof.index_singletons.as_ref().unwrap();
            assert!(!context.used_seed && !context.invalid);
            if short {
                assert!(result.is_err());
                assert!(context.local_decisions.is_empty());
            } else {
                assert_eq!(result.unwrap(), IndexSingletonLocalV1::Assignment);
                assert_eq!(proof.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
                assert_eq!(context.local_decisions.len(), usize::from(cached));
            }
        }
    }
}

#[test]
fn index_singleton_same_block_freshness_cost_is_independent_of_graph_extent() {
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: 2,
    };
    let mut costs = Vec::new();
    let mut legacy_costs = Vec::new();
    for count in [6, 16, 64] {
        let mut fixture = SingletonIndexFixtureV1::new(128);
        while fixture.blocks.len() < count {
            fixture.blocks.push(block(
                100 + fixture.blocks.len() as u8,
                vec![],
                SemanticTerminatorKindV1::Return,
            ));
        }
        let function = fixture.function();
        assert_eq!(function.blocks().len(), count);
        let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert!(proof.index_singleton_fresh_v1(5, 3, 1, 3, site).unwrap());
        assert!(
            legacy
                .index_singleton_fresh_graph_v1(5, 3, 1, 3, site)
                .unwrap()
        );
        costs.push(proof.work);
        legacy_costs.push(legacy.work);
    }
    assert!(costs.iter().all(|cost| *cost == costs[0]));
    assert!(legacy_costs.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn index_singleton_same_block_freshness_leaves_cross_block_and_seed_paths_unchanged() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    for (local, start, statement, refresh, use_block, use_statement) in
        [(3, 2, 0, 1, 2, 1), (3, 2, 0, 1, 3, 2), (5, 3, 1, 3, 4, 0)]
    {
        let site = ScalarAssignmentSiteV1 {
            block: use_block,
            statement: use_statement,
        };
        let mut current = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        let mut legacy = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert_eq!(
            current
                .index_singleton_fresh_v1(local, start, statement, refresh, site)
                .unwrap(),
            legacy
                .index_singleton_fresh_graph_v1(local, start, statement, refresh, site)
                .unwrap()
        );
        assert_eq!(current.work, legacy.work);
    }
}

#[test]
fn index_singleton_ineligible_cache_publication_still_obeys_exact_work_budget() {
    let fixture = SingletonIndexFixtureV1::new(128);
    let function = fixture.function();
    let site = ScalarAssignmentSiteV1 {
        block: 3,
        statement: 2,
    };
    for short in [false, true] {
        let mut proofs = singleton_local_cache_proofs_v1(&fixture, &function, true);
        proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - 2 + usize::from(short);
        let result = proofs.index_singleton_local_v1(7, site);
        let context = proofs.index_singletons.as_ref().unwrap();
        assert!(!context.used_seed);
        if short {
            assert!(matches!(
                result,
                Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
            ));
            assert!(context.local_decisions.is_empty());
            assert!(!context.invalid);
        } else {
            assert_eq!(result.unwrap(), IndexSingletonLocalV1::Unavailable);
            assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
            assert_eq!(context.local_decisions.len(), 1);
            assert!(context.invalid);
            proofs.index_singletons.as_mut().unwrap().invalid = false;
            assert!(proofs.index_singleton_local_v1(7, site).is_err());
            assert!(!proofs.index_singletons.as_ref().unwrap().invalid);
            proofs.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - 1;
            assert_eq!(
                proofs.index_singleton_local_v1(7, site).unwrap(),
                IndexSingletonLocalV1::Unavailable
            );
            assert_eq!(proofs.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
            assert!(proofs.index_singletons.as_ref().unwrap().invalid);
        }
    }
}
