use fe2o3_kernel_ir::{
    CanonicalKernelIrVerificationResourceBudgetV1 as FusionBudget,
    CanonicalKernelIrVerificationResourceErrorV1 as FusionResource,
    CanonicalKernelIrWorkBudgetV1 as FusionWork,
};

fn fusion_block(
    operations: Vec<ProductionRankedOperationV1>,
    terminator: ProductionRankedTerminatorV1,
) -> ProductionRankedBlockV1 {
    ProductionRankedBlockV1::new(operations, terminator)
}

fn fusion_branch(target: u32) -> ProductionRankedBlockV1 {
    fusion_block(Vec::new(), ProductionRankedTerminatorV1::Branch { target })
}

fn fusion_constant(value: u32) -> ProductionRankedOperationV1 {
    ProductionRankedOperationV1::IndexConstant {
        result: ProductionRankedValueIdV1::new(value),
        value: value.into(),
    }
}

fn fusion_atomic(kind: AccessKindAttr) -> ProductionRankedOperationV1 {
    ProductionRankedOperationV1::AtomicAccess {
        kind,
        ordering: if kind == AccessKindAttr::Read {
            AtomicOrderingAttr::Acquire
        } else {
            AtomicOrderingAttr::Release
        },
        scope: AtomicScopeAttr::System,
        view: ProductionRankedValueV1::Argument(0),
        indices: vec![ProductionRankedValueV1::Argument(1)],
    }
}

fn fusion_run(blocks: Vec<ProductionRankedBlockV1>) -> Vec<ProductionRankedBlockV1> {
    let mut work = FusionWork::new(1_000_000);
    let mut budget = FusionBudget::new(&mut work, 64 * 1024 * 1024);
    let result = cfg_linear_fusion_v1::fuse(
        blocks, Vec::new(), Vec::new(), Vec::new(), &mut budget,
    ).unwrap();
    assert_eq!(budget.storage(), 0);
    result.0
}

fn fusion_edges(blocks: &[ProductionRankedBlockV1]) -> usize {
    blocks.iter().map(|block| match block.terminator() {
        ProductionRankedTerminatorV1::Branch { .. } => 1,
        ProductionRankedTerminatorV1::IndexLessThan { .. }
        | ProductionRankedTerminatorV1::IndexEqual { .. }
        | ProductionRankedTerminatorV1::AnalysisSplit { .. } => 2,
        ProductionRankedTerminatorV1::Return | ProductionRankedTerminatorV1::Trap => 0,
        _ => panic!("fixture edge census requires plain terminators"),
    }).sum()
}

// The untransformed graph is the independent differential control. Record all
// operations, predicate operands, choices and terminal outcomes, not block IDs.
fn fusion_trace(blocks: &[ProductionRankedBlockV1], choices: usize) -> Vec<String> {
    fusion_trace_with_choices(blocks, |decision| {
        (choices >> (decision % usize::BITS as usize)) & 1 == 0
    })
}

fn fusion_trace_with_choices(
    blocks: &[ProductionRankedBlockV1],
    mut choice: impl FnMut(usize) -> bool,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = 0;
    let mut decision = 0;
    for _ in 0..(blocks.len() * 2 + 1) {
        let block = &blocks[current];
        for operation in block.operations() {
            result.push(format!("{operation:?}"));
        }
        let choose = choice(decision);
        match block.terminator() {
            ProductionRankedTerminatorV1::Branch { target } => current = *target as usize,
            ProductionRankedTerminatorV1::IndexLessThan { lhs, rhs, true_block, false_block } => {
                result.push(format!("less {lhs:?} {rhs:?} {choose}"));
                decision += 1;
                current = if choose { *true_block } else { *false_block } as usize;
            }
            ProductionRankedTerminatorV1::IndexEqual { lhs, rhs, true_block, false_block } => {
                result.push(format!("equal {lhs:?} {rhs:?} {choose}"));
                decision += 1;
                current = if choose { *true_block } else { *false_block } as usize;
            }
            ProductionRankedTerminatorV1::AnalysisSplit { control_dependencies, first_block, second_block } => {
                result.push(format!("split {control_dependencies:?} {choose}"));
                decision += 1;
                current = if choose { *first_block } else { *second_block } as usize;
            }
            ProductionRankedTerminatorV1::Return => {
                result.push("return".into());
                return result;
            }
            ProductionRankedTerminatorV1::Trap => {
                result.push("trap".into());
                return result;
            }
            _ => panic!("fixture trace requires plain terminators"),
        }
    }
    panic!("fixture trace did not terminate");
}

