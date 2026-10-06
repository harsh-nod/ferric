type CfgCompactionOutputV1 = (
    Vec<ProductionRankedBlockV1>,
    Vec<ProjectedAccessSourceV1>,
    Vec<ProductionRankedExecutableEffectSourceV1>,
);

fn compaction_access_v1(index: u32, atomic: bool) -> GuardedRankedAccessV1 {
    GuardedRankedAccessV1 {
        view: ProductionRankedValueIdV1::new(index),
        indices: vec![ProductionRankedValueV1::Argument(index * 2)],
        checked_success: None,
        failure: GuardedAccessFailureV1::Trap,
        atomic: atomic.then_some(SemanticAtomicAccessV1::new(
            SemanticAtomicOrderingV1::Acquire,
            SemanticAtomicScopeV1::System,
        )),
        atomic_failure: None,
        comparisons: vec![(
            ProductionRankedValueV1::Argument(index * 2),
            ProductionRankedValueV1::Argument(index * 2 + 1),
        )],
        access: if atomic {
            AccessKindAttr::AtomicRead
        } else {
            AccessKindAttr::Read
        },
        memory_space: MemorySpaceAttr::Global,
        source: SemanticSourceProvenanceV1::unavailable(),
        semantic_site: Some(ProjectedSemanticAccessSiteV1 {
            block: 0,
            statement: Some(index as usize),
        }),
    }
}

fn compaction_build_v1(
    function: &SemanticFunctionDeclV1,
    projected: Vec<ProjectedSemanticBlockV1>,
    compact: bool,
    waves: &mut Vec<ProjectedWaveSyncSiteV1>,
) -> Result<CfgCompactionOutputV1, ProductionRankedProjectionErrorV1> {
    build_ranked_cfg_with_wave_sync_layout_v1(
        &projection_types(),
        function,
        &[],
        &vec![None; function.blocks().len()],
        &vec![None; function.blocks().len()],
        &[],
        None,
        vec![],
        projected,
        &mut ComponentDynamicAssertionFactsV1,
        Some(waves),
        compact,
    )
}

fn compaction_one_block_v1(
    items: Vec<ProjectedBlockItemV1>,
    compact: bool,
) -> Result<CfgCompactionOutputV1, ProductionRankedProjectionErrorV1> {
    compaction_build_v1(
        &cfg_projected_diagnostic_chain_v1(1),
        vec![ProjectedSemanticBlockV1 { items }],
        compact,
        &mut vec![],
    )
}

#[derive(Debug, PartialEq)]
enum CfgCompactionTraceV1 {
    Compare(ProductionRankedValueV1, ProductionRankedValueV1, bool),
    Effect(ProductionRankedOperationV1),
    Return,
    Trap,
}

// Interpret only these fixtures' control edges; never silently skip an operation
// or unsupported terminator. This compares exact checks and ordered effects.
fn compaction_trace_v1(
    blocks: &[ProductionRankedBlockV1],
    arguments: &[u64],
) -> Vec<CfgCompactionTraceV1> {
    let mut trace = vec![];
    let mut block = 0;
    for _ in 0..blocks.len() + 1 {
        let current = &blocks[block];
        assert_eq!(current.index_argument_count(), 0);
        trace.extend(current.operations().iter().cloned().map(CfgCompactionTraceV1::Effect));
        match current.terminator() {
            ProductionRankedTerminatorV1::Branch { target } => block = *target as usize,
            ProductionRankedTerminatorV1::IndexLessThan {
                lhs,
                rhs,
                true_block,
                false_block,
            } => {
                let value = |operand: &ProductionRankedValueV1| match operand {
                    ProductionRankedValueV1::Argument(index) => arguments[*index as usize],
                    _ => panic!("fixture predicate must use an argument"),
                };
                let success = value(lhs) < value(rhs);
                trace.push(CfgCompactionTraceV1::Compare(*lhs, *rhs, success));
                block = if success { *true_block } else { *false_block } as usize;
            }
            ProductionRankedTerminatorV1::Return => {
                trace.push(CfgCompactionTraceV1::Return);
                return trace;
            }
            ProductionRankedTerminatorV1::Trap => {
                trace.push(CfgCompactionTraceV1::Trap);
                return trace;
            }
            other => panic!("unexpected fixture terminator: {other:?}"),
        }
    }
    panic!("fixture did not terminate within its acyclic block bound");
}

