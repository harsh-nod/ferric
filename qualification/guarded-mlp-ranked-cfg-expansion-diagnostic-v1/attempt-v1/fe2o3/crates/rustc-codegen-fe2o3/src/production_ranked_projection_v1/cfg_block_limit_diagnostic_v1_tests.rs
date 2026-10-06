// Component fixtures exercise the existing cap sites, not semantic admission.
fn cfg_diagnostic_blocks_v1(count: usize) -> Vec<SemanticBasicBlockV1> {
    (0..count)
        .map(|index| {
            let mut identity = [0_u8; 32];
            identity[..8].copy_from_slice(&(index as u64).to_le_bytes());
            SemanticBasicBlockV1::new(
                SemanticBlockIdentityV1::from_sha256(identity),
                SemanticSourceProvenanceV1::unavailable(),
                vec![],
                SemanticTerminatorV1::new(
                    SemanticSourceProvenanceV1::unavailable(),
                    SemanticTerminatorKindV1::Return,
                ),
            )
            .unwrap()
        })
        .collect()
}

fn cfg_diagnostic_function_v1(count: usize) -> SemanticFunctionDeclV1 {
    projection_function_with_locals(
        cfg_diagnostic_blocks_v1(count),
        vec![local(20, SCALAR_TYPE, SemanticLocalRoleV1::Return)],
    )
}

fn cfg_diagnostic_errors_v1(
    function: &SemanticFunctionDeclV1,
    initial_work: usize,
) -> Vec<ProductionRankedProjectionErrorV1> {
    let mut inventory_work = initial_work;
    let first = charge_loop_switch_inventory_v1(function, &mut inventory_work).unwrap_err();
    assert_eq!(inventory_work, initial_work);
    let inventory = assertion_definition_inventory(function).unwrap();
    let mut graph_work = initial_work;
    let second = projected_loop_cfg_with_inventory_v1(
        &[],
        function,
        &inventory,
        &mut graph_work,
    )
    .unwrap_err();
    assert_eq!(graph_work, initial_work);
    vec![first, second]
}

fn assert_cfg_diagnostic_v1(
    error: &ProductionRankedProjectionErrorV1,
    function: &SemanticFunctionDeclV1,
    site: CfgBlockLimitSiteV1,
) {
    let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = error else {
        panic!("expected the bounded CFG diagnostic: {error}");
    };
    assert_eq!(diagnostic.function, function.identity());
    assert_eq!(diagnostic.role, function.role());
    assert_eq!(diagnostic.source, function.source());
    assert!(diagnostic.kernel_export.is_none());
    assert_eq!(diagnostic.blocks, function.blocks().len());
    assert_eq!(diagnostic.semantic_blocks, function.blocks().len());
    assert_eq!(diagnostic.limit, MAX_RANKED_BOUNDS_BLOCKS);
    assert_eq!(diagnostic.site, site);
    assert!(std::error::Error::source(error).is_none());
    assert!(diagnostic.root.is_none());
    assert!(diagnostic.callable_function_index.is_none());
    let rendered = error.to_string();
    assert!(rendered.starts_with(
        "semantic-to-ranked projection rejected semantic CFG exceeds the ranked block limit before loop analysis; cfg-block-diagnostic-v1",
    ));
    assert!(rendered.contains(&format!(
        "blocks={} limit=2048", function.blocks().len(),
    )));
    assert!(rendered.contains(&format!("function_sha256={}", "0b".repeat(32))));
    assert!(rendered.ends_with(" phase=unattributed"));
    assert!(rendered.len() < 512);
}

#[test]
fn cfg_block_limit_diagnostic_zero_preserves_both_refusals() {
    let function = cfg_diagnostic_function_v1(0);
    for (error, site) in cfg_diagnostic_errors_v1(&function, 17).iter().zip([
        CfgBlockLimitSiteV1::LoopSwitchInventory,
        CfgBlockLimitSiteV1::LoopCfgWithInventory,
    ]) {
        assert_cfg_diagnostic_v1(error, &function, site);
        assert!(error.to_string().contains(" reason=empty "));
    }
}

#[test]
fn cfg_block_limit_diagnostic_exact_limit_preserves_graph_acceptance() {
    assert_eq!(MAX_RANKED_BOUNDS_BLOCKS, 2_048);
    for count in [1_024, 1_121, 2_048] {
        let function = cfg_diagnostic_function_v1(count);
        let mut work = 0;
        charge_loop_switch_inventory_v1(&function, &mut work).unwrap();
        assert_eq!(work, 3 * function.locals().len() + 3 * count);
        let inventory = assertion_definition_inventory(&function).unwrap();
        let graph = projected_loop_cfg_with_inventory_v1(&[], &function, &inventory, &mut work)
            .unwrap();
        assert_eq!(work, 3 * function.locals().len() + 4 * count);
        assert_eq!(graph.successors.len(), count);
        assert_eq!(graph.reachable.iter().filter(|&&value| value).count(), 1);
        let wrapper = projected_loop_cfg_graph_v1(&[], &function).unwrap();
        assert_eq!(wrapper.reachable, graph.reachable);
    }
}

