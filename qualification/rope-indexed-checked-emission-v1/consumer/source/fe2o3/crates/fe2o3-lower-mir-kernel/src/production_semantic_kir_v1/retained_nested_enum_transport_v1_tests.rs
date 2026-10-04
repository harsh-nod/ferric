use fe2o3_mir_model::semantic_mir_v1::{
    SemanticLayoutNicheV1, SemanticNicheEnumEncodingV1, SemanticNichePathComponentV1,
    SemanticNicheSourceV1,
};

fn nested_variant_layout(
    variant: u32,
    size: u64,
    alignment: u64,
    offsets: Vec<u64>,
    niche: Option<SemanticLayoutNicheV1>,
) -> SemanticEnumVariantLayoutV1 {
    SemanticEnumVariantLayoutV1::from_rustc(
        variant,
        size,
        alignment,
        SemanticFieldsShapeV1::arbitrary(offsets.clone(), (0..offsets.len() as u32).collect())
            .unwrap(),
        SemanticBackendReprV1::memory(true),
        niche,
        false,
        None,
        alignment,
        0,
        SemanticAggregateLayoutV1::new(offsets, vec![]).unwrap(),
    )
    .unwrap()
}

struct NestedRetainedEnumFixture {
    f: ReferenceFixture,
    wave: SemanticTypeIdV1,
    option: SemanticTypeIdV1,
    norm: u32,
    key: u32,
    inner_norm: u32,
    inner_key: u32,
    other_norm: u32,
    other_inner: u32,
    outer: u32,
    sibling: u32,
    outer_tag: u32,
    inner_tag: u32,
    tag_copy: u32,
    scalar: u32,
    half: u32,
    visited: u32,
}

#[derive(Clone, Copy)]
enum NestedSourceCase {
    Plain,
    CopiedTag,
    OuterTag,
    SiblingTag,
    SiblingPathTag,
    MovedScalar,
    MovedScalarAliasOnly,
    RepairedScalar,
    DeadWorker,
    KnownReplacement,
    CursorReplacement,
    StaleLoop,
}

