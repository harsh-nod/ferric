use fe2o3_device::{Bf16MfmaAMatrix, Bf16MfmaBMatrix};
use ferric_qwen3_tp_c1_splitk_gate_up_kernels_device_v1::{
    EXPERIMENT_ENABLED, MERGE_ROOT, PARTIAL_ROOT,
};
use quote::ToTokens;
use syn::{Expr, Item, Stmt};

#[path = "../build/target_contract.rs"]
mod target_contract;

const SOURCE: &str = include_str!("../src/projection.rs");

fn source_contract(source: &str) -> bool {
    let Ok(parsed) = syn::parse_file(source) else {
        return false;
    };
    let functions: Vec<_> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        })
        .collect();
    if functions.len() != 2
        || functions[0].sig.ident != PARTIAL_ROOT
        || functions[1].sig.ident != MERGE_ROOT
    {
        return false;
    }
    let partial_signature: syn::ItemFn = syn::parse_quote! {
        pub fn ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1(
            a: &[u16], weights_kn: &[u16],
            mut partials: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
            rows: u32, n: u32, k: u32, world_size: u32, projection: u32,
        ) {}
    };
    let merge_signature: syn::ItemFn = syn::parse_quote! {
        pub fn ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1(
            partials: &[f32], mut output: WriteOnlyDisjointSlice<u16, Index1D>,
        ) {}
    };
    for (actual, expected) in functions.iter().zip([partial_signature, merge_signature]) {
        if actual.sig.to_token_stream().to_string() != expected.sig.to_token_stream().to_string() {
            return false;
        }
    }
    let launch_attributes: [syn::Attribute; 2] = [
        syn::parse_quote!(#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [3072, 1, 1]), control_flow(loop_bounds(64)))]),
        syn::parse_quote!(#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [192, 1, 1]), control_flow(loop_bounds(4)))]),
    ];
    for (actual, expected) in functions.iter().zip(launch_attributes) {
        let attributes: Vec<_> = actual
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("kernel"))
            .collect();
        if attributes.len() != 1
            || attributes[0].to_token_stream().to_string() != expected.to_token_stream().to_string()
        {
            return false;
        }
    }
    let guards: [Expr; 2] = [
        syn::parse_quote!(
            rows != 1
                || n != 12288
                || k != 4096
                || world_size != 1
                || !(projection == 4 || projection == 5)
                || a.len() < 4096
                || a.len() > 32 * 4096
                || weights_kn.len() != 4096 * 12288
                || partials.len() != 4 * 12288
                || thread::grid_dim_x() != 3072
                || thread::block_dim_x() != 64
        ),
        syn::parse_quote!(
            partials.len() != 4 * 12288
                || output.len() < 12288
                || output.len() > 32 * 12288
                || thread::grid_dim_x() != 192
                || thread::block_dim_x() != 64
        ),
    ];
    let reject: syn::Block = syn::parse_quote!({
        fe2o3_device::trap();
    });
    for (function, guard) in functions.iter().zip(guards) {
        let Some(Stmt::Expr(Expr::If(actual), _)) = function.block.stmts.first() else {
            return false;
        };
        if actual.cond.to_token_stream().to_string() != guard.to_token_stream().to_string()
            || actual.then_branch.to_token_stream().to_string()
                != reject.to_token_stream().to_string()
            || actual.else_branch.is_some()
        {
            return false;
        }
    }
    let expected = [
        "rows != 1",
        "n != 12288",
        "k != 4096",
        "world_size != 1",
        "!(projection == 4 || projection == 5)",
        "a.len() < 4096",
        "a.len() > 32 * 4096",
        "weights_kn.len() != 4096 * 12288",
        "partials.len() != 4 * 12288",
        "thread::grid_dim_x() != 3072",
        "thread::block_dim_x() != 64",
        "group = raw / 64",
        "partition = group / 768",
        "tile_column = group % 768",
        "partition < 4 && tile_column < 768",
        "partition as u8 as usize",
        "tile_column as u16 as usize",
        "Bf16MfmaAMatrix::row_major(a, 0, 1, 4096, 4096)",
        "Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 12288, 12288)",
        "reduction_base = partition * 1024 + step * 16",
        "let mut step = 0_usize",
        "step += 1",
        "left.load_m16k16(&lane, 0, reduction_base)",
        "right.load_k16n16(&lane, reduction_base, tile_column * 16)",
        "matrix.multiply_accumulate(a_fragment, b_fragment, accumulator)",
        "let [value_0, _, _, _] = accumulator.into_values()",
        "raw % 64 < 16",
        "!value_0.is_finite()",
        "partials.write_row_striped_2d(&stripe, 0, 3072, 16, 16, value_0)",
        "output.len() < 12288",
        "output.len() > 32 * 12288",
        "thread::grid_dim_x() != 192",
        "StridedReadView2D::from_shared_slice(partials, 0, 4, 12288, 12288)",
        "column < 12288",
        "view.load_or(partition, column, f32::INFINITY)",
        "sum += value",
        "finite &= value.is_finite() & sum.is_finite()",
        "let mut partition = 0_usize",
        "partition += 1",
        "let value = Bf16::from_f32(sum)",
        "!finite || !value.is_finite() || !output.write(invocation, value.to_bits())",
    ];
    let tokens = parsed.to_token_stream().to_string();
    if !expected.iter().all(|expected| {
        let statement = syn::parse_str::<Stmt>(&format!("{expected};")).unwrap();
        let fragment = statement.to_token_stream().to_string();
        tokens.contains(fragment.trim_end_matches(';').trim_end())
    }) {
        return false;
    }
    let loops: Vec<_> = functions
        .iter()
        .flat_map(|function| &function.block.stmts)
        .filter_map(|statement| match statement {
            Stmt::Expr(Expr::While(repeated), _) => Some(repeated),
            _ => None,
        })
        .collect();
    loops.len() == 2
        && loops[0].cond.to_token_stream().to_string() == "step < 64"
        && loops[1].cond.to_token_stream().to_string() == "partition < 4"
        && tokens.matches("multiply_accumulate (").count() == 1
        && tokens.matches("write_row_striped_2d (").count() == 1
        && tokens.matches("Bf16 :: from_f32 (").count() == 1
        && !["unsafe", "mul_add", "as_ptr", "from_raw_parts"]
            .iter()
            .any(|forbidden| source.contains(forbidden))
}