#[test]
fn cfg_block_limit_diagnostic_over_limit_preserves_both_refusals() {
    let function = cfg_diagnostic_function_v1(MAX_RANKED_BOUNDS_BLOCKS + 1);
    for (error, site) in cfg_diagnostic_errors_v1(&function, 17).iter().zip([
        CfgBlockLimitSiteV1::LoopSwitchInventory,
        CfgBlockLimitSiteV1::LoopCfgWithInventory,
    ]) {
        assert_cfg_diagnostic_v1(error, &function, site);
        assert!(error.to_string().contains(" reason=above-limit "));
    }
}

#[test]
fn cfg_block_limit_diagnostic_counts_unreachable_declared_blocks() {
    let function = cfg_diagnostic_function_v1(MAX_RANKED_BOUNDS_BLOCKS + 1);
    assert_eq!(function.entry().index(), 0);
    assert!(function.blocks().iter().all(|block| matches!(
        block.terminator().kind(),
        SemanticTerminatorKindV1::Return,
    )));
    // Only entry is reachable, but the gate still counts all 2,049 declarations.
    let error = projected_loop_cfg_graph_v1(&[], &function).unwrap_err();
    assert_cfg_diagnostic_v1(&error, &function, CfgBlockLimitSiteV1::LoopSwitchInventory);
    assert!(error.to_string().contains("blocks=2049"));
}

#[test]
fn cfg_block_limit_diagnostic_precedes_work_without_changing_budget() {
    for count in [0, MAX_RANKED_BOUNDS_BLOCKS + 1] {
        let function = cfg_diagnostic_function_v1(count);
        for error in cfg_diagnostic_errors_v1(&function, MAX_PROJECTED_LOOP_GRAPH_WORK_V1) {
            assert!(matches!(error, ProductionRankedProjectionErrorV1::CfgBlockLimit(_)));
        }
    }
    let function = cfg_diagnostic_function_v1(MAX_RANKED_BOUNDS_BLOCKS);
    let mut work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
    assert!(matches!(
        charge_loop_switch_inventory_v1(&function, &mut work),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
            if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
    ));
}

#[test]
fn cfg_block_limit_diagnostic_callable_summary_has_no_fabricated_root() {
    let mut blocks = cfg_diagnostic_blocks_v1(MAX_RANKED_BOUNDS_BLOCKS + 1);
    blocks[0] = block(
        70,
        vec![],
        SemanticTerminatorKindV1::Assert {
            condition: typed_constant(BOOL_TYPE, 1, 1),
            expected: true,
            message: SemanticAssertMessageV1::BoundsCheck {
                length: constant(1),
                index: constant(0),
            },
            target: cfg_edge(SemanticEdgeRoleV1::AssertSuccess, 1),
            unwind: SemanticUnwindActionV1::Unreachable,
        },
    );
    let base = scalar_summary_helper(71, blocks);
    let origin = SemanticSourceOriginV1::new(
        SemanticSourceFileIdentityV1::from_sha256(bytes(0xab)),
        100,
        120,
        37,
        11,
        37,
        31,
    )
    .unwrap();
    let source = SemanticSourceProvenanceV1::new(Some(origin), Some(origin));
    let function = SemanticFunctionDeclV1::new(
        base.identity(),
        base.role(),
        base.item_definition_identity(),
        base.monomorphization_identity(),
        base.generic_type_arguments_identity(),
        base.const_generic_arguments_identity(),
        source,
        base.abi().clone(),
        base.locals().to_vec(),
        base.entry(),
        base.blocks().to_vec(),
    )
    .unwrap();
    let expected_identity = function.identity();
    let functions = [scalar_summary_leaf(61), function];
    let Err(error) = derive_defined_callable_empty_effect_summaries_v1(
        &assertion_proof_types(),
        &functions,
        &[],
    ) else {
        panic!("the actual pre-root callable path must retain its block cap");
    };
    let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = &error else {
        panic!("expected a pre-root CFG diagnostic: {error}");
    };
    assert_eq!(diagnostic.function, expected_identity);
    assert_eq!(diagnostic.blocks, MAX_RANKED_BOUNDS_BLOCKS + 1);
    assert_eq!(diagnostic.callable_function_index, Some(1));
    assert_eq!(diagnostic.role, SemanticFunctionRoleV1::InternalHelper);
    assert_eq!(diagnostic.source, source);
    assert!(diagnostic.kernel_export.is_none());
    assert!(diagnostic.root.is_none());
    let rendered = error.to_string();
    assert!(rendered.ends_with("phase=defined-callable-summary function_index=1"));
    assert!(rendered.contains("role=InternalHelper; source=Rust source abababababab:37:11"));
    assert!(!rendered.contains("; root "));
}