impl NestedRetainedEnumFixture {
    fn new() -> Self {
        let mut f = ReferenceFixture::new();
        let logical_tag = add_type(
            &mut f.types,
            scalar_layout(SemanticBackendPrimitiveV1::integer(true, 64, 8), 8),
            SemanticTypeShapeV1::Scalar(SemanticScalarTypeV1::Integer {
                signed: true,
                bits: 64,
            }),
        );
        let norm_type = add_type(
            &mut f.types,
            aggregate_layout(32, 8, vec![0, 24, 8, 16]),
            SemanticTypeShapeV1::Aggregate(
                SemanticAggregateTypeV1::new(vec![
                    f.local_types[STORAGE_REF as usize],
                    f.word,
                    f.local_types[WRITTEN_REF as usize],
                    f.local_types[VALID_REF as usize],
                ])
                .unwrap(),
            ),
        );
        let key_type = add_type(
            &mut f.types,
            aggregate_layout(40, 8, vec![0, 24, 32, 8, 16]),
            SemanticTypeShapeV1::Aggregate(
                SemanticAggregateTypeV1::new(vec![
                    f.local_types[STORAGE_REF as usize],
                    f.word,
                    f.word,
                    f.local_types[WRITTEN_REF as usize],
                    f.local_types[VALID_REF as usize],
                ])
                .unwrap(),
            ),
        );
        // Match V88's physical pointer niche, without confusing that physical
        // tag with the signed-I64 logical discriminant consumed by MIR switches.
        let pointer = SemanticBackendPrimitiveV1::pointer(0, 8, 8);
        let niche = SemanticLayoutNicheV1::new(
            0,
            pointer,
            SemanticScalarValidityRangeV1::new(1, u128::from(u64::MAX)),
        )
        .unwrap();
        let wave = add_type(
            &mut f.types,
            SemanticTypeLayoutV1::enum_layout(
                40,
                8,
                SemanticEnumLayoutV1::new(
                    vec![
                        nested_variant_layout(0, 40, 8, vec![8], None),
                        nested_variant_layout(1, 40, 8, vec![0], Some(niche)),
                    ],
                    SemanticEnumEncodingV1::Niche(
                        SemanticNicheEnumEncodingV1::new(
                            0,
                            SemanticNicheSourceV1::new(
                                vec![
                                    SemanticNichePathComponentV1::Field(0),
                                    SemanticNichePathComponentV1::Field(0),
                                ],
                                0,
                            )
                            .unwrap(),
                            niche,
                            SemanticBackendScalarV1::initialized(
                                pointer,
                                SemanticScalarValidityRangeV1::new(1, 0),
                            ),
                            1,
                            0,
                            0,
                            0,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap(),
            )
            .unwrap(),
            SemanticTypeShapeV1::Enum {
                discriminant: logical_tag,
                variants: vec![
                    SemanticEnumVariantV1::new(
                        0,
                        SemanticAggregateTypeV1::new(vec![norm_type]).unwrap(),
                    ),
                    SemanticEnumVariantV1::new(
                        1,
                        SemanticAggregateTypeV1::new(vec![key_type]).unwrap(),
                    ),
                ]
                .into_boxed_slice(),
            },
        );
        let option = add_type(
            &mut f.types,
            SemanticTypeLayoutV1::enum_layout(
                48,
                8,
                SemanticEnumLayoutV1::new(
                    vec![
                        nested_variant_layout(0, 8, 1, vec![], None),
                        nested_variant_layout(1, 48, 8, vec![8], None),
                    ],
                    SemanticEnumEncodingV1::Direct(SemanticDirectEnumEncodingV1::new(
                        0,
                        0,
                        SemanticBackendScalarV1::initialized(
                            SemanticBackendPrimitiveV1::integer(false, 64, 8),
                            SemanticScalarValidityRangeV1::new(0, 1),
                        ),
                    )),
                )
                .unwrap(),
            )
            .unwrap(),
            SemanticTypeShapeV1::Enum {
                discriminant: logical_tag,
                variants: vec![
                    SemanticEnumVariantV1::new(0, SemanticAggregateTypeV1::new(vec![]).unwrap()),
                    SemanticEnumVariantV1::new(
                        1,
                        SemanticAggregateTypeV1::new(vec![wave]).unwrap(),
                    ),
                ]
                .into_boxed_slice(),
            },
        );
        let norm = f.temporary(norm_type);
        let key = f.temporary(key_type);
        let inner_norm = f.temporary(wave);
        let inner_key = f.temporary(wave);
        let other_norm = f.temporary(norm_type);
        let other_inner = f.temporary(wave);
        let outer = f.temporary(option);
        let sibling = f.temporary(option);
        let outer_tag = f.temporary(logical_tag);
        let inner_tag = f.temporary(logical_tag);
        let tag_copy = f.temporary(logical_tag);
        let scalar = f.temporary(f.word);
        let half = f.temporary(f.local_types[HALF as usize]);
        let visited = f.temporary(f.boolean);
        Self {
            f,
            wave,
            option,
            norm,
            key,
            inner_norm,
            inner_key,
            other_norm,
            other_inner,
            outer,
            sibling,
            outer_tag,
            inner_tag,
            tag_copy,
            scalar,
            half,
            visited,
        }
    }

    fn setup(&self) -> Vec<SemanticStatementV1> {
        let mut statements = self.f.setup();
        // The real constructors copy the three live references into their
        // respective task variants; do not consume them into the old Task helper.
        statements.remove(7);
        statements
    }

    fn some(&self, variant: u32) -> Vec<SemanticStatementV1> {
        let f = &self.f;
        let (payload, inner) = if variant == 0 {
            (self.norm, self.inner_norm)
        } else {
            (self.key, self.inner_key)
        };
        let mut fields = vec![
            f.copy(STORAGE_REF),
            f.word(if variant == 0 { 311 } else { 313 }),
        ];
        if variant == 1 {
            fields.push(f.word(317));
        }
        fields.extend([f.copy(WRITTEN_REF), f.copy(VALID_REF)]);
        let mut statements = vec![
            f.aggregate(f.local(payload), fields),
            f.variant(inner, variant, vec![f.move_local(payload)]),
        ];
        let SemanticTypeShapeV1::Enum { variants, .. } =
            f.types[self.option.index() as usize].shape()
        else {
            unreachable!()
        };
        let mut outer_fields = vec![f.move_local(inner)];
        if variants[1].fields().fields().len() == 2 {
            statements.push(f.aggregate(
                f.local(self.other_norm),
                vec![
                    f.copy(STORAGE_REF),
                    f.word(331),
                    f.copy(WRITTEN_REF),
                    f.copy(VALID_REF),
                ],
            ));
            statements.push(f.variant(self.other_inner, 0, vec![f.move_local(self.other_norm)]));
            outer_fields.push(f.move_local(self.other_inner));
        }
        statements.push(f.variant(self.outer, 1, outer_fields));
        statements
    }

    fn inner_place(&self) -> SemanticPlaceV1 {
        self.f.variant_field(self.f.local(self.outer), 1, 0)
    }

    fn payload(&self, variant: u32) -> SemanticPlaceV1 {
        self.f.variant_field(self.inner_place(), variant, 0)
    }

    fn output(
        &self,
        variant: u32,
        index: u64,
        operand: SemanticOperandV1,
    ) -> Vec<SemanticStatementV1> {
        let f = &self.f;
        let storage = f.deref(f.field(self.payload(variant), 0));
        let array = f.deref(f.field(storage, Role::Normalized.field_index()));
        let element = f.project(
            array,
            SemanticProjectionKindV1::ConstantIndex {
                offset: index,
                minimum_length: Role::Normalized.elements(),
                from_end: false,
            },
            f.local_types[HALF as usize],
        );
        vec![
            f.set(f.local(self.scalar), operand),
            f.assign(
                f.local(self.half),
                SemanticRvalueKindV1::Cast {
                    kind: SemanticCastKindV1::Integer,
                    operand: f.copy(self.scalar),
                },
            ),
            f.set(element, f.copy(self.half)),
        ]
    }

    fn observe(
        &self,
        variant: u32,
        move_scalars: bool,
        reload_round: bool,
    ) -> Vec<SemanticStatementV1> {
        let f = &self.f;
        let payload = self.payload(variant);
        let read = |place| {
            if move_scalars {
                SemanticOperandV1::Move(place)
            } else {
                SemanticOperandV1::Copy(place)
            }
        };
        let mut statements = self.output(
            variant,
            u64::from(variant),
            read(f.field(payload.clone(), 1)),
        );
        if variant == 1 {
            statements.extend(self.output(variant, 2, read(f.field(payload.clone(), 2))));
        }
        // Reference fields must remain usable after moving the disjoint lane/base
        // scalars. The Round reload also proves the original logical alias.
        let written = f.deref(f.field(payload, if variant == 0 { 2 } else { 3 }));
        statements.push(f.set(written, f.word(if variant == 0 { 401 } else { 409 })));
        if reload_round {
            statements.extend(self.output(
                variant,
                3 + u64::from(variant),
                SemanticOperandV1::Copy(f.field(f.local(ROUND), 1)),
            ));
        }
        statements
    }

    fn request(
        mut self,
        case: NestedSourceCase,
        repeat: bool,
    ) -> (
        InertSemanticMirRequestV1,
        u32,
        SemanticTypeIdV1,
        SemanticTypeIdV1,
    ) {
        let stale_loop = matches!(case, NestedSourceCase::StaleLoop);
        assert!(!repeat || matches!(case, NestedSourceCase::Plain | NestedSourceCase::StaleLoop));
        assert!(!stale_loop || repeat);
        if matches!(case, NestedSourceCase::SiblingPathTag) {
            // This separate valid two-field enum isolates path identity while
            // preserving the same root SSA value and exact inner niche type.
            let logical_tag = self.f.local_types[self.outer_tag as usize];
            self.option = add_type(
                &mut self.f.types,
                SemanticTypeLayoutV1::enum_layout(
                    88,
                    8,
                    SemanticEnumLayoutV1::new(
                        vec![
                            nested_variant_layout(0, 8, 1, vec![], None),
                            nested_variant_layout(1, 88, 8, vec![8, 48], None),
                        ],
                        SemanticEnumEncodingV1::Direct(SemanticDirectEnumEncodingV1::new(
                            0,
                            0,
                            SemanticBackendScalarV1::initialized(
                                SemanticBackendPrimitiveV1::integer(false, 64, 8),
                                SemanticScalarValidityRangeV1::new(0, 1),
                            ),
                        )),
                    )
                    .unwrap(),
                )
                .unwrap(),
                SemanticTypeShapeV1::Enum {
                    discriminant: logical_tag,
                    variants: vec![
                        SemanticEnumVariantV1::new(
                            0,
                            SemanticAggregateTypeV1::new(vec![]).unwrap(),
                        ),
                        SemanticEnumVariantV1::new(
                            1,
                            SemanticAggregateTypeV1::new(vec![self.wave, self.wave]).unwrap(),
                        ),
                    ]
                    .into_boxed_slice(),
                },
            );
            self.f.local_types[self.outer as usize] = self.option;
        }
        let f = &self.f;
        let mut setup = self.setup();
        if matches!(case, NestedSourceCase::SiblingTag) {
            let mut sibling = self.some(0);
            sibling[2] = f.variant(self.sibling, 1, vec![f.move_local(self.inner_norm)]);
            setup.extend(sibling);
        }
        let mut entry = if repeat {
            vec![setup.remove(0), setup.pop().unwrap()]
        } else {
            std::mem::take(&mut setup)
        };
        if stale_loop {
            entry.push(f.set(f.local(self.visited), f.boolean(false)));
            entry.push(f.variant(self.outer, 0, vec![]));
        }
        let mut tag = vec![match case {
            NestedSourceCase::OuterTag => f.set(f.local(self.inner_tag), f.copy(self.outer_tag)),
            NestedSourceCase::SiblingTag => f.assign(
                f.local(self.inner_tag),
                SemanticRvalueKindV1::Discriminant(f.variant_field(f.local(self.sibling), 1, 0)),
            ),
            NestedSourceCase::SiblingPathTag => f.assign(
                f.local(self.inner_tag),
                SemanticRvalueKindV1::Discriminant(f.variant_field(f.local(self.outer), 1, 1)),
            ),
            _ => f.assign(
                f.local(self.inner_tag),
                SemanticRvalueKindV1::Discriminant(self.inner_place()),
            ),
        }];
        if matches!(case, NestedSourceCase::CopiedTag) {
            tag.push(f.set(f.local(self.tag_copy), f.copy(self.inner_tag)));
        }
        let selected_tag = if matches!(case, NestedSourceCase::CopiedTag) {
            self.tag_copy
        } else {
            self.inner_tag
        };
        let mut outer_guard = match case {
            NestedSourceCase::MovedScalar
            | NestedSourceCase::MovedScalarAliasOnly
            | NestedSourceCase::RepairedScalar => {
                vec![scalar_move_take(f)]
            }
            NestedSourceCase::DeadWorker => vec![SemanticStatementV1::new(
                SemanticSourceProvenanceV1::unavailable(),
                SemanticStatementKindV1::StorageDead(SemanticLocalIdV1::from_index(WORKER)),
            )],
            _ => vec![],
        };
        if matches!(case, NestedSourceCase::RepairedScalar) {
            outer_guard.push(f.set(f.field(f.local(ROUND), 1), f.word(439)));
        }
        outer_guard.push(f.assign(
            f.local(self.outer_tag),
            SemanticRvalueKindV1::Discriminant(f.local(self.outer)),
        ));
        let mut norm_observe = match case {
            NestedSourceCase::KnownReplacement => self.some(1),
            NestedSourceCase::CursorReplacement => vec![
                f.assign(
                    f.local(self.inner_tag),
                    SemanticRvalueKindV1::Discriminant(self.inner_place()),
                ),
                f.set(f.local(self.outer), f.copy(self.sibling)),
                f.set(f.local(self.outer), f.copy(self.sibling)),
            ],
            _ => vec![],
        };
        let reload_round = !matches!(case, NestedSourceCase::MovedScalarAliasOnly);
        norm_observe.extend(self.observe(0, !stale_loop, reload_round));
        let mut norm_constructor = self.some(0);
        let mut key_constructor = self.some(1);
        let mut none_constructor = vec![f.variant(self.outer, 0, vec![])];
        if matches!(case, NestedSourceCase::CursorReplacement) {
            // Both carriers genuinely merge; the sibling's opposite variant is
            // not inferred by a switch on the original carrier's inner tag.
            let mut sibling_key = self.some(1);
            *sibling_key.last_mut().unwrap() =
                f.variant(self.sibling, 1, vec![f.move_local(self.inner_key)]);
            norm_constructor.extend(sibling_key);
            let mut sibling_norm = self.some(0);
            *sibling_norm.last_mut().unwrap() =
                f.variant(self.sibling, 1, vec![f.move_local(self.inner_norm)]);
            key_constructor.extend(sibling_norm);
            none_constructor.push(f.variant(self.sibling, 0, vec![]));
        }
        let finish = || {
            if repeat {
                f.goto(10)
            } else {
                SemanticTerminatorKindV1::Return
            }
        };
        let mut blocks = vec![
            f.block(
                0,
                entry,
                if repeat {
                    f.goto(11)
                } else {
                    f.switch_value(f.copy(HALF), 0, 3, 4)
                },
            ),
            f.block(1, norm_constructor, f.goto(5)),
            f.block(2, key_constructor, f.goto(5)),
            f.block(3, none_constructor, f.goto(5)),
            f.block(4, vec![], f.switch_value(f.copy(HALF), 1, 1, 2)),
            f.block(
                5,
                outer_guard,
                f.switch_value(f.copy(self.outer_tag), 1, 6, 9),
            ),
            f.block(6, tag, f.switch_value(f.copy(selected_tag), 0, 7, 8)),
            f.block(7, norm_observe, finish()),
            f.block(8, self.observe(1, !stale_loop, reload_round), finish()),
            f.block(9, vec![], SemanticTerminatorKindV1::Return),
        ];
        if repeat {
            let mut retire = vec![
                SemanticStatementV1::new(
                    SemanticSourceProvenanceV1::unavailable(),
                    SemanticStatementKindV1::StorageDead(SemanticLocalIdV1::from_index(ROUND)),
                ),
                SemanticStatementV1::new(
                    SemanticSourceProvenanceV1::unavailable(),
                    SemanticStatementKindV1::StorageDead(SemanticLocalIdV1::from_index(WORKER)),
                ),
            ];
            if stale_loop {
                retire.push(f.set(f.local(self.visited), f.boolean(true)));
            }
            blocks.push(f.block(10, retire, f.goto(11)));
            blocks.push(f.block(
                11,
                setup,
                if stale_loop {
                    f.switch_value(f.copy(self.visited), 1, 5, 12)
                } else {
                    f.switch_value(f.copy(HALF), 0, 3, 4)
                },
            ));
            if stale_loop {
                blocks.push(f.block(12, vec![], f.switch_value(f.copy(HALF), 0, 3, 4)));
            }
        }
        let (outer, wave, option) = (self.outer, self.wave, self.option);
        (self.f.request(blocks), outer, wave, option)
    }
}

fn assert_nested_source_shape(
    request: &InertSemanticMirRequestV1,
    outer: u32,
    wave: SemanticTypeIdV1,
    option: SemanticTypeIdV1,
) {
    let admitted = request
        .clone()
        .admit(SemanticMirLimitsV1::default())
        .unwrap();
    let SemanticRustcVariantsV1::Multiple(layout) =
        admitted.types()[wave.index() as usize].layout().variants()
    else {
        panic!("inner enum lost its multiple-variant layout");
    };
    assert!(matches!(
        layout.encoding(),
        SemanticEnumEncodingV1::Niche(_)
    ));
    let SemanticRustcVariantsV1::Multiple(layout) = admitted.types()[option.index() as usize]
        .layout()
        .variants()
    else {
        panic!("outer enum lost its multiple-variant layout");
    };
    assert!(matches!(
        layout.encoding(),
        SemanticEnumEncodingV1::Direct(_)
    ));
    let SemanticTypeShapeV1::Enum { discriminant, .. } =
        admitted.types()[wave.index() as usize].shape()
    else {
        unreachable!()
    };
    assert!(matches!(
        admitted.types()[discriminant.index() as usize].shape(),
        SemanticTypeShapeV1::Scalar(SemanticScalarTypeV1::Integer {
            signed: true,
            bits: 64
        })
    ));
    let ssa = reference_semantic_ssa(request.clone()).unwrap();
    let plan = ssa
        .plan_for_function(SemanticFunctionIdV1::from_index(0))
        .unwrap()
        .plan();
    assert!(
        plan.transport_variables(fe2o3_mir_model::SsaBlockIdV1::new(5))
            .unwrap()
            .iter()
            .any(|variable| variable.get() == outer)
    );
    let incoming: BTreeSet<_> = [1, 2, 3]
        .into_iter()
        .map(|block| {
            plan.edge_arguments(fe2o3_mir_model::SsaEdgeIdV1::new(
                fe2o3_mir_model::SsaBlockIdV1::new(block),
                0,
            ))
            .unwrap()
            .iter()
            .find(|argument| argument.variable().get() == outer)
            .unwrap()
            .value()
        })
        .collect();
    assert_eq!(incoming.len(), 3);
}

fn assert_nested_selected_payload_outputs(owner: &ProductionPreRankedKirOwnerV1) {
    let roots = owner
        .wave_task_access_roots_for_root_v1(SemanticFunctionIdV1::from_index(0))
        .unwrap()
        .unwrap();
    assert_eq!(roots.accesses().len(), 6);
    assert_eq!(roots.accesses()[0].root().role(), Role::Input);
    assert!(
        roots.accesses()[1..]
            .iter()
            .all(|row| row.root().role() == Role::Normalized)
    );
    let function = &owner.executable().module().functions[0];
    let body = function.body.as_ref().unwrap();
    let operations: Vec<_> = body
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(block, body)| {
            body.operations
                .iter()
                .map(move |operation| (block, operation))
        })
        .collect();
    let definition = |value| {
        operations
            .iter()
            .copied()
            .find(|(_, operation)| operation.results.iter().any(|result| result.id == value))
            .unwrap()
    };
    let mut correlation_budget = UnsupportedIndexCorrelationBudgetV1 { remaining: 16384 };
    let index = build_kir_correlation_index(body, 16384, &mut correlation_budget).unwrap();
    for (output_index, expected) in [311, 313, 317, 401, 409].into_iter().enumerate() {
        let outputs: Vec<_> = operations.iter().filter_map(|(block, operation)| {
            let OperationKind::Store { pointer, value, access } = operation.kind else { return None; };
            if access.address_space != AddressSpace::Global { return None; }
            let address = definition(pointer).1;
            let OperationKind::GetElementPointer { base, offset } = address.kind else { return None; };
            if !matches!(definition(offset).1.kind, OperationKind::Constant(Constant::Index(value)) if value == output_index as u64) { return None; }
            let origin = external_allocation_parameter_v1(function, &index, base, &mut BTreeSet::new(), &mut correlation_budget).unwrap();
            assert_eq!(body.parameters[origin as usize], roots.accesses()[1].root().parameter());
            assert_eq!(address.results[0].ty, Type::pointer(Type::Scalar(ScalarType::U16), AddressSpace::Global, AccessMode::ReadWrite));
            Some((*block, value))
        }).collect();
        assert_eq!(outputs.len(), 1);
        let (output_block, stored) = outputs[0];
        let OperationKind::Cast { value, ref to, .. } = definition(stored).1.kind else {
            panic!("nested field output lost its U16 conversion");
        };
        assert_eq!(*to, Type::Scalar(ScalarType::U16));
        let (load_block, load) = definition(value);
        let OperationKind::Load { pointer, access } = load.kind else {
            panic!("nested field output was not restored from its private slot");
        };
        assert_eq!(access.address_space, AddressSpace::Private);
        assert_eq!(load.results[0].ty, Type::Scalar(ScalarType::U64));
        let constants: BTreeSet<_> = operations.iter().filter_map(|(_, operation)| {
            matches!(operation.kind, OperationKind::Constant(Constant::U64(value)) if value == expected).then(|| operation.results[0].id)
        }).collect();
        assert!(operations.iter().any(|(store_block, operation)| {
            matches!(operation.kind, OperationKind::Store { pointer: destination, value, access }
                if destination == pointer && constants.contains(&value) && access.address_space == AddressSpace::Private)
                && if output_index < 3 { *store_block != load_block } else { *store_block == load_block }
        }), "output {output_index} must consume its own payload or original Round alias, not a sibling value");
        if output_index < 3 {
            assert_eq!(load_block, output_block);
            assert!(operations.iter().filter(|(_, operation)| matches!(operation.kind, OperationKind::Load { pointer: read, .. } if read == pointer))
                .all(|(block, _)| *block == output_block), "inactive variants must not load this payload slot");
        }
    }
}

#[test]
fn retained_nested_enum_niche_join_restores_only_selected_payload_and_exact_aliases() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::Plain, false);
    assert_nested_source_shape(&request, outer, wave, option);
    let owner = materialize_retained_request(request).unwrap();
    assert_nested_selected_payload_outputs(&owner);
}

