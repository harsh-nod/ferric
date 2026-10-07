use quote::{ToTokens, format_ident};
use syn::{Expr, Item, ItemFn, Stmt};

const SOURCE: &str = include_str!("../src/projection.rs");
const ROOTS: [&str; 2] = [
    "ferric_qwen3_prefill32_down_k16_control_f32_r1",
    "ferric_qwen3_prefill32_down_k16_paired_f32_r1",
];

fn expected(root: &str, paired: bool) -> ItemFn {
    let name = format_ident!("{root}");
    let mut function: ItemFn = syn::parse_quote! {
        pub fn #name(
            a: &[u16],
            weights_kn: &[u16],
            mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
            rows: u32,
            n: u32,
            k: u32,
            world_size: u32,
            projection: u32,
        ) {
            if rows != 32 || n != 4096 || k != 12288 || world_size != 1 || projection != 2 {
                fe2o3_device::trap();
            }
            if a.len() != 32 * 12288
                || weights_kn.len() != 12288 * 4096
                || output.len() != 32 * 4096
            {
                fe2o3_device::trap();
            }
            if thread::grid_dim_x() != 512 || thread::block_dim_x() != 64 {
                fe2o3_device::trap();
            }
            let invocation = thread::index_1d();
            let group = invocation.get() / 64;
            if group < 512 {
            } else {
                fe2o3_device::trap();
            }
            let tile_row = (group / 256) as u8 as usize;
            let tile_column = (group % 256) as u8 as usize;
            let lane = WaveLane::<Wave64>::current();
            let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 32, 12288, 12288) else {
                fe2o3_device::trap();
            };
            let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 12288, 4096, 4096) else {
                fe2o3_device::trap();
            };
            let matrix = DeviceMatrix::current();
            let mut accumulator = F32AccumulatorFragment::zero(&lane);
        }
    };
    let update: syn::Block = if paired {
        syn::parse_quote! {
            {
                let mut pair = 0_usize;
                while pair < 384 {
                    let reduction_base = pair * 32;
                    let next_reduction_base = reduction_base + 16;
                    let a_fragment = left.load_m16k16(&lane, tile_row * 16, reduction_base);
                    let b_fragment = right.load_k16n16(&lane, reduction_base, tile_column * 16);
                    let next_a_fragment = left.load_m16k16(&lane, tile_row * 16, next_reduction_base);
                    let next_b_fragment = right.load_k16n16(&lane, next_reduction_base, tile_column * 16);
                    accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
                    accumulator = matrix.multiply_accumulate(next_a_fragment, next_b_fragment, accumulator);
                    pair += 1;
                }
            }
        }
    } else {
        syn::parse_quote! {
            {
                let mut step = 0_usize;
                while step < 768 {
                    let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16);
                    let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
                    accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
                    step += 1;
                }
            }
        }
    };
    function.block.stmts.extend(update.stmts);
    let finish: syn::Block = syn::parse_quote! {
        {
            let [value_0, value_1, value_2, value_3] = accumulator.into_values();
            let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
                fe2o3_device::trap();
            };
        }
    };
    function.block.stmts.extend(finish.stmts);
    for component in 0..4 {
        let value = format_ident!("value_{component}");
        let index = syn::LitInt::new(&component.to_string(), name.span());
        let store: syn::Block = syn::parse_quote! {
            {
                if !#value.is_finite()
                    || !output.write_tiled_2d(&tile, #index, 32, 4096, 4096, #value)
                {
                    fe2o3_device::trap();
                }
            }
        };
        function.block.stmts.extend(store.stmts);
    }
    function
}