#[test]
fn cfg_block_limit_diagnostic_root_is_bounded_and_escaped() {
    let mut name = vec![b'x'; MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1 * 4];
    name[3] = b'\n';
    name[4] = 0xff;
    let function = cfg_diagnostic_function_v1(MAX_RANKED_BOUNDS_BLOCKS + 1)
        .with_kernel_entry(SemanticKernelEntryV1::new(
            SemanticLinkSymbolV1::new(name.clone()).unwrap(),
            SemanticKernelBindingIdentityV1::from_sha256(bytes(247)),
            SemanticKernelSourceContractV1::new(None, None, None).unwrap(),
        ));
    for error in cfg_diagnostic_errors_v1(&function, 0) {
        let error = error.with_deterministic_root_context(
            SemanticFunctionIdV1::from_index(7),
            SemanticFunctionIdV1::from_index(9),
            &name,
        );
        let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = &error else {
            panic!("typed error lost");
        };
        let root = diagnostic.root.as_ref().unwrap();
        assert_eq!(root.name_len, MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1);
        assert!(root.truncated);
        let export = diagnostic.kernel_export.as_ref().unwrap();
        assert_eq!(export.len, MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1);
        assert!(export.truncated);
        assert!(diagnostic.callable_function_index.is_none());
        let rendered = error.to_string();
        assert!(rendered.contains("phase=root-projection; root "));
        assert!(rendered.contains("; kernel_export=xxx\\n\\xff"));
        assert!(rendered.contains("\\n\\xff"));
        assert!(!rendered.contains('\n'));
        assert!(rendered.ends_with("... (semantic root 7, body 9)"));
        assert!(rendered.len() < 2048);
    }
}

#[test]
fn cfg_block_limit_diagnostic_context_leaves_other_errors_unchanged() {
    for error in [
        ProductionRankedProjectionErrorV1::Unsupported("other refusal"),
        ProductionRankedProjectionErrorV1::Incomplete("other incomplete"),
    ] {
        let before = error.to_string();
        let error = error
            .with_cfg_callable_context_v1(13)
            .with_deterministic_root_context(
                SemanticFunctionIdV1::from_index(7),
                SemanticFunctionIdV1::from_index(9),
                b"not_attached",
            );
        assert_eq!(error.to_string(), before);
    }
}

#[test]
fn cfg_block_cap_preserves_fixed_graph_work_and_edge_caps() {
    assert_eq!(MAX_RANKED_BOUNDS_BLOCKS, 2_048);
    assert_eq!(MAX_RANKED_BOUNDS_EDGES, 2_048);
    assert_eq!(MAX_PROJECTED_LOOP_GRAPH_WORK_V1, 3_145_728);
    let mut work = 0;
    project_loop_graph_charge_v1(&mut work, 3_145_728).unwrap();
    assert_eq!(work, 3_145_728);
    assert!(matches!(
        project_loop_graph_charge_v1(&mut work, 1),
        Err(ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic))
            if diagnostic.reason == GraphWorkFailureV1::AboveLimit,
    ));
    assert_eq!(work, 3_145_729);
}

fn cfg_projected_diagnostic_build_v1(
    function: &SemanticFunctionDeclV1,
) -> Result<Vec<ProductionRankedBlockV1>, ProductionRankedProjectionErrorV1> {
    build_ranked_cfg(
        &projection_types(),
        function,
        &[],
        &vec![None; function.blocks().len()],
        &vec![None; function.blocks().len()],
        &[],
        vec![],
        (0..function.blocks().len())
            .map(|_| ProjectedSemanticBlockV1 { items: vec![] })
            .collect(),
    )
    .map(|(blocks, _, _)| blocks)
}

fn cfg_projected_diagnostic_chain_v1(count: usize) -> SemanticFunctionDeclV1 {
    let blocks = (0..count)
        .map(|index| {
            let mut identity = [0_u8; 32];
            identity[..8].copy_from_slice(&(index as u64).to_le_bytes());
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
                vec![],
                SemanticTerminatorV1::new(
                    SemanticSourceProvenanceV1::unavailable(),
                    terminator,
                ),
            )
            .unwrap()
        })
        .collect();
    projection_function_with_locals(
        blocks,
        vec![local(20, SCALAR_TYPE, SemanticLocalRoleV1::Return)],
    )
}