fn fusion_chain(count: usize) -> Vec<ProductionRankedBlockV1> {
    (0..count).map(|index| {
        fusion_block(
            vec![fusion_constant(index as u32)],
            if index + 1 == count {
                ProductionRankedTerminatorV1::Return
            } else {
                ProductionRankedTerminatorV1::Branch { target: (index + 1) as u32 }
            },
        )
    }).collect()
}

#[test]
fn cfg_linear_fusion_conditional_effect_traces() {
    let blocks = vec![
        fusion_branch(1),
        fusion_block(vec![fusion_constant(0)], ProductionRankedTerminatorV1::IndexLessThan {
            lhs: ProductionRankedValueV1::Argument(0),
            rhs: ProductionRankedValueV1::Argument(1),
            true_block: 2, false_block: 7,
        }),
        fusion_block(vec![fusion_atomic(AccessKindAttr::Read)], ProductionRankedTerminatorV1::Branch { target: 3 }),
        fusion_branch(4),
        fusion_block(vec![fusion_constant(1)], ProductionRankedTerminatorV1::IndexEqual {
            lhs: ProductionRankedValueV1::Argument(2),
            rhs: ProductionRankedValueV1::Argument(3),
            true_block: 5, false_block: 7,
        }),
        fusion_block(vec![fusion_atomic(AccessKindAttr::Write)], ProductionRankedTerminatorV1::Branch { target: 6 }),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Trap),
    ];
    let fused = fusion_run(blocks.clone());
    assert_eq!((fused.len(), fusion_edges(&fused)), (5, 5));
    for choices in 0..4 {
        assert_eq!(fusion_trace(&blocks, choices), fusion_trace(&fused, choices));
    }
    assert!(!fusion_trace(&fused, 1).iter().any(|line| line.contains("AtomicAccess")));
    assert_eq!(fusion_trace(&fused, 0).iter().filter(|line| line.contains("AtomicAccess")).count(), 2);
}

#[test]
fn cfg_linear_fusion_source_wave_generated_coordinates() {
    let barrier = ProductionRankedOperationV1::Barrier {
        execution_scope: HierarchyAttr::Workgroup,
        memory_scope: MemoryScopeAttr::Workgroup,
        address_space: AddressSpaceAttr::Global,
        order: MemoryOrderAttr::AcquireRelease,
    };
    let blocks = vec![
        fusion_branch(1),
        fusion_block(vec![fusion_constant(3), fusion_atomic(AccessKindAttr::Read)], ProductionRankedTerminatorV1::Branch { target: 2 }),
        fusion_block(vec![barrier, fusion_atomic(AccessKindAttr::Write)], ProductionRankedTerminatorV1::Branch { target: 3 }),
        fusion_block(vec![fusion_constant(4), ProductionRankedOperationV1::AtomicCompareExchangeAccess {
            ordering: AtomicOrderingAttr::AcquireRelease,
            failure_ordering: AtomicOrderingAttr::Acquire,
            scope: AtomicScopeAttr::System,
            view: ProductionRankedValueV1::Argument(0),
            indices: vec![ProductionRankedValueV1::Argument(1)],
        }], ProductionRankedTerminatorV1::Return),
    ];
    let sources = vec![
        ProjectedAccessSourceV1 {
            block: 1, operation: 1, access: AccessKindAttr::Read,
            memory_space: MemorySpaceAttr::Global,
            source: SemanticSourceProvenanceV1::unavailable(),
            semantic_site: Some(ProjectedSemanticAccessSiteV1 { block: 7, statement: Some(8) }),
        },
        ProjectedAccessSourceV1 {
            block: 2, operation: 1, access: AccessKindAttr::Write,
            memory_space: MemorySpaceAttr::Global,
            source: SemanticSourceProvenanceV1::unavailable(),
            semantic_site: Some(ProjectedSemanticAccessSiteV1 { block: 9, statement: None }),
        },
    ];
    let effects = vec![ProductionRankedExecutableEffectSourceV1::new(
        9, 13, 2, 1,
        ProductionRankedExecutableEffectOriginV1::GeneratedFromSemanticTerminator,
        [17; 32],
    )];
    let waves = vec![ProjectedWaveSyncSiteV1 {
        semantic_block: 9, ranked_block: 2, ranked_operation: 0,
    }];
    let mut work = FusionWork::new(10_000);
    let mut budget = FusionBudget::new(&mut work, 1_000_000);
    let (fused, mapped, generated, sync) = cfg_linear_fusion_v1::fuse(
        blocks.clone(), sources.clone(), effects.clone(), waves, &mut budget,
    ).unwrap();
    assert_eq!(fused.len(), 2);
    for (old, new) in sources.iter().zip(&mapped) {
        assert_eq!(blocks[old.block].operations()[old.operation], fused[new.block].operations()[new.operation]);
        let mut expected = *old;
        expected.block = 1;
        expected.operation = if old.block == 1 { 1 } else { 3 };
        assert_eq!(expected, *new);
    }
    assert_eq!(generated[0], ProductionRankedExecutableEffectSourceV1::new(
        9, 13, 1, 3, effects[0].origin(), [17; 32],
    ));
    assert_eq!((sync[0].semantic_block, sync[0].ranked_block, sync[0].ranked_operation), (9, 1, 2));
    assert_eq!(budget.storage(), 0);
    assert_eq!(fusion_trace(&blocks, 0), fusion_trace(&fused, 0));
}