fn compaction_raw_edges_v1(blocks: &[ProductionRankedBlockV1]) -> usize {
    blocks.iter().map(|block| match block.terminator() {
        ProductionRankedTerminatorV1::Branch { .. } => 1,
        ProductionRankedTerminatorV1::IndexLessThan { .. } => 2,
        ProductionRankedTerminatorV1::Return | ProductionRankedTerminatorV1::Trap => 0,
        other => panic!("unexpected fixture terminator: {other:?}"),
    }).sum()
}

#[test]
fn cfg_compaction_differential_checks_effects_and_traps() {
    let mut executions = 0;
    for count in 1..=4 {
        for two_comparisons in [false, true] {
            let items = (0..count).map(|index| {
                let mut access = compaction_access_v1(index, index % 2 == 0);
                if index == 2 {
                    access.access = AccessKindAttr::AtomicWrite;
                    access.atomic = Some(SemanticAtomicAccessV1::new(
                        SemanticAtomicOrderingV1::Release,
                        SemanticAtomicScopeV1::System,
                    ));
                } else if index == 3 {
                    access.access = AccessKindAttr::AtomicReadModifyWrite;
                    access.atomic = Some(SemanticAtomicAccessV1::new(
                        SemanticAtomicOrderingV1::AcquireRelease,
                        SemanticAtomicScopeV1::System,
                    ));
                    access.atomic_failure = Some(SemanticAtomicOrderingV1::Acquire);
                }
                if two_comparisons {
                    access.comparisons.push((
                        ProductionRankedValueV1::Argument(count * 2),
                        ProductionRankedValueV1::Argument(count * 2 + 1),
                    ));
                }
                ProjectedBlockItemV1::Guarded(access)
            }).collect::<Vec<_>>();
            let (old, _, _) = compaction_one_block_v1(items.clone(), false).unwrap();
            let (new, _, _) = compaction_one_block_v1(items, true).unwrap();
            assert_eq!(new.len(), old.len() - 2 * count as usize + 1);
            assert_eq!(compaction_raw_edges_v1(&new), compaction_raw_edges_v1(&old) - count as usize);
            assert_eq!(
                new.iter().filter(|block| matches!(
                    block.terminator(), ProductionRankedTerminatorV1::Trap,
                )).count(),
                1,
            );
            for mask in 0..1_u32 << (count + 1) {
                let arguments = (0..=count).flat_map(|i| {
                    [if mask & (1 << i) == 0 { 0 } else { 1 }, 1]
                }).collect::<Vec<_>>();
                let old_trace = compaction_trace_v1(&old, &arguments);
                let new_trace = compaction_trace_v1(&new, &arguments);
                assert_eq!(new_trace, old_trace);
                if mask == 0 {
                    assert_eq!(new_trace.last(), Some(&CfgCompactionTraceV1::Return));
                    assert_eq!(
                        new_trace.iter().filter(|event| matches!(
                            event, CfgCompactionTraceV1::Effect(_),
                        )).count(),
                        count as usize,
                    );
                } else if !two_comparisons && mask == 1 << count {
                    assert_eq!(new_trace.last(), Some(&CfgCompactionTraceV1::Return));
                } else {
                    assert_eq!(new_trace.last(), Some(&CfgCompactionTraceV1::Trap));
                }
                executions += 1;
            }
        }
    }
    assert_eq!(executions, 120);
}

