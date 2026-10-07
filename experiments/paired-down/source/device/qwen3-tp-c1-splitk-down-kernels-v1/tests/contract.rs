use fe2o3_device::{Bf16MfmaAMatrix, Bf16MfmaBMatrix};
use ferric_qwen3_tp_c1_splitk_down_kernels_device_v1::{
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
        pub fn ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1(
            a: &[u16], weights_kn: &[u16],
            mut partials: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
            rows: u32, n: u32, k: u32, world_size: u32, projection: u32,
        ) {}
    };
    let merge_signature: syn::ItemFn = syn::parse_quote! {
        pub fn ferric_qwen3_c1_down_splitk8_merge_f32_r1(
            partials: &[f32], mut output: WriteOnlyDisjointSlice<f32, Index1D>,
        ) {}
    };
    for (actual, expected) in functions.iter().zip([partial_signature, merge_signature]) {
        if actual.sig.to_token_stream().to_string() != expected.sig.to_token_stream().to_string() {
            return false;
        }
    }
    let expected = [
        "rows != 1",
        "n != 4096",
        "k != 12288",
        "world_size != 1",
        "projection != 2",
        "a.len() < 12288",
        "a.len() > 32 * 12288",
        "weights_kn.len() != 12288 * 4096",
        "partials.len() != 8 * 4096",
        "thread::grid_dim_x() != 2048",
        "thread::block_dim_x() != 64",
        "group = raw / 64",
        "partition = group / 256",
        "tile_column = group % 256",
        "partition < 8 && tile_column < 256",
        "partition as u8 as usize",
        "tile_column as u8 as usize",
        "Bf16MfmaAMatrix::row_major(a, 0, 1, 12288, 12288)",
        "Bf16MfmaBMatrix::row_major(weights_kn, 0, 12288, 4096, 4096)",
        "reduction_base = partition * 1536 + pair * 32",
        "left.load_m16k16(&lane, 0, reduction_base)",
        "right.load_k16n16(&lane, reduction_base, tile_column * 16)",
        "matrix.multiply_accumulate(a_fragment, b_fragment, accumulator)",
        "let [value_0, _, _, _] = accumulator.into_values()",
        "raw % 64 < 16",
        "!value_0.is_finite()",
        "partials.write_row_striped_2d(&stripe, 0, 2048, 16, 16, value_0)",
        "output.len() < 4096",
        "output.len() > 32 * 4096",
        "thread::grid_dim_x() != 64",
        "StridedReadView2D::from_shared_slice(partials, 0, 8, 4096, 4096)",
        "column < 4096",
        "view.load_or(partition, column, f32::INFINITY)",
        "sum += value",
        "finite &= value.is_finite() & sum.is_finite()",
        "!finite || !output.write(invocation, sum)",
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
        && loops[0].cond.to_token_stream().to_string() == "pair < 48"
        && loops[1].cond.to_token_stream().to_string() == "partition < 8"
        && tokens.matches("multiply_accumulate (").count() == 2
        && tokens.matches("write_row_striped_2d (").count() == 1
        && ![
            "unsafe",
            "mul_add",
            "as_ptr",
            "from_raw_parts",
            "Bf16::from_f32",
        ]
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
        ("pair < 48", "pair < 47"),
        (
            "partition * 1536 + pair * 32",
            "partition * 1520 + pair * 32",
        ),
        ("raw % 64 < 16", "raw % 64 <= 16"),
        ("2048, 16, 16, value_0", "2048, 16, 17, value_0"),
        ("sum += value", "sum = value + sum"),
        ("world_size != 1", "world_size != 2"),
        ("projection != 2", "projection != 1"),
        ("a: &[u16]", "a: &[u32]"),
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
    assert_eq!(EXPERIMENT_ENABLED, cfg!(feature = "splitk8-down-r1"));
    assert_eq!(
        manifest
            .matches("rev = \"84ab85a424c821c9c2eca155baa5e33ea064349a\"")
            .count(),
        2
    );
}

#[cfg(feature = "splitk8-down-r1")]
#[test]
fn generated_roster_contains_exactly_two_distinct_roots() {
    let roster = ferric_qwen3_tp_c1_splitk_down_kernels_device_v1::compiler_expectation_roster();
    assert_eq!(roster.len(), 2);
    assert_eq!(roster[0].export_name(), PARTIAL_ROOT);
    assert_eq!(roster[1].export_name(), MERGE_ROOT);
    assert_ne!(roster[0].export_name(), roster[1].export_name());
}

#[test]
fn actual_checked_matrix_constructors_reject_undersized_storage() {
    let a = [0_u16; 12288];
    assert!(Bf16MfmaAMatrix::row_major(&a, 0, 1, 12288, 12288).is_ok());
    assert!(Bf16MfmaAMatrix::row_major(&a[..12287], 0, 1, 12288, 12288).is_err());
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
