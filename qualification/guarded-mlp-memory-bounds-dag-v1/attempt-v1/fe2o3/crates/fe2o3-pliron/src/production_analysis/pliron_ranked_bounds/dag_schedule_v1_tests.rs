use super::*;
use crate::production_analysis::pliron_function_inventory::BoundedPlironFunctionInventoryV1;
use crate::production_analysis::pliron_ir_identity::LivePlironStructuralIdentityProviderV1;
use crate::production_analysis::pliron_pass_contract::PlironStructuralIdentityProviderV1;
use dialect_kernel::{DIALECT_NAME, register_dialect};
use pliron::{dialect::DialectName, parsable::parse_from_str};
use std::collections::BTreeSet;
use std::fmt::Write as _;

const DIRECT: &str = r#"
builtin.func @dag_bounds: builtin.function <() -> ()>
{
  ^entry():
    i = kernel.index_unknown () [] []: <() -> (kernel.index)>;
    n = kernel.index_unknown () [] []: <() -> (kernel.index)>;
    view = kernel.ranked_view (n) [] [kernel_memory_space: kernel.memory_space Global]: <(kernel.index) -> (kernel.ranked_view <32,true,[0]>)>;
    kernel.index_lt_br (i, n) [^access, ^exit] []: <(kernel.index, kernel.index) -> ()>
  ^access():
    kernel.access (view, i) [] [kernel_access_kind: kernel.access_kind AtomicRead]: <(kernel.ranked_view <32,true,[0]>, kernel.index) -> ()>;
    kernel.br () [^exit] []: <() -> ()>
  ^exit():
    kernel.return () [] []: <() -> ()>
}
"#;

fn parsed(source: &str) -> (Context, FuncOp) {
    let mut context = Context::new();
    fe2o3_pliron_owner_core::ensure_context_identity(&mut context).unwrap();
    register_dialect(&mut context, &DialectName::try_new(DIALECT_NAME).unwrap()).unwrap();
    dialect_gpu::register_dialect(&mut context).unwrap();
    let function = parse_in(&mut context, source);
    (context, function)
}

fn parse_in(context: &mut Context, source: &str) -> FuncOp {
    let operation = parse_from_str(Operation::top_level_parser(), context, source).unwrap();
    verify_operation(operation, context).unwrap();
    assert!(Operation::is_op::<FuncOp>(operation, context));
    FuncOp::from_operation(operation)
}

fn actual_census(context: &Context, function: &FuncOp) -> ProductionAnalysisInputCensusV1 {
    LivePlironStructuralIdentityProviderV1::new(context, function)
        .capture_with_resource_limits_v1(ProductionAnalysisResourceLimitsV1::new(
            usize::MAX,
            usize::MAX,
        ))
        .ok()
        .unwrap()
        .input_census
}

fn schedule(context: &Context, function: &FuncOp) -> RankedBoundsDagScheduleV1 {
    let inventory = BoundedPlironFunctionInventoryV1::collect(context, function).unwrap();
    collect_ranked_bounds_dag_v1(context, function, &inventory, actual_census(context, function))
        .expect("fixture is a whole-function DAG")
}

fn copy_schedule(dag: &RankedBoundsDagScheduleV1) -> RankedBoundsDagScheduleV1 {
    RankedBoundsDagScheduleV1 {
        function: dag.function,
        blocks: dag.blocks.clone(),
        successors: dag.successors.clone(),
        order: dag.order.clone(),
    }
}

fn scheduled_report(
    context: &Context,
    function: &FuncOp,
    dag: &RankedBoundsDagScheduleV1,
) -> RankedBoundsReportV1 {
    run_pliron_ranked_bounds_check_with_schedule_v1(
        context,
        function,
        &mut PlironAnalysisManagerV1::new(function),
        Some(dag),
    )
}

