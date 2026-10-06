// Component fixtures exercise the production range proof, not semantic admission.
// The joined Option/enum path mirrors the worker's detached task and lane capture.
struct ProjectedRangeFixtureV1 {
    types: Vec<SemanticTypeDeclV1>,
    locals: Vec<SemanticLocalDeclV1>,
    blocks: Vec<SemanticBasicBlockV1>,
    payload: SemanticTypeIdV1,
    inner: SemanticTypeIdV1,
    outer: SemanticTypeIdV1,
    outer_pointer: SemanticTypeIdV1,
    inner_pointer: SemanticTypeIdV1,
    tag_pointer: SemanticTypeIdV1,
}

fn projected_range_goto_v1(target: u32) -> SemanticTerminatorKindV1 {
    SemanticTerminatorKindV1::Goto(cfg_edge(SemanticEdgeRoleV1::Goto, target))
}

fn projected_range_switch_v1(
    local: u32,
    cases: &[(u128, u32)],
    otherwise: u32,
) -> SemanticTerminatorKindV1 {
    SemanticTerminatorKindV1::SwitchInt {
        discriminant: typed_operand(local, SCALAR_TYPE),
        targets: SemanticSwitchTargetsV1::new(
            cases
                .iter()
                .map(|(value, target)| {
                    SemanticSwitchTargetV1::new(
                        *value,
                        cfg_edge(SemanticEdgeRoleV1::SwitchValue, *target),
                    )
                })
                .collect(),
            cfg_edge(SemanticEdgeRoleV1::SwitchOtherwise, otherwise),
        )
        .unwrap(),
    }
}

fn projected_range_aggregate_v1(
    kind: SemanticAggregateKindV1,
    operands: Vec<SemanticOperandV1>,
) -> SemanticRvalueKindV1 {
    SemanticRvalueKindV1::Aggregate(SemanticAggregateRvalueV1::new(kind, operands).unwrap())
}

fn projected_range_assert_v1(
    pair: u32,
    operation: SemanticBinaryOpV1,
    left: SemanticOperandV1,
    right: SemanticOperandV1,
    target: u32,
) -> SemanticTerminatorKindV1 {
    SemanticTerminatorKindV1::Assert {
        condition: checked_field_operand(pair, 1, BOOL_TYPE),
        expected: false,
        message: SemanticAssertMessageV1::Overflow {
            operation,
            left,
            right,
        },
        target: cfg_edge(SemanticEdgeRoleV1::AssertSuccess, target),
        unwind: SemanticUnwindActionV1::Unreachable,
    }
}