#[test]
fn cfg_compaction_source_wave_and_generated_identity() {
    let function = cfg_projected_diagnostic_chain_v1(1);
    let source = SemanticSourceProvenanceV1::unavailable();
    let barrier = ProductionRankedOperationV1::Barrier {
        execution_scope: HierarchyAttr::Workgroup,
        memory_scope: MemoryScopeAttr::Workgroup,
        address_space: AddressSpaceAttr::Workgroup,
        order: MemoryOrderAttr::AcquireRelease,
    };
    let effect = |operation| ProjectedBlockItemV1::Effect { operation, source: None };
    let generated = |ordinal| ProjectedBlockItemV1::GeneratedFromSemanticTerminator(
        ProjectedGeneratedExecutableEffectV1 {
            operation: ProductionRankedOperationV1::IndexConstant {
                result: ProductionRankedValueIdV1::new(100 + ordinal),
                value: u64::from(ordinal),
            },
            semantic_effect_ordinal: ordinal,
            origin: ProjectedExecutableEffectOriginV1::GeneratedFromSemanticTerminator { parent: source },
            recipe_identity: bytes(80 + ordinal as u8),
        },
    );
    let items = vec![
        effect(barrier.clone()),
        ProjectedBlockItemV1::Guarded(compaction_access_v1(0, true)),
        effect(barrier.clone()),
        generated(0),
        ProjectedBlockItemV1::Effect {
            operation: ProductionRankedOperationV1::Access {
                kind: AccessKindAttr::Read,
                view: ProductionRankedValueV1::Local(ProductionRankedValueIdV1::new(99)),
                indices: vec![ProductionRankedValueV1::Argument(0)],
            },
            source: Some(ProjectedEffectSourceV1 {
                access: AccessKindAttr::Read,
                memory_space: MemorySpaceAttr::Global,
                source,
                semantic_site: Some(ProjectedSemanticAccessSiteV1 {
                    block: 0,
                    statement: Some(99),
                }),
            }),
        },
        ProjectedBlockItemV1::Guarded(compaction_access_v1(1, false)),
        effect(barrier),
        generated(1),
    ];
    let mut old_waves = vec![];
    let mut new_waves = vec![];
    let old = compaction_build_v1(
        &function,
        vec![ProjectedSemanticBlockV1 { items: items.clone() }],
        false,
        &mut old_waves,
    ).unwrap();
    let new = compaction_build_v1(
        &function,
        vec![ProjectedSemanticBlockV1 { items }],
        true,
        &mut new_waves,
    ).unwrap();
    assert_eq!(compaction_trace_v1(&new.0, &[0, 1, 0, 1]), compaction_trace_v1(&old.0, &[0, 1, 0, 1]));
    assert_eq!(new.1.len(), 3);
    for (before, after) in old.1.iter().zip(&new.1) {
        assert_eq!(
            (before.access, before.memory_space, before.source, before.semantic_site),
            (after.access, after.memory_space, after.source, after.semantic_site),
        );
        assert_eq!(
            old.0[before.block].operations()[before.operation],
            new.0[after.block].operations()[after.operation],
        );
    }
    assert_eq!(new.1[1].block, new.1[0].block);
    assert_eq!(new.1[1].operation, 3);
    assert_eq!(new_waves.len(), 3);
    for (before, after) in old_waves.iter().zip(&new_waves) {
        assert_eq!(before.semantic_block, after.semantic_block);
        assert_eq!(
            old.0[before.ranked_block].operations()[before.ranked_operation],
            new.0[after.ranked_block].operations()[after.ranked_operation],
        );
    }
    assert_eq!(new_waves[1].ranked_block, new.1[0].block);
    assert_eq!(new_waves[1].ranked_operation, new.1[0].operation + 1);
    assert_eq!(new.2.len(), 2);
    for (before, after) in old.2.iter().zip(&new.2) {
        assert_eq!(
            (before.semantic_block(), before.semantic_effect_ordinal(), before.origin(), before.recipe_identity()),
            (after.semantic_block(), after.semantic_effect_ordinal(), after.origin(), after.recipe_identity()),
        );
        assert_eq!(
            old.0[before.ranked_block() as usize].operations()[before.ranked_operation() as usize],
            new.0[after.ranked_block() as usize].operations()[after.ranked_operation() as usize],
        );
    }
    assert_eq!(new.2[0].ranked_operation(), 2);
}

#[test]
fn cfg_compaction_mixed_failure_layout() {
    let mut continuing = compaction_access_v1(1, false);
    continuing.failure = GuardedAccessFailureV1::ContinueWithoutAccess;
    let items = vec![
        ProjectedBlockItemV1::Guarded(compaction_access_v1(0, true)),
        ProjectedBlockItemV1::Guarded(continuing.clone()),
        ProjectedBlockItemV1::Guarded(compaction_access_v1(2, false)),
    ];
    let old = compaction_one_block_v1(items.clone(), false).unwrap();
    let new = compaction_one_block_v1(items, true).unwrap();
    assert_eq!(new.0.len(), old.0.len() - 3);
    assert_eq!(compaction_raw_edges_v1(&new.0), compaction_raw_edges_v1(&old.0) - 2);
    for mask in 0..8 {
        let args = (0..3).flat_map(|i| [if mask & (1 << i) == 0 { 0 } else { 1 }, 1]).collect::<Vec<_>>();
        assert_eq!(compaction_trace_v1(&new.0, &args), compaction_trace_v1(&old.0, &args));
    }
    let only = vec![ProjectedBlockItemV1::Guarded(continuing)];
    assert_eq!(
        compaction_one_block_v1(only.clone(), false).unwrap(),
        compaction_one_block_v1(only, true).unwrap(),
    );
}

