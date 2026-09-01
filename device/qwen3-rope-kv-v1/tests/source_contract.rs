use quote::ToTokens as _;
use syn::visit::{self, Visit as _};
use syn::{Expr, ExprMethodCall, FnArg, Item, ItemFn, Lit, Pat};

const SOURCE: &str = include_str!("../../qwen3-all-kernels-v1/src/rope_kv.rs");

fn kernels() -> Vec<ItemFn> {
    syn::parse_file(SOURCE)
        .expect("device source parses as ordinary Rust")
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function)
                if function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("kernel")) =>
            {
                Some(function)
            }
            _ => None,
        })
        .collect()
}

fn compact_type(argument: &FnArg) -> String {
    let FnArg::Typed(argument) = argument else {
        panic!("device roots cannot have a receiver");
    };
    argument
        .ty
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn argument_name(argument: &FnArg) -> String {
    let FnArg::Typed(argument) = argument else {
        panic!("device roots cannot have a receiver");
    };
    let Pat::Ident(pattern) = argument.pat.as_ref() else {
        panic!("device roots use identifier arguments");
    };
    pattern.ident.to_string()
}

#[derive(Default)]
struct WriteBlockComponents {
    components: Vec<usize>,
    non_literal_components: usize,
}

impl<'ast> syn::visit::Visit<'ast> for WriteBlockComponents {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == "write_block" {
            let component = call
                .args
                .iter()
                .nth(1)
                .expect("write_block has a component operand");
            match component {
                Expr::Lit(literal) => match &literal.lit {
                    Lit::Int(value) => self
                        .components
                        .push(value.base10_parse().expect("component literal fits usize")),
                    _ => self.non_literal_components += 1,
                },
                _ => self.non_literal_components += 1,
            }
        }
        visit::visit_expr_method_call(self, call);
    }
}

#[test]
fn source_has_exact_two_root_roster_and_no_escape_hatch() {
    let kernels = kernels();
    assert_eq!(kernels.len(), 2);
    assert_eq!(
        kernels
            .iter()
            .map(|function| function.sig.ident.to_string())
            .collect::<Vec<_>>(),
        vec![
            String::from("qwen3_rope_v1"),
            String::from("qwen3_paged_kv_write_v1"),
        ]
    );
    let lowercase = SOURCE.to_ascii_lowercase();
    for forbidden in [
        "compilerhandoff",
        "pinnedworker",
        "std::process",
        "command::new",
        "include_bytes!",
        "llvm assembly",
        "unsafe {",
        "get_mut_at",
    ] {
        assert!(!lowercase.contains(forbidden), "found {forbidden}");
    }
}

#[test]
fn immutable_inputs_use_the_exact_volatile_load_custody() {
    assert_eq!(SOURCE.matches("memory::volatile_load").count(), 41);
    for input in [
        "query_bf16",
        "key_bf16",
        "position_ids",
        "cos_table_f32",
        "sin_table_f32",
        "rotated_key_bf16",
        "value_bf16",
        "logical_starts",
        "page_indices",
    ] {
        assert!(
            !SOURCE.contains(&format!("{input}[")),
            "immutable input {input} bypasses volatile-load custody"
        );
    }

    for component in 0..16 {
        let key_load = format!("memory::volatile_load(rotated_key_bf16, input_index_{component})");
        let value_load = format!("memory::volatile_load(value_bf16, input_index_{component})");
        assert_eq!(SOURCE.matches(&key_load).count(), 1);
        assert_eq!(SOURCE.matches(&value_load).count(), 1);
        let value_component = format!("value_component_{component}");
        assert_eq!(
            SOURCE
                .split(|character: char| {
                    !(character.is_ascii_alphanumeric() || character == '_')
                })
                .filter(|token| *token == value_component.as_str())
                .count(),
            17,
            "value component must be loaded once and reused by all page slots"
        );
    }

    let owner_guard = SOURCE
        .find("if physical_page == owned_physical_page")
        .expect("physical-page owner guard is present");
    let first_payload_load = SOURCE
        .find("memory::volatile_load(rotated_key_bf16, input_index_0)")
        .expect("first owned-row payload load is present");
    assert!(owner_guard < first_payload_load);
}

#[test]
fn attributes_pin_wave64_flat_grid_and_exact_loop_bounds() {
    let kernels = kernels();
    let rope = kernels[0]
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"));
    let kv = kernels[1]
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"));
    let rope = rope
        .expect("RoPE must retain its kernel attribute")
        .to_token_stream()
        .to_string();
    let kv = kv
        .expect("KV must retain its kernel attribute")
        .to_token_stream()
        .to_string();
    for attribute in [&rope, &kv] {
        assert!(attribute.contains("required = [64 , 1 , 1]"));
        assert!(attribute.contains("max = [64 , 1 , 1]"));
    }
    assert!(rope.contains("max_grid = [2048 , 1 , 1]"));
    assert!(kv.contains("max_grid = [16384 , 1 , 1]"));
    assert!(rope.contains("loop_bounds (32 , 8)"));
    assert!(kv.contains("loop_bounds (2048)"));
}