fn compare_reports(source: &str) -> RankedBoundsReportV1 {
    let (context, function) = parsed(source);
    let dag = schedule(&context, &function);
    let original = run_pliron_ranked_bounds_check_v1(&context, &function);
    let scheduled = scheduled_report(&context, &function, &dag);
    assert_eq!(scheduled, original, "{source}");
    scheduled
}

fn graph_source(successors: &[Vec<usize>], physical_order: &[usize]) -> String {
    assert_eq!(physical_order[0], 0);
    let mut source = String::from(
        "builtin.func @dag_graph: builtin.function <() -> ()>\n{\n",
    );
    for block in physical_order.iter().copied() {
        writeln!(source, "  ^b{block}():").unwrap();
        if block == 0 {
            source.push_str("    n = kernel.index_unknown () [] []: <() -> (kernel.index)>;\n");
            for index in 0..successors.len() {
                writeln!(source, "    i{index} = kernel.index_constant () [] [kernel_index_value: kernel.index_value {index}]: <() -> (kernel.index)>;").unwrap();
            }
        }
        match successors[block].as_slice() {
            [] => source.push_str("    kernel.return () [] []: <() -> ()>\n"),
            [target] => writeln!(source, "    kernel.br () [^b{target}] []: <() -> ()>").unwrap(),
            [left, right] => writeln!(source, "    kernel.index_lt_br (i{block}, n) [^b{left}, ^b{right}] []: <(kernel.index, kernel.index) -> ()>").unwrap(),
            _ => panic!("fixture supports at most two raw successors"),
        }
    }
    source.push_str("}\n");
    source
}

fn reachable(successors: &[Vec<usize>]) -> Vec<bool> {
    let mut reached = vec![false; successors.len()];
    reached[0] = true;
    let mut pending = vec![0];
    while let Some(block) = pending.pop() {
        for target in successors[block].iter().copied() {
            if !reached[target] {
                reached[target] = true;
                pending.push(target);
            }
        }
    }
    reached
}

// Independent set-based greatest-fixed-point oracle. It neither uses the
// production bitset intersection nor relies on a topological processing order.
fn reference_inputs(successors: &[Vec<usize>]) -> Vec<BTreeSet<usize>> {
    let count = successors.len();
    let top = (0..count).collect::<BTreeSet<_>>();
    let mut inputs = vec![top.clone(); count];
    inputs[0].clear();
    let reached = reachable(successors);
    for _ in 0..=count * (count + 1) {
        let mut changed = false;
        for block in (0..count).rev().filter(|block| reached[*block]) {
            let mut incoming = Vec::new();
            for (source, targets) in successors.iter().enumerate() {
                for (ordinal, target) in targets.iter().enumerate() {
                    if *target == block {
                        let mut edge = inputs[source].clone();
                        if targets.len() == 2 && ordinal == 0 {
                            edge.insert(source);
                        }
                        incoming.push(edge);
                    }
                }
            }
            let next = if block == 0 || incoming.is_empty() {
                BTreeSet::new()
            } else {
                incoming.into_iter().fold(top.clone(), |lhs, rhs| {
                    lhs.intersection(&rhs).copied().collect()
                })
            };
            changed |= next != inputs[block];
            inputs[block] = next;
        }
        if !changed {
            return inputs;
        }
    }
    panic!("finite descending reference lattice did not converge");
}