#[test]
fn cfg_compaction_live_induction_layout_unchanged() {
    let function = multi_block_induction_function(
        InductionCfgShape::Chain, SemanticLocalRoleV1::Argument(0), 1,
    );
    let (inductions, entry, _) = project_test_inductions(&function).unwrap();
    let mut projected = vec![ProjectedSemanticBlockV1 { items: vec![] }; function.blocks().len()];
    projected[2].items.push(ProjectedBlockItemV1::Guarded(compaction_access_v1(0, false)));
    let build = |compact| build_ranked_cfg_with_wave_sync_layout_v1(
        &projection_types(), &function, &[], &vec![None; function.blocks().len()],
        &vec![None; function.blocks().len()], &inductions, None, entry.clone(),
        projected.clone(), &mut ComponentDynamicAssertionFactsV1, None, compact,
    ).unwrap();
    let old = build(false);
    let new = build(true);
    assert_eq!(new, old);
    assert!(new.0.iter().any(|block| matches!(
        block.terminator(),
        ProductionRankedTerminatorV1::IndexLessThanArgs { false_arguments, .. }
            if false_arguments.is_empty(),
    )));
}

#[test]
fn cfg_compaction_unreachable_and_zero_eligible_unchanged() {
    let empty = compaction_one_block_v1(vec![], true).unwrap();
    assert_eq!(empty, compaction_one_block_v1(vec![], false).unwrap());
    assert_eq!(empty.0.len(), 2);
    assert!(!empty.0.iter().any(|block| matches!(block.terminator(), ProductionRankedTerminatorV1::Trap)));
    let entry = cfg_projected_diagnostic_chain_v1(1).blocks()[0].clone();
    let unreachable = cfg_projected_diagnostic_chain_v1(2).blocks()[1].clone();
    let function = projection_function_with_locals(
        vec![entry, unreachable],
        vec![local(20, SCALAR_TYPE, SemanticLocalRoleV1::Return)],
    );
    let projected = vec![
        ProjectedSemanticBlockV1 { items: vec![] },
        ProjectedSemanticBlockV1 {
            items: vec![ProjectedBlockItemV1::Guarded(compaction_access_v1(0, true))],
        },
    ];
    let old = compaction_build_v1(&function, projected.clone(), false, &mut vec![]).unwrap();
    let new = compaction_build_v1(&function, projected, true, &mut vec![]).unwrap();
    assert_eq!(new, old);
    assert_eq!(new.0.len(), 2);
    assert!(new.1.is_empty());
}

#[test]
fn cfg_compaction_atomic_and_predicate_refusals_unchanged() {
    let baseline = compaction_access_v1(0, true);
    let mut invalid = vec![];
    let mut access = baseline.clone();
    access.atomic = None;
    invalid.push(access);
    let mut access = baseline.clone();
    access.access = AccessKindAttr::Read;
    invalid.push(access);
    let mut access = baseline.clone();
    access.atomic_failure = Some(SemanticAtomicOrderingV1::Relaxed);
    invalid.push(access);
    let mut access = baseline.clone();
    access.atomic = Some(SemanticAtomicAccessV1::new(
        SemanticAtomicOrderingV1::Acquire, SemanticAtomicScopeV1::Agent,
    ));
    invalid.push(access);
    let mut access = baseline.clone();
    access.checked_success = Some(ProductionRankedValueV1::Argument(0));
    invalid.push(access);
    let mut access = baseline.clone();
    access.failure = GuardedAccessFailureV1::ContinueWithoutAccess;
    invalid.push(access);
    let mut access = baseline;
    access.comparisons.clear();
    invalid.push(access);
    assert_eq!(invalid.len(), 7);
    for access in invalid {
        let items = vec![ProjectedBlockItemV1::Guarded(access)];
        let old = compaction_one_block_v1(items.clone(), false).unwrap_err();
        let new = compaction_one_block_v1(items, true).unwrap_err();
        assert_eq!(new.to_string(), old.to_string());
    }
    let bad_ordinal = ProjectedBlockItemV1::GeneratedFromSemanticTerminator(
        ProjectedGeneratedExecutableEffectV1 {
            operation: ProductionRankedOperationV1::IndexConstant {
                result: ProductionRankedValueIdV1::new(10),
                value: 1,
            },
            semantic_effect_ordinal: 1,
            origin: ProjectedExecutableEffectOriginV1::GeneratedFromSemanticTerminator {
                parent: SemanticSourceProvenanceV1::unavailable(),
            },
            recipe_identity: bytes(70),
        },
    );
    let items = vec![ProjectedBlockItemV1::Guarded(compaction_access_v1(0, false)), bad_ordinal];
    assert_eq!(
        compaction_one_block_v1(items.clone(), true).unwrap_err().to_string(),
        compaction_one_block_v1(items, false).unwrap_err().to_string(),
    );
}