#[test]
fn retained_nested_enum_exact_discriminator_copy_preserves_path_provenance() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::CopiedTag, false);
    assert_nested_source_shape(&request, outer, wave, option);
    let owner = materialize_retained_request(request).unwrap();
    assert_nested_selected_payload_outputs(&owner);
}

#[test]
fn retained_nested_enum_loop_renews_every_root_reference_and_variant_payload() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::Plain, true);
    assert_nested_source_shape(&request, outer, wave, option);
    let owner = materialize_retained_request(request).unwrap();
    assert_nested_selected_payload_outputs(&owner);
}

#[test]
fn retained_nested_enum_outer_discriminator_does_not_authenticate_inner_payload() {
    let (request, ..) = NestedRetainedEnumFixture::new().request(NestedSourceCase::OuterTag, false);
    expect_unsupported(
        request,
        "nested enum path lacks an authenticated current SSA variant",
    );
}

#[test]
fn retained_nested_enum_same_type_sibling_tag_does_not_authenticate_this_carrier() {
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::SiblingTag, false);
    expect_unsupported(
        request,
        "nested enum path lacks an authenticated current SSA variant",
    );
}

#[test]
fn retained_nested_enum_same_root_sibling_path_does_not_authenticate_this_payload() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::SiblingPathTag, false);
    assert_nested_source_shape(&request, outer, wave, option);
    expect_unsupported(
        request,
        "nested enum path lacks an authenticated current SSA variant",
    );
}

