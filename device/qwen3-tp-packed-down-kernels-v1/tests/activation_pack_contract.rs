use quote::ToTokens;
use syn::{Item, ItemFn, ItemUse};

const SOURCE: &str = include_str!("../src/activation_pack.rs");

fn contract(source: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if file.items.len() != 2 || !file.attrs.is_empty() {
        return false;
    }
    let (Item::Use(import), Item::Fn(actual)) = (&file.items[0], &file.items[1]) else {
        return false;
    };
    let expected_import: ItemUse = syn::parse_quote! {
        use fe2o3_device::{Index1D, StridedReadView2D, WriteOnlyDisjointSlice, kernel, thread};
    };
    let mut actual = actual.clone();
    actual
        .attrs
        .retain(|attribute| !attribute.path().is_ident("doc"));
    let expected: ItemFn = syn::parse_quote! {
        #[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [96, 1, 1]))]
        pub fn ferric_qwen3_c1_down_activation_pack_u32_r1(
            source: &[u16],
            mut output: WriteOnlyDisjointSlice<u32, Index1D>,
            rows: u32,
            k: u32,
        ) {
            if rows != 1 || k != 12288 { fe2o3_device::trap(); }
            if source.len() < 12288 || source.len() > 32 * 12288
                || output.len() < 6144 || output.len() > 32 * 6144
                || thread::launch_extent_1d() != 6144
            { fe2o3_device::trap(); }
            let Ok(view) = StridedReadView2D::from_shared_slice(source, 0, 1, 12288, 12288) else {
                fe2o3_device::trap();
            };
            let invocation = thread::index_1d();
            let word = invocation.get();
            if word < 6144 {} else { fe2o3_device::trap(); }
            let group = word / 64;
            let lane = word % 64;
            let low = group * 128 + lane;
            let low_bits = view.load_or(0, low, 0);
            let high_bits = view.load_or(0, low + 64, 0);
            let packed = (low_bits as u32) | ((high_bits as u32) << 16);
            if !output.write(invocation, packed) { fe2o3_device::trap(); }
        }
    };
    import.to_token_stream().to_string() == expected_import.to_token_stream().to_string()
        && actual.to_token_stream().to_string() == expected.to_token_stream().to_string()
}

#[test]
fn exact_typed_pack_payload_binds_launch_bounds_layout_and_disjoint_store_ast() {
    assert!(contract(SOURCE));
    assert!(contract(
        &syn::parse_file(SOURCE)
            .unwrap()
            .to_token_stream()
            .to_string()
    ));
}

#[test]
fn source_contract_rejects_shape_bounds_mapping_bit_order_and_store_changes() {
    for (before, after) in [
        ("required = [64, 1, 1]", "required = [32, 1, 1]"),
        ("max_grid = [96, 1, 1]", "max_grid = [97, 1, 1]"),
        ("rows != 1", "rows != 2"),
        ("k != 12288", "k != 4096"),
        ("source.len() < 12288", "source.len() < 12287"),
        ("source.len() > 32 * 12288", "source.len() > 33 * 12288"),
        ("output.len() < 6144", "output.len() < 6143"),
        ("output.len() > 32 * 6144", "output.len() > 33 * 6144"),
        ("launch_extent_1d() != 6144", "launch_extent_1d() != 6145"),
        ("source, 0, 1, 12288, 12288", "source, 1, 1, 12288, 12288"),
        ("word < 6144", "word <= 6144"),
        ("word / 64", "word / 32"),
        ("word % 64", "word % 32"),
        ("group * 128 + lane", "group * 64 + lane"),
        ("low + 64", "low + 1"),
        ("low_bits as u32", "high_bits as u32"),
        ("<< 16", "<< 15"),
        (
            "output.write(invocation, packed)",
            "output.write(invocation, 0)",
        ),
    ] {
        assert_eq!(
            SOURCE.matches(before).count(),
            1,
            "missing unique mutation: {before}"
        );
        assert!(
            !contract(&SOURCE.replacen(before, after, 1)),
            "accepted mutation: {before}"
        );
    }
    assert!(!contract(&format!("{SOURCE}\npub fn hidden_extra() {{}}")));
}

#[test]
fn host_model_and_marker_roster_use_existing_current_sdk_without_raw_memory() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"96204434680e1e65b657bb2c78dd7f85b155d512\"")
            .count(),
        2
    );
    let host = include_str!("../src/activation_pack_host.rs");
    for text in [SOURCE, host] {
        for forbidden in [
            "unsafe",
            "transmute",
            "as_ptr",
            "from_raw_parts",
            "Bf16",
            "f32",
            "f64",
            "mul_add",
        ] {
            assert!(
                !text.contains(forbidden),
                "forbidden authored operation: {forbidden}"
            );
        }
    }
    let library = syn::parse_file(include_str!("../src/lib.rs")).unwrap();
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
    let body = roster.block.to_token_stream().to_string();
    for marker in [
        "projection::ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1_gpu::Marker",
        "activation_pack::ferric_qwen3_c1_down_activation_pack_u32_r1_gpu::Marker",
    ] {
        let marker = syn::parse_str::<syn::Path>(marker)
            .unwrap()
            .to_token_stream()
            .to_string();
        assert_eq!(body.matches(&marker).count(), 1);
    }
}