fn source_contract(source: &str) -> Result<(), String> {
    let parsed = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut functions = Vec::new();
    for item in parsed.items {
        match item {
            Item::Fn(function) => functions.push(function),
            Item::Use(_) => {},
            _ => return Err("unexpected device item".into()),
        }
    }
    if functions.len() != 2 {
        return Err("expected exactly two device roots".into());
    }
    for (index, function) in functions.iter().enumerate() {
        let expected = expected(ROOTS[index], index == 1);
        if function.sig.to_token_stream().to_string() != expected.sig.to_token_stream().to_string()
            || function.vis.to_token_stream().to_string() != "pub"
            || function.block.to_token_stream().to_string() != expected.block.to_token_stream().to_string()
        {
            return Err("root, guard, indexing, MFMA order or FP32 store contract changed".into());
        }
        let bound = syn::LitInt::new(if index == 0 { "768" } else { "384" }, function.sig.ident.span());
        let expected_kernel: syn::Attribute = syn::parse_quote! {
            #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [512, 1, 1]), control_flow(loop_bounds(#bound)))]
        };
        let kernels = function.attrs.iter().filter(|attr| attr.path().is_ident("kernel")).collect::<Vec<_>>();
        if kernels.len() != 1
            || kernels[0].to_token_stream().to_string() != expected_kernel.to_token_stream().to_string()
            || function.attrs.iter().any(|attr| !attr.path().is_ident("kernel")
                && !attr.path().is_ident("allow") && !attr.path().is_ident("doc"))
        {
            return Err("launch, feature or loop-bound contract changed".into());
        }
    }
    Ok(())
}

fn reject(from: &str, to: &str) {
    let changed = SOURCE.replace(from, to);
    assert_ne!(changed, SOURCE, "mutation must change source: {from}");
    assert!(source_contract(&changed).is_err(), "mutation must be rejected: {from}");
}

#[test]
fn both_roots_match_the_complete_independent_contract() {
    source_contract(SOURCE).unwrap();
}

#[test]
fn wrong_shape_extent_launch_or_projection_is_rejected() {
    for (from, to) in [
        ("rows != 32", "rows > 32"),
        ("k != 12288", "k != 4096"),
        ("world_size != 1", "world_size != 2"),
        ("projection != 2", "projection != 1"),
        ("output.len() != 32 * 4096", "output.len() < 32 * 4096"),
        ("a.len() != 32 * 12288", "a.len() < 32 * 12288"),
        ("weights_kn.len() != 12288 * 4096", "weights_kn.len() < 12288 * 4096"),
        ("thread::grid_dim_x() != 512", "thread::grid_dim_x() > 512"),
        ("max_grid = [512, 1, 1]", "max_grid = [1024, 1, 1]"),
    ] {
        reject(from, to);
    }
}

#[test]
fn operand_index_and_accumulator_mutations_are_rejected() {
    for (from, to) in [
        ("group / 256", "group / 128"),
        ("group % 256", "group % 128"),
        ("next_reduction_base = reduction_base + 16", "next_reduction_base = reduction_base + 32"),
        ("next_a_fragment, next_b_fragment, accumulator", "a_fragment, next_b_fragment, accumulator"),
        ("while step < 768", "while step < 767"),
        ("while pair < 384", "while pair < 383"),
        ("pair += 1", "pair += 2"),
        ("loop_bounds(384)", "loop_bounds(768)"),
    ] {
        reject(from, to);
    }
}

#[test]
fn moving_an_update_before_the_second_fragment_load_is_rejected() {
    let mut parsed = syn::parse_file(SOURCE).unwrap();
    let function = parsed.items.iter_mut().find_map(|item| match item {
        Item::Fn(function) if function.sig.ident == ROOTS[1] => Some(function),
        _ => None,
    }).unwrap();
    let repeated = function.block.stmts.iter_mut().find_map(|statement| match statement {
        Stmt::Expr(Expr::While(repeated), _) => Some(repeated),
        _ => None,
    }).unwrap();
    repeated.body.stmts.swap(5, 6);
    assert!(source_contract(&parsed.to_token_stream().to_string()).is_err());
}

#[test]
fn stores_keep_finite_checks_fp32_precision_and_unique_components() {
    for (from, to) in [
        ("!value_3.is_finite()", "false"),
        ("&tile, 2, 32, 4096, 4096, value_2", "&tile, 1, 32, 4096, 4096, value_2"),
        ("WriteOnlyDisjointSlice<f32", "WriteOnlyDisjointSlice<u16"),
        ("let [value_0, value_1, value_2, value_3] = accumulator.into_values();",
         "let [value_0, value_1, value_2, value_3] = accumulator.into_values(); let value_0 = Bf16::from_f32(value_0).to_f32();"),
    ] {
        reject(from, to);
    }
}

#[test]
fn extra_roots_helpers_or_modules_are_rejected() {
    for suffix in ["fn extra() {}", "mod extra {}", "pub const UNCHECKED: bool = true;"] {
        assert!(source_contract(&format!("{SOURCE}\n{suffix}")).is_err());
    }
}
