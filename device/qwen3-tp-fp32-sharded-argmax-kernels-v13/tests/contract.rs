use ferric_qwen3_tp_fp32_sharded_argmax_kernels_device_v13::{
    MAX_ROWS_V13, ROOTS_V13, SCRATCH_BYTES_V13, SHARDS_V13, VOCABULARY_V13,
    compiler_expectation_roster_v13,
};
use syn::{FnArg, GenericArgument, Item, PathArguments, Type};

const LOGITS: &str = include_str!("../src/logits.rs");

fn normalize(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
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
                    assert!(matches!(&owner_args.args[0], GenericArgument::Type(Type::Path(p)) if p.path.is_ident("Index1D")));
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
            block.block.stmts.iter().find_map(|statement| match statement {
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
    assert!(matches!(&right.lit, syn::Lit::Int(value) if value.base10_parse::<usize>().unwrap() == 38));
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
    let sentinel = producer.find("let shard_key = if any_invalid == 0.0").unwrap();
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
