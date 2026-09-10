use ferric_qwen3_tp_kernels_device_v1::contract;
use std::collections::BTreeMap;
use syn::{BinOp, Expr, Item, Lit, Pat, Stmt, UnOp};

type Values = BTreeMap<String, i64>;

fn evaluate_block(block: &syn::Block, values: &Values) -> i64 {
    let [Stmt::Expr(expression, None)] = block.stmts.as_slice() else {
        panic!("scalar branch")
    };
    evaluate(expression, values)
}

fn evaluate(expression: &Expr, values: &Values) -> i64 {
    match expression {
        Expr::Paren(value) => evaluate(&value.expr, values),
        Expr::Group(value) => evaluate(&value.expr, values),
        Expr::Cast(value) => evaluate(&value.expr, values),
        Expr::Lit(value) => match &value.lit {
            Lit::Int(value) => value.base10_parse().unwrap(),
            Lit::Bool(value) => i64::from(value.value),
            _ => panic!("unsupported literal"),
        },
        Expr::Path(value) => values[&value.path.get_ident().unwrap().to_string()],
        Expr::Unary(value) => match value.op {
            UnOp::Not(_) => i64::from(evaluate(&value.expr, values) == 0),
            _ => panic!("unsupported unary operation"),
        },
        Expr::If(value) => {
            if evaluate(&value.cond, values) != 0 {
                evaluate_block(&value.then_branch, values)
            } else {
                evaluate(&value.else_branch.as_ref().unwrap().1, values)
            }
        }
        Expr::Block(value) => evaluate_block(&value.block, values),
        Expr::Binary(value) => {
            let left = evaluate(&value.left, values);
            if matches!(value.op, BinOp::And(_)) && left == 0 {
                return 0;
            }
            if matches!(value.op, BinOp::Or(_)) && left != 0 {
                return 1;
            }
            let right = evaluate(&value.right, values);
            match value.op {
                BinOp::Add(_) => left.checked_add(right).unwrap(),
                BinOp::Mul(_) => left.checked_mul(right).unwrap(),
                BinOp::Div(_) => left.checked_div(right).unwrap(),
                BinOp::BitAnd(_) => left & right,
                BinOp::Eq(_) => i64::from(left == right),
                BinOp::Ne(_) => i64::from(left != right),
                BinOp::Lt(_) => i64::from(left < right),
                BinOp::Le(_) => i64::from(left <= right),
                BinOp::Gt(_) => i64::from(left > right),
                BinOp::Ge(_) => i64::from(left >= right),
                BinOp::And(_) | BinOp::Or(_) => i64::from(right != 0),
                _ => panic!("unsupported binary operation"),
            }
        }
        _ => panic!("unsupported geometry expression"),
    }
}

fn kernel(source: &str, name: &str) -> syn::ItemFn {
    syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .find_map(|item| {
            let Item::Fn(function) = item else {
                return None;
            };
            (function.sig.ident == name).then_some(function)
        })
        .unwrap()
}

fn admits(function: &syn::ItemFn, inputs: &[(&str, u32)]) -> bool {
    let mut values: Values = inputs
        .iter()
        .map(|(key, value)| ((*key).into(), i64::from(*value)))
        .collect();
    for statement in &function.block.stmts {
        match statement {
            Stmt::Local(local) => {
                let expression = &local.init.as_ref().unwrap().expr;
                if matches!(expression.as_ref(), Expr::Cast(_)) {
                    return true;
                }
                let Pat::Ident(name) = &local.pat else {
                    panic!("geometry binding")
                };
                values.insert(name.ident.to_string(), evaluate(expression, &values));
            }
            Stmt::Expr(Expr::If(guard), None) => {
                assert!(guard.else_branch.is_none());
                let [Stmt::Expr(Expr::Call(call), Some(_))] = guard.then_branch.stmts.as_slice()
                else {
                    panic!("trap guard")
                };
                let Expr::Path(path) = call.func.as_ref() else {
                    panic!("trap path")
                };
                assert_eq!(path.path.segments.last().unwrap().ident, "trap");
                if evaluate(&guard.cond, &values) != 0 {
                    return false;
                }
            }
            _ => panic!("unrecognized geometry prefix"),
        }
    }
    panic!("missing geometry-to-buffer boundary")
}