impl ProjectedRangeFixtureV1 {
    fn new(second_key_lane: u128) -> Self {
        let mut types = assertion_proof_types();
        let payload = SemanticTypeIdV1::from_index(types.len() as u32);
        types.push(SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(110)),
            SemanticLayoutIdentityV1::from_sha256(bytes(110)),
            SemanticTypeLayoutV1::new(Some(16), 8).unwrap(),
            SemanticTypeShapeV1::Aggregate(
                SemanticAggregateTypeV1::new(vec![U64_TYPE, U64_TYPE]).unwrap(),
            ),
        ));
        let inner = SemanticTypeIdV1::from_index(types.len() as u32);
        types.push(SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(111)),
            SemanticLayoutIdentityV1::from_sha256(bytes(111)),
            SemanticTypeLayoutV1::new(Some(24), 8).unwrap(),
            SemanticTypeShapeV1::enum_type(
                SCALAR_TYPE,
                vec![
                    SemanticEnumVariantV1::new(
                        0,
                        SemanticAggregateTypeV1::new(vec![payload]).unwrap(),
                    ),
                    SemanticEnumVariantV1::new(
                        1,
                        SemanticAggregateTypeV1::new(vec![payload]).unwrap(),
                    ),
                ],
            )
            .unwrap(),
        ));
        let outer = SemanticTypeIdV1::from_index(types.len() as u32);
        types.push(SemanticTypeDeclV1::new(
            SemanticTypeIdentityV1::from_sha256(bytes(112)),
            SemanticLayoutIdentityV1::from_sha256(bytes(112)),
            SemanticTypeLayoutV1::new(Some(32), 8).unwrap(),
            SemanticTypeShapeV1::enum_type(
                SCALAR_TYPE,
                vec![
                    SemanticEnumVariantV1::new(0, SemanticAggregateTypeV1::new(vec![]).unwrap()),
                    SemanticEnumVariantV1::new(
                        1,
                        SemanticAggregateTypeV1::new(vec![inner]).unwrap(),
                    ),
                ],
            )
            .unwrap(),
        ));
        let mut pointers = Vec::new();
        for (tag, pointee) in [(113, outer), (114, inner), (115, SCALAR_TYPE)] {
            pointers.push(SemanticTypeIdV1::from_index(types.len() as u32));
            types.push(SemanticTypeDeclV1::new(
                SemanticTypeIdentityV1::from_sha256(bytes(tag)),
                SemanticLayoutIdentityV1::from_sha256(bytes(tag)),
                SemanticTypeLayoutV1::new(Some(8), 8).unwrap(),
                SemanticTypeShapeV1::Pointer(
                    SemanticPointerTypeV1::new(
                        pointee,
                        SemanticMutabilityV1::Mutable,
                        5,
                        64,
                        SemanticPointerMetadataV1::None,
                    )
                    .unwrap(),
                ),
            ));
        }
        let mut fixture = Self {
            types,
            locals: vec![],
            blocks: vec![],
            payload,
            inner,
            outer,
            outer_pointer: pointers[0],
            inner_pointer: pointers[1],
            tag_pointer: pointers[2],
        };
        let local_types = [
            U64_TYPE,
            U64_TYPE,
            SCALAR_TYPE,
            U64_TYPE,
            U64_TYPE,
            payload,
            payload,
            inner,
            outer,
            SCALAR_TYPE,
            inner,
            SCALAR_TYPE,
            U64_TYPE,
            BOOL_TYPE,
            CHECKED_U64_TYPE,
            U64_TYPE,
            CHECKED_U64_TYPE,
            pointers[0],
            pointers[1],
            pointers[2],
            outer,
        ];
        fixture.locals = local_types
            .into_iter()
            .enumerate()
            .map(|(index, ty)| {
                let role = match index {
                    0 => SemanticLocalRoleV1::Return,
                    1..=3 => SemanticLocalRoleV1::Argument((index - 1) as u32),
                    20 => SemanticLocalRoleV1::Argument(3),
                    _ => SemanticLocalRoleV1::Temporary,
                };
                local(160 + index as u8, ty, role)
            })
            .collect();
        let copy_lane = SemanticOperandV1::Copy(fixture.lane_place());
        fixture.blocks = vec![
            block(
                80,
                vec![typed_assignment(
                    4,
                    U64_TYPE,
                    SemanticRvalueKindV1::Binary {
                        operation: SemanticBinaryOpV1::Remainder,
                        left: typed_operand(1, U64_TYPE),
                        right: typed_constant(U64_TYPE, 64, 8),
                    },
                )],
                projected_range_switch_v1(2, &[(0, 1), (1, 2), (2, 3)], 4),
            ),
            block(
                81,
                vec![typed_assignment(
                    8,
                    outer,
                    projected_range_aggregate_v1(SemanticAggregateKindV1::EnumVariant(0), vec![]),
                )],
                projected_range_goto_v1(5),
            ),
            block(
                82,
                fixture.constructor(0, typed_constant(U64_TYPE, u64::MAX.into(), 8)),
                projected_range_goto_v1(5),
            ),
            block(
                83,
                fixture.constructor(1, typed_operand(4, U64_TYPE)),
                projected_range_goto_v1(5),
            ),
            block(
                84,
                fixture.constructor(1, typed_constant(U64_TYPE, second_key_lane, 8)),
                projected_range_goto_v1(5),
            ),
            block(
                85,
                vec![typed_assignment(
                    9,
                    SCALAR_TYPE,
                    SemanticRvalueKindV1::Discriminant(typed_place(8, outer)),
                )],
                projected_range_switch_v1(9, &[(0, 11), (1, 6)], 12),
            ),
            block(
                86,
                vec![typed_assignment(
                    10,
                    inner,
                    SemanticRvalueKindV1::Use(SemanticOperandV1::Move(fixture.some_place())),
                )],
                projected_range_goto_v1(7),
            ),
            block(
                87,
                vec![typed_assignment(
                    11,
                    SCALAR_TYPE,
                    SemanticRvalueKindV1::Discriminant(typed_place(10, inner)),
                )],
                projected_range_switch_v1(11, &[(0, 11), (1, 8)], 12),
            ),
            block(
                88,
                vec![
                    typed_assignment(12, U64_TYPE, SemanticRvalueKindV1::Use(copy_lane)),
                    typed_assignment(
                        13,
                        BOOL_TYPE,
                        SemanticRvalueKindV1::Binary {
                            operation: SemanticBinaryOpV1::LessThan,
                            left: typed_operand(3, U64_TYPE),
                            right: typed_constant(U64_TYPE, 64, 8),
                        },
                    ),
                ],
                zero_switch(13, BOOL_TYPE, 11, 9),
            ),
            block(
                89,
                vec![typed_assignment(
                    14,
                    CHECKED_U64_TYPE,
                    SemanticRvalueKindV1::CheckedBinary(SemanticCheckedBinaryRvalueV1::new(
                        SemanticCheckedBinaryOpV1::Multiply,
                        typed_operand(3, U64_TYPE),
                        typed_constant(U64_TYPE, 64, 8),
                    )),
                )],
                projected_range_assert_v1(
                    14,
                    SemanticBinaryOpV1::Multiply,
                    typed_operand(3, U64_TYPE),
                    typed_constant(U64_TYPE, 64, 8),
                    10,
                ),
            ),
            block(
                90,
                vec![
                    typed_assignment(
                        15,
                        U64_TYPE,
                        SemanticRvalueKindV1::Use(checked_field_operand(14, 0, U64_TYPE)),
                    ),
                    typed_assignment(
                        16,
                        CHECKED_U64_TYPE,
                        SemanticRvalueKindV1::CheckedBinary(SemanticCheckedBinaryRvalueV1::new(
                            SemanticCheckedBinaryOpV1::Add,
                            typed_operand(15, U64_TYPE),
                            typed_operand(12, U64_TYPE),
                        )),
                    ),
                ],
                projected_range_assert_v1(
                    16,
                    SemanticBinaryOpV1::Add,
                    typed_operand(15, U64_TYPE),
                    typed_operand(12, U64_TYPE),
                    11,
                ),
            ),
            block(91, vec![], SemanticTerminatorKindV1::Return),
            block(92, vec![], SemanticTerminatorKindV1::Unreachable),
        ];
        fixture
    }

    fn constructor(&self, variant: u32, lane: SemanticOperandV1) -> Vec<SemanticStatementV1> {
        let aggregate = if variant == 0 { 5 } else { 6 };
        vec![
            typed_assignment(
                aggregate,
                self.payload,
                projected_range_aggregate_v1(
                    SemanticAggregateKindV1::Aggregate,
                    vec![typed_constant(U64_TYPE, 0, 8), lane],
                ),
            ),
            typed_assignment(
                7,
                self.inner,
                projected_range_aggregate_v1(
                    SemanticAggregateKindV1::EnumVariant(variant),
                    vec![typed_operand(aggregate, self.payload)],
                ),
            ),
            typed_assignment(
                8,
                self.outer,
                projected_range_aggregate_v1(
                    SemanticAggregateKindV1::EnumVariant(1),
                    vec![typed_operand(7, self.inner)],
                ),
            ),
        ]
    }

    fn some_place(&self) -> SemanticPlaceV1 {
        SemanticPlaceV1::new(
            SemanticLocalIdV1::from_index(8),
            vec![
                SemanticProjectionV1::new(SemanticProjectionKindV1::Downcast(1), self.outer)
                    .unwrap(),
                SemanticProjectionV1::new(SemanticProjectionKindV1::Field(0), self.inner).unwrap(),
            ],
            self.inner,
        )
        .unwrap()
    }

    fn lane_place(&self) -> SemanticPlaceV1 {
        SemanticPlaceV1::new(
            SemanticLocalIdV1::from_index(10),
            vec![
                SemanticProjectionV1::new(SemanticProjectionKindV1::Downcast(1), self.inner)
                    .unwrap(),
                SemanticProjectionV1::new(SemanticProjectionKindV1::Field(0), self.payload)
                    .unwrap(),
                SemanticProjectionV1::new(SemanticProjectionKindV1::Field(1), U64_TYPE).unwrap(),
            ],
            U64_TYPE,
        )
        .unwrap()
    }

    fn edit(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut Vec<SemanticStatementV1>, &mut SemanticTerminatorKindV1),
    ) {
        let mut statements = self.blocks[index].statements().to_vec();
        let mut terminator = self.blocks[index].terminator().kind().clone();
        edit(&mut statements, &mut terminator);
        self.blocks[index] = block(80 + index as u8, statements, terminator);
    }

    fn function(&self) -> SemanticFunctionDeclV1 {
        projection_function_with_locals(self.blocks.clone(), self.locals.clone())
    }

    fn range(&self) -> Option<UnsignedRangeProofV1> {
        let function = self.function();
        SemanticAssertProofsV1::new(&self.types, &function)
            .unwrap()
            .range_at_operand(&SemanticOperandV1::Copy(self.lane_place()), 8, 0)
            .unwrap()
    }

    fn sum_is_proved(&self) -> bool {
        let function = self.function();
        SemanticAssertProofsV1::analyze(&self.types, &function).unwrap()[10]
    }
}