#[test]
fn cfg_projected_block_limit_diagnostic_exact_and_next_boundaries() {
    // Every source block is reachable; the builder adds one ranked entry block.
    let exact = cfg_projected_diagnostic_chain_v1(MAX_RANKED_BOUNDS_BLOCKS - 1);
    assert_eq!(
        cfg_projected_diagnostic_build_v1(&exact).unwrap().len(),
        MAX_RANKED_BOUNDS_BLOCKS,
    );
    let next = cfg_projected_diagnostic_chain_v1(MAX_RANKED_BOUNDS_BLOCKS);
    let error = cfg_projected_diagnostic_build_v1(&next).unwrap_err();
    let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = &error else {
        panic!("expected the ranked expansion diagnostic: {error}");
    };
    assert_eq!(diagnostic.blocks, MAX_RANKED_BOUNDS_BLOCKS + 1);
    assert_eq!(diagnostic.semantic_blocks, MAX_RANKED_BOUNDS_BLOCKS);
    assert_eq!(diagnostic.limit, MAX_RANKED_BOUNDS_BLOCKS);
    assert_eq!(
        diagnostic.site,
        CfgBlockLimitSiteV1::ProjectedCfgExpansion {
            blocks: MAX_RANKED_BOUNDS_BLOCKS + 1,
        },
    );
    assert!(error.to_string().contains("blocks=2049 limit=2048 reason=above-limit"));
}

#[test]
fn cfg_projected_block_limit_diagnostic_expansion_counts_and_identity() {
    let function = explicit_multi_switch(MAX_RANKED_BOUNDS_BLOCKS / 2 + 1);
    let error = cfg_projected_diagnostic_build_v1(&function).unwrap_err();
    let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = &error else {
        panic!("expected the ranked expansion diagnostic: {error}");
    };
    assert_eq!(diagnostic.blocks, MAX_RANKED_BOUNDS_BLOCKS + 2);
    assert_eq!(diagnostic.semantic_blocks, function.blocks().len());
    assert_eq!(diagnostic.semantic_blocks, MAX_RANKED_BOUNDS_BLOCKS / 2 + 2);
    assert_eq!(diagnostic.function, function.identity());
    assert_eq!(diagnostic.role, function.role());
    assert_eq!(diagnostic.source, function.source());
    assert!(diagnostic.kernel_export.is_none());
    assert!(diagnostic.callable_function_index.is_none());
    assert!(diagnostic.root.is_none());
    assert!(std::error::Error::source(&error).is_none());
    let rendered = error.to_string();
    assert!(rendered.starts_with(
        "semantic-to-ranked projection rejected semantic CFG projection exceeds the ranked block limit; cfg-block-diagnostic-v1",
    ));
    assert!(rendered.contains("site=projected-cfg-expansion semantic_blocks=1026"));
    assert!(rendered.ends_with(" phase=unattributed"));
    assert!(!rendered.contains("before loop analysis"));
    assert!(rendered.len() < 512);
}

#[test]
fn cfg_projected_block_limit_diagnostic_context_is_bounded() {
    let mut name = vec![b'x'; MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1 * 4];
    name[3] = b'\n';
    name[4] = 0xff;
    let function = cfg_diagnostic_function_v1(1)
        .with_kernel_entry(SemanticKernelEntryV1::new(
            SemanticLinkSymbolV1::new(name.clone()).unwrap(),
            SemanticKernelBindingIdentityV1::from_sha256(bytes(247)),
            SemanticKernelSourceContractV1::new(None, None, None).unwrap(),
        ));
    // Exercise formatting at the integer boundary, not an observed CFG count.
    let error = ProductionRankedProjectionErrorV1::cfg_block_limit_v1(
        &function,
        CfgBlockLimitSiteV1::ProjectedCfgExpansion { blocks: usize::MAX },
    )
    .with_deterministic_root_context(
        SemanticFunctionIdV1::from_index(7),
        SemanticFunctionIdV1::from_index(9),
        &name,
    );
    let ProductionRankedProjectionErrorV1::CfgBlockLimit(diagnostic) = &error else {
        panic!("typed expansion diagnostic lost");
    };
    assert_eq!(diagnostic.blocks, usize::MAX);
    assert_eq!(diagnostic.semantic_blocks, 1);
    assert_eq!(diagnostic.function, function.identity());
    let root = diagnostic.root.as_ref().unwrap();
    assert_eq!(root.name_len, MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1);
    assert!(root.truncated);
    let export = diagnostic.kernel_export.as_ref().unwrap();
    assert_eq!(export.len, MAX_DETERMINISTIC_DIAGNOSTIC_ROOT_BYTES_V1);
    assert!(export.truncated);
    let rendered = error.to_string();
    assert!(rendered.contains("; kernel_export=xxx\\n\\xff"));
    assert!(rendered.contains("phase=root-projection; root "));
    assert!(!rendered.contains('\n'));
    assert!(rendered.ends_with("... (semantic root 7, body 9)"));
    assert!(rendered.len() < 2048);
}
