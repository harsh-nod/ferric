// Component fixtures exercise the production projector, not semantic admission.
struct LoopSwitchDomainFixtureV1 {
    types: Vec<SemanticTypeDeclV1>,
    locals: Vec<SemanticLocalDeclV1>,
    blocks: Vec<SemanticBasicBlockV1>,
}

impl LoopSwitchDomainFixtureV1 {
    fn new(values: &[u128]) -> Self {
        assert_eq!(values.len(), 4);
        let base = multi_block_induction_function(
            InductionCfgShape::Chain,
            SemanticLocalRoleV1::Argument(0),
            1,
        );
        let mut types = assertion_proof_types();
        types[ENUM_TYPE.index() as usize] = SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(230)),
            SemanticLayoutIdentityV1::from_sha256(bytes(230)),
            SemanticTypeLayoutV1::new(Some(8), 4).unwrap(),
            SemanticTypeShapeV1::enum_type(
                SCALAR_TYPE,
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        SemanticEnumVariantV1::new(
                            *value,
                            SemanticAggregateTypeV1::new(if index == 0 || index == 3 {
                                vec![SCALAR_TYPE]
                            } else {
                                vec![]
                            })
                            .unwrap(),
                        )
                    })
                    .collect(),
            )
            .unwrap(),
        );
        let mut locals = base.locals().to_vec();
        locals.truncate(5);
        locals[2] = local(232, BOOL_TYPE, SemanticLocalRoleV1::Temporary);
        locals[4] = local(234, SCALAR_TYPE, SemanticLocalRoleV1::Temporary);
        locals.push(local(230, ENUM_TYPE, SemanticLocalRoleV1::Argument(1)));
        let mut blocks = vec![
            base.blocks()[0].clone(),
            block(
                231,
                vec![typed_assignment(
                    2,
                    BOOL_TYPE,
                    SemanticRvalueKindV1::Binary {
                        operation: SemanticBinaryOpV1::LessThan,
                        left: typed_operand(1, SCALAR_TYPE),
                        right: typed_operand(3, SCALAR_TYPE),
                    },
                )],
                zero_switch(2, BOOL_TYPE, 8, 2),
            ),
            block(
                232,
                vec![enum_discriminant(
                    SemanticLocalIdV1::from_index(5),
                    SemanticLocalIdV1::from_index(4),
                )],
                Self::switch(values),
            ),
        ];
        for tag in 233..237 {
            blocks.push(block(
                tag,
                vec![],
                SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, 7)),
            ));
        }
        blocks.push(base.blocks()[4].clone());
        blocks.push(block(237, vec![], SemanticTerminatorKindV1::Return));
        blocks.push(block(238, vec![], SemanticTerminatorKindV1::Unreachable));
        Self {
            types,
            locals,
            blocks,
        }
    }

    fn switch(values: &[u128]) -> SemanticTerminatorKindV1 {
        SemanticTerminatorKindV1::SwitchInt {
            discriminant: typed_operand(4, SCALAR_TYPE),
            targets: SemanticSwitchTargetsV1::new(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        SemanticSwitchTargetV1::new(
                            *value,
                            cfg_edge(SemanticEdgeRoleV1::SwitchValue, index as u32 + 3),
                        )
                    })
                    .collect(),
                cfg_edge(SemanticEdgeRoleV1::SwitchOtherwise, 9),
            )
            .unwrap(),
        }
    }

    fn replace_statements(&mut self, index: usize, statements: Vec<SemanticStatementV1>) {
        self.blocks[index] = block(
            230 + index as u8,
            statements,
            self.blocks[index].terminator().kind().clone(),
        );
    }

    fn function(&self) -> SemanticFunctionDeclV1 {
        projection_function_with_locals(self.blocks.clone(), self.locals.clone())
    }

    fn accepted_fallback(
        &self,
        work: &mut usize,
    ) -> Result<bool, ProductionRankedProjectionErrorV1> {
        let function = self.function();
        let inventory = assertion_definition_inventory(&function).unwrap();
        authenticated_loop_switch_fallback_v1(&self.types, &function, 2, &inventory, work)
    }

    fn assert_retained_fallback(&self, case: &str) {
        assert!(!self.accepted_fallback(&mut 0).unwrap(), "{case}");
        let function = self.function();
        let graph = projected_loop_cfg_graph_v1(&self.types, &function).unwrap();
        assert!(graph.successors[2].contains(&9), "{case}");
        assert_eq!(
            typed_fallback_ranked_return_count_v1(&self.types, &function, &[], vec![]),
            2,
            "ranked CFG lost an unauthenticated fallback: {case}",
        );
        assert_incomplete(
            project_natural_loop_topology_v1(&[], &function, &graph, 1, 2, 8, &mut 0),
            "a uniform induction region does not have one unique header exit",
        );
    }
}

