use ferric_qwen3_tp_fp32_sharded_argmax_kernels_device_v13::{
    MAX_ROWS_V13, ROOTS_V13, SCRATCH_BYTES_V13, SHARDS_V13, VOCABULARY_V13,
    compiler_expectation_roster_v13,
};
use syn::{FnArg, GenericArgument, Item, PathArguments, Type};

const LOGITS: &str = include_str!("../src/logits.rs");

fn normalize(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

// Only the small expression vocabulary used by the sentinel and stores is admitted.
fn same_expression(actual: &syn::Expr, expected: &syn::Expr) -> bool {
    match (actual, expected) {
        (syn::Expr::Path(a), syn::Expr::Path(b)) => {
            a.qself.is_none() && b.qself.is_none() && a.path.get_ident() == b.path.get_ident()
                && a.path.get_ident().is_some()
        }
        (syn::Expr::Lit(a), syn::Expr::Lit(b)) => match (&a.lit, &b.lit) {
            (syn::Lit::Int(a), syn::Lit::Int(b)) => {
                a.base10_digits() == b.base10_digits() && a.suffix() == b.suffix()
            }
            (syn::Lit::Float(a), syn::Lit::Float(b)) => {
                a.base10_digits() == b.base10_digits() && a.suffix() == b.suffix()
            }
            _ => false,
        },
        (syn::Expr::Reference(a), syn::Expr::Reference(b)) => {
            a.mutability.is_none() && b.mutability.is_none()
                && same_expression(&a.expr, &b.expr)
        }
        (syn::Expr::Binary(a), syn::Expr::Binary(b)) => {
            core::mem::discriminant(&a.op) == core::mem::discriminant(&b.op)
                && same_expression(&a.left, &b.left)
                && same_expression(&a.right, &b.right)
        }
        (syn::Expr::If(a), syn::Expr::If(b)) => {
            same_expression(&a.cond, &b.cond)
                && same_tail(&a.then_branch, &b.then_branch)
                && match (&a.else_branch, &b.else_branch) {
                    (Some((_, a)), Some((_, b))) => same_expression(a, b),
                    _ => false,
                }
        }
        (syn::Expr::Block(a), syn::Expr::Block(b)) => {
            a.label.is_none() && b.label.is_none() && same_tail(&a.block, &b.block)
        }
        _ => false,
    }
}

fn same_tail(actual: &syn::Block, expected: &syn::Block) -> bool {
    match (actual.stmts.as_slice(), expected.stmts.as_slice()) {
        ([syn::Stmt::Expr(a, None)], [syn::Stmt::Expr(b, None)]) => same_expression(a, b),
        _ => false,
    }
}

fn expression_is(actual: &syn::Expr, expected: &str) -> bool {
    same_expression(actual, &syn::parse_str::<syn::Expr>(expected).unwrap())
}

fn lane_zero_block(function: &syn::ItemFn) -> Result<&syn::Block, &'static str> {
    let mut matches = function.block.stmts.iter().filter_map(|statement| match statement {
        syn::Stmt::Expr(syn::Expr::If(branch), _)
            if expression_is(&branch.cond, "lane == 0") && branch.else_branch.is_none() =>
        {
            Some(&branch.then_branch)
        }
        _ => None,
    });
    let block = matches.next().ok_or("missing lane-zero block")?;
    if matches.next().is_some() {
        return Err("duplicate lane-zero block");
    }
    Ok(block)
}

