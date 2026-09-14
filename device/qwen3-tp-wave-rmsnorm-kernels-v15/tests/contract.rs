use ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15::{ROOTS_V15, compiler_expectation_roster_v15};
use quote::{ToTokens, quote};
use syn::{Block, Expr, FnArg, Item, ItemFn, Pat, Stmt};

const SOURCE: &str = include_str!("../src/rmsnorm.rs");
const BASELINE: &str = include_str!("../../qwen3-all-kernels-v1/src/rmsnorm.rs");
const PREFIX: &str = r"{
    let shape_valid = rows != 0 && rows <= 32 && width == 4_096 && behavior == 0;
    let elements = rows as usize * width as usize;
    let required_lengths = input_bf16.len() == elements
        && weight_bf16.len() == width as usize
        && normalized_bf16.len() == elements;
    let auxiliary_lengths = residual_bf16.len() == 0 && fused_residual_bf16.len() == 0;
    let exact_grid =
        thread::grid_dim_x() == rows && thread::grid_dim_y() == 1 && thread::grid_dim_z() == 1;
    if !shape_valid || !required_lengths || !auxiliary_lengths
        || epsilon != QWEN3_RMSNORM_EPSILON_V1 || !exact_grid {
        fe2o3_device::trap();
    }
    let Ok(input_view) =
        StridedReadView2D::from_shared_slice(input_bf16, 0, rows as usize, 4_096, 4_096)
    else {
        fe2o3_device::trap();
    };
    let row = thread::block_idx_x() as usize;
    let lane = WaveLane::<Wave64>::current();
    let lane_index = lane.into_lane_id() as usize;
    let row_base = row * width as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut component = 0_usize;
    while component < 64 {
        let column = lane_index + component * 64;
        let input = Bf16::from_bits(input_view.load_or(row, column, 0x7fc0));
        let input_value = input.to_f32();
        let square = input_value * input_value;
        let next_sum = partial + square;
        finite &= input.is_finite() & square.is_finite() & next_sum.is_finite();
        partial = next_sum;
        component += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    if any_invalid != 0.0 || !sum.is_finite() {
        fe2o3_device::trap();
    }
}";

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn statements(value: &[Stmt]) -> String {
    quote!(#(#value)*).to_string()
}

fn binding(statement: &Stmt, name: &str) -> bool {
    matches!(statement, Stmt::Local(local) if matches!(&local.pat,
        Pat::Ident(pattern) if pattern.ident == name))
}

fn function(source: &str, name: &str) -> Result<ItemFn, &'static str> {
    syn::parse_file(source)
        .map_err(|_| "parse")?
        .items
        .into_iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .ok_or("missing function")
}