#[test]
fn cfg_linear_fusion_duplicate_edges_and_entry_are_preserved() {
    let blocks = vec![
        fusion_branch(1),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::AnalysisSplit {
            control_dependencies: Vec::new(), first_block: 2, second_block: 2,
        }),
        fusion_branch(3),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return),
    ];
    let fused = fusion_run(blocks.clone());
    assert_eq!(fused[0], blocks[0]);
    assert_eq!(fused.len(), 3);
    assert_eq!(fusion_edges(&fused), 3);
    assert!(matches!(fused[1].terminator(), ProductionRankedTerminatorV1::AnalysisSplit {
        first_block: 2, second_block: 2, ..
    }));
    for choices in 0..2 {
        assert_eq!(fusion_trace(&blocks, choices), fusion_trace(&fused, choices));
    }
    let back_to_entry = vec![fusion_branch(1), fusion_branch(0)];
    assert_eq!(fusion_run(back_to_entry.clone()), back_to_entry);
}

#[test]
fn cfg_linear_fusion_cycles_and_argument_fallback() {
    let unreachable_cycle = vec![
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return),
        fusion_branch(2), fusion_branch(1), fusion_branch(4),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return),
    ];
    assert_eq!(fusion_run(unreachable_cycle.clone()), unreachable_cycle);
    let self_loop = vec![fusion_branch(1), fusion_branch(1)];
    assert_eq!(fusion_run(self_loop.clone()), self_loop);
    let conditional_cycle = vec![
        fusion_branch(1),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::IndexLessThan {
            lhs: ProductionRankedValueV1::Argument(0), rhs: ProductionRankedValueV1::Argument(1),
            true_block: 2, false_block: 4,
        }),
        fusion_branch(3), fusion_branch(1),
        fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return),
    ];
    let fused = fusion_run(conditional_cycle);
    assert_eq!(fused.len(), 4);
    assert!(matches!(fused[2].terminator(), ProductionRankedTerminatorV1::Branch { target: 1 }));
    let mut args = fusion_chain(4);
    args[2] = ProductionRankedBlockV1::with_index_arguments(
        1, Vec::new(), ProductionRankedTerminatorV1::Branch { target: 3 },
    );
    assert_eq!(fusion_run(args.clone()), args);
    args[1] = fusion_block(vec![ProductionRankedOperationV1::DeterministicJoin {
        result: ProductionRankedValueIdV1::new(9),
        dependencies: vec![ProductionRankedValueV1::BlockArgument { block: 2, argument: 0 }],
    }], ProductionRankedTerminatorV1::Branch { target: 2 });
    assert_eq!(fusion_run(args.clone()), args);
    let mut branch_args = fusion_chain(4);
    branch_args[1] = fusion_block(Vec::new(), ProductionRankedTerminatorV1::BranchArgs {
        arguments: Vec::new(), target: 2,
    });
    assert_eq!(fusion_run(branch_args.clone()), branch_args);
    let mut unsupported = fusion_chain(4);
    unsupported[1] = fusion_block(vec![ProductionRankedOperationV1::PredicatedAccess {
        kind: AccessKindAttr::Read,
        view: ProductionRankedValueV1::Argument(0),
        index: ProductionRankedValueV1::Argument(1),
        success: ProductionRankedValueV1::Argument(2),
    }], ProductionRankedTerminatorV1::Branch { target: 2 });
    assert_eq!(fusion_run(unsupported.clone()), unsupported);
}

