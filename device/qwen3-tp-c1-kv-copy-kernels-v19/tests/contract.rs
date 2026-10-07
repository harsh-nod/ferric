use ferric_qwen3_tp_c1_kv_copy_kernels_device_v19::{ROOTS_V19, compiler_expectation_roster_v19};
use quote::ToTokens;
use syn::{Item, ItemFn};

const SOURCE: &str = include_str!("../src/copy.rs");

fn root(source: &str) -> ItemFn {
    let mut functions = syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| {
            if let Item::Fn(function) = item {
                Some(function)
            } else {
                None
            }
        });
    let function = functions.next().unwrap();
    assert!(functions.next().is_none());
    function
}

#[test]
fn copy_v19_has_one_distinct_typed_root_and_exact_scalar_and_slice_abi() {
    let function = root(SOURCE);
    assert_eq!(function.sig.ident, ROOTS_V19[0]);
    let expected: ItemFn = syn::parse_quote! {
        fn ferric_qwen3_tp_c1_kv_copy_bf16_v19(
            key: &[u16], value: &[u16], mut key_slot: WriteOnlyDisjointSlice<u16>,
            mut value_slot: WriteOnlyDisjointSlice<u16>, position: u32,
            physical_page: u32, physical_pages: u32,
        ) {}
    };
    assert_eq!(
        function.sig.to_token_stream().to_string(),
        expected.sig.to_token_stream().to_string()
    );
    let launch: syn::Attribute = syn::parse_quote!(#[kernel(typed,
        launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [16, 1, 1]))]);
    let actual = function
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .unwrap();
    assert_eq!(
        actual.to_token_stream().to_string(),
        launch.to_token_stream().to_string()
    );
    let roster = compiler_expectation_roster_v19();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V19[0]);
    assert_ne!(ROOTS_V19[0], "ferric_qwen3_tp_batch32_paged_kv_append_v5");
}

#[test]
fn copy_v19_checks_bounds_before_loads_and_preserves_raw_bits_without_serial_loops() {
    let function = root(SOURCE);
    let expected: syn::Block = syn::parse_quote!({
        if position >= 8192
            || physical_pages == 0
            || physical_pages > 512
            || physical_page >= physical_pages
            || key.len() != 1024
            || value.len() != 1024
            || key_slot.len() != 1024
            || value_slot.len() != 1024
            || thread::launch_extent_1d() != 1024
        {
            fe2o3_device::trap();
        }
        let key_invocation = thread::index_1d();
        let index = key_invocation.get();
        if index < 1024 {
        } else {
            fe2o3_device::trap();
        }
        let key_bits = memory::volatile_load(key, index);
        let value_bits = memory::volatile_load(value, index);
        if !key_slot.write(key_invocation, key_bits) {
            fe2o3_device::trap();
        }
        if !value_slot.write(thread::index_1d(), value_bits) {
            fe2o3_device::trap();
        }
    });
    assert_eq!(
        function.block.to_token_stream().to_string(),
        expected.to_token_stream().to_string()
    );
}

#[test]
fn copy_v19_pins_the_independent_producer_and_target() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"5a503c04f5ae107a3b3e951ec970b36c5d6a9a79\"")
            .count(),
        2
    );
    assert!(manifest.contains("default = [\"gfx950\"]"));
    assert!(include_str!("../src/lib.rs").contains("not(feature = \"gfx950\")"));
    assert!(!SOURCE.contains("unsafe"));
    assert!(!SOURCE.contains("grid_leader"));
    assert!(!SOURCE.contains("GridExclusive"));
    assert!(!SOURCE.contains("Bf16::"));
}