#[test]
fn assertion_projected_range_nested_snapshot_proves_checked_index() {
    let fixture = ProjectedRangeFixtureV1::new(7);
    assert_eq!(
        fixture.range(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 63
        })
    );
    let function = fixture.function();
    let proved = SemanticAssertProofsV1::analyze(&fixture.types, &function).unwrap();
    assert!(
        proved[9] && proved[10],
        "both checked operations need their own proof"
    );
    let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    assert_eq!(
        proof
            .range_at_operand(&checked_field_operand(16, 0, U64_TYPE), 10, 2)
            .unwrap(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 4095
        })
    );
}

#[test]
fn assertion_projected_range_joins_all_matching_constructors() {
    let larger = ProjectedRangeFixtureV1::new(127);
    assert_eq!(
        larger.range(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 127
        })
    );
    let overflow = ProjectedRangeFixtureV1::new(u64::MAX.into());
    assert_eq!(
        overflow.range(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: u64::MAX.into()
        })
    );
    assert!(
        !overflow.sum_is_proved(),
        "a feasible Key constructor cannot be dropped"
    );
    let mut unknown = ProjectedRangeFixtureV1::new(7);
    let outer = unknown.outer;
    unknown.edit(4, |statements, _| {
        *statements = vec![typed_assignment(
            8,
            outer,
            SemanticRvalueKindV1::Use(typed_operand(20, outer)),
        )]
    });
    assert_eq!(
        unknown.range(),
        None,
        "unknown matching payloads cannot be skipped"
    );
    assert!(!unknown.sum_is_proved());
    let mut call = ProjectedRangeFixtureV1::new(7);
    let outer = call.outer;
    call.edit(4, |statements, terminator| {
        statements.clear();
        *terminator = SemanticTerminatorKindV1::Call(
            SemanticDirectCallV1::new_callable(
                SemanticCallableIdV1::from_index(0),
                vec![],
                Some(SemanticCallDestinationV1::new(
                    typed_place(8, outer),
                    cfg_edge(SemanticEdgeRoleV1::CallReturn, 5),
                )),
                SemanticUnwindActionV1::Unreachable,
            )
            .unwrap(),
        );
    });
    assert_eq!(call.range(), None);
    let mut excluded = ProjectedRangeFixtureV1::new(7);
    for constructor in [3, 4] {
        let statements = excluded.constructor(0, typed_constant(U64_TYPE, 7, 8));
        excluded.edit(constructor, |current, _| *current = statements);
    }
    assert_eq!(
        excluded.range(),
        None,
        "excluding every constructor is not a scalar proof"
    );
}