#[test]
fn source_signatures_match_the_exact_host_abis() {
    let kernels = kernels();
    let rope = &kernels[0];
    assert_eq!(rope.sig.inputs.len(), 11);
    assert_eq!(
        rope.sig
            .inputs
            .iter()
            .map(argument_name)
            .collect::<Vec<_>>(),
        [
            "query_bf16",
            "key_bf16",
            "position_ids",
            "cos_table_f32",
            "sin_table_f32",
            "rotated_query_bf16",
            "rotated_key_bf16",
            "active_tokens",
            "sequences",
            "query_heads",
            "context_tokens",
        ]
    );
    assert_eq!(compact_type(&rope.sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&rope.sig.inputs[1]), "&[u16]");
    assert_eq!(compact_type(&rope.sig.inputs[2]), "&[u32]");
    assert_eq!(compact_type(&rope.sig.inputs[3]), "&[f32]");
    assert_eq!(compact_type(&rope.sig.inputs[4]), "&[f32]");
    assert_eq!(
        compact_type(&rope.sig.inputs[5]),
        "WriteOnlyDisjointSlice<u16,RowStriped2D<Index1D,64,64>>"
    );
    assert_eq!(
        compact_type(&rope.sig.inputs[6]),
        "WriteOnlyDisjointSlice<u16,RowStriped2D<Index1D,64,16>>"
    );
    for scalar in rope.sig.inputs.iter().skip(7) {
        assert_eq!(compact_type(scalar), "u32");
    }

    let kv = &kernels[1];
    assert_eq!(kv.sig.inputs.len(), 9);
    assert_eq!(
        kv.sig.inputs.iter().map(argument_name).collect::<Vec<_>>(),
        [
            "rotated_key_bf16",
            "value_bf16",
            "logical_starts",
            "page_indices",
            "key_cache_bf16",
            "value_cache_bf16",
            "active_tokens",
            "sequences",
            "context_tokens",
        ]
    );
    assert_eq!(compact_type(&kv.sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&kv.sig.inputs[1]), "&[u16]");
    assert_eq!(compact_type(&kv.sig.inputs[2]), "&[u32]");
    assert_eq!(compact_type(&kv.sig.inputs[3]), "&[u32]");
    assert_eq!(
        compact_type(&kv.sig.inputs[4]),
        "WriteOnlyDisjointSlice<u16,Blocked<Index1D,64,256>>"
    );
    assert_eq!(
        compact_type(&kv.sig.inputs[5]),
        "WriteOnlyDisjointSlice<u16,Blocked<Index1D,64,256>>"
    );
    for scalar in kv.sig.inputs.iter().skip(6) {
        assert_eq!(compact_type(scalar), "u32");
    }
}

#[test]
fn generated_kfd_adapters_preserve_effects_and_constructor_order() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_rope_kv_device_v1::{
        qwen3_paged_kv_write_v1_gpu as kv, qwen3_rope_v1_gpu as rope,
    };

    fn assert_adapter<'allocation, K, A>()
    where
        K: CompilerGeneratedKernelExpectationV1,
        A: CompilerGeneratedKfdArguments<'allocation, K>,
    {
    }

    type ReadU16 = GeneratedKfdReadSlice<'static, u16>;
    type ReadU32 = GeneratedKfdReadSlice<'static, u32>;
    type ReadF32 = GeneratedKfdReadSlice<'static, f32>;
    type WriteU16 = GeneratedKfdWriteSlice<'static, u16>;
    assert_adapter::<
        rope::Marker,
        rope::Arguments<'static, ReadU16, ReadU16, ReadU32, ReadF32, ReadF32, WriteU16, WriteU16>,
    >();
    assert_adapter::<
        kv::Marker,
        kv::Arguments<'static, ReadU16, ReadU16, ReadU32, ReadU32, WriteU16, WriteU16>,
    >();

    let bf16_a = [0_u16; 1];
    let bf16_b = [0_u16; 1];
    let u32_a = [0_u32; 1];
    let u32_b = [0_u32; 1];
    let f32_a = [0.0_f32; 1];
    let f32_b = [0.0_f32; 1];
    let mut rope_query = [0_u16; 1];
    let mut rope_key = [0_u16; 1];
    let _rope = rope::Arguments::new(
        GeneratedKfdReadSlice::new(&bf16_a),
        GeneratedKfdReadSlice::new(&bf16_b),
        GeneratedKfdReadSlice::new(&u32_a),
        GeneratedKfdReadSlice::new(&f32_a),
        GeneratedKfdReadSlice::new(&f32_b),
        GeneratedKfdWriteSlice::new(&mut rope_query),
        GeneratedKfdWriteSlice::new(&mut rope_key),
        128,
        8,
        32,
        128,
    );

    let mut key_cache = [0_u16; 1];
    let mut value_cache = [0_u16; 1];
    let _kv = kv::Arguments::new(
        GeneratedKfdReadSlice::new(&bf16_a),
        GeneratedKfdReadSlice::new(&bf16_b),
        GeneratedKfdReadSlice::new(&u32_a),
        GeneratedKfdReadSlice::new(&u32_b),
        GeneratedKfdWriteSlice::new(&mut key_cache),
        GeneratedKfdWriteSlice::new(&mut value_cache),
        2_048,
        1,
        2_048,
    );
}

