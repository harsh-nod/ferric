use fe2o3_kernel_ir::*;

#[derive(Clone, Copy)]
enum Effect {
    Read,
    Write,
    Atomic(AtomicKind),
}

fn fixture(parameters: &[(Type, AccessMode)], effects: &[(usize, u64, Effect)]) -> Module {
    let mut next = parameters.len() as u32;
    let mut operations = Vec::new();
    let mut pointers = Vec::new();
    for (parameter, (element, access)) in parameters.iter().enumerate() {
        let pointer = ValueId(next);
        next += 1;
        operations.push(Operation::effect_free(
            ValueDef::new(
                pointer,
                Type::pointer(element.clone(), AddressSpace::Global, *access),
            ),
            OperationKind::SliceData {
                slice: ValueId(parameter as u32),
            },
        ));
        pointers.push(pointer);
    }
    for &(parameter, offset, effect) in effects {
        let (element, mode) = &parameters[parameter];
        let width = match element {
            Type::Scalar(ScalarType::U16) => 2,
            Type::Scalar(ScalarType::U32 | ScalarType::F32) => 4,
            _ => panic!("unsupported fixture element"),
        };
        let access = MemoryAccess::new(AddressSpace::Global, width);
        let offset_value = ValueId(next);
        next += 1;
        operations.push(Operation::effect_free(
            ValueDef::new(offset_value, Type::INDEX),
            OperationKind::Constant(Constant::Index(offset)),
        ));
        let pointer = ValueId(next);
        next += 1;
        operations.push(Operation::effect_free(
            ValueDef::new(
                pointer,
                Type::pointer(element.clone(), AddressSpace::Global, *mode),
            ),
            OperationKind::GetElementPointer {
                base: pointers[parameter],
                offset: offset_value,
            },
        ));
        let writes = matches!(effect, Effect::Write)
            || matches!(effect, Effect::Atomic(kind) if kind != AtomicKind::Load);
        let value = writes.then(|| {
            let id = ValueId(next);
            next += 1;
            let constant = match element {
                Type::Scalar(ScalarType::U16) => Constant::U16(7),
                Type::Scalar(ScalarType::U32) => Constant::U32(7),
                _ => panic!("unsupported fixture write element"),
            };
            operations.push(Operation::effect_free(
                ValueDef::new(id, element.clone()),
                OperationKind::Constant(constant),
            ));
            id
        });
        let mut results = Vec::new();
        let returns_value = !matches!(effect, Effect::Write | Effect::Atomic(AtomicKind::Store));
        if returns_value {
            results.push(ValueDef::new(ValueId(next), element.clone()));
            next += 1;
        }
        if matches!(effect, Effect::Atomic(AtomicKind::CompareExchange)) {
            results.push(ValueDef::new(ValueId(next), Type::BOOL));
            next += 1;
        }
        let kind = match effect {
            Effect::Read => OperationKind::Load { pointer, access },
            Effect::Write => OperationKind::Store {
                pointer,
                value: value.expect("write value"),
                access,
            },
            Effect::Atomic(kind) => OperationKind::Atomic(Atomic {
                kind,
                pointer,
                value,
                compare: (kind == AtomicKind::CompareExchange)
                    .then(|| value.expect("compare-exchange value")),
                access,
                scope: SynchronizationScope::Device,
                ordering: if kind == AtomicKind::Load {
                    MemoryOrdering::Acquire
                } else {
                    MemoryOrdering::Relaxed
                },
                failure_ordering: (kind == AtomicKind::CompareExchange)
                    .then_some(MemoryOrdering::Relaxed),
            }),
        };
        operations.push(Operation::new(results, kind));
    }
    let mut block = BasicBlock::new(BlockId(0));
    block.operations = operations;
    block.terminator = Some(Terminator::Return { values: vec![] });
    let mut module = Module::new("formal-atomic-load-alias");
    module.functions.push(Function::kernel_entry(
        "entry",
        Signature::new(
            parameters
                .iter()
                .map(|(element, access)| {
                    Type::slice(element.clone(), AddressSpace::Global, *access)
                })
                .collect(),
            vec![],
        ),
        (0..parameters.len())
            .map(|index| ValueId(index as u32))
            .collect(),
        vec![block],
    ));
    module.kernels.push(Kernel::new(
        "kernel",
        "entry",
        LaunchDomain::D1 {
            x: LaunchExtent::Dynamic,
        },
    ));
    module
}

fn atomic_parameters(count: usize) -> Vec<(Type, AccessMode)> {
    vec![(Type::Scalar(ScalarType::U32), AccessMode::ReadWrite); count]
}

