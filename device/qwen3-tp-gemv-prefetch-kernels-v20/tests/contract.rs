use ferric_qwen3_tp_gemv_prefetch_kernels_device_v20::{
    ROOTS_V20, compiler_expectation_roster_v20,
};
use quote::ToTokens;
use syn::{Expr, ExprWhile, Item, ItemFn, Stmt};

#[path = "../build/target_contract.rs"]
mod target_contract;

const SOURCE: &str = include_str!("../src/projection.rs");
const BASELINE: &str = include_str!("fixtures/v5-gemv.rs");

fn roots(source: &str) -> Vec<ItemFn> {
    syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .collect()
}

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn parts(function: &ItemFn) -> (&[Stmt], &Stmt, &ExprWhile, &[Stmt]) {
    let loops: Vec<_> = function
        .block
        .stmts
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| {
            if let Stmt::Expr(Expr::While(value), _) = statement {
                Some((index, value))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(loops.len(), 1);
    let (index, body) = loops[0];
    (
        &function.block.stmts[..index - 1],
        &function.block.stmts[index - 1],
        body,
        &function.block.stmts[index + 1..],
    )
}

fn normalize_pair_comma(block: &mut syn::Block) {
    if let Some(Stmt::Expr(Expr::Tuple(tuple), None)) = block.stmts.last_mut()
        && tuple.elems.len() == 2
        && tuple.elems.trailing_punct()
    {
        tuple.elems = tuple.elems.iter().cloned().collect();
    }
}

fn statement_tokens(value: &Stmt) -> String {
    let mut value = value.clone();
    // rustfmt may add a trailing comma to the two-value prefetch result.
    // Normalize only that punctuation, not bounds, calls, or statement order.
    if let Stmt::Local(local) = &mut value
        && let Some(initializer) = &mut local.init
        && let Expr::If(branches) = initializer.expr.as_mut()
    {
        normalize_pair_comma(&mut branches.then_branch);
        if let Some((_, alternative)) = &mut branches.else_branch
            && let Expr::Block(block) = alternative.as_mut()
        {
            normalize_pair_comma(&mut block.block);
        }
    }
    tokens(&value)
}

fn statement(actual: &Stmt, expected: &str) {
    let expected: Stmt = syn::parse_str(expected).unwrap();
    assert_eq!(statement_tokens(actual), statement_tokens(&expected));
}

fn arithmetic(index: usize) -> String {
    format!("let product_{index} = Bf16::from_bits(left_bits_{index}).to_f32() * Bf16::from_bits(right_bits_{index}).to_f32();
        partial += product_{index}; finite &= product_{index}.is_finite() & partial.is_finite();")
}

#[test]
fn roots_preserve_every_v5_abi_guard_view_reduction_and_store_statement() {
    let candidate = roots(SOURCE);
    let baseline = roots(BASELINE);
    assert_eq!(candidate.len(), 2);
    assert_eq!(baseline.len(), 2);
    for (index, (new, old)) in candidate.iter().zip(&baseline).enumerate() {
        assert_eq!(new.sig.ident, ROOTS_V20[index]);
        let mut old_signature = old.sig.clone();
        old_signature.ident = new.sig.ident.clone();
        assert_eq!(tokens(&new.sig), tokens(&old_signature));
        assert_eq!(tokens(&new.vis), tokens(&old.vis));
        let (new_prefix, counter, _, new_suffix) = parts(new);
        let (old_prefix, _, _, old_suffix) = parts(old);
        assert_eq!(
            new_prefix.iter().map(tokens).collect::<Vec<_>>(),
            old_prefix.iter().map(tokens).collect::<Vec<_>>()
        );
        assert_eq!(
            new_suffix.iter().map(tokens).collect::<Vec<_>>(),
            old_suffix.iter().map(tokens).collect::<Vec<_>>()
        );
        statement(counter, "let mut group = 0_usize;");
        let grid = if index == 0 { 4_861_952 } else { 131_072 };
        let iterations = if index == 0 { 16 } else { 48 };
        let expected: ItemFn = syn::parse_str(&format!(
            "#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [{grid}, 1, 1]), control_flow(loop_bounds({iterations})))] fn expected() {{}}"
        )).unwrap();
        let actual = new
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("kernel"))
            .unwrap();
        assert_eq!(tokens(actual), tokens(&expected.attrs[0]));
    }
    let roster = compiler_expectation_roster_v20();
    assert_eq!(roster.len(), 2);
    for (entry, root) in roster.iter().zip(ROOTS_V20) {
        assert_eq!(entry.export_name(), root);
        assert!(!root.ends_with("_v5"));
    }
}

#[test]
fn four_pairs_are_loaded_before_four_strictly_ordered_arithmetic_updates() {
    let plain: Stmt = syn::parse_quote! {
        let pair = if inner < k { (left, right) } else { (0_u16, 0_u16) };
    };
    let formatted: Stmt = syn::parse_quote! {
        let pair = if inner < k { (left, right,) } else { (0_u16, 0_u16,) };
    };
    assert_eq!(statement_tokens(&plain), statement_tokens(&formatted));
    for changed in [
        "let pair = if inner <= k { (left, right,) } else { (0_u16, 0_u16,) };",
        "let pair = if inner < k { (right, left,) } else { (0_u16, 0_u16,) };",
        "let pair = if inner < k { (left, right,) } else { (1_u16, 0_u16,) };",
        "let pair = if inner < k { (left,) } else { (0_u16,) };",
    ] {
        let changed: Stmt = syn::parse_str(changed).unwrap();
        assert_ne!(statement_tokens(&plain), statement_tokens(&changed));
    }
    for (root_index, function) in roots(SOURCE).iter().enumerate() {
        let (_, _, loop_expr, _) = parts(function);
        let partial = root_index == 1;
        let condition: Expr =
            syn::parse_str(if partial { "group < 48" } else { "group < 16" }).unwrap();
        assert_eq!(tokens(&loop_expr.cond), tokens(&condition));
        let statements = &loop_expr.body.stmts;
        assert_eq!(statements.len(), if partial { 13 } else { 25 });
        statement(&statements[0], "let inner_0 = group * 256 + lane;");
        for (index, actual) in statements.iter().enumerate().take(4).skip(1) {
            statement(
                actual,
                &format!("let inner_{index} = inner_0 + {};", index * 64),
            );
        }
        for index in 0..4 {
            if partial {
                statement(
                    &statements[4 + index],
                    &format!(
                        "let (left_bits_{index}, right_bits_{index}) = if inner_{index} < k {{
                        let inner = inner_{index} as u16 as usize;
                        (left_view.load_or(row, inner, 0), right_view.load_or(column, inner, 0))
                    }} else {{ (0_u16, 0_u16) }};"
                    ),
                );
                statement(
                    &statements[8 + index],
                    &format!("if inner_{index} < k {{ {} }}", arithmetic(index)),
                );
            } else {
                statement(
                    &statements[4 + index * 2],
                    &format!("let left_bits_{index} = left_view.load_or(row, inner_{index}, 0);"),
                );
                statement(
                    &statements[5 + index * 2],
                    &format!(
                        "let right_bits_{index} = right_view.load_or(column, inner_{index}, 0);"
                    ),
                );
                let expected: syn::Block =
                    syn::parse_str(&format!("{{ {} }}", arithmetic(index))).unwrap();
                for (actual, expected) in statements[12 + index * 3..15 + index * 3]
                    .iter()
                    .zip(&expected.stmts)
                {
                    assert_eq!(tokens(actual), tokens(expected));
                }
            }
        }
        statement(statements.last().unwrap(), "group += 1;");
        assert!(!tokens(&loop_expr.body).contains("trap"));
        assert!(!tokens(&loop_expr.body).contains("reduce_sum"));
    }
}

#[test]
fn producer_and_build_target_remain_closed_and_source_has_no_other_math_path() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b\"")
            .count(),
        2
    );
    assert!(manifest.contains("default = [\"gfx950\"]"));
    for forbidden in [
        "unsafe",
        "mul_add",
        "Mfma",
        "reduce_max",
        "memory::",
        "GridExclusive",
    ] {
        assert!(!SOURCE.contains(forbidden));
    }
    let cpu = "gfx950";
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    let flags = format!("-Ctarget-cpu={cpu}\u{1f}-Ctarget-feature={features}");
    assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_ok());
    for flags in [
        String::new(),
        flags.replace(cpu, "gfx942"),
        flags.replace("-xnack", "+xnack"),
        format!("{flags}\u{1f}-Ctarget-cpu=gfx950"),
    ] {
        assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_err());
    }
}