fn typed_fallback_ranked_return_count_v1(
    types: &[SemanticTypeDeclV1],
    function: &SemanticFunctionDeclV1,
    inductions: &[ProjectedUniformInductionV1],
    operations: Vec<ProductionRankedOperationV1>,
) -> usize {
    let (blocks, accesses, effects) = build_ranked_cfg(
        types,
        function,
        &[],
        &vec![None; function.locals().len()],
        &[],
        inductions,
        operations,
        function
            .blocks()
            .iter()
            .map(|_| ProjectedSemanticBlockV1 { items: vec![] })
            .collect(),
    )
    .unwrap();
    assert!(accesses.is_empty());
    assert!(effects.is_empty());
    blocks
        .iter()
        .filter(|block| matches!(block.terminator(), ProductionRankedTerminatorV1::Return))
        .count()
}

#[test]
fn typed_fallback_ranked_cfg_elides_exhaustive_enum_dispatch() {
    for values in [[0, 1, 2, 3], [2, 7, 19, 41]] {
        let fixture = LoopSwitchDomainFixtureV1::new(&values);
        let function = fixture.function();
        let graph = projected_loop_cfg_graph_v1(&fixture.types, &function).unwrap();
        assert_eq!(graph.successors[2], [3, 4, 5, 6]);
        let (inductions, operations, _) =
            project_test_inductions_with_types(&fixture.types, &function).unwrap();
        assert_eq!(inductions.len(), 1);
        assert_eq!(
            typed_fallback_ranked_return_count_v1(
                &fixture.types,
                &function,
                &inductions,
                operations,
            ),
            1,
            "typed exhaustive enum fallback remained in ranked CFG",
        );
    }
}

#[test]
fn typed_fallback_ranked_cfg_distinguishes_bool_and_integer_domains() {
    let types = assertion_proof_types();
    let boolean = boolean_domain_switch(&[0, 1]);
    assert_eq!(
        typed_fallback_ranked_return_count_v1(&types, &boolean, &[], vec![]),
        2,
    );
    let integer =
        explicit_binary_switch_with_fallback([0, 1], vec![], SemanticTerminatorKindV1::Unreachable);
    assert_eq!(
        typed_fallback_ranked_return_count_v1(&types, &integer, &[], vec![]),
        3,
        "untyped integer fallback was elided from ranked CFG",
    );
}

#[test]
fn typed_fallback_ranked_cfg_keeps_the_captured_scalar_snapshot() {
    let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
    let mut statements = fixture.blocks[2].statements().to_vec();
    statements.push(enum_definition(SemanticLocalIdV1::from_index(5), 1));
    fixture.replace_statements(2, statements);
    assert_eq!(
        typed_fallback_ranked_return_count_v1(&fixture.types, &fixture.function(), &[], vec![]),
        1,
    );
    let mut statements = fixture.blocks[2].statements().to_vec();
    statements.push(typed_assignment(
        4,
        SCALAR_TYPE,
        SemanticRvalueKindV1::Use(constant(99)),
    ));
    fixture.replace_statements(2, statements);
    fixture.assert_retained_fallback("overwritten captured tag");
}