fn analyze(module: &Module) -> FormalMemoryObligationAnalysis {
    let analysis = derive_kernel_memory_obligations(
        module,
        &KernelId::new("kernel"),
        ExplicitLaunchExtent1d::Exact(64),
        FormalIndexWidth::Bits64,
    )
    .expect("verified fixture");
    assert!(analysis.is_complete(), "{analysis:?}");
    analysis
}

fn pairs(analysis: &FormalMemoryObligationAnalysis) -> Vec<(u32, u32)> {
    analysis
        .obligations()
        .runtime_alias_requirements()
        .iter()
        .map(|alias| {
            (
                alias.left().parameter_index(),
                alias.right().parameter_index(),
            )
        })
        .collect()
}

fn receipt(
    analysis: &FormalMemoryObligationAnalysis,
) -> InertCanonicalFormalMemoryObligationReceiptV1 {
    InertCanonicalFormalMemoryObligationReceiptV1::from_obligations(analysis.obligations())
        .expect("inert canonical receipt")
}

// Parse only the fixed record layout needed for an independent wire-tag check.
fn receipt_access_tags(bytes: &[u8]) -> Vec<u8> {
    fn u32_at(bytes: &[u8], position: &mut usize) -> usize {
        let end = *position + 4;
        let value = u32::from_le_bytes(bytes[*position..end].try_into().unwrap()) as usize;
        *position = end;
        value
    }
    assert_eq!(&bytes[..8], b"FE2O3FM\0");
    let mut position = 20;
    for _ in 0..2 {
        let length = u32_at(bytes, &mut position);
        position += length;
    }
    position += 4; // Index width, analysis basis, and reserved bytes.
    assert_eq!(bytes[position], 1); // Explicit invocation range.
    position += 17;
    let allocations = u32_at(bytes, &mut position);
    position += allocations * 12;
    let accesses = u32_at(bytes, &mut position);
    let mut tags = Vec::new();
    for _ in 0..accesses {
        position += 16; // Location and formal allocation.
        tags.push(bytes[position]);
        position += 2 + 20 + 16 + 16; // Kind/space, byte expression, width/alignment, range.
    }
    tags
}

#[test]
fn atomic_load_alias_load_only_allocations_have_no_alias_requirements() {
    let module = fixture(
        &atomic_parameters(2),
        &[
            (0, 0, Effect::Atomic(AtomicKind::Load)),
            (1, 3, Effect::Atomic(AtomicKind::Load)),
        ],
    );
    let analysis = analyze(&module);
    assert!(pairs(&analysis).is_empty());
    assert_eq!(analysis.obligations().accesses().len(), 2);
    assert!(
        analysis
            .obligations()
            .accesses()
            .iter()
            .all(|access| access.kind() == FormalMemoryAccessKind::Atomic)
    );
    assert!(
        analysis
            .obligations()
            .inter_invocation_conflicts()
            .is_empty()
    );
}

#[test]
fn atomic_load_alias_r2_readers_only_require_output_separation() {
    let parameters = [
        (Type::F32, AccessMode::ReadOnly),
        (Type::F32, AccessMode::ReadOnly),
        (Type::Scalar(ScalarType::U16), AccessMode::ReadOnly),
        (Type::Scalar(ScalarType::U16), AccessMode::WriteOnly),
        (Type::Scalar(ScalarType::U32), AccessMode::ReadWrite),
        (Type::Scalar(ScalarType::U32), AccessMode::ReadWrite),
    ];
    let effects = [
        (0, 0, Effect::Read),
        (1, 0, Effect::Read),
        (2, 0, Effect::Read),
        (3, 0, Effect::Write),
        (4, 0, Effect::Atomic(AtomicKind::Load)),
        (5, 0, Effect::Atomic(AtomicKind::Load)),
    ];
    let analysis = analyze(&fixture(&parameters, &effects));
    assert_eq!(
        pairs(&analysis),
        vec![(0, 3), (1, 3), (2, 3), (3, 4), (3, 5)]
    );
    assert_eq!(
        analysis
            .obligations()
            .accesses()
            .iter()
            .filter(|a| a.kind() == FormalMemoryAccessKind::Atomic)
            .count(),
        2
    );
    // This fixture isolates alias extraction, not the R2 invocation-disjointness proof.
    assert!(
        !analysis
            .obligations()
            .inter_invocation_conflicts()
            .is_empty()
    );
    let mut ordinary_only = effects;
    ordinary_only[4].2 = Effect::Read;
    ordinary_only[5].2 = Effect::Read;
    assert_eq!(
        pairs(&analysis),
        pairs(&analyze(&fixture(&parameters, &ordinary_only)))
    );
}