#[test]
fn assertion_projected_range_requires_current_variant_guards() {
    for (guard, target, tag) in [(5, 6, 9), (7, 8, 11)] {
        let mut missing = ProjectedRangeFixtureV1::new(7);
        missing.edit(guard, |_, terminator| {
            *terminator = projected_range_goto_v1(target)
        });
        assert_eq!(missing.range(), None, "missing guard block {guard}");
        let mut shared = ProjectedRangeFixtureV1::new(7);
        shared.edit(guard, |_, terminator| {
            *terminator = projected_range_switch_v1(tag, &[(0, target), (1, target)], 12)
        });
        assert_eq!(shared.range(), None, "shared successor at guard {guard}");
        let mut stale = ProjectedRangeFixtureV1::new(7);
        stale.edit(guard, |statements, _| {
            statements.push(typed_assignment(
                tag,
                SCALAR_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(SCALAR_TYPE, 1, 4)),
            ))
        });
        assert_eq!(stale.range(), None, "overwritten discriminator {tag}");
    }
    let mut separated = ProjectedRangeFixtureV1::new(7);
    let inner = separated.inner;
    separated.edit(6, |statements, _| {
        statements.push(typed_assignment(
            11,
            SCALAR_TYPE,
            SemanticRvalueKindV1::Discriminant(typed_place(10, inner)),
        ))
    });
    separated.edit(7, |statements, _| statements.clear());
    assert_eq!(
        separated.range(),
        None,
        "this bounded guard rule requires a same-block tag capture"
    );
    let mut wrong_carrier = ProjectedRangeFixtureV1::new(7);
    wrong_carrier.edit(7, |statements, _| {
        statements[0] = typed_assignment(
            11,
            SCALAR_TYPE,
            SemanticRvalueKindV1::Discriminant(typed_place(7, inner)),
        )
    });
    assert_eq!(
        wrong_carrier.range(),
        None,
        "a tag from another logical enum is not authority"
    );
}