#[test]
fn retained_nested_enum_referent_scalar_move_invalidates_saved_references() {
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::MovedScalar, false);
    // RPO visits block 8 before 7; both directly reload the moved Round field.
    expect_scalar_move_ssa_unavailable(request, 8, 7, ROUND);
    // Without that direct reload, use of the saved nested reference must reach
    // and fail the retained-origin proof rather than the syntactic move gate.
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::MovedScalarAliasOnly, false);
    expect_unsupported(
        request,
        "retained reference origin is stale after a scalar field move",
    );
}

#[test]
fn retained_nested_enum_scalar_repair_does_not_revive_saved_references() {
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::RepairedScalar, false);
    expect_unsupported(
        request,
        "retained reference origin is stale after a scalar field move",
    );
}

#[test]
fn retained_nested_enum_payload_requires_the_original_worker_to_remain_alive() {
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::DeadWorker, false);
    match materialize_retained_request(request) {
        Err(ProductionPreRankedKirErrorV1::Lowering(
            ProductionSemanticKirErrorV1::MissingLocalDefinition {
                function: 0,
                block: 5,
                statement: Some(1),
                local: WORKER,
            },
        )) => {}
        Err(error) => panic!("unexpected dead nested-carrier origin refusal: {error:?}"),
        Ok(_) => panic!("nested carrier restored a dead Worker"),
    }
}