fn validate_write_bindings(source: &str) -> Result<(), &'static str> {
    let file = syn::parse_file(source).map_err(|_| "invalid Rust source")?;
    for (index, name) in ROOTS_V13.iter().enumerate() {
        let mut functions = file.items.iter().filter_map(|item| match item {
            Item::Fn(function) if function.sig.ident == *name => Some(function),
            _ => None,
        });
        let function = functions.next().ok_or("missing root")?;
        if functions.next().is_some() {
            return Err("duplicate root");
        }
        if index == 0 {
            let mut bindings = function.block.stmts.iter().filter_map(|statement| {
                let syn::Stmt::Local(local) = statement else { return None; };
                let syn::Pat::Ident(pattern) = &local.pat else { return None; };
                (pattern.ident == "shard_key").then_some(local)
            });
            let binding = bindings.next().ok_or("missing sentinel binding")?;
            let value = binding.init.as_ref().ok_or("missing sentinel value")?;
            if bindings.next().is_some() || value.diverge.is_some()
                || !expression_is(&value.expr,
                    "if any_invalid == 0.0 { winning_key } else { 0.0 }")
            {
                return Err("wrong producer sentinel expression");
            }
        }
        let block = lane_zero_block(function)?;
        let Some(syn::Stmt::Local(witness)) = block.stmts.first() else {
            return Err("missing writer witness");
        };
        let syn::Pat::TupleStruct(pattern) = &witness.pat else {
            return Err("wrong writer witness pattern");
        };
        if !pattern.path.is_ident("Some") || pattern.elems.len() != 1
            || !matches!(&pattern.elems[0], syn::Pat::Ident(p) if p.ident == "stripe"
                && p.by_ref.is_none() && p.mutability.is_none() && p.subpat.is_none())
        {
            return Err("wrong writer witness binding");
        }
        let initializer = witness.init.as_ref().ok_or("missing witness initializer")?;
        let syn::Expr::MethodCall(call) = initializer.expr.as_ref() else {
            return Err("wrong witness initializer");
        };
        let arguments = &call.turbofish.as_ref().ok_or("missing witness geometry")?.args;
        if !expression_is(&call.receiver, "invocation")
            || call.method != "checked_row_striped_2d" || !call.args.is_empty()
            || initializer.diverge.is_none() || arguments.len() != 2
        {
            return Err("wrong writer witness");
        }
        for (argument, expected) in arguments.iter().zip(["64", "1"]) {
            if !matches!(argument, GenericArgument::Const(value) if expression_is(value, expected)) {
                return Err("wrong witness geometry");
            }
        }
        let writes: Vec<_> = block.stmts.iter().filter_map(|statement| {
            let syn::Stmt::Expr(syn::Expr::If(branch), _) = statement else { return None; };
            let syn::Expr::Unary(negative) = branch.cond.as_ref() else { return None; };
            let syn::Expr::MethodCall(call) = negative.expr.as_ref() else { return None; };
            (matches!(negative.op, syn::UnOp::Not(_))
                && call.method == "write_row_striped_2d").then_some(call)
        }).collect();
        let expected = if index == 0 {
            vec![("maxima", ["&stripe", "0", "rows * 64", "1", "1", "maximum"]),
                ("keys", ["&stripe", "0", "rows * 64", "1", "1", "shard_key"])]
        } else {
            vec![("choices", ["&stripe", "0", "rows", "1", "1", "winner"])]
        };
        if writes.len() != expected.len() {
            return Err("wrong write count");
        }
        for (write, (receiver, arguments)) in writes.into_iter().zip(expected) {
            if !expression_is(&write.receiver, receiver) || write.turbofish.is_some()
                || write.args.len() != arguments.len()
                || !write.args.iter().zip(arguments).all(|(value, expected)| expression_is(value, expected))
            {
                return Err("wrong write receiver or operand");
            }
        }
    }
    Ok(())
}

