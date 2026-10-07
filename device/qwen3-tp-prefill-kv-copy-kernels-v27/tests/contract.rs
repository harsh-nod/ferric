use ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27::{
    ROOTS_V27, compiler_expectation_roster_v27,
};
use quote::ToTokens;
use syn::{Item, ItemFn};

const SOURCE: &str = include_str!("../src/copy.rs");

fn root(source: &str) -> ItemFn {
    let mut functions = syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        });
    let function = functions.next().unwrap();
    assert!(functions.next().is_none());
    function
}

fn exact_body(source: &str) -> bool {
    let expected: syn::Block = syn::parse_quote!({
        if first_position > 8176
            || first_position & 15 != 0
            || physical_pages == 0
            || physical_pages > 512
            || physical_page >= physical_pages
            || last_row_first > 1
            || key.len() != 16384
            || value.len() != 16384
            || key_page.len() != 16384
            || value_page.len() != 16384
            || thread::launch_extent_1d() != 16384
        {
            fe2o3_device::trap();
        }
        let key_invocation = thread::index_1d();
        let index = key_invocation.get();
        if index < 16384 {
        } else {
            fe2o3_device::trap();
        }
        let row = index / 1024;
        let source_row = if last_row_first == 0 {
            row
        } else if row == 15 {
            0
        } else {
            row + 1
        };
        if source_row < 16 {
        } else {
            fe2o3_device::trap();
        }
        let source_index = source_row * 1024 + index % 1024;
        if source_index < 16384 {
        } else {
            fe2o3_device::trap();
        }
        let key_bits = memory::volatile_load(key, source_index);
        let value_bits = memory::volatile_load(value, source_index);
        if !key_page.write(key_invocation, key_bits) {
            fe2o3_device::trap();
        }
        if !value_page.write(thread::index_1d(), value_bits) {
            fe2o3_device::trap();
        }
    });
    root(source).block.to_token_stream().to_string() == expected.to_token_stream().to_string()
}

#[test]
fn v27_has_one_distinct_typed_root_and_exact_eighty_byte_explicit_abi() {
    let function = root(SOURCE);
    let expected: ItemFn = syn::parse_quote! {
        fn ferric_qwen3_tp_prefill16_kv_copy_bf16_v27(
            key: &[u16], value: &[u16], mut key_page: WriteOnlyDisjointSlice<u16>,
            mut value_page: WriteOnlyDisjointSlice<u16>, first_position: u32,
            physical_page: u32, physical_pages: u32, last_row_first: u32,
        ) {}
    };
    assert_eq!(
        function.sig.to_token_stream().to_string(),
        expected.sig.to_token_stream().to_string()
    );
    let roster = compiler_expectation_roster_v27();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V27[0]);
    assert_eq!(function.sig.ident, ROOTS_V27[0]);
    assert_ne!(ROOTS_V27[0], "ferric_qwen3_tp_batch32_paged_kv_append_v5");
    assert_ne!(ROOTS_V27[0], "ferric_qwen3_tp_c1_kv_copy_bf16_v19");
}

#[test]
fn v27_has_exact_wave64_launch_without_collective_or_serial_copy_loop() {
    let expected: syn::Attribute = syn::parse_quote!(#[kernel(typed,
        launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [256, 1, 1]))]);
    let function = root(SOURCE);
    let actual = function
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .unwrap();
    assert_eq!(
        actual.to_token_stream().to_string(),
        expected.to_token_stream().to_string()
    );
    for forbidden in [
        "unsafe",
        "grid_leader",
        "GridExclusive",
        "Bf16::",
        "reduce_sum",
        "barrier",
        "while ",
        "for ",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden source shape: {forbidden}"
        );
    }
}

#[test]
fn v27_checks_all_scalar_and_slice_bounds_before_raw_bit_copy() {
    assert!(exact_body(SOURCE));
}

#[test]
fn v27_contract_rejects_changed_bounds_permutation_or_writes() {
    for (old, new) in [
        ("first_position > 8176", "first_position > 8192"),
        ("first_position & 15 != 0", "first_position & 7 != 0"),
        (
            "physical_page >= physical_pages",
            "physical_page > physical_pages",
        ),
        ("last_row_first > 1", "last_row_first > 2"),
        ("key.len() != 16384", "key.len() < 16384"),
        ("row == 15", "row == 14"),
        ("row + 1", "row + 2"),
        ("source_row < 16", "source_row < 17"),
        ("source_index < 16384", "source_index < 16385"),
        (
            "volatile_load(value, source_index)",
            "volatile_load(key, source_index)",
        ),
        (
            "value_page.write(thread::index_1d(), value_bits)",
            "value_page.write(thread::index_1d(), key_bits)",
        ),
    ] {
        assert!(SOURCE.contains(old), "missing mutation anchor: {old}");
        assert!(
            !exact_body(&SOURCE.replacen(old, new, 1)),
            "accepted mutation: {old}"
        );
    }
}

#[test]
fn v27_pins_the_independent_kernel_sdk_and_target() {
    let manifest = include_str!("../Cargo.toml");
    assert_eq!(
        manifest
            .matches("rev = \"d10f49bfedc26848285d20ec1399c193b3340f47\"")
            .count(),
        2
    );
    assert!(manifest.contains("default = [\"gfx950\"]"));
    assert!(include_str!("../src/lib.rs").contains("not(feature = \"gfx950\")"));
    assert!(include_str!("../build.rs").contains("-wavefrontsize32,+wavefrontsize64,-xnack"));
}