#[test]
fn retained_nested_enum_known_reassignment_cannot_reuse_the_previous_inner_variant() {
    let (request, ..) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::KnownReplacement, false);
    expect_unsupported(
        request,
        "nested enum path lacks an authenticated current SSA variant",
    );
}

#[test]
fn retained_nested_enum_reinitialized_loop_roots_do_not_revive_the_saved_carrier() {
    let (request, ..) = NestedRetainedEnumFixture::new().request(NestedSourceCase::StaleLoop, true);
    expect_unsupported(
        request,
        "retained reference carrier is not valid for this source use",
    );
}

#[test]
fn retained_nested_enum_same_block_unknown_reassignment_uses_the_current_ssa_definition() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::CursorReplacement, false);
    assert_nested_source_shape(&request, outer, wave, option);
    let admitted = request
        .clone()
        .admit(SemanticMirLimitsV1::default())
        .unwrap();
    let function = &admitted.functions()[0];
    let semantic_ssa = reference_semantic_ssa(request).unwrap();
    let option_producers = semantic_option_producers_v1(function, admitted.callables()).unwrap();
    let option_dominance = SemanticOptionDominanceV1::analyze(function, &option_producers).unwrap();
    let bound = ProductionSemanticKirLimitsV1::default().max_operations;
    let control_flow = SemanticControlFlowSsaPlanV1::analyze(
        SemanticSsaTransportInputV1 {
            types: admitted.types(),
            callables: admitted.callables(),
            function,
            semantic_function: SemanticFunctionIdV1::from_index(0),
        },
        semantic_ssa
            .plan_for_function(SemanticFunctionIdV1::from_index(0))
            .unwrap(),
        &option_dominance,
        &BTreeMap::new(),
        bound,
        bound,
    )
    .unwrap();
    let facts =
        analyze_promoted_enum_paths_v1(admitted.types(), function, &control_flow, bound, bound)
            .unwrap();
    let entry = control_flow.entry_value(function, 7, outer).unwrap();
    let definitions = control_flow.definition_values.get(&(7, outer)).unwrap();
    assert_eq!(definitions.len(), 2);
    assert_ne!(definitions[0], entry);
    assert_ne!(definitions[0], definitions[1]);
    let key = |root| SemanticEnumPlaceKeyV1 {
        root,
        path: vec![
            SemanticEnumPathStepV1::Downcast(1),
            SemanticEnumPathStepV1::Field(0),
        ]
        .into_boxed_slice(),
        semantic_type: wave,
    };
    assert_eq!(facts.paths.get(&(7, key(entry))), Some(&0));
    let mut pending: VecDeque<_> = definitions.iter().copied().collect();
    assert_eq!(
        current_enum_ssa_from_definition_cursor_v1(Some(entry), Some(definitions), Some(&pending))
            .unwrap(),
        Some(entry)
    );
    for definition in definitions {
        assert_eq!(pending.pop_front(), Some(*definition));
        let current = current_enum_ssa_from_definition_cursor_v1(
            Some(entry),
            Some(definitions),
            Some(&pending),
        )
        .unwrap()
        .unwrap();
        assert_eq!(current, *definition);
        assert_eq!(
            facts.paths.get(&(7, key(current))),
            None,
            "a consumed unknown replacement cannot use the old block-entry Norm fact"
        );
    }
    let too_many: VecDeque<_> = definitions.iter().copied().chain([entry]).collect();
    assert!(matches!(
        current_enum_ssa_from_definition_cursor_v1(Some(entry), Some(definitions), Some(&too_many)),
        Err(ProductionSemanticKirErrorV1::CorrespondenceMismatch)
    ));
    let wrong_front = VecDeque::from([entry]);
    assert!(matches!(
        current_enum_ssa_from_definition_cursor_v1(
            Some(entry),
            Some(definitions),
            Some(&wrong_front)
        ),
        Err(ProductionSemanticKirErrorV1::CorrespondenceMismatch)
    ));
    assert!(matches!(
        current_enum_ssa_from_definition_cursor_v1(Some(entry), None, Some(&wrong_front)),
        Err(ProductionSemanticKirErrorV1::CorrespondenceMismatch)
    ));
    // This is an admitted SSA/fact-analysis test. Whole unrefined carrier copies
    // remain a separate, explicitly rejected materialization boundary.
}

