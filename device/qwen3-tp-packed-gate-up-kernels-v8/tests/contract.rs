use quote::ToTokens;
use syn::{Item, ItemFn, ItemUse};

#[path = "../build/target_contract.rs"]
mod target_contract;

const FUSED: &str = include_str!("../src/fused_gate_up.rs");
const PACK: &str = include_str!("../src/activation_pack.rs");

fn tokens(source: &str) -> String {
    syn::parse_str::<syn::Macro>(&format!("fragment! {{ {source} }}"))
        .unwrap()
        .tokens
        .to_string()
}

fn exact_function(source: &str, import: ItemUse, expected: ItemFn) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if !file.attrs.is_empty() || file.items.len() != 2 {
        return false;
    }
    let (Item::Use(actual_import), Item::Fn(actual)) = (&file.items[0], &file.items[1]) else {
        return false;
    };
    let mut actual = actual.clone();
    actual
        .attrs
        .retain(|attribute| !attribute.path().is_ident("doc"));
    actual_import.to_token_stream().to_string() == import.to_token_stream().to_string()
        && actual.to_token_stream().to_string() == expected.to_token_stream().to_string()
}

fn fused_contract(source: &str) -> bool {
    exact_function(
        source,
        syn::parse_quote! {
            use fe2o3_device::{
                Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D,
                WriteOnlyDisjointSlice, kernel, thread,
            };
        },
        syn::parse_quote! {
            #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [12288, 1, 1]), control_flow(loop_bounds(32)))]
            pub fn ferric_qwen3_c1_gate_up_packed_u32_bf16_v8(
                a: &[u32], gate_weights: &[u32], up_weights: &[u32],
                mut gate_output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
                mut up_output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
            ) {
                if a.len() != 2048 || gate_weights.len() != 12288 * 2048
                    || up_weights.len() != 12288 * 2048 || gate_output.len() != 12288
                    || up_output.len() != 12288 || thread::launch_extent_1d() != 12288 * 64
                { fe2o3_device::trap(); }
                let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, 2048, 2048) else {
                    fe2o3_device::trap();
                };
                let Ok(gate_view) = StridedReadView2D::from_shared_slice(gate_weights, 0, 12288, 2048, 2048) else {
                    fe2o3_device::trap();
                };
                let Ok(up_view) = StridedReadView2D::from_shared_slice(up_weights, 0, 12288, 2048, 2048) else {
                    fe2o3_device::trap();
                };
                let invocation = thread::index_1d();
                let raw = invocation.get();
                let element = thread::block_idx_x() as usize;
                let lane = raw % 64;
                if element < 12288 {} else { fe2o3_device::trap(); }
                let column = element as u16 as usize;
                let subgroup = Gfx950Subgroup::current();
                let mut gate_partial = 0.0_f32;
                let mut up_partial = 0.0_f32;
                let mut gate_finite = true;
                let mut up_finite = true;
                let mut group = 0_usize;
                while group < 32 {
                    let packed_inner = group * 64 + lane;
                    let left_bits = left_view.load_or(0, packed_inner, 0);
                    let gate_bits = gate_view.load_or(column, packed_inner, 0);
                    let up_bits = up_view.load_or(column, packed_inner, 0);
                    {
                        let left = Bf16::from_bits(left_bits as u16).to_f32();
                        let gate = Bf16::from_bits(gate_bits as u16).to_f32();
                        let gate_product = left * gate;
                        gate_partial += gate_product;
                        gate_finite &= gate_product.is_finite() & gate_partial.is_finite();
                        let up = Bf16::from_bits(up_bits as u16).to_f32();
                        let up_product = left * up;
                        up_partial += up_product;
                        up_finite &= up_product.is_finite() & up_partial.is_finite();
                    }
                    {
                        let left = Bf16::from_bits((left_bits >> 16) as u16).to_f32();
                        let gate = Bf16::from_bits((gate_bits >> 16) as u16).to_f32();
                        let gate_product = left * gate;
                        gate_partial += gate_product;
                        gate_finite &= gate_product.is_finite() & gate_partial.is_finite();
                        let up = Bf16::from_bits((up_bits >> 16) as u16).to_f32();
                        let up_product = left * up;
                        up_partial += up_product;
                        up_finite &= up_product.is_finite() & up_partial.is_finite();
                    }
                    group += 1;
                }
                let gate_sum = subgroup.reduce_sum_f32::<64>(gate_partial);
                let up_sum = subgroup.reduce_sum_f32::<64>(up_partial);
                let gate_narrowed = Bf16::from_f32(gate_sum);
                let up_narrowed = Bf16::from_f32(up_sum);
                if !gate_finite || !up_finite || !gate_sum.is_finite() || !up_sum.is_finite()
                    || !gate_narrowed.is_finite() || !up_narrowed.is_finite()
                { fe2o3_device::trap(); }
                if lane == 0 {
                    let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
                        fe2o3_device::trap();
                    };
                    if !gate_output.write_row_striped_2d(&stripe, 0, 12288, 1, 1, gate_narrowed.to_bits()) {
                        fe2o3_device::trap();
                    }
                    if !up_output.write_row_striped_2d(&stripe, 0, 12288, 1, 1, up_narrowed.to_bits()) {
                        fe2o3_device::trap();
                    }
                }
            }
        },
    )
}