#[test]
fn assertion_projected_range_rejects_mutation_epochs_and_escape() {
    for local_index in [8, 10, 9, 11] {
        let guard = if matches!(local_index, 8 | 9) { 5 } else { 7 };
        for live in [false, true] {
            let mut fixture = ProjectedRangeFixtureV1::new(7);
            fixture.edit(guard, |statements, _| {
                statements.push(statement(if live {
                    SemanticStatementKindV1::StorageLive(SemanticLocalIdV1::from_index(local_index))
                } else {
                    SemanticStatementKindV1::StorageDead(SemanticLocalIdV1::from_index(local_index))
                }))
            });
            assert_eq!(
                fixture.range(),
                None,
                "epoch marker local {local_index}, live {live}"
            );
        }
    }
    for local_index in [8, 10, 9, 11] {
        let mut fixture = ProjectedRangeFixtureV1::new(7);
        let (ty, pointer, destination, guard) = match local_index {
            8 => (fixture.outer, fixture.outer_pointer, 17, 5),
            10 => (fixture.inner, fixture.inner_pointer, 18, 7),
            9 => (SCALAR_TYPE, fixture.tag_pointer, 19, 5),
            _ => (SCALAR_TYPE, fixture.tag_pointer, 19, 7),
        };
        fixture.edit(guard, |statements, _| {
            statements.push(typed_assignment(
                destination,
                pointer,
                SemanticRvalueKindV1::AddressOf {
                    mutability: SemanticMutabilityV1::Mutable,
                    place: typed_place(local_index, ty),
                },
            ))
        });
        assert_eq!(fixture.range(), None, "escaped local {local_index}");
    }
    let mut partial = ProjectedRangeFixtureV1::new(7);
    let lane = partial.lane_place();
    partial.edit(7, |statements, _| {
        statements.push(statement(SemanticStatementKindV1::Assign(
            SemanticAssignmentV1::new(
                lane,
                SemanticRvalueV1::new(
                    U64_TYPE,
                    SemanticRvalueKindV1::Use(typed_constant(U64_TYPE, u64::MAX.into(), 8)),
                ),
            ),
        )))
    });
    assert_eq!(
        partial.range(),
        None,
        "partial carrier writes must not reuse old field facts"
    );
    // These malformed value-lifetime mutants are component refusal tests, not
    // admitted Rust programs: an earlier Move must not leave reusable facts.
    for (source, destination, guard) in [(8, 20, 5), (10, 7, 7)] {
        let mut moved = ProjectedRangeFixtureV1::new(7);
        let ty = if source == 8 {
            moved.outer
        } else {
            moved.inner
        };
        moved.edit(guard, |statements, _| {
            statements.push(typed_assignment(
                destination,
                ty,
                SemanticRvalueKindV1::Use(SemanticOperandV1::Move(typed_place(source, ty))),
            ))
        });
        assert_eq!(
            moved.range(),
            None,
            "intervening statement Move of {source}"
        );
    }
    let mut call_move = ProjectedRangeFixtureV1::new(7);
    let inner = call_move.inner;
    call_move.edit(6, |_, terminator| {
        *terminator = SemanticTerminatorKindV1::Call(
            SemanticDirectCallV1::new_callable(
                SemanticCallableIdV1::from_index(0),
                vec![SemanticOperandV1::Move(typed_place(10, inner))],
                Some(SemanticCallDestinationV1::new(
                    typed_place(0, U64_TYPE),
                    cfg_edge(SemanticEdgeRoleV1::CallReturn, 7),
                )),
                SemanticUnwindActionV1::Unreachable,
            )
            .unwrap(),
        )
    });
    assert_eq!(
        call_move.range(),
        None,
        "intervening call-argument Move of detached enum"
    );
}