#[test]
fn closed_two_root_roster_has_exact_explicit_argument_types() {
    let roster = compiler_expectation_roster_v13();
    assert_eq!(roster.len(), 2);
    for (entry, root) in roster.iter().zip(ROOTS_V13) {
        assert_eq!(entry.export_name(), root);
    }
    let source = syn::parse_file(LOGITS).unwrap();
    let functions: Vec<_> = source
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.attrs.iter().any(|a| a.path().is_ident("kernel")) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(functions.len(), 2);
    for (index, function) in functions.iter().enumerate() {
        assert_eq!(function.sig.ident, ROOTS_V13[index]);
        assert!(function.sig.unsafety.is_none());
        let mut kinds = Vec::new();
        let mut widths = Vec::new();
        for arg in &function.sig.inputs {
            let FnArg::Typed(arg) = arg else {
                panic!("receiver")
            };
            match arg.ty.as_ref() {
                Type::Reference(reference) => {
                    assert!(reference.mutability.is_none());
                    let Type::Slice(slice) = reference.elem.as_ref() else {
                        panic!("not a slice")
                    };
                    assert!(matches!(slice.elem.as_ref(), Type::Path(p) if p.path.is_ident("f32")));
                    kinds.push("read_f32");
                    widths.push(16);
                }
                Type::Path(path)
                    if path.path.segments.last().unwrap().ident == "WriteOnlyDisjointSlice" =>
                {
                    let segment = path.path.segments.last().unwrap();
                    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                        panic!("missing disjoint arguments")
                    };
                    assert_eq!(arguments.args.len(), 2);
                    let GenericArgument::Type(Type::Path(element)) = &arguments.args[0] else {
                        panic!("missing element type")
                    };
                    let GenericArgument::Type(Type::Path(owner)) = &arguments.args[1] else {
                        panic!("missing owner type")
                    };
                    let owner = owner.path.segments.last().unwrap();
                    assert_eq!(owner.ident, "RowStriped2D");
                    let PathArguments::AngleBracketed(owner_args) = &owner.arguments else {
                        panic!("missing owner arguments")
                    };
                    assert_eq!(owner_args.args.len(), 3);
                    assert!(
                        matches!(&owner_args.args[0], GenericArgument::Type(Type::Path(p)) if p.path.is_ident("Index1D"))
                    );
                    for (argument, expected) in owner_args.args.iter().skip(1).zip([64, 1]) {
                        let GenericArgument::Const(syn::Expr::Lit(literal)) = argument else {
                            panic!("nonliteral ownership width")
                        };
                        let syn::Lit::Int(integer) = &literal.lit else {
                            panic!("noninteger ownership width")
                        };
                        assert_eq!(integer.base10_parse::<usize>().unwrap(), expected);
                    }
                    kinds.push(if element.path.is_ident("f32") {
                        "write_f32"
                    } else {
                        assert!(element.path.is_ident("u32"));
                        "write_u32"
                    });
                    widths.push(16);
                }
                Type::Path(path) if path.path.is_ident("u32") => {
                    kinds.push("u32");
                    widths.push(4);
                }
                _ => panic!("unexpected ABI type"),
            }
        }
        let expected = if index == 0 {
            ["read_f32", "write_f32", "write_f32", "u32"]
        } else {
            ["read_f32", "read_f32", "write_u32", "u32"]
        };
        assert_eq!(kinds, expected);
        assert_eq!(widths, [16, 16, 16, 4]);
        assert_eq!(widths.iter().sum::<usize>(), 52);
    }
}

#[test]
fn shape_and_launch_guards_keep_exact_bounded_scratch() {
    assert_eq!(VOCABULARY_V13, 151936);
    assert_eq!(MAX_ROWS_V13, 32);
    assert_eq!(SHARDS_V13, 64);
    assert_eq!(SCRATCH_BYTES_V13, 16384);
    let source = normalize(LOGITS);
    for guard in [
        "rows == 0 || rows > 32",
        "logits.len() < rows * 151936",
        "logits.len() > 32 * 151936",
        "maxima.len() != 32 * 64",
        "keys.len() != 32 * 64",
        "choices.len() < rows",
        "choices.len() > 32",
        "thread::launch_extent_1d() != rows * 4096",
        "thread::launch_extent_1d() != rows * 64",
        "required = [64, 1, 1]",
        "max_grid = [2048, 1, 1]",
        "max_grid = [32, 1, 1]",
        "control_flow(loop_bounds(38))",
        "let row = group / 64",
        "let shard = group % 64",
        "checked_row_striped_2d::<64, 1>()",
        "winning_key < 151937",
        "winning_key == 0",
    ] {
        assert!(source.contains(&normalize(guard)), "missing {guard}");
    }
    assert_eq!(source.matches("rows==0||rows>32").count(), 2);
    assert_eq!(source.matches("ifrow<rows").count(), 2);
    assert_eq!(source.matches("maxima.len()!=32*64").count(), 2);
    assert_eq!(source.matches("keys.len()!=32*64").count(), 2);
    assert!(!source.contains("unsafe"));
    assert!(!source.contains("Bf16"));
    assert!(!source.contains("atomic"));
}

