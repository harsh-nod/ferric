//! Source-schedule contracts, not a substitute for native MFMA numerical tests.

use ferric_qwen3_tp_perf_kernels_device_v3::compiler_expectation_roster_v3;
use syn::{
    BinOp, Block, Expr, ExprMethodCall, ExprWhile, FnArg, Item, ItemFn, Lit, Pat, Stmt, Type,
};

const SOURCE: &str = include_str!("../src/projection.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const ROOTS: [&str; 2] = [
    "ferric_qwen3_tp_mfma_gemm_bf16_v3",
    "ferric_qwen3_tp_mfma_gemm_partial_f32_v3",
];

fn roots() -> Vec<ItemFn> {
    syn::parse_file(SOURCE)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function) if ROOTS.iter().any(|name| function.sig.ident == *name) => {
                Some(function)
            }
            _ => None,
        })
        .collect()
}

fn integer(expression: &Expr, step: usize) -> usize {
    match expression {
        Expr::Lit(literal) => match &literal.lit {
            Lit::Int(value) => value.base10_parse().unwrap(),
            _ => panic!("noninteger literal"),
        },
        Expr::Path(path) if path.path.is_ident("step") => step,
        Expr::Binary(binary) => match binary.op {
            BinOp::Add(_) => integer(&binary.left, step) + integer(&binary.right, step),
            BinOp::Mul(_) => integer(&binary.left, step) * integer(&binary.right, step),
            _ => panic!("unexpected index operator"),
        },
        _ => panic!("unexpected index expression"),
    }
}

fn name(expression: &Expr) -> String {
    let Expr::Path(path) = expression else {
        panic!("expected a named value")
    };
    path.path.get_ident().unwrap().to_string()
}

#[derive(Default)]
struct Loops<'a>(Vec<&'a ExprWhile>);

impl<'ast> syn::visit::Visit<'ast> for Loops<'ast> {
    fn visit_expr_while(&mut self, expression: &'ast ExprWhile) {
        self.0.push(expression);
        syn::visit::visit_expr_while(self, expression);
    }
}

fn paired(loop_body: &ExprWhile) -> bool {
    matches!(&loop_body.body.stmts[0], Stmt::Local(local)
        if matches!(&local.pat, Pat::Ident(binding) if binding.ident == "a_first"))
}

fn bound(loop_body: &ExprWhile) -> usize {
    let Expr::Binary(condition) = loop_body.cond.as_ref() else {
        panic!("expected a static loop condition")
    };
    assert!(matches!(condition.op, BinOp::Lt(_)));
    assert_eq!(name(&condition.left), "step");
    integer(&condition.right, 0)
}

fn load(statement: &Stmt) -> (String, &ExprMethodCall) {
    let Stmt::Local(local) = statement else {
        panic!("expected a fragment binding")
    };
    let Pat::Ident(binding) = &local.pat else {
        panic!("expected a named fragment")
    };
    let Expr::MethodCall(call) = local.init.as_ref().unwrap().expr.as_ref() else {
        panic!("expected a typed fragment load")
    };
    (binding.ident.to_string(), call)
}

fn accumulator_call(statement: &Stmt) -> &ExprMethodCall {
    let Stmt::Expr(Expr::Assign(assignment), _) = statement else {
        panic!("expected an accumulator assignment")
    };
    assert_eq!(name(&assignment.left), "accumulator");
    let Expr::MethodCall(call) = assignment.right.as_ref() else {
        panic!("expected MFMA")
    };
    assert_eq!(name(&call.receiver), "matrix");
    assert_eq!(call.method, "multiply_accumulate");
    assert_eq!(name(&call.args[2]), "accumulator");
    call
}

fn assert_step_increment(statement: &Stmt) {
    let Stmt::Expr(Expr::Binary(increment), _) = statement else {
        panic!("expected a unit step increment")
    };
    assert!(matches!(increment.op, BinOp::AddAssign(_)));
    assert_eq!(name(&increment.left), "step");
    assert_eq!(integer(&increment.right, 0), 1);
}

#[derive(Clone, Copy)]
struct Schedule {
    enabled: bool,
    rows: usize,
    world_size: usize,
    k: usize,
}