fn compare_fact_inputs(dag: &RankedBoundsDagScheduleV1) {
    let count = dag.blocks.len();
    let reached = reachable(&dag.successors);
    let mut predecessors = vec![Vec::new(); count];
    for (source, targets) in dag.successors.iter().enumerate() {
        for (ordinal, target) in targets.iter().copied().enumerate() {
            predecessors[target].push(PredecessorEdge {
                block: source,
                successor: ordinal,
                guard_fact: (targets.len() == 2 && ordinal == 0).then_some(source),
            });
        }
    }
    let mut inputs = vec![FactSet::full(count); count];
    inputs[0] = FactSet::empty(count);
    for block in dag.order.iter().copied().filter(|block| reached[*block]) {
        inputs[block] = if block == 0 {
            FactSet::empty(count)
        } else {
            intersect_predecessor_facts(
                block,
                &predecessors,
                &inputs,
                count,
                &mut RankedBoundsBudget::default(),
            )
            .unwrap()
        };
    }
    let actual = inputs
        .iter()
        .map(|facts| (0..count).filter(|fact| facts.contains(*fact)).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();
    assert_eq!(actual, reference_inputs(&dag.successors));
}

fn dense_guard_source(guards: usize, reverse: bool) -> String {
    let mut source = String::from(
        "builtin.func @dense_dag_bounds: builtin.function <() -> ()>\n{\n  ^entry():\n    n = kernel.index_unknown () [] []: <() -> (kernel.index)>;\n    view = kernel.ranked_view (n) [] [kernel_memory_space: kernel.memory_space Global]: <(kernel.index) -> (kernel.ranked_view <32,true,[0]>)>;\n",
    );
    for index in 0..guards {
        writeln!(source, "    i{index} = kernel.index_constant () [] [kernel_index_value: kernel.index_value {index}]: <() -> (kernel.index)>;").unwrap();
    }
    // Match the observed structural census, without claiming this constructed
    // fixture reproduces the candidate's semantic payload or source mapping.
    for index in 0..567 {
        writeln!(source, "    pad{index} = kernel.index_constant () [] [kernel_index_value: kernel.index_value {index}]: <() -> (kernel.index)>;").unwrap();
    }
    source.push_str("    kernel.br () [^p0] []: <() -> ()>\n");
    for prefix in 0..12 {
        writeln!(source, "  ^p{prefix}():").unwrap();
        if prefix < 5 {
            writeln!(source, "    kernel.index_eq_br (i0, i0) [^p{}, ^p{}] []: <(kernel.index, kernel.index) -> ()>", prefix + 1, prefix + 2).unwrap();
        } else if prefix == 11 {
            source.push_str("    kernel.br () [^g0] []: <() -> ()>\n");
        } else {
            writeln!(source, "    kernel.br () [^p{}] []: <() -> ()>", prefix + 1).unwrap();
        }
    }
    let mut order = (0..guards).collect::<Vec<_>>();
    if reverse {
        order.reverse();
    }
    for index in order {
        writeln!(source, "  ^g{index}():").unwrap();
        if index != 0 {
            let prior = index - 1;
            let kind = if prior < 548 { "AtomicRead" } else { "AtomicWrite" };
            writeln!(source, "    kernel.access (view, i{prior}) [] [kernel_access_kind: kernel.access_kind {kind}]: <(kernel.ranked_view <32,true,[0]>, kernel.index) -> ()>;").unwrap();
        }
        let next = if index + 1 == guards { "done".into() } else { format!("g{}", index + 1) };
        writeln!(source, "    kernel.index_lt_br (i{index}, n) [^{next}, ^trap] []: <(kernel.index, kernel.index) -> ()>").unwrap();
    }
    let last = guards - 1;
    let kind = if last < 548 { "AtomicRead" } else { "AtomicWrite" };
    writeln!(source, "  ^done():\n    kernel.access (view, i{last}) [] [kernel_access_kind: kernel.access_kind {kind}]: <(kernel.ranked_view <32,true,[0]>, kernel.index) -> ()>;\n    kernel.return () [] []: <() -> ()>\n  ^trap():\n    kernel.trap () [] []: <() -> ()>\n}}").unwrap();
    source
}

#[test]
fn dag_schedule_preserves_previously_admitted_path() {
    let (context, function) = parsed(DIRECT);
    let census = actual_census(&context, &function);
    let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
    let limits = ProductionAnalysisResourceLimitsV1::production_hard_ceiling();
    let old = preflight_ranked_bounds_resource_upper_bound_v1(census, limits).unwrap();
    let selected = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, limits).unwrap();
    assert_eq!(selected.upper_bound(), old);
    assert!(selected.dag_schedule().is_none());
    let ownership = ProductionAnalysisInputCensusV1 { ownership_contracts: 1, ..census };
    let old = preflight_ranked_bounds_resource_upper_bound_v1(ownership, limits).unwrap();
    let selected = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, ownership, limits).unwrap();
    assert_eq!(selected.upper_bound(), old);
    assert!(selected.dag_schedule().is_none());
    assert!(compare_reports(DIRECT).is_clean());
}