#[test]
fn producer_has_literal_canonical_loop_and_guarded_last_stripe() {
    let source = syn::parse_file(LOGITS).unwrap();
    let producer = source
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == ROOTS_V13[0] => Some(function),
            _ => None,
        })
        .unwrap();
    let scan = producer
        .block
        .stmts
        .iter()
        .find_map(|statement| {
            let syn::Stmt::Local(local) = statement else {
                return None;
            };
            let syn::Expr::Block(block) = local.init.as_ref()?.expr.as_ref() else {
                return None;
            };
            block
                .block
                .stmts
                .iter()
                .find_map(|statement| match statement {
                    syn::Stmt::Expr(syn::Expr::While(scan), _) => Some(scan),
                    _ => None,
                })
        })
        .unwrap();
    let syn::Expr::Binary(condition) = scan.cond.as_ref() else {
        panic!("nonbinary loop header")
    };
    assert!(matches!(condition.op, syn::BinOp::Lt(_)));
    assert!(matches!(condition.left.as_ref(), syn::Expr::Path(path) if path.path.is_ident("step")));
    let syn::Expr::Lit(right) = condition.right.as_ref() else {
        panic!("nonliteral loop bound")
    };
    assert!(
        matches!(&right.lit, syn::Lit::Int(value) if value.base10_parse::<usize>().unwrap() == 38)
    );
    let source = normalize(LOGITS);
    for text in [
        "let mut step = 1_usize",
        "let stripe = 64 * step + shard",
        "if stripe < 2374",
        "let token = 64 * stripe + lane",
        "step += 1",
    ] {
        assert!(source.contains(&normalize(text)));
    }
}

#[test]
fn producer_reductions_precede_both_stores_and_final_rejection_precedes_selection() {
    let producer = LOGITS.split("// BEGIN shard_lane_argmax").nth(1).unwrap();
    let producer = producer.split("/// Requires completed").next().unwrap();
    let invalid = producer.find("reduce_max_f32::<64>(invalid)").unwrap();
    let maximum = producer.find("reduce_max_f32::<64>(value)").unwrap();
    let key = producer.find("reduce_max_f32::<64>(key)").unwrap();
    let sentinel = producer
        .find("let shard_key = if any_invalid == 0.0")
        .unwrap();
    let lane_zero = producer.find("if lane == 0").unwrap();
    let first_store = producer.find("maxima.write_row_striped_2d").unwrap();
    let second_store = producer.find("keys.write_row_striped_2d").unwrap();
    assert!(invalid < maximum && maximum < key && key < sentinel);
    assert!(sentinel < lane_zero && lane_zero < first_store && first_store < second_store);
    let finalizer = LOGITS.split("// BEGIN final_lane_input").nth(1).unwrap();
    let invalid = finalizer.find("reduce_max_f32::<64>(invalid)").unwrap();
    let reject = finalizer.find("if any_invalid != 0.0").unwrap();
    let maximum = finalizer.find("reduce_max_f32::<64>(value)").unwrap();
    let key = finalizer.find("reduce_max_f32::<64>(key)").unwrap();
    let lane_zero = finalizer.find("if lane == 0").unwrap();
    let store = finalizer.find("choices.write_row_striped_2d").unwrap();
    assert!(invalid < reject && reject < maximum && maximum < key);
    assert!(key < lane_zero && lane_zero < store);
    assert_eq!(LOGITS.matches("reduce_max_f32::<64>").count(), 6);
    assert_eq!(LOGITS.matches("Gfx950Subgroup::current()").count(), 2);
    assert_eq!(LOGITS.matches(".write_row_striped_2d(").count(), 3);
    assert!(!LOGITS.contains("return;"));
}

#[test]
fn actual_root_sentinel_and_all_write_bindings_are_exact() {
    assert_eq!(validate_write_bindings(LOGITS), Ok(()));
}

