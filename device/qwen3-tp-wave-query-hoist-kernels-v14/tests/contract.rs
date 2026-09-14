use ferric_qwen3_tp_wave_query_hoist_kernels_device_v14::{
    ROOTS_V14, compiler_expectation_roster_v14,
};
use quote::ToTokens;
use syn::{Block, Expr, FnArg, Item, ItemFn, Pat, Stmt, Type};

const SOURCE: &str = include_str!("../src/attention.rs");
const BASELINE: &str = include_str!("../../qwen3-tp-batch32-kernels-v5/src/attention.rs");

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn binding(statement: &Stmt, name: &str) -> bool {
    matches!(statement, Stmt::Local(local) if matches!(&local.pat,
        Pat::Ident(pattern) if pattern.ident == name))
}

fn root(file: &mut syn::File) -> Result<&mut ItemFn, &'static str> {
    let mut roots = file.items.iter_mut().filter_map(|item| match item {
        Item::Fn(function) => Some(function),
        _ => None,
    });
    let function = roots.next().ok_or("missing root")?;
    if roots.next().is_some() {
        return Err("extra function");
    }
    Ok(function)
}

fn visible_body(function: &mut ItemFn) -> Result<&mut Block, &'static str> {
    let local = function
        .block
        .stmts
        .iter_mut()
        .find_map(|statement| {
            let Stmt::Local(local) = statement else {
                return None;
            };
            matches!(&local.pat, Pat::Tuple(pattern) if pattern.elems.len() == 2).then_some(local)
        })
        .ok_or("missing recurrence binding")?;
    let Expr::Block(recurrence) = local.init.as_mut().ok_or("recurrence init")?.expr.as_mut()
    else {
        return Err("recurrence block");
    };
    let loop_body = recurrence
        .block
        .stmts
        .iter_mut()
        .find_map(|statement| match statement {
            Stmt::Expr(Expr::While(value), _) => Some(value),
            _ => None,
        })
        .ok_or("missing token loop")?;
    let Some(Stmt::Expr(Expr::If(visible), _)) = loop_body.body.stmts.first_mut() else {
        return Err("visible-token branch");
    };
    Ok(&mut visible.then_branch)
}

// Restore only the two moved AST statements, then compare the complete source AST.
fn relocation_only(candidate: &str) -> Result<(), &'static str> {
    let mut old = syn::parse_file(BASELINE).map_err(|_| "baseline parse")?;
    let mut new = syn::parse_file(candidate).map_err(|_| "candidate parse")?;
    let original = root(&mut old)?;
    let candidate = root(&mut new)?;
    if candidate.sig.ident != ROOTS_V14[0] {
        return Err("candidate identity");
    }
    let insert_at = original
        .block
        .stmts
        .iter()
        .position(|s| binding(s, "math"))
        .ok_or("baseline admission boundary")?;
    if !candidate
        .block
        .stmts
        .get(insert_at)
        .is_some_and(|s| binding(s, "query_0"))
        || !candidate
            .block
            .stmts
            .get(insert_at + 1)
            .is_some_and(|s| binding(s, "query_1"))
        || !candidate
            .block
            .stmts
            .get(insert_at + 2)
            .is_some_and(|s| binding(s, "math"))
    {
        return Err("hoist boundary or order");
    }
    let moved: Vec<_> = candidate
        .block
        .stmts
        .drain(insert_at..insert_at + 2)
        .collect();
    let old_body = visible_body(original)?;
    let old_at = old_body
        .stmts
        .iter()
        .position(|s| binding(s, "query_0"))
        .ok_or("baseline query binding")?;
    if tokens(&moved[0]) != tokens(&old_body.stmts[old_at])
        || tokens(&moved[1]) != tokens(&old_body.stmts[old_at + 1])
    {
        return Err("query load operands or conversion");
    }
    let new_body = visible_body(candidate)?;
    if old_at > new_body.stmts.len() {
        return Err("inner relocation position");
    }
    new_body.stmts.splice(old_at..old_at, moved);
    candidate.sig.ident = original.sig.ident.clone();
    if tokens(&old) != tokens(&new) {
        return Err("change beyond two query bindings and root identity");
    }
    Ok(())
}

#[test]
fn complete_ast_diff_is_only_root_identity_and_two_hoisted_bindings() {
    assert_eq!(relocation_only(SOURCE), Ok(()));
}