fn nested_loop_source_definitions() -> (
    SsaValueV1,
    SsaValueV1,
    SsaValueV1,
    SsaValueV1,
    SemanticTypeIdV1,
) {
    let fixture = NestedRetainedEnumFixture::new();
    let (inner_tag, outer_tag) = (fixture.inner_tag, fixture.outer_tag);
    let (request, outer, wave, option) = fixture.request(NestedSourceCase::Plain, true);
    assert_nested_source_shape(&request, outer, wave, option);
    let ssa = reference_semantic_ssa(request).unwrap();
    let plan = ssa
        .plan_for_function(SemanticFunctionIdV1::from_index(0))
        .unwrap()
        .plan();
    let definition = |block, local| {
        plan.resolved_events(fe2o3_mir_model::SsaBlockIdV1::new(block))
            .unwrap()
            .iter()
            .find_map(|(_, event)| match *event {
                SsaResolvedEventV1::Define { variable, value } if variable.get() == local => {
                    Some(value)
                }
                _ => None,
            })
            .unwrap()
    };
    (
        definition(1, outer),
        definition(2, outer),
        definition(6, inner_tag),
        definition(5, outer_tag),
        wave,
    )
}

#[test]
fn retained_nested_enum_loop_reexecution_cannot_reuse_the_same_static_entry_fact() {
    let (root, _, _, _, wave) = nested_loop_source_definitions();
    let definitions = [root];
    let mut pending = VecDeque::from(definitions);
    let key = (
        7,
        SemanticEnumPlaceKeyV1 {
            root,
            path: vec![
                SemanticEnumPathStepV1::Downcast(1),
                SemanticEnumPathStepV1::Field(0),
            ]
            .into_boxed_slice(),
            semantic_type: wave,
        },
    );
    let facts = BTreeMap::from([(key.clone(), 0)]);
    assert!(
        enum_entry_path_variant_is_current_v1(
            &facts,
            &key,
            0,
            Some(root),
            Some(&definitions),
            Some(&pending)
        )
        .unwrap()
    );
    pending.pop_front();
    // On a backedge, the old and freshly evaluated values can have the SAME
    // static SSA identity. Merely comparing the current identity is insufficient.
    assert_eq!(
        current_enum_ssa_from_definition_cursor_v1(Some(root), Some(&definitions), Some(&pending))
            .unwrap(),
        Some(root)
    );
    assert!(
        !enum_entry_path_variant_is_current_v1(
            &facts,
            &key,
            0,
            Some(root),
            Some(&definitions),
            Some(&pending)
        )
        .unwrap()
    );
    // This calls the emitter's shared availability query. Replacing that query
    // with the old exact-map lookup would accept the stale fact and fail here.
    assert_eq!(facts.get(&key), Some(&0));
}