fn condition(expression: &Expr, schedule: Schedule) -> bool {
    match expression {
        Expr::Path(path) if path.path.is_ident("MFMA_PAIRED_PREFETCH") => schedule.enabled,
        Expr::Binary(binary) => match binary.op {
            BinOp::And(_) => {
                condition(&binary.left, schedule) && condition(&binary.right, schedule)
            }
            BinOp::Eq(_) => {
                let value = |expression: &Expr| match expression {
                    Expr::Path(path) if path.path.is_ident("rows") => schedule.rows,
                    Expr::Path(path) if path.path.is_ident("world_size") => schedule.world_size,
                    Expr::Path(path) if path.path.is_ident("k") => schedule.k,
                    _ => integer(expression, 0),
                };
                value(&binary.left) == value(&binary.right)
            }
            _ => panic!("unexpected schedule condition"),
        },
        _ => panic!("unexpected schedule expression"),
    }
}

fn selected_loop(expression: &Expr, schedule: Schedule) -> &ExprWhile {
    match expression {
        Expr::While(body) => body,
        Expr::Block(block) => selected_block_loop(&block.block, schedule),
        Expr::If(branch) => {
            if condition(&branch.cond, schedule) {
                selected_block_loop(&branch.then_branch, schedule)
            } else {
                selected_loop(&branch.else_branch.as_ref().unwrap().1, schedule)
            }
        }
        _ => panic!("expected a schedule loop or branch"),
    }
}

fn selected_block_loop(block: &Block, schedule: Schedule) -> &ExprWhile {
    block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            Stmt::Expr(expression, _) => Some(selected_loop(expression, schedule)),
            _ => None,
        })
        .unwrap()
}

fn schedule_root(function: &ItemFn) -> &Expr {
    let start = function
        .block
        .stmts
        .iter()
        .position(|statement| {
            matches!(statement, Stmt::Local(local)
                if matches!(&local.pat, Pat::Ident(binding) if binding.ident == "accumulator"))
        })
        .unwrap();
    let Stmt::Expr(expression, _) = &function.block.stmts[start + 1] else {
        panic!("expected schedule immediately after the single accumulator")
    };
    expression
}

#[test]
fn paired_prefetch_is_default_off_and_implies_mfma() {
    let defaults = MANIFEST
        .lines()
        .find(|line| line.starts_with("default = "))
        .unwrap();
    assert!(!defaults.contains("mfma-paired-prefetch"));
    assert!(MANIFEST.contains("mfma-paired-prefetch = [\"mfma\"]"));
}

#[test]
fn both_pairs_are_loaded_before_two_in_order_accumulations() {
    use syn::visit::Visit;
    let functions = roots();
    let mut observed = Vec::new();
    for function in &functions {
        let mut loops = Loops::default();
        loops.visit_item_fn(function);
        for body in loops.0.into_iter().filter(|body| paired(body)) {
            let statements = &body.body.stmts;
            assert_eq!(statements.len(), 7);
            for (index, expected) in ["a_first", "b_first", "a_second", "b_second"]
                .into_iter()
                .enumerate()
            {
                let (binding, call) = load(&statements[index]);
                assert_eq!(binding, expected);
                assert_eq!(
                    call.method,
                    if index % 2 == 0 {
                        "load_m16k16"
                    } else {
                        "load_k16n16"
                    }
                );
                assert_eq!(
                    name(&call.receiver),
                    if index % 2 == 0 { "left" } else { "right" }
                );
            }
            for (index, expected) in [(4, ["a_first", "b_first"]), (5, ["a_second", "b_second"])] {
                let call = accumulator_call(&statements[index]);
                assert_eq!(name(&call.args[0]), expected[0]);
                assert_eq!(name(&call.args[1]), expected[1]);
            }
            assert_step_increment(&statements[6]);
            observed.push((function.sig.ident.to_string(), bound(body)));
        }
    }
    assert_eq!(
        observed,
        [
            (ROOTS[0].into(), 128),
            (ROOTS[1].into(), 128),
            (ROOTS[1].into(), 384)
        ]
    );
}