fn pack_contract(source: &str) -> bool {
    exact_function(
        source,
        syn::parse_quote! {
            use fe2o3_device::{Index1D, StridedReadView2D, WriteOnlyDisjointSlice, kernel, thread};
        },
        syn::parse_quote! {
            #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]))]
            pub fn ferric_qwen3_c1_gate_up_activation_pack_u32_v8(
                source: &[u16], mut output: WriteOnlyDisjointSlice<u32, Index1D>,
            ) {
                if source.len() != 4096 || output.len() != 2048 || thread::launch_extent_1d() != 2048 {
                    fe2o3_device::trap();
                }
                let Ok(view) = StridedReadView2D::from_shared_slice(source, 0, 1, 4096, 4096) else {
                    fe2o3_device::trap();
                };
                let invocation = thread::index_1d();
                let word = invocation.get();
                if word < 2048 {} else { fe2o3_device::trap(); }
                let group = word / 64;
                let lane = word % 64;
                let low = group * 128 + lane;
                let low_bits = view.load_or(0, low, 0);
                let high_bits = view.load_or(0, low + 64, 0);
                let packed = (low_bits as u32) | ((high_bits as u32) << 16);
                if !output.write(invocation, packed) { fe2o3_device::trap(); }
            }
        },
    )
}

#[test]
fn exact_fused_ast_pins_independent_strict_mac_order_collectives_and_trap_before_stores() {
    assert!(fused_contract(FUSED));
    assert!(fused_contract(
        &syn::parse_file(FUSED)
            .unwrap()
            .to_token_stream()
            .to_string()
    ));
}

#[test]
fn fused_ast_rejects_abi_bounds_load_mac_reduction_finite_and_output_mutations() {
    let source = tokens(FUSED);
    let reflowed = tokens(&FUSED.replace("||", "\n||\n").replace("&=", "\n&=\n"));
    assert_eq!(source, reflowed);
    for candidate in [source, reflowed] {
        assert!(fused_contract(&candidate));
        for (before, after) in [
            ("required = [64, 1, 1]", "required = [32, 1, 1]"),
            ("max_grid = [12288, 1, 1]", "max_grid = [12289, 1, 1]"),
            ("loop_bounds(32)", "loop_bounds(64)"),
            ("a.len() != 2048", "a.len() < 2048"),
            (
                "up_weights.len() != 12288 * 2048",
                "up_weights.len() < 12288 * 2048",
            ),
            ("gate_output.len() != 12288", "gate_output.len() < 12288"),
            (
                "launch_extent_1d() != 12288 * 64",
                "launch_extent_1d() > 12288 * 64",
            ),
            (
                "up_weights, 0, 12288, 2048, 2048",
                "gate_weights, 0, 12288, 2048, 2048",
            ),
            ("element < 12288", "element <= 12288"),
            (
                "left_view.load_or(0, packed_inner, 0)",
                "left_view.load_or(0, packed_inner + 1, 0)",
            ),
            (
                "up_view.load_or(column, packed_inner, 0)",
                "gate_view.load_or(column, packed_inner, 0)",
            ),
            ("left_bits as u16", "(left_bits >> 16) as u16"),
            ("gate_partial += gate_product", "gate_partial += up_partial"),
            (
                "up_partial += up_product",
                "up_partial = left.mul_add(up, up_partial)",
            ),
            (
                "gate_finite &= gate_product.is_finite() & gate_partial.is_finite()",
                "gate_finite = gate_partial.is_finite()",
            ),
            (
                "up_finite &= up_product.is_finite() & up_partial.is_finite()",
                "up_finite = gate_finite",
            ),
            (
                "reduce_sum_f32::<64>(up_partial)",
                "reduce_sum_f32::<32>(up_partial)",
            ),
            ("Bf16::from_f32(up_sum)", "Bf16::from_f32(gate_sum)"),
            ("!gate_finite || !up_finite", "!gate_finite && !up_finite"),
            (
                "!gate_narrowed.is_finite() || !up_narrowed.is_finite()",
                "false",
            ),
            ("lane == 0", "lane < 2"),
            ("up_narrowed.to_bits()", "gate_narrowed.to_bits()"),
        ] {
            let before = tokens(before);
            let after = tokens(after);
            assert!(
                candidate.contains(before.as_str()),
                "missing mutation: {before}"
            );
            assert!(
                !fused_contract(&candidate.replacen(before.as_str(), &after, 1)),
                "accepted mutation: {before}"
            );
        }
    }
    assert!(!fused_contract(&format!(
        "{FUSED}\npub fn hidden_extra() {{}}"
    )));
}