#[test]
fn typed_fallback_graph_preserves_exact_shared_work_limits() {
    let fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
    let function = fixture.function();
    let inventory = assertion_definition_inventory(&function).unwrap();
    let mut measured = 0;
    let graph =
        projected_loop_cfg_with_inventory_v1(&fixture.types, &function, &inventory, &mut measured)
            .unwrap();
    assert!(measured >= function.blocks().len());
    let mut exact = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - measured;
    let exact_graph =
        projected_loop_cfg_with_inventory_v1(&fixture.types, &function, &inventory, &mut exact)
            .unwrap();
    assert_eq!(exact, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    assert_eq!(exact_graph.successors, graph.successors);
    assert_eq!(exact_graph.reachable, graph.reachable);
    let mut short = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - measured + 1;
    assert!(matches!(
        projected_loop_cfg_with_inventory_v1(&fixture.types, &function, &inventory, &mut short,),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
    ));
    let mut overflow = usize::MAX;
    assert!(matches!(
        projected_loop_cfg_with_inventory_v1(&fixture.types, &function, &inventory, &mut overflow,),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::Overflow
    ));
}

#[test]
fn loop_switch_domain_exhaustive_enum_dispatch_preserves_induction() {
    for values in [[0, 1, 2, 3], [2, 7, 19, 41]] {
        let fixture = LoopSwitchDomainFixtureV1::new(&values);
        let function = fixture.function();
        let SemanticTerminatorKindV1::SwitchInt {
            discriminant,
            targets,
        } = function.blocks()[2].terminator().kind()
        else {
            panic!("fixture lost its switch")
        };
        assert!(authenticated_switch_targets_exhaust_domain_v1(
            &fixture.types,
            &function,
            discriminant,
            targets,
            &local_definition_counts(&function),
        ));
        assert!(fixture.accepted_fallback(&mut 0).unwrap());
        let graph = projected_loop_cfg_graph_v1(&fixture.types, &function).unwrap();
        assert_eq!(graph.successors[2], [3, 4, 5, 6]);
        assert!(!graph.reachable[9]);
        let (inductions, _, _) =
            project_test_inductions_with_types(&fixture.types, &function).unwrap();
        assert_eq!(inductions.len(), 1);
        assert_eq!(inductions[0].header, 1);
        assert_eq!(inductions[0].latch, 7);
        assert_eq!(inductions[0].loop_blocks, [1, 2, 3, 4, 5, 6, 7]);
    }
    let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
    let SemanticTypeShapeV1::Enum { variants, .. } =
        fixture.types[ENUM_TYPE.index() as usize].shape()
    else {
        panic!("fixture lost its enum")
    };
    let mut variants = variants.to_vec();
    variants.push(SemanticEnumVariantV1::new_with_inhabitedness(
        99,
        SemanticAggregateTypeV1::new(vec![]).unwrap(),
        true,
    ));
    fixture.types[ENUM_TYPE.index() as usize] = SemanticTypeDeclV1::new(
        SemanticTypeIdentityV1::from_sha256(bytes(230)),
        SemanticLayoutIdentityV1::from_sha256(bytes(230)),
        SemanticTypeLayoutV1::new(Some(8), 4).unwrap(),
        SemanticTypeShapeV1::enum_type(SCALAR_TYPE, variants).unwrap(),
    );
    assert!(fixture.accepted_fallback(&mut 0).unwrap());
    assert_eq!(
        project_test_inductions_with_types(&fixture.types, &fixture.function())
            .unwrap()
            .0
            .len(),
        1
    );

    let boolean = boolean_domain_switch(&[0, 1]);
    let types = assertion_proof_types();
    let inventory = assertion_definition_inventory(&boolean).unwrap();
    assert!(
        authenticated_loop_switch_fallback_v1(&types, &boolean, 0, &inventory, &mut 0).unwrap()
    );
    assert_eq!(
        projected_loop_cfg_graph_v1(&types, &boolean)
            .unwrap()
            .successors[0],
        [1, 2]
    );
    let scalar =
        explicit_binary_switch_with_fallback([0, 1], vec![], SemanticTerminatorKindV1::Unreachable);
    let inventory = assertion_definition_inventory(&scalar).unwrap();
    assert!(
        !authenticated_loop_switch_fallback_v1(&types, &scalar, 0, &inventory, &mut 0).unwrap()
    );
    assert_eq!(
        projected_loop_cfg_graph_v1(&types, &scalar)
            .unwrap()
            .successors[0],
        [1, 2, 3]
    );
}

#[test]
fn loop_switch_domain_requires_exact_inhabited_values_and_declared_types() {
    for values in [&[0, 1, 2][..], &[0, 1, 2, 4][..], &[0, 1, 2, 3, 4][..]] {
        let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
        fixture.blocks[2] = block(
            232,
            fixture.blocks[2].statements().to_vec(),
            LoopSwitchDomainFixtureV1::switch(values),
        );
        fixture.assert_retained_fallback("missing or substituted inhabited variant");
    }
    for case in 0..4 {
        let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
        match case {
            0 => fixture.replace_statements(
                2,
                vec![typed_assignment(
                    4,
                    SCALAR_TYPE,
                    SemanticRvalueKindV1::Use(constant(0)),
                )],
            ),
            1 => fixture.locals[5] = local(230, SCALAR_TYPE, SemanticLocalRoleV1::Argument(1)),
            2 => fixture.replace_statements(
                2,
                vec![typed_assignment(
                    4,
                    SCALAR_TYPE,
                    SemanticRvalueKindV1::Discriminant(typed_place(5, SCALAR_TYPE)),
                )],
            ),
            3 => fixture.locals[4] = local(234, ENUM_TYPE, SemanticLocalRoleV1::Temporary),
            _ => unreachable!(),
        }
        fixture.assert_retained_fallback("unproven or mismatched discriminant type");
    }
    // Duplicates are rejected before a typed switch can reach normalization.
    assert!(matches!(
        SemanticSwitchTargetsV1::new(
            vec![
                SemanticSwitchTargetV1::new(0, cfg_edge(SemanticEdgeRoleV1::SwitchValue, 3)),
                SemanticSwitchTargetV1::new(0, cfg_edge(SemanticEdgeRoleV1::SwitchValue, 4)),
            ],
            cfg_edge(SemanticEdgeRoleV1::SwitchOtherwise, 9),
        ),
        Err(fe2o3_mir_model::semantic_mir_v1::SemanticMirErrorV1::NonDeterministicOrder { .. })
    ));
}

#[test]
fn loop_switch_domain_requires_current_unescaped_direct_discriminant() {
    for case in 0..6 {
        let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
        let mut statements = fixture.blocks[2].statements().to_vec();
        match case {
            0 => {
                let mut entry = fixture.blocks[0].statements().to_vec();
                entry.append(&mut statements);
                fixture.replace_statements(0, entry);
            }
            1 => statements.push(typed_assignment(
                4,
                SCALAR_TYPE,
                SemanticRvalueKindV1::Use(constant(0)),
            )),
            2 => statements.push(statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(4),
            ))),
            3 => statements.push(statement(SemanticStatementKindV1::StorageLive(
                SemanticLocalIdV1::from_index(4),
            ))),
            4 => {
                fixture
                    .locals
                    .push(local(239, SCALAR_TYPE, SemanticLocalRoleV1::Temporary));
                statements = vec![
                    enum_discriminant(
                        SemanticLocalIdV1::from_index(5),
                        SemanticLocalIdV1::from_index(6),
                    ),
                    typed_assignment(
                        4,
                        SCALAR_TYPE,
                        SemanticRvalueKindV1::Use(typed_operand(6, SCALAR_TYPE)),
                    ),
                ];
            }
            5 => {
                let mut entry = fixture.blocks[0].statements().to_vec();
                entry.push(typed_assignment(
                    4,
                    SCALAR_TYPE,
                    SemanticRvalueKindV1::Use(constant(0)),
                ));
                fixture.replace_statements(0, entry);
            }
            _ => unreachable!(),
        }
        fixture.replace_statements(2, statements);
        fixture.assert_retained_fallback("not one current same-block discriminant");
    }
    for (exposed, pointee) in [(4, SCALAR_TYPE), (5, ENUM_TYPE)] {
        for raw in [false, true] {
            let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
            let pointer = SemanticTypeIdV1::from_index(fixture.types.len() as u32);
            fixture.types.push(SemanticTypeDeclV1::new(
                SemanticTypeIdentityV1::from_sha256(bytes(240)),
                SemanticLayoutIdentityV1::from_sha256(bytes(240)),
                SemanticTypeLayoutV1::new(Some(8), 8).unwrap(),
                SemanticTypeShapeV1::Pointer(
                    SemanticPointerTypeV1::new_with_kind(
                        pointee,
                        if raw {
                            fe2o3_mir_model::semantic_mir_v1::SemanticPointerKindV1::Raw
                        } else {
                            fe2o3_mir_model::semantic_mir_v1::SemanticPointerKindV1::Reference
                        },
                        SemanticMutabilityV1::Mutable,
                        0,
                        64,
                        SemanticPointerMetadataV1::None,
                    )
                    .unwrap(),
                ),
            ));
            fixture
                .locals
                .push(local(240, pointer, SemanticLocalRoleV1::Temporary));
            let place = typed_place(exposed, pointee);
            let value = if raw {
                SemanticRvalueKindV1::AddressOf {
                    mutability: SemanticMutabilityV1::Mutable,
                    place,
                }
            } else {
                SemanticRvalueKindV1::Borrow {
                    kind: SemanticBorrowKindV1::Mutable,
                    place,
                }
            };
            let mut statements = fixture.blocks[2].statements().to_vec();
            statements.push(typed_assignment(6, pointer, value));
            fixture.replace_statements(2, statements);
            if exposed == 4 {
                fixture.assert_retained_fallback("address-exposed captured tag");
            } else {
                // The copied discriminant is a scalar snapshot, not a live alias of the enum.
                assert!(fixture.accepted_fallback(&mut 0).unwrap());
                let mut statements = fixture.blocks[2].statements().to_vec();
                statements.push(enum_definition(SemanticLocalIdV1::from_index(5), 1));
                fixture.replace_statements(2, statements);
                assert!(fixture.accepted_fallback(&mut 0).unwrap());
            }
        }
    }
}