fn verify(candidate: &str) -> Result<(), &'static str> {
    let new = function(candidate, ROOTS_V15[0])?;
    let old = function(BASELINE, "qwen3_rmsnorm_v1")?;
    let parsed = syn::parse_file(candidate).map_err(|_| "parse")?;
    if parsed
        .items
        .iter()
        .filter(|item| matches!(item, Item::Fn(_)))
        .count()
        != 1
    {
        return Err("root count");
    }
    let constants: Vec<_> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Const(value) => Some(value),
            _ => None,
        })
        .collect();
    if constants.len() != 1
        || constants[0].ident != "QWEN3_RMSNORM_EPSILON_V1"
        || tokens(&constants[0].expr) != "1e-6_f32"
    {
        return Err("epsilon");
    }
    if new.sig.inputs.len() != 9 {
        return Err("argument count");
    }
    for (actual, expected) in new.sig.inputs.iter().zip(&old.sig.inputs) {
        let (FnArg::Typed(actual), FnArg::Typed(expected)) = (actual, expected) else {
            return Err("argument receiver");
        };
        let (Pat::Ident(a), Pat::Ident(b)) = (actual.pat.as_ref(), expected.pat.as_ref()) else {
            return Err("argument name");
        };
        if a.ident != b.ident || tokens(&actual.ty) != tokens(&expected.ty) {
            return Err("argument ABI");
        }
    }
    let expected: syn::Attribute = syn::parse_quote!(#[kernel(
        typed,
        launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]),
        control_flow(loop_bounds(64, 64))
    )]);
    let actual = new
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("kernel"))
        .ok_or("kernel attribute")?;
    if tokens(actual) != tokens(&expected) {
        return Err("launch or loop bound");
    }
    let begin = new
        .block
        .stmts
        .iter()
        .position(|s| binding(s, "mean_square"))
        .ok_or("normalization boundary")?;
    let prefix: Block = syn::parse_str(PREFIX).map_err(|_| "expected prefix")?;
    if statements(&new.block.stmts[..begin]) != statements(&prefix.stmts) {
        return Err("entry or convergent reduction");
    }
    let old_begin = old
        .block
        .stmts
        .iter()
        .position(|s| binding(s, "mean_square"))
        .ok_or("old normalization boundary")?;
    let mut tail = old.block.stmts[old_begin..].to_vec();
    let output_loop = tail
        .iter_mut()
        .find_map(|s| match s {
            Stmt::Expr(Expr::While(value), _) => Some(value),
            _ => None,
        })
        .ok_or("old output loop")?;
    let Some(Stmt::Expr(Expr::If(active), _)) = output_loop.body.stmts.get_mut(1) else {
        return Err("old active output");
    };
    let fused = active
        .then_branch
        .stmts
        .iter_mut()
        .find(|s| binding(s, "normalized_input"))
        .ok_or("old fused branch")?;
    *fused = syn::parse_quote!(let normalized_input = input_value;);
    if statements(&new.block.stmts[begin..]) != statements(&tail) {
        return Err("pure post-sum arithmetic or second output pass");
    }
    Ok(())
}

#[test]
fn one_distinct_root_retains_five_slices_four_scalars_and_96_explicit_bytes() {
    verify(SOURCE).unwrap();
    let roster = compiler_expectation_roster_v15();
    assert_eq!(roster.len(), 1);
    assert_eq!(ROOTS_V15, ["ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"]);
    assert_eq!(roster[0].export_name(), ROOTS_V15[0]);
    assert_ne!(roster[0].generated_host_contract_identity(), [0; 32]);
    let root = function(SOURCE, ROOTS_V15[0]).unwrap();
    assert!(root.sig.unsafety.is_none());
    let sizes: Vec<_> = root
        .sig
        .inputs
        .iter()
        .map(|input| {
            let FnArg::Typed(input) = input else {
                panic!("receiver")
            };
            let ty = tokens(&input.ty);
            if ty == "u32" || ty == "f32" { 4 } else { 16 }
        })
        .collect();
    assert_eq!(sizes, [16, 16, 16, 16, 16, 4, 4, 4, 4]);
    assert_eq!(sizes.iter().sum::<usize>(), 96);
}

#[test]
fn entry_and_both_convergent_collectives_are_closed() {
    verify(SOURCE).unwrap();
    assert_eq!(SOURCE.matches("subgroup.reduce_sum_f32::<64>").count(), 1);
    assert_eq!(SOURCE.matches("subgroup.reduce_max_f32::<64>").count(), 1);
    assert_eq!(
        SOURCE
            .matches("memory::volatile_load(input_bf16, index)")
            .count(),
        1
    );
    assert!(!SOURCE.contains("fused_residual_bf16.write"));
}

#[test]
fn first_pass_has_guarded_nan_fallback_and_no_lane_local_exit() {
    verify(SOURCE).unwrap();
    let root = function(SOURCE, ROOTS_V15[0]).unwrap();
    let first_loop = root
        .block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            Stmt::Expr(Expr::While(value), _) => Some(value),
            _ => None,
        })
        .unwrap();
    let body = tokens(&first_loop.body);
    for forbidden in ["volatile_load", "trap", "return", "break"] {
        assert!(
            !body.contains(forbidden),
            "first-pass lane exit: {forbidden}"
        );
    }
    assert_eq!(
        SOURCE
            .matches("input_view.load_or(row, column, 0x7fc0)")
            .count(),
        1
    );
}