#[test]
fn retained_nested_enum_loop_reexecution_forgets_projected_and_saved_tag_provenance() {
    let (root, other, tag, other_tag, wave) = nested_loop_source_definitions();
    assert_ne!(root, other);
    assert_ne!(tag, other_tag);
    let key = |root| SemanticEnumPlaceKeyV1 {
        root,
        path: vec![
            SemanticEnumPathStepV1::Downcast(1),
            SemanticEnumPathStepV1::Field(0),
        ]
        .into_boxed_slice(),
        semantic_type: wave,
    };
    // This directly exercises the transfer operation at a repeated static
    // definition, independently of source-reference lifetime rejection.
    let mut paths = SemanticEnumPathFactsV1::default();
    paths.insert_variant(key(root), 0);
    paths.insert_variant(key(other), 1);
    paths.insert_discriminant(tag, key(root));
    paths.insert_discriminant(other_tag, key(other));
    let nodes = paths.path_entries();
    let mut path_budget = SemanticEnumAnalysisBudgetV1::new(nodes, nodes);
    path_budget.reserve_temporary_entries(nodes).unwrap();
    paths
        .forget_definition_paths(root, &mut path_budget)
        .unwrap();
    assert_eq!(paths.variants.get(&key(root)), None);
    assert_eq!(paths.discriminants.get(&tag), None);
    assert_eq!(paths.variants.get(&key(other)), Some(&1));
    assert_eq!(paths.discriminants.get(&other_tag), Some(&key(other)));
    assert_eq!(path_budget.storage, paths.path_entries());

    let original = SemanticEnumSsaFactsV1 {
        variants: BTreeMap::from([(root, 0), (other, 1)]),
        discriminants: BTreeMap::from([(tag, root), (other_tag, other)]),
    };
    let work = 1 + original.discriminants.len();
    let mut overwritten_enum = original.clone();
    overwritten_enum
        .forget_enum_definition(root, &mut SemanticEnumAnalysisBudgetV1::new(work, 0))
        .unwrap();
    assert_eq!(overwritten_enum.variants.get(&root), None);
    assert_eq!(
        overwritten_enum.discriminants.get(&tag),
        None,
        "an unchanged saved tag must not identify the freshly evaluated enum"
    );
    assert_eq!(overwritten_enum.variants.get(&other), Some(&1));
    assert_eq!(overwritten_enum.discriminants.get(&other_tag), Some(&other));
    let mut overwritten_tag = original;
    overwritten_tag
        .forget_enum_definition(tag, &mut SemanticEnumAnalysisBudgetV1::new(work, 0))
        .unwrap();
    assert_eq!(overwritten_tag.discriminants.get(&tag), None);
    assert_eq!(overwritten_tag.variants.get(&root), Some(&0));
    assert_eq!(overwritten_tag.discriminants.get(&other_tag), Some(&other));
}