#[test]
fn cfg_linear_fusion_malformed_references_fail_closed() {
    let mut bad_target = fusion_chain(4);
    bad_target[0] = ProductionRankedBlockV1::with_index_arguments(
        1, Vec::new(), ProductionRankedTerminatorV1::Branch { target: u32::MAX },
    );
    let mut bad_value = fusion_chain(4);
    bad_value[2] = fusion_block(vec![ProductionRankedOperationV1::DeterministicJoin {
        result: ProductionRankedValueIdV1::new(5),
        dependencies: vec![ProductionRankedValueV1::BlockArgument { block: 2, argument: 0 }],
    }], ProductionRankedTerminatorV1::Return);
    for blocks in [bad_target, bad_value] {
        let mut work = FusionWork::new(10_000);
        let mut budget = FusionBudget::new(&mut work, 1_000_000);
        assert!(cfg_linear_fusion_v1::fuse(blocks, Vec::new(), Vec::new(), Vec::new(), &mut budget).is_err());
        assert_eq!(budget.storage(), 0);
    }
    for which in 0..3 {
        let mut work = FusionWork::new(10_000);
        let mut budget = FusionBudget::new(&mut work, 1_000_000);
        let sources = if which == 0 { vec![ProjectedAccessSourceV1 {
            block: 4, operation: 0, access: AccessKindAttr::Read,
            memory_space: MemorySpaceAttr::Global,
            source: SemanticSourceProvenanceV1::unavailable(), semantic_site: None,
        }] } else { Vec::new() };
        let effects = if which == 1 { vec![ProductionRankedExecutableEffectSourceV1::new(
            0, 0, 1, 1, ProductionRankedExecutableEffectOriginV1::GeneratedFromSemanticTerminator, [0; 32],
        )] } else { Vec::new() };
        let waves = if which == 2 { vec![ProjectedWaveSyncSiteV1 {
            semantic_block: 0, ranked_block: 4, ranked_operation: 0,
        }] } else { Vec::new() };
        assert!(cfg_linear_fusion_v1::fuse(fusion_chain(4), sources, effects, waves, &mut budget).is_err());
        assert_eq!(budget.storage(), 0);
    }
}