#[test]
fn assertion_projected_range_preserves_typed_snapshot_boundaries() {
    let mut captured = ProjectedRangeFixtureV1::new(7);
    let outer = captured.outer;
    let inner = captured.inner;
    let source = captured.some_place();
    captured.edit(6, |statements, _| {
        statements[0] = typed_assignment(
            10,
            inner,
            SemanticRvalueKindV1::Use(SemanticOperandV1::Copy(source)),
        );
        statements.push(typed_assignment(
            8,
            outer,
            projected_range_aggregate_v1(SemanticAggregateKindV1::EnumVariant(0), vec![]),
        ));
    });
    assert_eq!(
        captured.range(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 63
        }),
        "a by-value capture keeps its earlier source value after source reassignment"
    );
    assert!(captured.sum_is_proved());
    let fixture = ProjectedRangeFixtureV1::new(7);
    let function = fixture.function();
    // Direct known constructors retain the preexisting unguarded projection case.
    let direct = SemanticPlaceV1::new(
        SemanticLocalIdV1::from_index(7),
        fixture.lane_place().projections().to_vec(),
        U64_TYPE,
    )
    .unwrap();
    let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    assert_eq!(
        proof
            .range_at_operand(&SemanticOperandV1::Copy(direct), 3, 2)
            .unwrap(),
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 63
        })
    );
    for projections in [
        vec![
            SemanticProjectionV1::new(SemanticProjectionKindV1::Downcast(1), fixture.outer)
                .unwrap(),
            SemanticProjectionV1::new(SemanticProjectionKindV1::Field(0), fixture.payload).unwrap(),
            SemanticProjectionV1::new(SemanticProjectionKindV1::Field(1), U64_TYPE).unwrap(),
        ],
        vec![
            SemanticProjectionV1::new(SemanticProjectionKindV1::Downcast(1), fixture.inner)
                .unwrap(),
            SemanticProjectionV1::new(SemanticProjectionKindV1::Field(0), fixture.payload).unwrap(),
            SemanticProjectionV1::new(SemanticProjectionKindV1::Field(2), U64_TYPE).unwrap(),
        ],
        vec![SemanticProjectionV1::new(SemanticProjectionKindV1::Dereference, U64_TYPE).unwrap()],
    ] {
        let malformed =
            SemanticPlaceV1::new(SemanticLocalIdV1::from_index(10), projections, U64_TYPE).unwrap();
        let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert_eq!(
            proof
                .range_at_operand(&SemanticOperandV1::Copy(malformed), 8, 0)
                .unwrap(),
            None
        );
    }
}