#[test]
fn source_keeps_fixed_shape_checked_views_and_ordered_fp32_merge() {
    assert!(source_contract(SOURCE));
}

#[test]
fn source_contract_rejects_partition_store_and_precision_mutations() {
    for (before, after) in [
        ("step < 64", "step < 63"),
        ("let mut step = 0_usize", "let mut step = 1_usize"),
        ("step += 1", "step += 2"),
        ("let mut partition = 0_usize", "let mut partition = 1_usize"),
        ("partition += 1", "partition += 2"),
        ("fe2o3_device::trap();", "return;"),
        (
            "partition * 1024 + step * 16",
            "partition * 1008 + step * 16",
        ),
        ("raw % 64 < 16", "raw % 64 <= 16"),
        ("3072, 16, 16, value_0", "3072, 16, 17, value_0"),
        ("sum += value", "sum = value + sum"),
        ("world_size != 1", "world_size != 2"),
        ("!(projection == 4 || projection == 5)", "projection != 1"),
        ("a: &[u16]", "a: &[u32]"),
        ("tile_column as u16 as usize", "tile_column as u8 as usize"),
        ("partition < 4", "partition < 3"),
        ("rows != 1", "rows != 2"),
        ("n != 12288", "n != 4096"),
        ("k != 4096", "k != 12288"),
        ("a.len() < 4096", "a.len() < 4095"),
        (
            "weights_kn.len() != 4096 * 12288",
            "weights_kn.len() < 4096 * 12288",
        ),
        ("partials.len() != 4 * 12288", "partials.len() < 4 * 12288"),
        ("output.len() < 12288", "output.len() < 12287"),
        ("!value.is_finite()", "false"),
        ("loop_bounds(64)", "loop_bounds(63)"),
        ("max_grid = [3072, 1, 1]", "max_grid = [3073, 1, 1]"),
        (
            "view.load_or(partition, column, f32::INFINITY)",
            "view.load_or(partition, column, 0.0)",
        ),
    ] {
        assert!(SOURCE.contains(before));
        assert!(
            !source_contract(&SOURCE.replacen(before, after, 1)),
            "accepted {after}"
        );
    }
}

#[test]
fn default_feature_is_disabled_and_source_pin_is_explicit() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("default = []"));
    assert_eq!(EXPERIMENT_ENABLED, cfg!(feature = "splitk4-gate-up-r1"));
    assert_eq!(
        manifest
            .matches("rev = \"55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9\"")
            .count(),
        2
    );
}

#[cfg(feature = "splitk4-gate-up-r1")]
#[test]
fn generated_roster_contains_exactly_two_distinct_roots() {
    let roster = ferric_qwen3_tp_c1_splitk_gate_up_kernels_device_v1::compiler_expectation_roster();
    assert_eq!(roster.len(), 2);
    assert_eq!(roster[0].export_name(), PARTIAL_ROOT);
    assert_eq!(roster[1].export_name(), MERGE_ROOT);
    assert_ne!(roster[0].export_name(), roster[1].export_name());
}

#[test]
fn actual_checked_matrix_constructors_reject_undersized_storage() {
    let a = [0_u16; 4096];
    assert!(Bf16MfmaAMatrix::row_major(&a, 0, 1, 4096, 4096).is_ok());
    assert!(Bf16MfmaAMatrix::row_major(&a[..4095], 0, 1, 4096, 4096).is_err());
    let b = [0_u16; 16 * 16];
    assert!(Bf16MfmaBMatrix::row_major(&b, 0, 16, 16, 16).is_ok());
    assert!(Bf16MfmaBMatrix::row_major(&b[..255], 0, 16, 16, 16).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&b, 0, 16, 16, 15).is_err());
}

#[test]
fn target_contract_rejects_wrong_cpu_wave_size_or_duplicate_flags() {
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    let valid = format!("-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature={features}");
    assert!(target_contract::validate_device_build("amdgpu", &valid, "gfx950", features).is_ok());
    for flags in [
        String::new(),
        valid.replace("gfx950", "gfx942"),
        valid.replace(
            "-wavefrontsize32,+wavefrontsize64",
            "+wavefrontsize32,-wavefrontsize64",
        ),
        format!("{valid}\u{1f}-Ctarget-cpu=gfx950"),
    ] {
        assert!(
            target_contract::validate_device_build("amdgpu", &flags, "gfx950", features).is_err()
        );
    }
}