#[test]
fn cfg_linear_fusion_work_storage_and_failure_boundaries() {
    let blocks = fusion_chain(8);
    let mut work = FusionWork::new(100_000);
    let mut budget = FusionBudget::new(&mut work, 1_000_000);
    budget.reserve_storage(29).unwrap();
    let expected = cfg_linear_fusion_v1::fuse(
        blocks.clone(), Vec::new(), Vec::new(), Vec::new(), &mut budget,
    ).unwrap().0;
    let exact_work = budget.work();
    let exact_storage = budget.peak_storage();
    assert_eq!(budget.storage(), 29);
    for limit in 0..=exact_work {
        let mut work = FusionWork::new(limit);
        let mut budget = FusionBudget::new(&mut work, exact_storage);
        budget.reserve_storage(29).unwrap();
        let result = cfg_linear_fusion_v1::fuse(
            blocks.clone(), Vec::new(), Vec::new(), Vec::new(), &mut budget,
        );
        if limit == exact_work {
            assert_eq!(result.unwrap().0, expected);
            assert_eq!(budget.work(), limit);
        } else {
            assert!(matches!(result, Err(ProductionRankedProjectionErrorV1::CanonicalAssertions(
                CanonicalAssertionErrorV1::Resource(FusionResource::Work(_))
            ))));
            assert!(budget.work() <= limit);
        }
        assert_eq!(budget.storage(), 29);
    }
    let mut work = FusionWork::new(exact_work);
    let mut budget = FusionBudget::new(&mut work, exact_storage - 1);
    budget.reserve_storage(29).unwrap();
    assert!(matches!(cfg_linear_fusion_v1::fuse(
        blocks, Vec::new(), Vec::new(), Vec::new(), &mut budget,
    ), Err(ProductionRankedProjectionErrorV1::CanonicalAssertions(
        CanonicalAssertionErrorV1::Resource(FusionResource::Storage(_))
    ))));
    assert_eq!(budget.storage(), 29);
    let mut work = FusionWork::new(usize::MAX);
    let mut budget = FusionBudget::new(&mut work, 1_000_000);
    budget.charge_work(usize::MAX).unwrap();
    assert!(matches!(cfg_linear_fusion_v1::fuse(
        fusion_chain(2), Vec::new(), Vec::new(), Vec::new(), &mut budget,
    ), Err(ProductionRankedProjectionErrorV1::CanonicalAssertions(
        CanonicalAssertionErrorV1::Resource(FusionResource::Work(_))
    ))));
    assert_eq!(budget.work(), usize::MAX);
    assert_eq!(budget.storage(), 0);
}

#[test]
fn cfg_linear_fusion_chain_and_identity_boundaries() {
    for count in [1, 2, 3, 1024, 1025, MAX_RANKED_BOUNDS_BLOCKS] {
        let original = fusion_chain(count);
        let fused = fusion_run(original.clone());
        assert_eq!(fused.len(), count.min(2));
        assert_eq!(fusion_trace(&original, 0), fusion_trace(&fused, 0));
    }
    for count in [0, MAX_RANKED_BOUNDS_BLOCKS + 1] {
        let mut work = FusionWork::new(1_000_000);
        let mut budget = FusionBudget::new(&mut work, 64 * 1024 * 1024);
        assert!(cfg_linear_fusion_v1::fuse(
            fusion_chain(count), Vec::new(), Vec::new(), Vec::new(), &mut budget,
        ).is_err());
        assert_eq!(budget.work(), 0);
        assert_eq!(budget.storage(), 0);
    }
    let identity = fe2o3_pliron::MAX_PLIRON_IDENTITY_BLOCKS_V1;
    assert_eq!(identity, 1024);
    let original = fusion_chain(identity);
    let untouched = cfg_linear_fusion_v1::for_projection(
        original.clone(), Vec::new(), Vec::new(), Vec::new(), &mut ComponentDynamicAssertionFactsV1,
    ).unwrap().0;
    assert_eq!(untouched, original);
    assert!(matches!(cfg_linear_fusion_v1::for_projection(
        fusion_chain(identity + 1), Vec::new(), Vec::new(), Vec::new(), &mut ComponentDynamicAssertionFactsV1,
    ), Err(ProductionRankedProjectionErrorV1::CanonicalAssertions(
        CanonicalAssertionErrorV1::Binding("synthetic assertion facts cannot lend the canonical budget")
    ))));
}

