use quote::{ToTokens, format_ident};
use syn::{Expr, Item, ItemFn, Stmt};

const SOURCE: &str = include_str!("../src/projection.rs");

fn source_contract(source: &str) -> Result<(), String> {
    let parsed = syn::parse_file(source).map_err(|error| error.to_string())?;
    if parsed
        .items
        .iter()
        .any(|item| !matches!(item, Item::Use(_) | Item::Fn(_)))
    {
        return Err("unexpected device item".into());
    }
    let functions = parsed
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if functions.len() != 1 {
        return Err("expected one device root".into());
    }
    let function = functions[0];
    let signature: ItemFn = syn::parse_quote! {
        pub fn ferric_qwen3_prefill32_m2_gate_up_bf16_r1(
            a: &[u16],
            weights_kn: &[u16],
            mut output_low: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
            mut output_high: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
            rows: u32,
            n: u32,
            k: u32,
            world_size: u32,
            projection: u32,
        ) {}
    };
    if function.sig.to_token_stream().to_string() != signature.sig.to_token_stream().to_string()
        || function.vis.to_token_stream().to_string() != "pub"
    {
        return Err("root signature changed".into());
    }
    let expected_kernel: syn::Attribute = syn::parse_quote! {
        #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [768, 1, 1]), control_flow(loop_bounds(256)))]
    };
    let kernels = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("kernel"))
        .collect::<Vec<_>>();
    if kernels.len() != 1
        || kernels[0].to_token_stream().to_string() != expected_kernel.to_token_stream().to_string()
        || function.attrs.iter().any(|attr| {
            !attr.path().is_ident("kernel")
                && !attr.path().is_ident("allow")
                && !attr.path().is_ident("doc")
        })
    {
        return Err("launch or control-flow contract changed".into());
    }
    let mut expected: syn::Block = syn::parse_quote! {
        {
            if rows != 32
                || n != 12288
                || k != 4096
                || world_size != 1
                || !(projection == 4 || projection == 5)
            {
                fe2o3_device::trap();
            }
            if a.len() != 32 * 4096
                || weights_kn.len() != 4096 * 12288
                || output_low.len() != 16 * 12288
                || output_high.len() != 16 * 12288
            {
                fe2o3_device::trap();
            }
            if thread::grid_dim_x() != 768 || thread::block_dim_x() != 64 {
                fe2o3_device::trap();
            }
            let invocation = thread::index_1d();
            let raw = invocation.get();
            let group = raw / 64;
            if group < 768 {
            } else {
                fe2o3_device::trap();
            }
            let group = group as u16 as usize;
            let lane = WaveLane::<Wave64>::current();
            let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 32, 4096, 4096) else {
                fe2o3_device::trap();
            };
            let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 12288, 12288) else {
                fe2o3_device::trap();
            };
            let matrix = DeviceMatrix::current();
            let mut accumulator_low = F32AccumulatorFragment::zero(&lane);
            let mut accumulator_high = F32AccumulatorFragment::zero(&lane);
            let mut step = 0_usize;
            while step < 256 {
                let a_low = left.load_m16k16(&lane, 0, step * 16);
                let a_high = left.load_m16k16(&lane, 16, step * 16);
                let b_low = right.load_k16n16(&lane, step * 16, group * 16);
                let b_high = right.load_k16n16(&lane, step * 16, group * 16);
                accumulator_low = matrix.multiply_accumulate(a_low, b_low, accumulator_low);
                accumulator_high = matrix.multiply_accumulate(a_high, b_high, accumulator_high);
                step += 1;
            }
            let [low_0, low_1, low_2, low_3] = accumulator_low.into_values();
            let [high_0, high_1, high_2, high_3] = accumulator_high.into_values();
            let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
                fe2o3_device::trap();
            };
        }
    };
    for half in ["low", "high"] {
        let output = format_ident!("output_{half}");
        for component in 0..4 {
            let value_name = format_ident!("{half}_{component}");
            let index = syn::LitInt::new(&component.to_string(), function.sig.ident.span());
            let pair: syn::Block = syn::parse_quote! {
                {
                    let value = Bf16::from_f32(#value_name);
                    if !#value_name.is_finite()
                        || !value.is_finite()
                        || !#output.write_tiled_2d(&tile, #index, 16, 12288, 12288, value.to_bits())
                    {
                        fe2o3_device::trap();
                    }
                }
            };
            expected.stmts.extend(pair.stmts);
        }
    }
    if function.block.to_token_stream().to_string() != expected.to_token_stream().to_string() {
        return Err("guard, coordinate, arithmetic order, or store contract changed".into());
    }
    Ok(())
}