#[test]
fn cfg_compaction_exact_block_boundary_and_next_refusal() {
    let items = |count| (0..count)
        .map(|_| ProjectedBlockItemV1::Guarded(compaction_access_v1(0, false)))
        .collect::<Vec<_>>();
    // One semantic block + entry + shared trap + one comparison block per site.
    let exact = compaction_one_block_v1(items(MAX_RANKED_BOUNDS_BLOCKS - 3), true).unwrap();
    assert_eq!(exact.0.len(), MAX_RANKED_BOUNDS_BLOCKS);
    assert_eq!(exact.1.len(), MAX_RANKED_BOUNDS_BLOCKS - 3);
    for compact in [false, true] {
        let error = compaction_one_block_v1(items(MAX_RANKED_BOUNDS_BLOCKS - 2), compact).unwrap_err();
        let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = error else {
            panic!("block limit was bypassed");
        };
        assert_eq!(diagnostic.limit, 2048);
        assert_eq!(diagnostic.blocks, if compact { 2049 } else { 6140 });
    }
}

#[test]
fn cfg_compaction_edge_budget_remains_independent() {
    assert_eq!(MAX_RANKED_BOUNDS_BLOCKS, 2048);
    assert_eq!(MAX_RANKED_BOUNDS_EDGES, 2048);
    assert_eq!(MAX_PROJECTED_LOOP_GRAPH_WORK_V1, 3_145_728);
    for semantic_blocks in 1..=3 {
        let function = cfg_projected_diagnostic_chain_v1(semantic_blocks);
        let mut projected = vec![ProjectedSemanticBlockV1 { items: vec![] }; semantic_blocks];
        projected[0].items = (0..1023)
            .map(|_| ProjectedBlockItemV1::Guarded(compaction_access_v1(0, false)))
            .collect();
        let compact = compaction_build_v1(&function, projected, true, &mut vec![]).unwrap();
        assert_eq!(compact.0.len(), 1025 + semantic_blocks);
        assert_eq!(compaction_raw_edges_v1(&compact.0), 2046 + semantic_blocks);
        assert!(compact.0.len() < MAX_RANKED_BOUNDS_BLOCKS);
        assert_eq!(compaction_raw_edges_v1(&compact.0) > MAX_RANKED_BOUNDS_EDGES, semantic_blocks == 3);
    }
    // The actual ranked-bounds preflight still owns edge admission. A compact
    // builder result is deliberately not asserted to pass that later gate.
}

#[test]
fn cfg_compaction_multiple_semantic_blocks_preserve_traces() {
    let function = cfg_projected_diagnostic_chain_v1(4);
    let projected = (0..4).map(|block| ProjectedSemanticBlockV1 {
        items: vec![ProjectedBlockItemV1::Guarded(compaction_access_v1(block, block % 2 == 0))],
    }).collect::<Vec<_>>();
    let old = compaction_build_v1(&function, projected.clone(), false, &mut vec![]).unwrap();
    let new = compaction_build_v1(&function, projected, true, &mut vec![]).unwrap();
    assert_eq!(old.0.len(), 17);
    assert_eq!(new.0.len(), 10);
    assert_eq!(compaction_raw_edges_v1(&old.0) - compaction_raw_edges_v1(&new.0), 4);
    for mask in 0..16 {
        let args = (0..4).flat_map(|i| {
            [if mask & (1 << i) == 0 { 0 } else { u64::MAX }, u64::MAX]
        }).collect::<Vec<_>>();
        assert_eq!(compaction_trace_v1(&new.0, &args), compaction_trace_v1(&old.0, &args));
    }
}