#[test]
fn cfg_linear_fusion_measured_guard_pattern() {
    // Synthetic 552-guard family, not a replay of the retained guarded kernel.
    // Every access has its own predicate and shared failure sink.
    let guards = 552;
    let end = 1 + guards * 3;
    let trap = end + 1;
    let mut original = vec![fusion_branch(1)];
    for guard in 0..guards {
        let check = 1 + guard * 3;
        original.push(fusion_block(Vec::new(), ProductionRankedTerminatorV1::IndexLessThan {
            lhs: ProductionRankedValueV1::Argument(0), rhs: ProductionRankedValueV1::Argument(1),
            true_block: (check + 1) as u32, false_block: trap as u32,
        }));
        let mut access = fusion_atomic(if guard < 548 {
            AccessKindAttr::Read
        } else { AccessKindAttr::Write });
        if (548..551).contains(&guard) {
            if let ProductionRankedOperationV1::AtomicAccess { ordering, .. } = &mut access {
                *ordering = AtomicOrderingAttr::Relaxed;
            }
        }
        original.push(fusion_block(vec![access], ProductionRankedTerminatorV1::Branch { target: (check + 2) as u32 }));
        original.push(fusion_branch((check + 3) as u32));
    }
    original.push(fusion_block(Vec::new(), ProductionRankedTerminatorV1::Return));
    original.push(fusion_block(Vec::new(), ProductionRankedTerminatorV1::Trap));
    assert_eq!((original.len(), fusion_edges(&original)), (1659, 2209));
    assert!(fusion_edges(&original) > MAX_RANKED_BOUNDS_EDGES);
    let fused = fusion_run(original.clone());
    assert_eq!((fused.len(), fusion_edges(&fused)), (555, 1105));
    assert!(fused.len() < fe2o3_pliron::MAX_PLIRON_IDENTITY_BLOCKS_V1);
    assert!(fusion_edges(&fused) < MAX_RANKED_BOUNDS_EDGES);
    assert_eq!(fusion_trace(&original, 0), fusion_trace(&fused, 0));
    for first_failure in 0..guards {
        let old = fusion_trace_with_choices(&original, |decision| decision != first_failure);
        let new = fusion_trace_with_choices(&fused, |decision| decision != first_failure);
        assert_eq!(old, new);
        assert_eq!(new.iter().filter(|line| line.contains("AtomicAccess")).count(), first_failure);
        assert_eq!(new.last().map(String::as_str), Some("trap"));
    }
    assert_eq!(fused.iter().flat_map(|block| block.operations()).filter(|op| matches!(
        op, ProductionRankedOperationV1::AtomicAccess { .. }
    )).count(), 552);
    for (ordering, expected) in [
        (AtomicOrderingAttr::Acquire, 548),
        (AtomicOrderingAttr::Relaxed, 3),
        (AtomicOrderingAttr::Release, 1),
    ] {
        assert_eq!(fused.iter().flat_map(|block| block.operations()).filter(|op| matches!(
            op, ProductionRankedOperationV1::AtomicAccess { ordering: actual, .. } if *actual == ordering
        )).count(), expected);
    }
}

mod fusion_production_fixture {
    use super::*;
    include!("canonical_assertion_fixtures_v1_tests.rs");

    pub(super) fn check() {
        // 1023/1024 semantic blocks plus the projector's ranked entry exercise
        // both sides of the production threshold with genuine source owners.
        for count in [1023, 1024] {
            let blocks = (0..count)
                .map(|index| {
                    let mut identity = [0; 32];
                    identity[..8].copy_from_slice(&(index as u64 + 1).to_le_bytes());
                    let terminator = if index + 1 == count {
                        SemanticTerminatorKindV1::Return
                    } else {
                        SemanticTerminatorKindV1::Goto(cfg_edge(
                            SemanticEdgeRoleV1::Goto,
                            (index + 1) as u32,
                        ))
                    };
                    SemanticBasicBlockV1::new(
                        SemanticBlockIdentityV1::from_sha256(identity),
                        SemanticSourceProvenanceV1::unavailable(),
                        Vec::new(),
                        SemanticTerminatorV1::new(
                            SemanticSourceProvenanceV1::unavailable(),
                            terminator,
                        ),
                    )
                    .unwrap()
                })
                .collect();
            let function = assertion_root(
                vec![(A_UNIT, SemanticLocalRoleV1::Return)],
                Vec::new(),
                blocks,
            );
            let owner = assertion_materialized(function);
            let program = assertion_project(owner).unwrap();
            assert_eq!(program.roots.len(), 1);
            let root = &program.roots[0];
            let ranked = root.lowering.kernel().blocks();
            assert_eq!(ranked.len(), if count == 1023 { 1024 } else { 2 });
            assert!(ranked.iter().flat_map(|block| block.operations()).any(
                |operation| matches!(operation, ProductionRankedOperationV1::Access { .. })
            ));
            assert_eq!(ranked.last().unwrap().terminator(), &ProductionRankedTerminatorV1::Return);
        }
    }
}

#[test]
fn cfg_linear_fusion_production_projection_route() {
    fusion_production_fixture::check();
}