#[test]
fn dag_schedule_differential_dags_and_physical_orders() {
    let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let orders = [[0, 1, 2, 3], [0, 1, 3, 2], [0, 2, 1, 3], [0, 2, 3, 1], [0, 3, 1, 2], [0, 3, 2, 1]];
    let mut cases = 0;
    for mask in 0..1_usize << edges.len() {
        let mut successors = vec![Vec::new(); 4];
        for (bit, (source, target)) in edges.iter().copied().enumerate() {
            if mask & (1 << bit) != 0 {
                successors[source].push(target);
            }
        }
        if successors.iter().any(|targets| targets.len() > 2) || reachable(&successors).contains(&false) {
            continue;
        }
        for order in orders {
            let source = graph_source(&successors, &order);
            let (context, function) = parsed(&source);
            let dag = schedule(&context, &function);
            compare_fact_inputs(&dag);
            assert_eq!(scheduled_report(&context, &function, &dag), run_pliron_ranked_bounds_check_v1(&context, &function));
            cases += 1;
        }
    }
    assert!(cases >= 60, "enumeration must include more than one graph family");
}

#[test]
fn dag_schedule_duplicate_edges_and_entry() {
    let source = graph_source(&[vec![1, 1], vec![2], vec![]], &[0, 2, 1]);
    let (context, function) = parsed(&source);
    let dag = schedule(&context, &function);
    assert_eq!(dag.successors[0], [2, 2]);
    compare_fact_inputs(&dag);
    let reference = reference_inputs(&dag.successors);
    assert!(reference[0].is_empty());
    assert!(reference[2].is_empty(), "true fact must not survive the parallel false edge");
    assert_eq!(scheduled_report(&context, &function, &dag), run_pliron_ranked_bounds_check_v1(&context, &function));

    let unreachable = DIRECT.replace("\n}\n", "\n  ^orphan():\n    kernel.br () [^access] []: <() -> ()>\n}\n");
    let report = compare_reports(&unreachable);
    assert!(report.findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::UnreachableBlock { .. })));
    assert!(!report.findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::UnprovedBound { .. })));
}

#[test]
fn dag_schedule_cycles_and_unreachable_fallback() {
    for source in [
        graph_source(&[vec![1], vec![1, 2], vec![]], &[0, 1, 2]),
        graph_source(&[vec![1], vec![0, 2], vec![]], &[0, 1, 2]),
        DIRECT.replace("\n}\n", "\n  ^orphan():\n    kernel.br () [^orphan] []: <() -> ()>\n}\n"),
    ] {
        let (context, function) = parsed(&source);
        let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
        let census = actual_census(&context, &function);
        assert!(collect_ranked_bounds_dag_v1(&context, &function, &inventory, census).is_none());
        let limits = ProductionAnalysisResourceLimitsV1::production_hard_ceiling();
        let old = preflight_ranked_bounds_resource_upper_bound_v1(census, limits).unwrap();
        let selected = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, limits).unwrap();
        assert!(selected.dag_schedule().is_none());
        assert_eq!(selected.upper_bound(), old);
    }
    let source = dense_guard_source(552, false).replace("kernel.return () [] []: <() -> ()>", "kernel.br () [^g0] []: <() -> ()>");
    let (context, function) = parsed(&source);
    let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
    let census = actual_census(&context, &function);
    let limits = ProductionAnalysisResourceLimitsV1::production_hard_ceiling();
    let old = preflight_ranked_bounds_resource_upper_bound_v1(census, limits).unwrap_err();
    assert_eq!(old.resource, "memory-bounds work hard limit");
    assert_eq!(preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, limits).err(), Some(old));
}