#[test]
fn atomic_load_alias_every_atomic_writer_retains_alias_requirements() {
    for writer in [
        AtomicKind::Store,
        AtomicKind::Exchange,
        AtomicKind::CompareExchange,
        AtomicKind::Add,
        AtomicKind::Subtract,
        AtomicKind::Min,
        AtomicKind::Max,
        AtomicKind::BitAnd,
        AtomicKind::BitOr,
        AtomicKind::BitXor,
    ] {
        for other in [Effect::Read, Effect::Atomic(AtomicKind::Load)] {
            for effects in [
                [(0, 0, Effect::Atomic(writer)), (1, 0, other)],
                [(1, 0, other), (0, 0, Effect::Atomic(writer))],
            ] {
                let analysis = analyze(&fixture(&atomic_parameters(2), &effects));
                assert_eq!(pairs(&analysis), vec![(0, 1)], "{writer:?}");
                assert_eq!(analysis.obligations().bounds_requirements().len(), 2);
            }
        }
    }
}

#[test]
fn atomic_load_alias_writes_dominate_loads_in_both_orders() {
    for writer in [
        Effect::Write,
        Effect::Atomic(AtomicKind::Store),
        Effect::Atomic(AtomicKind::Add),
        Effect::Atomic(AtomicKind::CompareExchange),
    ] {
        for effects in [
            [
                (0, 0, Effect::Atomic(AtomicKind::Load)),
                (0, 7, writer),
                (1, 1, Effect::Atomic(AtomicKind::Load)),
            ],
            [
                (0, 7, writer),
                (0, 0, Effect::Atomic(AtomicKind::Load)),
                (1, 1, Effect::Atomic(AtomicKind::Load)),
            ],
        ] {
            let analysis = analyze(&fixture(&atomic_parameters(2), &effects));
            assert_eq!(pairs(&analysis), vec![(0, 1)]);
            let alias = analysis.obligations().runtime_alias_requirements()[0];
            assert_eq!(
                (
                    alias.left_accessed_bytes().start(),
                    alias.left_accessed_bytes().end_exclusive()
                ),
                (0, 32)
            );
            assert_eq!(
                (
                    alias.right_accessed_bytes().start(),
                    alias.right_accessed_bytes().end_exclusive()
                ),
                (4, 8)
            );
        }
    }
}

#[test]
fn atomic_load_alias_bounds_conflicts_and_receipt_kind_are_preserved() {
    let module = fixture(
        &atomic_parameters(2),
        &[
            (0, 2, Effect::Atomic(AtomicKind::Load)),
            (1, 3, Effect::Atomic(AtomicKind::Load)),
        ],
    );
    let analysis = analyze(&module);
    assert_eq!(analysis.obligations().bounds_requirements().len(), 2);
    let encoded = receipt(&analysis);
    assert_eq!(receipt_access_tags(encoded.canonical_bytes()), vec![3, 3]);
    let decoded = InertCanonicalFormalMemoryObligationReceiptV1::from_canonical_bytes(
        encoded.canonical_bytes().to_vec(),
    )
    .unwrap();
    assert_eq!(decoded, encoded);
    // Ordinary/atomic accesses are not globally relabelled or exempted by this change.
    let mixed = analyze(&fixture(
        &atomic_parameters(1),
        &[
            (0, 0, Effect::Read),
            (0, 0, Effect::Atomic(AtomicKind::Load)),
        ],
    ));
    assert!(pairs(&mixed).is_empty());
    assert_eq!(mixed.obligations().inter_invocation_conflicts().len(), 1);
    assert_eq!(
        receipt_access_tags(receipt(&mixed).canonical_bytes()),
        vec![1, 3]
    );
}

#[test]
fn atomic_load_alias_ordinary_ranges_and_empty_accesses_are_unchanged() {
    let analysis = analyze(&fixture(
        &atomic_parameters(3),
        &[
            (0, 2, Effect::Read),
            (0, 5, Effect::Read),
            (1, 7, Effect::Write),
        ],
    ));
    assert_eq!(pairs(&analysis), vec![(0, 1)]);
    let alias = analysis.obligations().runtime_alias_requirements()[0];
    assert_eq!(
        (
            alias.left_accessed_bytes().start(),
            alias.left_accessed_bytes().end_exclusive()
        ),
        (8, 24)
    );
    assert_eq!(
        (
            alias.right_accessed_bytes().start(),
            alias.right_accessed_bytes().end_exclusive()
        ),
        (28, 32)
    );
    assert_eq!(analysis.obligations().allocations().len(), 3);
    assert!(pairs(&analyze(&fixture(&atomic_parameters(3), &[]))).is_empty());
}