#[test]
fn source_derived_pair_offsets_cover_every_k16_tile_exactly_once() {
    use syn::visit::Visit;
    for function in &roots() {
        let mut loops = Loops::default();
        loops.visit_item_fn(function);
        for body in loops.0.into_iter().filter(|body| paired(body)) {
            let reduction = bound(body) * 32;
            let mut consumed = Vec::new();
            for step in 0..bound(body) {
                for (left_index, right_index) in [(0, 1), (2, 3)] {
                    let (_, left) = load(&body.body.stmts[left_index]);
                    let (_, right) = load(&body.body.stmts[right_index]);
                    let left_offset = integer(&left.args[2], step);
                    let right_offset = integer(&right.args[1], step);
                    assert_eq!(integer(&left.args[1], step), 0);
                    assert_eq!(left_offset, right_offset);
                    assert_eq!(left_offset % 16, 0);
                    assert!(left_offset + 16 <= reduction);
                    consumed.push(left_offset);
                }
            }
            assert_eq!(consumed, (0..reduction).step_by(16).collect::<Vec<_>>());
        }
    }
}

#[test]
fn feature_selects_paired_tp1_for_every_active_row_and_preserves_tp_fallbacks() {
    for function in roots() {
        for enabled in [false, true] {
            for rows in 1..=16 {
                for world_size in [1, 2, 8] {
                    let widths = if function.sig.ident == ROOTS[0] {
                        vec![4096]
                    } else {
                        vec![4096 / world_size, 12288 / world_size]
                    };
                    for k in widths {
                        let schedule = Schedule {
                            enabled,
                            rows,
                            world_size,
                            k,
                        };
                        let body = selected_loop(schedule_root(&function), schedule);
                        let expected_pair = enabled && world_size == 1;
                        assert_eq!(paired(body), expected_pair);
                        assert_eq!(bound(body), k / if expected_pair { 32 } else { 16 });
                    }
                }
            }
        }
    }
}

#[test]
fn paired_addresses_cover_active_rows_and_zero_fill_inactive_rows() {
    use syn::visit::Visit;
    for function in &roots() {
        let mut loops = Loops::default();
        loops.visit_item_fn(function);
        for body in loops.0.into_iter().filter(|body| paired(body)) {
            let reduction = bound(body) * 32;
            for rows in 1..=16 {
                let mut activation_counts = vec![0_u8; rows * reduction];
                let mut weight_counts = vec![0_u8; reduction * 16];
                let mut zero_filled = 0;
                for step in 0..bound(body) {
                    for (left_index, right_index) in [(0, 1), (2, 3)] {
                        let (_, left) = load(&body.body.stmts[left_index]);
                        let (_, right) = load(&body.body.stmts[right_index]);
                        let row_base = integer(&left.args[1], step);
                        let left_base = integer(&left.args[2], step);
                        let right_base = integer(&right.args[1], step);
                        for lane in 0..64 {
                            for component in 0..4 {
                                // The pinned typed MFMA loads map four K values per lane.
                                let row = row_base + lane % 16;
                                let inner = left_base + lane / 16 * 4 + component;
                                assert!(inner < reduction);
                                if row < rows {
                                    activation_counts[row * reduction + inner] += 1;
                                } else {
                                    zero_filled += 1;
                                }
                                let inner = right_base + lane / 16 * 4 + component;
                                assert!(inner < reduction);
                                weight_counts[inner * 16 + lane % 16] += 1;
                            }
                        }
                    }
                }
                assert!(activation_counts.iter().all(|count| *count == 1));
                assert!(weight_counts.iter().all(|count| *count == 1));
                assert_eq!(zero_filled, (16 - rows) * reduction);
            }
        }
    }
}

