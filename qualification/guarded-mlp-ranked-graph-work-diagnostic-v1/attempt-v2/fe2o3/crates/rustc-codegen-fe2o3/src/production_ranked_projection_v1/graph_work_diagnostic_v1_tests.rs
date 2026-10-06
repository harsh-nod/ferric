// Component tests observe real charge paths; they do not qualify any kernel.
fn graph_work_diagnostic_v1(
    error: &ProductionRankedProjectionErrorV1,
) -> &GraphWorkDiagnosticV1 {
    let ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic) = error else {
        panic!("expected exact graph-work refusal: {error}");
    };
    assert!(std::error::Error::source(error).is_none());
    diagnostic
}

fn graph_work_failure_v1() -> ProductionRankedProjectionErrorV1 {
    let mut work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
    project_loop_graph_charge_v1(&mut work, 1).unwrap_err()
}

fn graph_work_source_function_v1(tag: u8) -> SemanticFunctionDeclV1 {
    let base = scalar_summary_leaf(tag);
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
    SemanticFunctionDeclV1::new(
        base.identity(),
        base.role(),
        base.item_definition_identity(),
        base.monomorphization_identity(),
        base.generic_type_arguments_identity(),
        base.const_generic_arguments_identity(),
        SemanticSourceProvenanceV1::new(Some(origin), Some(origin)),
        base.abi().clone(),
        base.locals().to_vec(),
        base.entry(),
        base.blocks().to_vec(),
    )
    .unwrap()
}