#[test]
fn atomic_load_alias_unreachable_writers_do_not_pollute_readers() {
    let mut module = fixture(
        &atomic_parameters(2),
        &[
            (0, 0, Effect::Atomic(AtomicKind::Load)),
            (1, 0, Effect::Atomic(AtomicKind::Load)),
            (0, 1, Effect::Atomic(AtomicKind::Store)),
        ],
    );
    let body = module.functions[0].body.as_mut().unwrap();
    let mut dead = BasicBlock::new(BlockId(1));
    // Move the full last effect's offset, GEP, value, and store together.
    let split = body.blocks[0].operations.len() - 4;
    dead.operations = body.blocks[0].operations.split_off(split);
    // Unreachable blocks may use function parameters, but not entry-block SSA
    // definitions. Give the dead GEP its own local slice-data definition.
    let dead_pointer = ValueId(999);
    let OperationKind::GetElementPointer { base, .. } = &mut dead.operations[1].kind else {
        panic!("last fixture effect must contain a GEP");
    };
    *base = dead_pointer;
    dead.operations.insert(
        0,
        Operation::effect_free(
            ValueDef::new(
                dead_pointer,
                Type::pointer(
                    Type::Scalar(ScalarType::U32),
                    AddressSpace::Global,
                    AccessMode::ReadWrite,
                ),
            ),
            OperationKind::SliceData { slice: ValueId(0) },
        ),
    );
    dead.terminator = Some(Terminator::Return { values: vec![] });
    body.blocks.push(dead);
    let analysis = analyze(&module);
    assert_eq!(analysis.obligations().accesses().len(), 2);
    assert!(pairs(&analysis).is_empty());
}

#[test]
fn atomic_load_alias_malformed_atomics_and_unknown_extents_fail_closed() {
    let baseline = fixture(
        &atomic_parameters(2),
        &[
            (0, 0, Effect::Atomic(AtomicKind::Load)),
            (1, 0, Effect::Atomic(AtomicKind::Load)),
        ],
    );
    for mutation in 0..3 {
        let mut module = baseline.clone();
        let atomic = module.functions[0].body.as_mut().unwrap().blocks[0]
            .operations
            .iter_mut()
            .find_map(|op| match &mut op.kind {
                OperationKind::Atomic(atomic) => Some(atomic),
                _ => None,
            })
            .unwrap();
        match mutation {
            0 => atomic.ordering = MemoryOrdering::Release,
            1 => atomic.scope = SynchronizationScope::Invocation,
            2 => atomic.access.alignment = 1,
            _ => unreachable!(),
        }
        assert!(
            derive_kernel_memory_obligations(
                &module,
                &KernelId::new("kernel"),
                ExplicitLaunchExtent1d::Exact(64),
                FormalIndexWidth::Bits64
            )
            .is_err()
        );
    }
    let unknown = derive_kernel_memory_obligations(
        &baseline,
        &KernelId::new("kernel"),
        ExplicitLaunchExtent1d::Unknown,
        FormalIndexWidth::Bits64,
    )
    .unwrap();
    assert!(!unknown.is_complete());
    assert!(!unknown.incomplete_reasons().is_empty());
}

#[test]
fn atomic_load_alias_opcode_and_metadata_remain_separate_from_receipt_authority() {
    let loads = fixture(
        &atomic_parameters(2),
        &[
            (0, 0, Effect::Atomic(AtomicKind::Load)),
            (1, 0, Effect::Atomic(AtomicKind::Load)),
        ],
    );
    let original = analyze(&loads);
    let original_receipt = receipt(&original);
    for writer in [
        AtomicKind::Store,
        AtomicKind::Exchange,
        AtomicKind::CompareExchange,
    ] {
        let changed = analyze(&fixture(
            &atomic_parameters(2),
            &[
                (0, 0, Effect::Atomic(AtomicKind::Load)),
                (1, 0, Effect::Atomic(writer)),
            ],
        ));
        assert_eq!(pairs(&changed), vec![(0, 1)]);
        assert_ne!(receipt(&changed), original_receipt);
    }
    for (scope, ordering) in [
        (SynchronizationScope::Device, MemoryOrdering::Relaxed),
        (
            SynchronizationScope::System,
            MemoryOrdering::SequentiallyConsistent,
        ),
    ] {
        let mut changed = loads.clone();
        for op in &mut changed.functions[0].body.as_mut().unwrap().blocks[0].operations {
            if let OperationKind::Atomic(atomic) = &mut op.kind {
                atomic.scope = scope;
                atomic.ordering = ordering;
            }
        }
        assert_ne!(changed, loads);
        let analysis = analyze(&changed);
        assert!(pairs(&analysis).is_empty());
        // The inert receipt does not encode ordering/scope or authorize reuse
        // across KIR identities. Producer/KIR lineage must still be checked.
        assert_eq!(receipt(&analysis), original_receipt);
    }
}
