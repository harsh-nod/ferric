use ferric_qwen3_down_f32_packed_u32_proposal::{
    ACTIVATION_PACK_ROOT, ROOT, compiler_expectation_roster,
};
use quote::ToTokens;
use syn::{Expr, ExprWhile, Item, ItemFn, Stmt};

#[path = "../build/target_contract.rs"]
mod target_contract;

const SOURCE: &str = include_str!("../src/projection.rs");
const BASELINE: &str = include_str!("fixtures/v5-partial-f32.rs");

fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

fn functions(source: &str) -> Option<Vec<ItemFn>> {
    Some(
        syn::parse_file(source)
            .ok()?
            .items
            .into_iter()
            .filter_map(|item| {
                if let Item::Fn(function) = item {
                    Some(function)
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn parts(function: &ItemFn) -> Option<(&[Stmt], &ExprWhile, &[Stmt])> {
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
    if loops.len() != 1 {
        return None;
    }
    let (index, value) = loops[0];
    Some((
        &function.block.stmts[..index],
        value,
        &function.block.stmts[index + 1..],
    ))
}

fn statements(statements: &[Stmt]) -> Vec<String> {
    statements.iter().map(tokens).collect()
}

fn statement_matches(actual: &Stmt, expected: &str) -> bool {
    syn::parse_str::<Stmt>(expected).is_ok_and(|expected| tokens(actual) == tokens(&expected))
}

fn contract(source: &str) -> bool {
    let Some(candidate) = functions(source) else {
        return false;
    };
    let baseline = functions(BASELINE).unwrap();
    if candidate.len() != 1 || baseline.len() != 1 {
        return false;
    }
    let candidate = &candidate[0];
    let old = &baseline[0];
    let expected: ItemFn = syn::parse_quote! {
        #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [4096, 1, 1]), control_flow(loop_bounds(96)))]
        pub fn ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1(
            a: &[u32], weights: &[u32],
            mut output: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
            rows: u32, n: u32, k: u32, world_size: u32, projection: u32,
        ) {}
    };
    if tokens(&candidate.sig) != tokens(&expected.sig)
        || tokens(&candidate.vis) != tokens(&expected.vis)
    {
        return false;
    }
    let attributes: Vec<_> = candidate
        .attrs
        .iter()
        .filter(|attribute| !attribute.path().is_ident("doc"))
        .collect();
    let allow: syn::Attribute = syn::parse_quote!(#[allow(clippy::too_many_arguments)]);
    if attributes.len() != 2
        || tokens(attributes[0]) != tokens(&expected.attrs[0])
        || tokens(attributes[1]) != tokens(&allow)
    {
        return false;
    }
    let Some((prefix, body, suffix)) = parts(candidate) else {
        return false;
    };
    let Some((_, old_body, old_suffix)) = parts(old) else {
        return false;
    };
    let expected_prefix: syn::Block = syn::parse_quote! {{
        if rows != 1 || n != 4096 || k != 12288 || world_size != 1 || projection != 2 {
            fe2o3_device::trap();
        }
        let rows = rows as usize;
        let n = n as usize;
        if rows < 2 && n == 4096 {} else { fe2o3_device::trap(); }
        if a.len() < 6144 || a.len() > 32 * 6144 || weights.len() != 4096 * 6144
            || output.len() < rows * n || output.len() > 32 * n
            || thread::launch_extent_1d() != rows * n * 64
        { fe2o3_device::trap(); }
        let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, rows, 6144, 6144) else {
            fe2o3_device::trap();
        };
        let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, 6144, 6144) else {
            fe2o3_device::trap();
        };
        let invocation = thread::index_1d();
        let raw = invocation.get();
        let element = thread::block_idx_x() as usize;
        let lane = raw % 64;
        if element < rows * n {} else { fe2o3_device::trap(); }
        let column = element as u16 as usize;
        let subgroup = Gfx950Subgroup::current();
        let mut partial = 0.0_f32;
        let mut finite = true;
        let mut group = 0_usize;
    }};
    let condition: Expr = syn::parse_quote!(group < 96);
    if statements(prefix) != statements(&expected_prefix.stmts)
        || statements(suffix) != statements(old_suffix)
        || tokens(&body.cond) != tokens(&condition)
        || body.body.stmts.len() != 6
        || old_body.body.stmts.len() != 3
    {
        return false;
    }
    let Stmt::Expr(Expr::If(old_guard), None) = &old_body.body.stmts[1] else {
        return false;
    };
    let expected_guard: Expr = syn::parse_quote!(inner < k);
    if old_guard.else_branch.is_some()
        || tokens(&old_guard.cond) != tokens(&expected_guard)
        || old_guard.then_branch.stmts.len() != 6
    {
        return false;
    }
    let old_arithmetic = &old_guard.then_branch.stmts[3..6];
    let body = &body.body.stmts;
    for (index, expected) in [
        (0, "let packed_inner = group * 64 + lane;"),
        (1, "let left_bits = left_view.load_or(0, packed_inner, 0);"),
        (
            2,
            "let right_bits = right_view.load_or(column, packed_inner, 0);",
        ),
        (5, "group += 1;"),
    ] {
        if !statement_matches(&body[index], expected) {
            return false;
        }
    }
    for (index, high) in [(3, false), (4, true)] {
        let Stmt::Expr(Expr::Block(block), None) = &body[index] else {
            return false;
        };
        let inner = &block.block.stmts;
        if block.label.is_some() || inner.len() != 5 {
            return false;
        }
        let left = if high {
            "(left_bits >> 16)"
        } else {
            "left_bits"
        };
        let right = if high {
            "(right_bits >> 16)"
        } else {
            "right_bits"
        };
        if !statement_matches(
            &inner[0],
            &format!("let left = Bf16::from_bits({left} as u16).to_f32();"),
        ) || !statement_matches(
            &inner[1],
            &format!("let right = Bf16::from_bits({right} as u16).to_f32();"),
        ) || statements(&inner[2..]) != statements(old_arithmetic)
        {
            return false;
        }
    }
    true
}

#[test]
fn typed_root_preserves_original_arithmetic_reduction_rejection_and_store_ast() {
    assert!(contract(SOURCE));
    let roster = compiler_expectation_roster();
    assert_eq!(roster.len(), 2);
    assert_eq!(roster[0].export_name(), ROOT);
    assert_eq!(roster[1].export_name(), ACTIVATION_PACK_ROOT);
    assert_ne!(roster[0].export_name(), roster[1].export_name());
    assert_eq!(SOURCE.matches("load_or(").count(), 2);
}

#[test]
fn source_contract_rejects_layout_guard_order_math_and_collective_mutations() {
    for (before, after) in [
        ("group * 64 + lane", "group * 64 + lane + 1"),
        ("group < 96", "group < 95"),
        ("left_bits >> 16", "left_bits >> 15"),
        ("right_bits as u16", "(right_bits >> 16) as u16"),
        ("partial += product;", "partial = product + partial;"),
        (
            "finite &= product.is_finite() & partial.is_finite();",
            "finite &= partial.is_finite();",
        ),
        ("left * right", "left.mul_add(right, 0.0)"),
        ("reduce_sum_f32::<64>", "reduce_sum_f32::<32>"),
        ("!finite || !sum.is_finite()", "!sum.is_finite()"),
        ("k != 12288", "k != 4096"),
        ("projection != 2", "projection != 1"),
        ("WriteOnlyDisjointSlice<f32,", "WriteOnlyDisjointSlice<u16,"),
        ("1, 1, sum)", "1, 1, Bf16::from_f32(sum).to_f32())"),
        ("a: &[u32]", "a: &[u16]"),
        (
            "weights.len() != 4096 * 6144",
            "weights.len() < 4096 * 6144",
        ),
        ("output.len() > 32 * n", "output.len() > 64 * n"),
        (
            "launch_extent_1d() != rows * n * 64",
            "launch_extent_1d() < rows * n * 64",
        ),
        (
            "left_view.load_or(0, packed_inner, 0)",
            "left_view.load_or(0, packed_inner, 1)",
        ),
        (
            "column = element as u16 as usize",
            "column = element as u8 as usize",
        ),
    ] {
        assert!(SOURCE.contains(before), "missing mutation target: {before}");
        assert!(
            !contract(&SOURCE.replacen(before, after, 1)),
            "accepted mutation: {after}"
        );
    }
}

#[test]
fn current_source_sdk_pin_and_exact_wave64_target_are_closed_without_raw_memory_or_payload_fma() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"96204434680e1e65b657bb2c78dd7f85b155d512\"")
            .count(),
        2
    );
    assert!(manifest.contains("default = [\"gfx950\"]"));
    for forbidden in [
        "unsafe",
        "mul_add",
        "Mfma",
        "GpuSimd",
        "transmute",
        "as_ptr",
        "from_raw_parts",
        "memory::",
        "reduce_max",
        "Bf16::from_f32",
        "narrowed",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden source operation: {forbidden}"
        );
    }
    let host = include_str!("../src/host.rs");
    for forbidden in ["unsafe", "transmute", "as_ptr", "from_raw_parts", "mul_add"] {
        assert!(
            !host.contains(forbidden),
            "forbidden host operation: {forbidden}"
        );
    }
    let cpu = "gfx950";
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    let flags = format!("-Ctarget-cpu={cpu}\u{1f}-Ctarget-feature={features}");
    assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_ok());
    assert!(target_contract::validate_device_build("x86_64", "", cpu, features).is_ok());
    for flags in [
        String::new(),
        flags.replace(cpu, "gfx942"),
        flags.replace("-xnack", "+xnack"),
        format!("{flags}\u{1f}-Ctarget-cpu=gfx950"),
    ] {
        assert!(target_contract::validate_device_build("amdgpu", &flags, cpu, features).is_err());
    }
}