#[test]
fn dag_schedule_forwarded_arguments_preserve_transport() {
    for source in [
        include_str!("../tests/bounds-transport-lit/direct.pliron"),
        include_str!("../tests/bounds-transport-lit/diamond.pliron"),
        include_str!("../tests/bounds-transport-lit/parallel.pliron"),
        include_str!("../tests/bounds-transport-lit/split.pliron"),
        include_str!("../tests/bounds-transport-lit/view-forwarding.pliron"),
        include_str!("../tests/bounds-transport-lit/view-asymmetric.pliron"),
    ] {
        let source = source.lines().filter(|line| !line.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        compare_reports(&source);
    }
}

#[test]
fn dag_schedule_certificate_mismatch_fails_closed() {
    let (mut context, function) = parsed(DIRECT);
    let dag = schedule(&context, &function);
    let foreign = parse_in(&mut context, &DIRECT.replace("@dag_bounds", "@foreign_bounds"));
    for mutation in 0..8 {
        let mut changed = copy_schedule(&dag);
        match mutation {
            0 => changed.function = foreign.get_operation(),
            1 => changed.blocks.swap(0, 1),
            2 => changed.successors[0].reverse(),
            3 => changed.order[1] = changed.order[0],
            4 => changed.order[1] = usize::MAX,
            5 => { changed.order.pop(); }
            6 => changed.order.reverse(),
            7 => changed.successors[0][0] = usize::MAX,
            _ => unreachable!(),
        }
        let report = scheduled_report(&context, &function, &changed);
        assert!(report.findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::StructuralVerificationFailed)), "mutation {mutation}: {report:?}");
    }
    assert!(scheduled_report(&context, &foreign, &dag).findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::StructuralVerificationFailed)));
    let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
    let original = actual_census(&context, &function);
    for mutation in 0..3 {
        let mut changed = original;
        match mutation {
            0 => changed.blocks += 1,
            1 => changed.operations += 1,
            2 => changed.successors += 1,
            _ => unreachable!(),
        }
        assert!(collect_ranked_bounds_dag_v1(&context, &function, &inventory, changed).is_none());
    }
    assert!(collect_ranked_bounds_dag_v1(&context, &foreign, &inventory, original).is_none());
}

#[test]
fn dag_schedule_work_storage_boundaries() {
    assert!(reserved_ranked_bounds_vec_v1::<usize>(usize::MAX).is_none());
    let (context, function) = parsed(DIRECT);
    let dag = schedule(&context, &function);
    let mut measured = RankedBoundsBudget::default();
    dag.validate_runtime(&context, &function, &dag.blocks, &dag.successors, &mut measured).unwrap();
    assert_eq!(measured.work_units, 8 * 3 + 4 * 3);
    assert_eq!(measured.storage_items, 4 * 3 + 3);
    for (work_short, storage_short, success) in [(0, 0, true), (1, 0, false), (0, 1, false)] {
        let mut budget = RankedBoundsBudget {
            work_units: MAX_RANKED_BOUNDS_WORK_UNITS - measured.work_units + work_short,
            storage_items: MAX_RANKED_BOUNDS_STORAGE_ITEMS - measured.storage_items + storage_short,
            ..RankedBoundsBudget::default()
        };
        let result = dag.validate_runtime(&context, &function, &dag.blocks, &dag.successors, &mut budget);
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(budget.work_units, MAX_RANKED_BOUNDS_WORK_UNITS);
            assert_eq!(budget.storage_items, MAX_RANKED_BOUNDS_STORAGE_ITEMS);
        } else {
            assert!(matches!(result, Err(RankedBoundsFindingV1::ResourceLimitExceeded { .. })));
        }
    }
    for mutation in 0..3 {
        let mut census = ProductionAnalysisInputCensusV1::default();
        match mutation {
            0 => census.blocks = usize::MAX,
            1 => census.operations = usize::MAX,
            2 => census.successors = usize::MAX,
            _ => unreachable!(),
        }
        assert!(ranked_bounds_dag_resources_v1(census).is_err());
    }
    let mut exhausted = RankedBoundsBudget { work_units: usize::MAX, ..RankedBoundsBudget::default() };
    assert!(dag.validate_runtime(&context, &function, &dag.blocks, &dag.successors, &mut exhausted).is_err());
    assert_eq!(exhausted.work_units, usize::MAX);
}