#[test]
fn parsed_binding_policy_rejects_semantic_source_mutations() {
    assert_eq!(validate_write_bindings(LOGITS), Ok(()));
    let mutate = |before: &str, after: &str| {
        assert_eq!(LOGITS.matches(before).count(), 1);
        let changed = LOGITS.replacen(before, after, 1);
        assert!(syn::parse_file(&changed).is_ok());
        assert!(validate_write_bindings(&changed).is_err(), "accepted {after}");
    };
    let sentinel = "let shard_key = if any_invalid == 0.0 { winning_key } else { 0.0 };";
    for changed in [
        "let shard_key = if any_invalid == 0.0 { winning_key + 1.0 } else { 0.0 };",
        "let shard_key = if any_invalid == 0.0 { winning_key } else { 1.0 };",
        "let shard_key = if any_invalid != 0.0 { winning_key } else { 0.0 };",
    ] {
        mutate(sentinel, changed);
    }
    for (receiver, rows, value) in [
        ("maxima", "rows * 64", "maximum"),
        ("keys", "rows * 64", "shard_key"),
        ("choices", "rows", "winner"),
    ] {
        let arguments = ["&stripe", "0", rows, "1", "1", value];
        let original = format!("{receiver}.write_row_striped_2d({})", arguments.join(", "));
        mutate(&original, &format!("wrong_receiver.write_row_striped_2d({})", arguments.join(", ")));
        let wrong_value = if receiver == "choices" { "winner + 1" } else { "winning_key" };
        for (index, wrong) in ["&wrong_stripe", "1", "rows + 1", "2", "2", wrong_value].into_iter().enumerate() {
            let mut changed = arguments;
            changed[index] = wrong;
            mutate(&original, &format!("{receiver}.write_row_striped_2d({})", changed.join(", ")));
        }
    }
    let witness = "invocation.checked_row_striped_2d::<64, 1>()";
    assert_eq!(LOGITS.matches(witness).count(), 2);
    for wrong in ["other_invocation.checked_row_striped_2d::<64, 1>()",
        "invocation.checked_row_striped_2d::<32, 1>()"]
    {
        let first = LOGITS.replacen(witness, wrong, 1);
        assert!(syn::parse_file(&first).is_ok());
        assert!(validate_write_bindings(&first).is_err());
        let (prefix, suffix) = LOGITS.rsplit_once(witness).unwrap();
        let last = format!("{prefix}{wrong}{suffix}");
        assert!(syn::parse_file(&last).is_ok());
        assert!(validate_write_bindings(&last).is_err());
    }
}

#[test]
fn every_active_scratch_and_choice_slot_has_one_writer_and_tails_have_none() {
    for rows in 1..=32 {
        let mut scratch = [0_u8; 32 * 64];
        let mut choices = [0_u8; 32];
        for raw in 0..rows * 4096 {
            let group = raw / 64;
            let row = group / 64;
            let shard = group % 64;
            if raw % 64 == 0 {
                assert!(row < rows);
                scratch[row * 64 + shard] += 1;
            }
        }
        for raw in 0..rows * 64 {
            if raw % 64 == 0 {
                choices[raw / 64] += 1;
            }
        }
        assert!(scratch[..rows * 64].iter().all(|&count| count == 1));
        assert!(scratch[rows * 64..].iter().all(|&count| count == 0));
        assert!(choices[..rows].iter().all(|&count| count == 1));
        assert!(choices[rows..].iter().all(|&count| count == 0));
    }
}

#[test]
fn host_numeric_macros_match_inline_device_bodies() {
    for name in [
        "shard_lane_argmax",
        "stable_token_key",
        "final_lane_input",
        "winning_shard_key",
        "decode_key",
    ] {
        let macro_body = LOGITS
            .split(&format!("macro_rules! {name}"))
            .nth(1)
            .unwrap()
            .split("=> {{")
            .nth(1)
            .unwrap()
            .split("}};")
            .next()
            .unwrap();
        let inline = LOGITS
            .split(&format!("// BEGIN {name}"))
            .nth(1)
            .unwrap()
            .split(&format!("// END {name}"))
            .next()
            .unwrap();
        assert_eq!(normalize(macro_body).replace('$', ""), normalize(inline));
    }
}