#[test]
fn exact_pack_ast_keeps_integer_bits_and_checks_exact_scratch_geometry() {
    assert!(pack_contract(PACK));
    let source = tokens(PACK);
    let reflowed = tokens(&PACK.replace("<<", "\n<<\n"));
    assert_eq!(source, reflowed);
    for candidate in [source, reflowed] {
        assert!(pack_contract(&candidate));
        for (before, after) in [
            ("source.len() != 4096", "source.len() < 4096"),
            ("output.len() != 2048", "output.len() < 2048"),
            ("word < 2048", "word <= 2048"),
            ("group * 128 + lane", "group * 64 + lane"),
            ("low + 64", "low + 1"),
            ("<< 16", "<< 15"),
            (
                "output.write(invocation, packed)",
                "output.write(invocation, 0)",
            ),
        ] {
            let before = tokens(before);
            let after = tokens(after);
            assert_eq!(candidate.matches(before.as_str()).count(), 1);
            assert!(!pack_contract(&candidate.replacen(
                before.as_str(),
                &after,
                1
            )));
        }
    }
    for forbidden in [
        "unsafe",
        "as_ptr",
        "from_raw_parts",
        "Bf16",
        "f32",
        "f64",
        "mul_add",
    ] {
        assert!(!PACK.contains(forbidden));
    }
}

#[test]
fn new_roots_are_explicitly_gated_and_use_the_fixed_sdk_not_historical_image_identity() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(manifest.matches("default = []").count(), 1);
    assert_eq!(
        manifest
            .matches("fused-gate-up-r1 = [\"gfx950\", \"dep:fe2o3-host\"]")
            .count(),
        1
    );
    let host = manifest
        .lines()
        .find(|line| line.starts_with("fe2o3-host = "))
        .unwrap();
    assert!(host.ends_with(", optional = true }"));
    assert_eq!(
        manifest
            .matches("rev = \"b7d5f2bf7bf9e66037df1ff1d7ca2738ffdf98dd\"")
            .count(),
        2
    );
    let library = syn::parse_file(include_str!("../src/lib.rs")).unwrap();
    let gated: syn::Attribute = syn::parse_quote!(#[cfg(feature = "fused-gate-up-r1")]);
    for name in ["activation_pack", "fused_gate_up"] {
        let module = library
            .items
            .iter()
            .find_map(|item| match item {
                Item::Mod(module) if module.ident == name => Some(module),
                _ => None,
            })
            .unwrap();
        assert_eq!(module.attrs.len(), 1);
        assert_eq!(
            module.attrs[0].to_token_stream().to_string(),
            gated.to_token_stream().to_string()
        );
    }
    let roster = library
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == "compiler_expectation_roster" => {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let selected: syn::Attribute = syn::parse_quote!(#[cfg(all(feature = "fused-gate-up-r1", not(target_arch = "amdgpu"), not(test)))]);
    assert_eq!(
        roster.attrs[0].to_token_stream().to_string(),
        selected.to_token_stream().to_string()
    );
    let body = roster.block.to_token_stream().to_string();
    for marker in [
        "fused_gate_up::ferric_qwen3_c1_gate_up_packed_u32_bf16_v8_gpu::Marker",
        "activation_pack::ferric_qwen3_c1_gate_up_activation_pack_u32_v8_gpu::Marker",
    ] {
        assert_eq!(
            body.matches(
                &syn::parse_str::<syn::Path>(marker)
                    .unwrap()
                    .to_token_stream()
                    .to_string()
            )
            .count(),
            1
        );
    }
    assert!(!body.contains("packed_u32_bf16_r2_gpu"));
}

#[test]
fn device_target_contract_rejects_missing_duplicate_or_foreign_cpu_and_features() {
    let good = "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack";
    assert_eq!(
        target_contract::validate_device_build("amdgpu", good),
        Ok(())
    );
    for bad in [
        String::new(),
        good.replace("gfx950", "gfx942"),
        good.replace(
            "-wavefrontsize32,+wavefrontsize64,-xnack",
            "+wavefrontsize32,-wavefrontsize64,-xnack",
        ),
        format!("{good}\u{1f}-Ctarget-cpu=gfx950"),
        format!("{good}\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack"),
    ] {
        assert!(target_contract::validate_device_build("amdgpu", &bad).is_err());
    }
    assert_eq!(target_contract::validate_device_build("x86_64", ""), Ok(()));
}