#[test]
fn rope_row_stripes_are_injective_and_cover_each_output_row() {
    for query_heads in [16_usize, 32] {
        let query_columns = query_heads * 128;
        let key_columns = 8 * 128;
        for row in [0_usize, 1, 2_047] {
            let mut query = std::collections::BTreeSet::new();
            let mut key = std::collections::BTreeSet::new();
            for lane in 0..64 {
                let raw = row * 64 + lane;
                assert_eq!(raw / 64, row);
                for component in 0..query_heads * 2 {
                    assert!(query.insert(row * query_columns + component * 64 + lane));
                }
                for component in 0..16 {
                    assert!(key.insert(row * key_columns + component * 64 + lane));
                }
            }
            assert_eq!(query.len(), query_columns);
            assert_eq!(key.len(), key_columns);
            assert_eq!(*query.first().unwrap(), row * query_columns);
            assert_eq!(*query.last().unwrap(), (row + 1) * query_columns - 1);
            assert_eq!(*key.first().unwrap(), row * key_columns);
            assert_eq!(*key.last().unwrap(), (row + 1) * key_columns - 1);
        }
    }
}

#[test]
fn paged_cache_uses_only_literal_static_blocked_write_sites() {
    let kernels = kernels();
    let mut visitor = WriteBlockComponents::default();
    visitor.visit_item_fn(&kernels[1]);
    assert_eq!(visitor.non_literal_components, 0);
    assert_eq!(visitor.components.len(), 512);
    for component in 0..256 {
        assert_eq!(
            visitor
                .components
                .iter()
                .filter(|candidate| **candidate == component)
                .count(),
            2,
            "key and value must each have one literal store for component {component}"
        );
    }
    assert_eq!(SOURCE.matches("if token_in_page == ").count(), 16);
    for token_in_page in 0..16 {
        assert_eq!(
            SOURCE
                .matches(&format!("if token_in_page == {token_in_page} {{"))
                .count(),
            1,
            "each physical-page token slot must have one static store case"
        );
    }
    assert!(SOURCE.contains("page_lane_index.checked_block::<64, 256>()"));
    assert!(!SOURCE.contains("grid_leader"));
    assert!(!SOURCE.contains("write_exclusive"));
    assert!(!SOURCE.contains("get_mut_at"));
    assert!(!SOURCE.contains("key_cache_bf16["));
    assert!(!SOURCE.contains("value_cache_bf16["));
}

#[test]
fn launch_and_numerical_source_contracts_are_explicit() {
    let compact: String = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert_eq!(compact.matches("max_grid=[2048,1,1]").count(), 1);
    assert_eq!(compact.matches("max_grid=[16384,1,1]").count(), 1);
    assert_eq!(
        SOURCE
            .matches("thread::launch_extent_1d() != rows * 64")
            .count(),
        1
    );
    assert!(
        SOURCE.contains("thread::launch_extent_1d() != QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1")
    );
    for required in [
        "let first_cos = first_value * cos;",
        "let second_sin = second_value * sin;",
        "let rotated_first = first_cos - second_sin;",
        "let second_cos = second_value * cos;",
        "let first_sin = first_value * sin;",
        "let rotated_second = second_cos + first_sin;",
        "Bf16::from_f32(rotated_first)",
        "Bf16::from_f32(rotated_second)",
    ] {
        assert!(SOURCE.contains(required), "missing {required}");
    }
    assert!(!SOURCE.contains("mul_add"));
}

#[test]
fn profile_guards_and_launch_checks_are_inlined_in_both_roots() {
    let kernels = kernels();
    let rope = kernels[0].block.to_token_stream().to_string();
    let kv = kernels[1].block.to_token_stream().to_string();
    assert!(rope.contains("let profile_is_admitted"));
    assert!(kv.contains("let profile_is_admitted"));
    assert!(!rope.contains("rope_profile_is_admitted_v1 !"));
    assert!(!kv.contains("kv_profile_is_admitted_v1 !"));
    assert!(!rope.contains("qwen3_rope_profile_is_admitted_v1 ("));
    assert!(!kv.contains("qwen3_paged_kv_write_profile_is_admitted_v1 ("));
    assert!(rope.contains("thread :: launch_extent_1d () != rows * 64"));
    assert!(kv.contains("thread :: launch_extent_1d () != QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1"));
}