#[test]
fn loop_switch_domain_elides_only_an_empty_unreachable_fallback() {
    for (statements, terminator) in [
        (
            vec![statement(SemanticStatementKindV1::Nop)],
            SemanticTerminatorKindV1::Unreachable,
        ),
        (vec![], SemanticTerminatorKindV1::Return),
        (
            vec![typed_assignment(
                0,
                SCALAR_TYPE,
                SemanticRvalueKindV1::Use(constant(7)),
            )],
            SemanticTerminatorKindV1::Unreachable,
        ),
    ] {
        let mut fixture = LoopSwitchDomainFixtureV1::new(&[0, 1, 2, 3]);
        fixture.blocks[9] = block(238, statements, terminator);
        fixture.assert_retained_fallback("fallback must be empty and unreachable");
    }
}

#[test]
fn loop_switch_domain_charges_exact_shared_work_and_fails_one_short() {
    let fixture = LoopSwitchDomainFixtureV1::new(&[2, 7, 19, 41]);
    let mut measured = 0;
    assert!(fixture.accepted_fallback(&mut measured).unwrap());
    assert!(measured >= 4);
    let mut exact = MAX_PROJECTED_LOOP_GRAPH_WORK_V1
        .checked_sub(measured)
        .unwrap();
    assert!(fixture.accepted_fallback(&mut exact).unwrap());
    assert_eq!(exact, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    let mut short = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - measured + 1;
    assert!(matches!(
        fixture.accepted_fallback(&mut short),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
    ));
    let mut shared = 17;
    assert!(fixture.accepted_fallback(&mut shared).unwrap());
    assert!(fixture.accepted_fallback(&mut shared).unwrap());
    assert_eq!(shared, 17 + 2 * measured);
    let mut overflow = usize::MAX;
    assert!(matches!(
        fixture.accepted_fallback(&mut overflow),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::Overflow
    ));
}