#[test]
fn graph_work_diagnostic_exact_limit_and_over_limit_preserve_counter() {
    let mut work = 0;
    project_loop_graph_charge_v1(&mut work, 0).unwrap();
    project_loop_graph_charge_v1(&mut work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1).unwrap();
    assert_eq!(work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    project_loop_graph_charge_v1(&mut work, 0).unwrap();
    let error = project_loop_graph_charge_v1(&mut work, 1).unwrap_err();
    let diagnostic = graph_work_diagnostic_v1(&error);
    assert_eq!(diagnostic.reason, GraphWorkFailureV1::AboveLimit);
    assert_eq!(diagnostic.before, MAX_PROJECTED_LOOP_GRAPH_WORK_V1);
    assert_eq!(diagnostic.amount, 1);
    assert_eq!(diagnostic.sum, Some(MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1));
    assert_eq!(work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1);
    assert!(diagnostic.function.is_none());
    assert!(diagnostic.root.is_none());
    assert!(error.to_string().starts_with(
        "semantic-to-ranked projection rejected uniform induction CFG analysis exceeds its work limit; loop-graph-work-diagnostic-v1 reason=above-limit",
    ));
    assert!(error.to_string().ends_with(" phase=unattributed"));
}

#[test]
fn graph_work_diagnostic_overflow_preserves_counter() {
    for (before, amount) in [(usize::MAX, 1), (usize::MAX - 1, 2)] {
        let mut work = before;
        let error = project_loop_graph_charge_v1(&mut work, amount).unwrap_err();
        let diagnostic = graph_work_diagnostic_v1(&error);
        assert_eq!(diagnostic.reason, GraphWorkFailureV1::Overflow);
        assert_eq!(diagnostic.before, before);
        assert_eq!(diagnostic.amount, amount);
        assert_eq!(diagnostic.sum, None);
        assert_eq!(work, before);
        assert!(error.to_string().starts_with(
            "semantic-to-ranked projection rejected uniform induction CFG analysis work overflow; loop-graph-work-diagnostic-v1 reason=overflow",
        ));
        assert!(error.to_string().contains(" sum=none limit=3145728 "));
    }
}

#[test]
fn graph_work_diagnostic_direct_callers_are_distinct() {
    let mut first_work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
    let first_line = line!() + 1;
    let first = project_loop_graph_charge_v1(&mut first_work, 1).unwrap_err();
    let mut second_work = usize::MAX;
    let second_line = line!() + 1;
    let second = project_loop_graph_charge_v1(&mut second_work, 1).unwrap_err();
    for (error, expected) in [(&first, first_line), (&second, second_line)] {
        let diagnostic = graph_work_diagnostic_v1(error);
        assert_eq!(diagnostic.line, expected);
        assert!(diagnostic.column > 0);
        assert_eq!(
            &diagnostic.path.bytes[..diagnostic.path.len],
            &file!().as_bytes()[file!().len().saturating_sub(MAX_GRAPH_WORK_PATH_BYTES_V1)..],
        );
    }
    assert_ne!(first_line, second_line);
}

#[test]
fn graph_work_diagnostic_proof_forwarder_keeps_caller_and_counter() {
    let function = cfg_diagnostic_function_v1(2);
    let mut proof = SemanticAssertProofsV1::new(&[], &function).unwrap();
    proof.work = MAX_PROJECTED_LOOP_GRAPH_WORK_V1;
    let expected_line = line!() + 1;
    let error = proof.charge(1).unwrap_err();
    let diagnostic = graph_work_diagnostic_v1(&error);
    assert_eq!(diagnostic.line, expected_line);
    assert_eq!(diagnostic.reason, GraphWorkFailureV1::AboveLimit);
    assert_eq!(diagnostic.sum, Some(MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1));
    assert_eq!(proof.work, MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1);
    assert!(proof.indexed_atomic_cfg_acyclic.is_none());
}

#[test]
fn graph_work_diagnostic_atomic_forwarder_preserves_cell_on_failure() {
    let atomic = AuthenticatedAtomicAllocationsV1::default();
    atomic.charge(3).unwrap();
    assert_eq!(atomic.work.get(), 3);
    for (before, reason, sum) in [
        (
            MAX_PROJECTED_LOOP_GRAPH_WORK_V1,
            GraphWorkFailureV1::AboveLimit,
            Some(MAX_PROJECTED_LOOP_GRAPH_WORK_V1 + 1),
        ),
        (usize::MAX, GraphWorkFailureV1::Overflow, None),
    ] {
        atomic.work.set(before);
        let expected_line = line!() + 1;
        let error = atomic.charge(1).unwrap_err();
        let diagnostic = graph_work_diagnostic_v1(&error);
        assert_eq!(diagnostic.line, expected_line);
        assert_eq!(diagnostic.reason, reason);
        assert_eq!(diagnostic.before, before);
        assert_eq!(diagnostic.sum, sum);
        assert_eq!(atomic.work.get(), before);
    }
}

#[test]
fn graph_work_diagnostic_body_context_uses_actual_body_not_root() {
    let functions = [scalar_summary_leaf(61), graph_work_source_function_v1(71)];
    assert_ne!(functions[0].identity(), functions[1].identity());
    let error = graph_work_failure_v1()
        .with_graph_work_body_v1(&functions, SemanticFunctionIdV1::from_index(1))
        .with_deterministic_root_context(
            SemanticFunctionIdV1::from_index(0),
            SemanticFunctionIdV1::from_index(1),
            b"separate_logical_root",
        );
    let diagnostic = graph_work_diagnostic_v1(&error);
    let function = diagnostic.function.as_ref().unwrap();
    assert_eq!(function.index, 1);
    assert_eq!(function.identity, functions[1].identity());
    assert_eq!(function.role, functions[1].role());
    assert_eq!(function.source, functions[1].source());
    assert_ne!(function.source, functions[0].source());
    assert_eq!(function.context, GraphWorkContextV1::RootProjection);
    let root = diagnostic.root.as_ref().unwrap();
    assert_eq!(root.root, SemanticFunctionIdV1::from_index(0));
    assert_eq!(root.body, SemanticFunctionIdV1::from_index(1));
    let error = graph_work_failure_v1()
        .with_graph_work_body_v1(&functions, SemanticFunctionIdV1::from_index(2));
    assert!(graph_work_diagnostic_v1(&error).function.is_none());
    assert!(graph_work_diagnostic_v1(&error).root.is_none());
}

#[test]
fn graph_work_diagnostic_first_function_context_is_retained() {
    let functions = [scalar_summary_leaf(61), scalar_summary_leaf(71)];
    let error = graph_work_failure_v1().with_graph_work_function_v1(
        0,
        &functions[0],
        GraphWorkContextV1::DefinedCallableSummary,
    );
    let diagnostic = graph_work_diagnostic_v1(&error);
    assert!(diagnostic.root.is_none());
    assert!(error.to_string().contains("phase=defined-callable-summary"));
    let error = error
        .with_graph_work_body_v1(&functions, SemanticFunctionIdV1::from_index(1))
        .with_deterministic_root_context(
            SemanticFunctionIdV1::from_index(0),
            SemanticFunctionIdV1::from_index(1),
            b"first_root",
        )
        .with_deterministic_root_context(
            SemanticFunctionIdV1::from_index(1),
            SemanticFunctionIdV1::from_index(0),
            b"must_not_replace",
        );
    let diagnostic = graph_work_diagnostic_v1(&error);
    let function = diagnostic.function.as_ref().unwrap();
    assert_eq!(function.index, 0);
    assert_eq!(function.identity, functions[0].identity());
    assert_eq!(function.context, GraphWorkContextV1::DefinedCallableSummary);
    assert_eq!(diagnostic.root.as_ref().unwrap().body, SemanticFunctionIdV1::from_index(1));
    assert!(!error.to_string().contains("must_not_replace"));
}

#[test]
fn graph_work_diagnostic_bounded_paths_names_and_exports_are_escaped() {
    for raw in [vec![], vec![b'a'; 128], vec![0xff; 1024]] {
        let path = GraphWorkPathV1::new(&raw);
        assert_eq!(path.len, raw.len().min(128));
        assert_eq!(path.truncated, raw.len() > 128);
        assert_eq!(&path.bytes[..path.len], &raw[raw.len().saturating_sub(128)..]);
        assert!(path.to_string().len() <= 512);
    }
    let mut name = vec![0xff; 1024];
    name[3] = b'\n';
    let function = cfg_diagnostic_function_v1(2).with_kernel_entry(SemanticKernelEntryV1::new(
        SemanticLinkSymbolV1::new(name.clone()).unwrap(),
        SemanticKernelBindingIdentityV1::from_sha256(bytes(247)),
        SemanticKernelSourceContractV1::new(None, None, None).unwrap(),
    ));
    let mut error = graph_work_failure_v1()
        .with_graph_work_function_v1(0, &function, GraphWorkContextV1::RootProjection)
        .with_deterministic_root_context(
            SemanticFunctionIdV1::from_index(0),
            SemanticFunctionIdV1::from_index(0),
            &name,
        );
    let ProductionRankedProjectionErrorV1::GraphWorkLimit(diagnostic) = &mut error else {
        unreachable!();
    };
    diagnostic.path = GraphWorkPathV1::new(&vec![0xff; 1024]);
    let rendered = error.to_string();
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\u{fffd}'));
    assert!(rendered.contains("\\xff"));
    assert!(rendered.contains("\\n"));
    assert!(rendered.contains("path_truncated=true"));
    assert!(rendered.len() < 3072);
}

#[test]
fn graph_work_diagnostic_context_preserves_other_errors() {
    let functions = [scalar_summary_leaf(61)];
    for error in [
        ProductionRankedProjectionErrorV1::Unsupported("other refusal"),
        ProductionRankedProjectionErrorV1::Incomplete("other incomplete"),
    ] {
        let before = error.to_string();
        let error = error
            .with_graph_work_function_v1(0, &functions[0], GraphWorkContextV1::DefinedCallableSummary)
            .with_graph_work_body_v1(&functions, SemanticFunctionIdV1::from_index(0))
            .with_deterministic_root_context(
                SemanticFunctionIdV1::from_index(0),
                SemanticFunctionIdV1::from_index(0),
                b"not_attached",
            );
        assert_eq!(error.to_string(), before);
        assert!(std::error::Error::source(&error).is_none());
    }
}