#[test]
fn source_mutations_cannot_change_admission_math_loads_or_output() {
    for (old, new) in [
        (
            "query_view.load_or(head_row, lane, 0)",
            "query_view.load_or(head_row, lane + 1, 0)",
        ),
        (
            "query_view.load_or(head_row, lane + 64, 0)",
            "query_view.load_or(head_row, lane, 0)",
        ),
        (
            "query_view.load_or(head_row, lane, 0)",
            "key_view.load_or(head_row, lane, 0)",
        ),
        ("let query_0 =", "let wrong_query_0 ="),
        (
            "if position < max_context_tokens",
            "if position <= max_context_tokens",
        ),
        (
            "physical_page < physical_pages",
            "physical_page <= physical_pages",
        ),
        (
            "while token < max_context_tokens",
            "while token <= max_context_tokens",
        ),
        ("token += 1", "token += 2"),
        ("query_0 * key_0", "query_1 * key_0"),
        ("product_0 + product_1", "product_0 - product_1"),
        (
            "reduce_sum_f32::<64>(partial)",
            "reduce_sum_f32::<32>(partial)",
        ),
        (
            "finite &= product_0.is_finite()",
            "finite |= product_0.is_finite()",
        ),
        (
            "math.exp_f32(maximum - next_maximum)",
            "math.exp_f32(score - next_maximum)",
        ),
        ("numerator_0 / denominator", "numerator_1 / denominator"),
        ("max_grid = [1024, 1, 1]", "max_grid = [1025, 1, 1]"),
        ("lane + 64", "lane + 63"),
    ] {
        assert!(
            SOURCE.contains(old),
            "mutation must reach actual source: {old}"
        );
        assert!(
            relocation_only(&SOURCE.replacen(old, new, 1)).is_err(),
            "{old}"
        );
    }
    let query_0 =
        "    let query_0 = Bf16::from_bits(query_view.load_or(head_row, lane, 0)).to_f32();\n";
    let query_1 =
        "    let query_1 = Bf16::from_bits(query_view.load_or(head_row, lane + 64, 0)).to_f32();\n";
    let pair = format!("{query_0}{query_1}");
    assert!(SOURCE.contains(&pair));
    let early = SOURCE.replace(&pair, "").replace(
        "    let query_base =",
        &(pair.clone() + "    let query_base ="),
    );
    assert!(relocation_only(&early).is_err());
    let reversed = SOURCE.replace(&pair, &format!("{query_1}{query_0}"));
    assert!(relocation_only(&reversed).is_err());
}

#[test]
fn single_root_and_six_slice_five_scalar_abi_remain_closed() {
    let roster = compiler_expectation_roster_v14();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V14[0]);
    assert_ne!(roster[0].generated_host_contract_identity(), [0; 32]);
    let mut file = syn::parse_file(SOURCE).unwrap();
    let function = root(&mut file).unwrap();
    assert!(function.sig.unsafety.is_none());
    let widths: Vec<_> = function
        .sig
        .inputs
        .iter()
        .map(|arg| {
            let FnArg::Typed(arg) = arg else {
                panic!("receiver")
            };
            match arg.ty.as_ref() {
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
                _ => panic!("unsupported argument"),
            }
        })
        .collect();
    assert_eq!(widths, [16, 16, 16, 16, 16, 16, 4, 4, 4, 4, 4]);
    assert_eq!(widths.iter().sum::<usize>(), 116);
    assert_eq!(relocation_only(SOURCE), Ok(()));
}

#[test]
fn two_component_wave_ownership_covers_active_rows_only() {
    for world in [1, 2, 8] {
        let heads = 32 / world;
        for rows in 1..=32 {
            let mut owners = vec![0_u8; 32 * heads * 128];
            for head_row in 0..rows * heads {
                for lane in 0..64 {
                    owners[head_row * 128 + lane] += 1;
                    owners[head_row * 128 + lane + 64] += 1;
                }
            }
            assert!(owners[..rows * heads * 128].iter().all(|&count| count == 1));
            assert!(owners[rows * heads * 128..].iter().all(|&count| count == 0));
        }
    }
}

#[test]
fn standalone_manifest_keeps_exact_compiler_and_no_old_routes() {
    let manifest = include_str!("../Cargo.toml");
    let library = include_str!("../src/lib.rs");
    assert_eq!(
        manifest
            .matches("c6b4050dd6c18e1868b24e4580da3eed8a3d19fa")
            .count(),
        2
    );
    assert!(manifest.contains("[workspace]"));
    assert!(manifest.contains("default = [\"gfx950\"]"));
    assert!(library.contains("#[cfg(not(feature = \"gfx950\"))]"));
    assert!(!manifest.contains("ferric-m1-engineering"));
    assert!(!SOURCE.contains("_bf16_v5("));
}