#[test]
fn all_seven_original_scalar_step_loops_remain_as_fallbacks() {
    use syn::visit::Visit;
    let mut observed = Vec::new();
    for function in &roots() {
        let mut loops = Loops::default();
        loops.visit_item_fn(function);
        for body in loops.0.into_iter().filter(|body| !paired(body)) {
            assert_eq!(body.body.stmts.len(), 4);
            let (left_name, left) = load(&body.body.stmts[0]);
            let (right_name, right) = load(&body.body.stmts[1]);
            assert_eq!(left_name, "a_fragment");
            assert_eq!(right_name, "b_fragment");
            let consume = accumulator_call(&body.body.stmts[2]);
            assert_eq!(name(&consume.args[0]), left_name);
            assert_eq!(name(&consume.args[1]), right_name);
            for step in 0..bound(body) {
                assert_eq!(integer(&left.args[2], step), step * 16);
                assert_eq!(integer(&right.args[1], step), step * 16);
            }
            assert_step_increment(&body.body.stmts[3]);
            observed.push(bound(body));
        }
    }
    assert_eq!(observed, [256, 32, 96, 128, 256, 384, 768]);
}

#[test]
fn declared_proof_bounds_match_every_source_loop_in_order() {
    use syn::visit::Visit;
    for function in &roots() {
        let mut loops = Loops::default();
        loops.visit_item_fn(function);
        let expected: Vec<_> = loops.0.into_iter().map(bound).collect();
        let kernel = function
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("kernel"))
            .unwrap();
        let syn::Meta::List(metadata) = &kernel.meta else {
            panic!("kernel arguments")
        };
        let compact: String = metadata.tokens.to_string().split_whitespace().collect();
        let joined = expected
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        assert!(compact.contains(&format!("control_flow(loop_bounds({joined}))")));
    }
}

#[test]
fn feature_keeps_the_exact_closed_roster_and_projection_abi() {
    let roster = compiler_expectation_roster_v3();
    assert_eq!(roster.len(), if cfg!(feature = "mfma") { 15 } else { 13 });
    for root in ROOTS {
        let matches: Vec<_> = roster
            .iter()
            .filter(|entry| entry.export_name() == root)
            .collect();
        assert_eq!(matches.len(), usize::from(cfg!(feature = "mfma")));
        if let Some(entry) = matches.first() {
            println!(
                "{root} host_contract={:02x?}",
                entry.generated_host_contract_identity()
            );
        }
    }
    for function in roots() {
        assert_eq!(function.sig.inputs.len(), 8);
        let widths: Vec<_> = function
            .sig
            .inputs
            .iter()
            .map(|argument| {
                let FnArg::Typed(argument) = argument else {
                    panic!("receiver")
                };
                match argument.ty.as_ref() {
                    Type::Reference(reference) => {
                        assert!(reference.mutability.is_none());
                        assert!(matches!(reference.elem.as_ref(), Type::Slice(_)));
                        16
                    }
                    Type::Path(path)
                        if path.path.segments.last().unwrap().ident == "WriteOnlyDisjointSlice" =>
                    {
                        16
                    }
                    Type::Path(path) if path.path.is_ident("u32") => 4,
                    _ => panic!("unexpected ABI type"),
                }
            })
            .collect();
        assert_eq!(widths, [16, 16, 16, 4, 4, 4, 4, 4]);
        assert_eq!(widths.iter().sum::<usize>(), 68);
        let kernel = function
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("kernel"))
            .unwrap();
        let syn::Meta::List(metadata) = &kernel.meta else {
            panic!("kernel arguments")
        };
        let tokens = metadata.tokens.to_string();
        assert!(tokens.contains("required = [64 , 1 , 1]"));
        assert!(tokens.contains("max = [64 , 1 , 1]"));
        let expected_grid = if function.sig.ident == ROOTS[0] {
            9496
        } else {
            256
        };
        assert!(tokens.contains(&format!("max_grid = [{expected_grid} , 1 , 1]")));
    }
    assert_eq!(
        SOURCE
            .matches("F32AccumulatorFragment::zero(&lane)")
            .count(),
        2
    );
    let compact: String = SOURCE.split_whitespace().collect();
    assert!(
        compact.contains("constMFMA_PAIRED_PREFETCH:bool=cfg!(feature=\"mfma-paired-prefetch\");")
    );
    assert_eq!(compact.matches("ifMFMA_PAIRED_PREFETCH{").count(), 2);
    assert_eq!(
        compact
            .matches("ifMFMA_PAIRED_PREFETCH&&world_size==1{")
            .count(),
        1
    );
}