#[test]
fn dag_schedule_preserves_bounds_refusals() {
    for source in [
        DIRECT.replace("[^access, ^exit]", "[^exit, ^access]"),
        DIRECT.replace("[^access, ^exit]", "[^access, ^access]"),
        DIRECT.replace("kernel.index_lt_br (i, n) [^access, ^exit] []: <(kernel.index, kernel.index) -> ()>", "kernel.br () [^access] []: <() -> ()>"),
        DIRECT.replace("(view, i)", "(view, n)"),
    ] {
        assert_ne!(source, DIRECT);
        let report = compare_reports(&source);
        assert!(!report.is_clean());
        assert!(report.findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::UnprovedBound { .. })));
    }
}

#[test]
fn dag_schedule_dense_guard_production_route() {
    for reverse in [false, true] {
        let source = dense_guard_source(552, reverse);
        let (context, function) = parsed(&source);
        let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
        let census = actual_census(&context, &function);
        assert_eq!((census.blocks, census.operations, census.successors, census.memory_bounds_guard_candidates), (567, 2240, 1122, 552));
        let limits = ProductionAnalysisResourceLimitsV1::production_hard_ceiling();
        assert_eq!(preflight_ranked_bounds_resource_upper_bound_v1(census, limits).unwrap_err().resource, "memory-bounds work hard limit");
        let ownership = ProductionAnalysisInputCensusV1 { ownership_contracts: 1, ..census };
        assert_eq!(preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, ownership, limits).err().unwrap().resource, "memory-bounds work hard limit");
        let selected = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, limits).unwrap();
        assert!(selected.dag_schedule().is_some());
        let bound = selected.upper_bound();
        for (work_short, storage_short, success) in [(0, 0, true), (1, 0, false), (0, 1, false)] {
            let result = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, ProductionAnalysisResourceLimitsV1::new(bound.work_upper_bound() - work_short, bound.peak_storage_upper_bound() - storage_short));
            assert_eq!(result.is_ok(), success);
        }
        let report = require_pliron_ranked_bounds_with_schedule_v1(&context, &function, &mut PlironAnalysisManagerV1::new(&function), selected.dag_schedule()).unwrap();
        assert!(report.is_clean());
        assert!(!report.grants_compiler_refinement_authority());
        assert!(!report.grants_artifact_or_launch_authority());
        assert_eq!(MAX_RANKED_BOUNDS_WORK_UNITS, 8_388_608);
        assert_eq!(MAX_RANKED_BOUNDS_STORAGE_ITEMS, 131_072);
        assert_eq!(MAX_RANKED_BOUNDS_EDGES, 2048);
        assert_eq!(MAX_RANKED_BOUNDS_FACTS, 1024);

        let denied = source.replace("(i0, n) [^g1, ^trap]", "(i0, n) [^trap, ^g1]");
        assert_ne!(denied, source);
        let (context, function) = parsed(&denied);
        let inventory = BoundedPlironFunctionInventoryV1::collect(&context, &function).unwrap();
        let census = actual_census(&context, &function);
        let selected = preflight_ranked_bounds_with_dag_v1(&context, &function, &inventory, census, limits).unwrap();
        let error = require_pliron_ranked_bounds_with_schedule_v1(&context, &function, &mut PlironAnalysisManagerV1::new(&function), selected.dag_schedule()).unwrap_err();
        assert!(error.report().findings().iter().any(|finding| matches!(finding, RankedBoundsFindingV1::UnprovedBound { .. })));
    }
}