#[test]
fn retained_nested_enum_analysis_uses_the_existing_shared_resource_budget() {
    let (request, outer, wave, option) =
        NestedRetainedEnumFixture::new().request(NestedSourceCase::Plain, false);
    assert_nested_source_shape(&request, outer, wave, option);
    let mut upper = ProductionSemanticKirLimitsV1::default().max_operations;
    let baseline =
        materialize_retained_request_with_limits(request.clone(), request_limits(&request, upper))
            .unwrap();
    assert_nested_selected_payload_outputs(&baseline);
    let mut lower = 0;
    expect_shared_resource_limit(
        materialize_retained_request_with_limits(request.clone(), request_limits(&request, lower)),
        lower,
    );
    while upper - lower > 1 {
        let middle = lower + (upper - lower) / 2;
        match materialize_retained_request_with_limits(
            request.clone(),
            request_limits(&request, middle),
        ) {
            Ok(_) => upper = middle,
            error => {
                expect_shared_resource_limit(error, middle);
                lower = middle;
            }
        }
    }
    let exact =
        materialize_retained_request_with_limits(request.clone(), request_limits(&request, upper))
            .unwrap();
    assert_nested_selected_payload_outputs(&exact);
    assert_eq!(operation_count(&exact), operation_count(&baseline));
    let (resource, actual) = expect_shared_resource_limit(
        materialize_retained_request_with_limits(
            request.clone(),
            request_limits(&request, upper - 1),
        ),
        upper - 1,
    );
    assert!(matches!(
        resource,
        ProductionSemanticKirResourceV1::AnalysisWork
            | ProductionSemanticKirResourceV1::AnalysisStorage
    ));
    assert_eq!(actual, upper);
}