#[test]
fn assertion_projected_range_bounds_cycles_and_shared_work() {
    let mut cycle = ProjectedRangeFixtureV1::new(7);
    cycle.edit(4, |statements, terminator| {
        statements.clear();
        *terminator = projected_range_switch_v1(2, &[(0, 4)], 5);
    });
    assert_eq!(
        cycle.range(),
        None,
        "unseeded reaching cycles cannot establish a payload range"
    );
    let fixture = ProjectedRangeFixtureV1::new(7);
    let function = fixture.function();
    let operand = SemanticOperandV1::Copy(fixture.lane_place());
    let mut measured = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    let expected = measured.range_at_operand(&operand, 8, 0).unwrap();
    assert_eq!(
        expected,
        Some(UnsignedRangeProofV1 {
            minimum: 0,
            maximum: 63
        })
    );
    let required = measured.work;
    assert!(required > 1 && required < MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    let mut exact = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    exact.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - required;
    assert_eq!(exact.range_at_operand(&operand, 8, 0).unwrap(), expected);
    assert_eq!(exact.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    let mut short = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    short.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - required + 1;
    assert!(matches!(
        short.range_at_operand(&operand, 8, 0),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
    ));
    let mut overflow = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    overflow.work = usize::MAX;
    assert!(matches!(
        overflow.range_at_operand(&operand, 8, 0),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::Overflow
    ));
}

fn projected_range_retry_fixture_v1(retry_none: bool, retry_norm: bool) -> ProjectedRangeFixtureV1 {
    let mut fixture = ProjectedRangeFixtureV1::new(7);
    if retry_none {
        fixture.edit(5, |_, terminator| {
            *terminator = projected_range_switch_v1(9, &[(0, 13), (1, 6)], 12)
        });
    }
    if retry_norm {
        fixture.edit(7, |_, terminator| {
            *terminator = projected_range_switch_v1(11, &[(0, 13), (1, 8)], 12)
        });
    }
    // Retry changes the selector to Key, so a rejected task can reach a later
    // selected use. Both enum captures are refreshed before that use.
    fixture.blocks.push(block(
        93,
        vec![
            statement(SemanticStatementKindV1::StorageDead(
                SemanticLocalIdV1::from_index(8),
            )),
            statement(SemanticStatementKindV1::StorageLive(
                SemanticLocalIdV1::from_index(8),
            )),
            typed_assignment(
                2,
                SCALAR_TYPE,
                SemanticRvalueKindV1::Use(typed_constant(SCALAR_TYPE, 2, 4)),
            ),
        ],
        projected_range_goto_v1(0),
    ));
    fixture
}

#[test]
fn assertion_projected_range_revalidates_nested_loop_guards() {
    for (retry_none, retry_norm) in [(true, false), (false, true), (true, true)] {
        let fixture = projected_range_retry_fixture_v1(retry_none, retry_norm);
        assert_eq!(
            fixture.range(),
            Some(UnsignedRangeProofV1 {
                minimum: 0,
                maximum: 63
            }),
            "fresh loop guards: retry_none={retry_none}, retry_norm={retry_norm}"
        );
        let function = fixture.function();
        let proved = SemanticAssertProofsV1::analyze(&fixture.types, &function).unwrap();
        assert!(
            proved[9] && proved[10],
            "each checked operation needs its own proof"
        );
        let mut proof = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
        assert_eq!(
            proof
                .range_at_operand(&checked_field_operand(16, 0, U64_TYPE), 10, 2)
                .unwrap(),
            Some(UnsignedRangeProofV1 {
                minimum: 0,
                maximum: 4095
            })
        );
    }
    let fixture = projected_range_retry_fixture_v1(true, true);
    let function = fixture.function();
    let operand = SemanticOperandV1::Copy(fixture.lane_place());
    let mut measured = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    let expected = measured.range_at_operand(&operand, 8, 0).unwrap();
    let required = measured.work;
    assert!(required > 1 && required < MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    let mut exact = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    exact.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - required;
    assert_eq!(exact.range_at_operand(&operand, 8, 0).unwrap(), expected);
    assert_eq!(exact.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    let mut short = SemanticAssertProofsV1::new(&fixture.types, &function).unwrap();
    short.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1 - required + 1;
    assert!(matches!(
        short.range_at_operand(&operand, 8, 0),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
                if diagnostic.reason == GraphWorkFailureV1::AboveLimit
    ));
}

#[test]
fn assertion_projected_range_rejects_post_guard_mutation_and_bypasses() {
    for (guard, tag, carrier, destination, selected) in [(5, 9, 8, 20, 6), (7, 11, 10, 7, 8)] {
        for mutation in 0..7 {
            let mut fixture = projected_range_retry_fixture_v1(true, true);
            let ty = if carrier == 8 {
                fixture.outer
            } else {
                fixture.inner
            };
            fixture.edit(guard, |_, terminator| {
                *terminator = projected_range_switch_v1(tag, &[(0, 13), (1, 14)], 12)
            });
            let mut statements = vec![];
            let mut terminator = projected_range_goto_v1(selected);
            match mutation {
                0 => statements.push(typed_assignment(
                    carrier,
                    ty,
                    SemanticRvalueKindV1::Use(typed_operand(destination, ty)),
                )),
                1 | 2 => statements.push(statement(if mutation == 1 {
                    SemanticStatementKindV1::StorageDead(SemanticLocalIdV1::from_index(carrier))
                } else {
                    SemanticStatementKindV1::StorageLive(SemanticLocalIdV1::from_index(carrier))
                })),
                3 => statements.push(typed_assignment(
                    destination,
                    ty,
                    SemanticRvalueKindV1::Use(SemanticOperandV1::Move(typed_place(carrier, ty))),
                )),
                4 | 5 => {
                    let arguments = if mutation == 4 {
                        vec![SemanticOperandV1::Move(typed_place(carrier, ty))]
                    } else {
                        vec![]
                    };
                    let result = if mutation == 5 {
                        typed_place(carrier, ty)
                    } else {
                        typed_place(0, U64_TYPE)
                    };
                    terminator = SemanticTerminatorKindV1::Call(
                        SemanticDirectCallV1::new_callable(
                            SemanticCallableIdV1::from_index(0),
                            arguments,
                            Some(SemanticCallDestinationV1::new(
                                result,
                                cfg_edge(SemanticEdgeRoleV1::CallReturn, selected),
                            )),
                            SemanticUnwindActionV1::Unreachable,
                        )
                        .unwrap(),
                    );
                }
                _ => {
                    // Mutating the carrier after tag capture but before its
                    // switch is not repaired by taking that switch's edge.
                    fixture.edit(guard, |current, _| {
                        current.push(typed_assignment(
                            carrier,
                            ty,
                            SemanticRvalueKindV1::Use(typed_operand(destination, ty)),
                        ))
                    });
                }
            }
            fixture.blocks.push(block(94, statements, terminator));
            assert_eq!(
                fixture.range(),
                None,
                "carrier {carrier}, mutation {mutation}"
            );
            assert!(
                !fixture.sum_is_proved(),
                "carrier {carrier}, mutation {mutation}"
            );
        }
    }
    for target in [6, 8] {
        let mut bypass = projected_range_retry_fixture_v1(true, true);
        bypass.edit(13, |_, terminator| {
            *terminator = projected_range_goto_v1(target)
        });
        assert_eq!(
            bypass.range(),
            None,
            "retry bypasses fresh guard before block {target}"
        );
        assert!(!bypass.sum_is_proved());
    }
    for replace_with_norm in [false, true] {
        let mut reentry = projected_range_retry_fixture_v1(true, true);
        reentry.edit(8, |_, terminator| {
            *terminator = projected_range_switch_v1(2, &[(0, 11)], 14)
        });
        let mut statements = vec![];
        if replace_with_norm {
            statements.push(typed_assignment(
                5,
                reentry.payload,
                projected_range_aggregate_v1(
                    SemanticAggregateKindV1::Aggregate,
                    vec![
                        typed_constant(U64_TYPE, 0, 8),
                        typed_constant(U64_TYPE, u64::MAX.into(), 8),
                    ],
                ),
            ));
            statements.push(typed_assignment(
                10,
                reentry.inner,
                projected_range_aggregate_v1(
                    SemanticAggregateKindV1::EnumVariant(0),
                    vec![typed_operand(5, reentry.payload)],
                ),
            ));
        }
        // The second visit exits, but it has no fresh Key guard. In particular,
        // an old Key fact must not discard the intervening Norm constructor.
        statements.push(typed_assignment(
            2,
            SCALAR_TYPE,
            SemanticRvalueKindV1::Use(typed_constant(SCALAR_TYPE, 0, 4)),
        ));
        reentry
            .blocks
            .push(block(94, statements, projected_range_goto_v1(8)));
        let function = reentry.function();
        let mut proof = SemanticAssertProofsV1::new(&reentry.types, &function).unwrap();
        assert!(
            !proof
                .projected_range_variant_guard_v1(
                    10,
                    1,
                    ScalarAssignmentSiteV1 {
                        block: 8,
                        statement: 0
                    },
                )
                .unwrap(),
            "the guard itself must reject use reentry, replace_with_norm={replace_with_norm}"
        );
        assert_eq!(
            reentry.range(),
            None,
            "use reentry without a fresh guard, replace_with_norm={replace_with_norm}"
        );
    }
    let mut stale = projected_range_retry_fixture_v1(true, true);
    let inner = stale.inner;
    stale.edit(7, |statements, _| {
        statements.push(typed_assignment(
            10,
            inner,
            SemanticRvalueKindV1::Use(typed_operand(7, inner)),
        ));
    });
    assert_eq!(
        stale.range(),
        None,
        "a saved tag cannot certify a later carrier value"
    );
    assert!(!stale.sum_is_proved());
}