#[test]
fn actual_projection_prefixes_match_contracted_axes() {
    let source = include_str!("../src/projection.rs");
    let column = kernel(source, "ferric_qwen3_tp_gemv_bf16_f32_bf16_v1");
    let partial = kernel(source, "ferric_qwen3_tp_gemv_partial_bf16_f32_v1");
    for role in [0, 1, 2, 3] {
        for world in [0, 1, 2, 3, 4, 8, 16] {
            for operation in 0..7 {
                for n in [
                    0, 128, 256, 384, 512, 1024, 1536, 2048, 3072, 4096, 6144, 12288, 12289,
                ] {
                    for k in [
                        0, 128, 256, 384, 512, 1024, 1536, 2048, 3072, 4096, 6144, 12288,
                    ] {
                        let inputs = [
                            ("n", n),
                            ("k", k),
                            ("model_role", role),
                            ("world_size", world),
                            ("projection", operation),
                        ];
                        assert_eq!(
                            admits(&column, &inputs),
                            contract::column_shape(n, k, role, world, operation)
                        );
                        assert_eq!(
                            admits(&partial, &inputs),
                            contract::partial_shape(n, k, role, world, operation)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn actual_cache_rope_and_activation_prefixes_reject_unknown_geometry() {
    let source = include_str!("../src/rope_kv.rs");
    let rope = kernel(source, "ferric_qwen3_tp_rope_v1");
    let append = kernel(source, "ferric_qwen3_tp_kv_append_v1");
    let attention = kernel(
        include_str!("../src/attention.rs"),
        "ferric_qwen3_tp_gqa_decode_bf16_f32_v1",
    );
    let activation = kernel(
        include_str!("../src/activation.rs"),
        "ferric_qwen3_tp_swiglu_bf16_f32_v1",
    );
    for role in [0, 1, 2, 3] {
        for world in [0, 1, 2, 3, 4, 8, 16] {
            let valid = contract::geometry(role, world).is_some();
            for capacity in [0, 1, 8192, 8193] {
                for position in [0, 1, 8191, 8192, 8193] {
                    let inputs = [
                        ("model_role", role),
                        ("world_size", world),
                        ("position", position),
                        ("capacity", capacity),
                        ("count", position),
                    ];
                    assert_eq!(admits(&rope, &inputs), valid && position < 8192);
                    assert_eq!(
                        admits(&append, &inputs),
                        valid && contract::append_position(position, capacity)
                    );
                    assert_eq!(
                        admits(&attention, &inputs),
                        valid && contract::attention_prefix(position, capacity)
                    );
                }
            }
            assert_eq!(
                admits(&activation, &[("model_role", role), ("world_size", world)]),
                valid
            );
        }
    }
}

fn literal_loop_guard_matches(
    loop_: &syn::ExprWhile,
    counter: &str,
    bound: &str,
    maximum: i64,
) -> bool {
    let [
        Stmt::Expr(Expr::If(active), None),
        Stmt::Expr(Expr::Binary(increment), Some(_)),
    ] = loop_.body.stmts.as_slice()
    else {
        return false;
    };
    if active.else_branch.is_some() || !matches!(increment.op, BinOp::AddAssign(_)) {
        return false;
    }
    let Expr::Path(left) = increment.left.as_ref() else {
        return false;
    };
    if !left.path.is_ident(counter) || evaluate(&increment.right, &Values::new()) != 1 {
        return false;
    }
    for extent in [1, 2, 4, 8, 16, 32, 128, 512, 1024] {
        if extent > maximum {
            continue;
        }
        for index in 0..=maximum {
            let values = Values::from([(counter.into(), index), (bound.into(), extent)]);
            if (evaluate(&loop_.cond, &values) != 0) != (index < maximum)
                || (evaluate(&active.cond, &values) != 0) != (index < extent)
            {
                return false;
            }
        }
    }
    true
}

#[test]
fn literal_maximum_loops_guard_every_memory_and_numeric_statement() {
    let source = include_str!("../src/rope_kv.rs");
    for (name, expected) in [
        (
            "ferric_qwen3_tp_rope_v1",
            vec![("head", "query_heads", 32), ("head", "kv_heads", 8)],
        ),
        (
            "ferric_qwen3_tp_kv_append_v1",
            vec![("component", "columns", 1024)],
        ),
    ] {
        let function = kernel(source, name);
        let loops: Vec<_> = function
            .block
            .stmts
            .iter()
            .filter_map(|statement| {
                let Stmt::Expr(Expr::While(loop_), None) = statement else {
                    return None;
                };
                Some(loop_)
            })
            .collect();
        assert_eq!(loops.len(), expected.len());
        for (loop_, (counter, bound, maximum)) in loops.into_iter().zip(expected) {
            assert!(literal_loop_guard_matches(loop_, counter, bound, maximum));
            let mut mutant = loop_.clone();
            let Stmt::Expr(Expr::If(guard), None) = &mut mutant.body.stmts[0] else {
                unreachable!()
            };
            guard.cond = Box::new(syn::parse_quote!(true));
            assert!(!literal_loop_guard_matches(
                &mutant, counter, bound, maximum
            ));
            let mut mutant = loop_.clone();
            mutant
                .body
                .stmts
                .insert(0, syn::parse_quote!(memory::volatile_load(key, component);));
            assert!(!literal_loop_guard_matches(
                &mutant, counter, bound, maximum
            ));
        }
    }
}