#[test]
fn post_sum_arithmetic_and_second_pass_match_the_old_pure_path() {
    verify(SOURCE).unwrap();
    assert!(!SOURCE.contains("rsqrt"));
    assert!(!SOURCE.contains("mul_add"));
    assert!(!SOURCE.contains("unsafe {"));
}

#[test]
fn malformed_source_cannot_relax_guards_association_or_output() {
    for (from, to) in [
        ("rows <= 32", "rows <= 33"),
        ("width == 4_096", "width == 1_024"),
        ("behavior == 0", "behavior <= 1"),
        (
            "input_bf16.len() == elements",
            "input_bf16.len() >= elements",
        ),
        (
            "residual_bf16.len() == 0",
            "residual_bf16.len() <= elements",
        ),
        ("thread::grid_dim_y() == 1", "thread::grid_dim_y() >= 1"),
        ("1e-6_f32", "1e-5_f32"),
        ("loop_bounds(64, 64)", "loop_bounds(63, 64)"),
        ("lane_index + component * 64", "lane_index + component * 63"),
        (
            "from_shared_slice(input_bf16, 0, rows as usize, 4_096, 4_096)",
            "from_shared_slice(input_bf16, 0, rows as usize, 4_095, 4_096)",
        ),
        (
            "input_view.load_or(row, column, 0x7fc0)",
            "input_view.load_or(row, column, 0)",
        ),
        (
            "input_view.load_or(row, column, 0x7fc0)",
            "memory::volatile_load(input_bf16, row_base + column)",
        ),
        ("finite &=", "finite |= "),
        (
            "partial = next_sum;",
            "partial = next_sum; if !finite { fe2o3_device::trap(); }",
        ),
        ("reduce_sum_f32::<64>", "reduce_sum_f32::<32>"),
        ("reduce_max_f32::<64>(invalid)", "reduce_max_f32::<64>(0.0)"),
        ("sum / width as f32", "sum / 4_095.0_f32"),
        (
            "normalized_input * inverse_rms",
            "normalized_input / inverse_rms",
        ),
        (
            "normalized * weight.to_f32()",
            "weight.to_f32() * normalized",
        ),
        ("Bf16::from_f32(weighted)", "Bf16::from_f32(normalized)"),
    ] {
        let mutated = SOURCE.replace(from, to);
        assert_ne!(mutated, SOURCE, "missing mutation {from}");
        assert!(verify(&mutated).is_err(), "accepted mutation {from}");
    }
}

#[test]
fn stripes_cover_each_active_element_once_and_no_capacity_tail() {
    for rows in 1..=32 {
        let mut owners = vec![0_u8; 32 * 4096];
        for row in 0..rows {
            for lane in 0..64 {
                for component in 0..64 {
                    let column = lane + component * 64;
                    let index = row * 4096 + column;
                    assert_eq!(index / 4096, row);
                    assert!(index < 131_072);
                    owners[index] += 1;
                }
            }
        }
        assert!(owners[..rows * 4096].iter().all(|&count| count == 1));
        assert!(owners[rows * 4096..].iter().all(|&count| count == 0));
    }
}

#[test]
fn standalone_manifest_pins_latest_compiler_without_old_route_changes() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("85255498a021ed2b7a1a511f99f577ee4b11d175")
            .count(),
        2
    );
    assert!(manifest.contains("[workspace]"));
    assert!(manifest.contains("default = [\"gfx950\"]"));
    assert!(!manifest.contains("8efd4fd"));
    assert!(!manifest.contains("path = \"../"));
}