fn rejected_replacement(from: &str, to: &str) {
    let changed = SOURCE.replace(from, to);
    assert_ne!(changed, SOURCE, "mutation must affect the source");
    assert!(source_contract(&changed).is_err());
}

fn mutated_loop(change: fn(&mut syn::Block)) -> String {
    let mut parsed = syn::parse_file(SOURCE).unwrap();
    let function = parsed
        .items
        .iter_mut()
        .find_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        })
        .unwrap();
    let repeated = function
        .block
        .stmts
        .iter_mut()
        .find_map(|statement| {
            if let Stmt::Expr(Expr::While(repeated), _) = statement {
                Some(repeated)
            } else {
                None
            }
        })
        .unwrap();
    change(&mut repeated.body);
    parsed.to_token_stream().to_string()
}

#[test]
fn actual_device_root_matches_the_complete_contract() {
    source_contract(SOURCE).unwrap();
}

#[test]
fn both_b_fragments_must_be_loaded_before_either_consuming_mma() {
    let changed = mutated_loop(|body| body.stmts.swap(3, 4));
    assert!(source_contract(&changed).is_err());
}

#[test]
fn each_half_retains_all_256_ascending_k16_accumulations() {
    for (from, to) in [
        ("let mut step = 0_usize;", "let mut step = 1_usize;"),
        ("while step < 256", "while step < 255"),
        ("step += 1;", "step += 2;"),
        ("step * 16", "(255 - step) * 16"),
    ] {
        rejected_replacement(from, to);
    }
}

#[test]
fn a_halves_and_shared_b_coordinates_cannot_be_reassigned() {
    rejected_replacement(
        "left.load_m16k16(&lane, 16, step * 16)",
        "left.load_m16k16(&lane, 0, step * 16)",
    );
    rejected_replacement("group * 16", "group * 16 + 16");
    rejected_replacement(
        "let group = group as u16 as usize;",
        "let group = group as u8 as usize;",
    );
}

#[test]
fn independent_accumulator_chains_and_single_final_narrowing_are_required() {
    rejected_replacement(
        "matrix.multiply_accumulate(a_high, b_high, accumulator_high)",
        "matrix.multiply_accumulate(a_high, b_high, accumulator_low)",
    );
    rejected_replacement(
        "let mut accumulator_high = F32AccumulatorFragment::zero(&lane);",
        "let mut accumulator_high = F32AccumulatorFragment::zero(&lane); let extra = Bf16::from_f32(0.0);",
    );
    rejected_replacement("Bf16::from_f32(low_0)", "Bf16::from_bits(0)");
}

#[test]
fn all_eight_stores_keep_finite_checks_components_and_distinct_views() {
    rejected_replacement("!value.is_finite()", "false");
    rejected_replacement("!high_3.is_finite()", "false");
    rejected_replacement("output_high.write_tiled_2d", "output_low.write_tiled_2d");
    rejected_replacement("&tile, 3, 16, 12288, 12288", "&tile, 2, 16, 12288, 12288");
}

#[test]
fn tail_padding_or_inexact_launch_guards_are_not_admitted() {
    rejected_replacement("rows != 32", "rows > 32");
    rejected_replacement("a.len() != 32 * 4096", "a.len() < 32 * 4096");
    rejected_replacement(
        "output_high.len() != 16 * 12288",
        "output_high.len() < 16 * 12288",
    );
    rejected_replacement("thread::grid_dim_x() != 768", "thread::grid_dim_x() > 768");
    rejected_replacement(
        "projection == 4 || projection == 5",
        "projection == 4 || projection == 6",
    );
}

#[test]
fn m32_typed_view_cannot_replace_the_two_m16_view_abi() {
    rejected_replacement(
        "Tiled2D<Index1D, 64, 16, 16, 4>",
        "Tiled2D<Index1D, 64, 32, 16, 8>",
    );
    rejected_replacement("max_grid = [768, 1, 1]", "max_grid = [1536, 1, 1]");
}
