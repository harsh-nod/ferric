use quote::ToTokens as _;
use syn::{FnArg, Item, ItemFn, Pat};

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
    assert_eq!(SOURCE.matches("memory::volatile_load").count(), 11);
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

    let key_load = "memory::volatile_load(rotated_key_bf16, input_index)";
    let value_load = "memory::volatile_load(value_bf16, input_index)";
    assert_eq!(SOURCE.matches(key_load).count(), 1);
    assert_eq!(SOURCE.matches(value_load).count(), 1);
    assert_eq!(SOURCE.matches("let key_component =").count(), 1);
    assert_eq!(SOURCE.matches("let value_component =").count(), 1);

    let owner_guard = SOURCE
        .find("if let Some(current_leader) = thread::grid_leader()")
        .expect("grid-exclusive owner guard is present");
    let first_payload_load = SOURCE
        .find(key_load)
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
    assert!(kv.contains("max_grid = [1 , 1 , 1]"));
    assert!(rope.contains("loop_bounds (32 , 8)"));
    assert!(kv.contains("loop_bounds (2048 , 1024)"));
}

#[test]
fn profile_row_guards_dominate_all_row_extent_products() {
    let compact = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let guard = "ifrows<(QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1asusize+1){}else{fe2o3_device::trap();}";
    assert_eq!(compact.matches(guard).count(), 2);

    let (_, rope_tail) = compact
        .split_once("pubfnqwen3_rope_v1(")
        .expect("RoPE root is present");
    let (rope, kv_tail) = rope_tail
        .split_once("pubfnqwen3_paged_kv_write_v1(")
        .expect("paged-KV root follows RoPE");
    let rope_guard = rope.find(guard).expect("RoPE row guard is present");
    let query_extent = rope
        .find("letquery_elements=rows*query_columns;")
        .expect("RoPE query extent is explicit");
    let key_extent = rope
        .find("letkey_elements=rows*key_columns;")
        .expect("RoPE key extent is explicit");
    assert!(rope_guard < query_extent);
    assert!(rope_guard < key_extent);
    assert_eq!(rope.matches("rows*query_columns").count(), 1);
    assert_eq!(rope.matches("rows*key_columns").count(), 1);
    let launch_extent = rope
        .find("thread::launch_extent_1d()!=rows*64")
        .expect("RoPE launch extent is explicit");
    assert!(rope_guard < launch_extent);
    assert_eq!(rope.matches("rows*64").count(), 1);

    let kv_guard = kv_tail.find(guard).expect("paged-KV row guard is present");
    let kv_extent = kv_tail
        .find("letkv_elements=rows*kv_columns;")
        .expect("paged-KV extent is explicit");
    assert!(kv_guard < kv_extent);
    assert_eq!(kv_tail.matches("rows*kv_columns").count(), 1);
}

#[test]
fn paged_kv_active_token_guard_dominates_division_and_remainder() {
    let compact = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let (_, kv) = compact
        .split_once("pubfnqwen3_paged_kv_write_v1(")
        .expect("paged-KV root is present");
    let guard = "ifactive_tokens==0{fe2o3_device::trap();}";
    let guard_position = kv.find(guard).expect("active-token guard is present");
    let coordinate_marker = "let(sequence,local_token)=ifactive_tokens!=0{(row/active_tokens,row%active_tokens)}else{fe2o3_device::trap()}";
    let coordinates = kv
        .find(coordinate_marker)
        .expect("guarded active-token coordinates are present");
    let first_effect = kv
        .find("memory::volatile_load(logical_starts,sequence)")
        .expect("first dependent read is present");
    assert!(guard_position < coordinates);
    assert!(coordinates < first_effect);
    assert_eq!(kv.matches(guard).count(), 1);
    assert_eq!(kv.matches(coordinate_marker).count(), 1);
    assert_eq!(kv.matches("active_tokens!=0").count(), 1);
    assert_eq!(kv.matches("/active_tokens").count(), 1);
    assert_eq!(kv.matches("%active_tokens").count(), 1);
}

fn guarded_paged_kv_row_coordinates(row: usize, active_tokens: usize) -> Option<(usize, usize)> {
    if active_tokens == 0 {
        return None;
    }
    let (sequence, local_token) = if active_tokens != 0 {
        (row / active_tokens, row % active_tokens)
    } else {
        return None;
    };
    Some((sequence, local_token))
}

#[test]
fn paged_kv_row_coordinate_model_rejects_zero_and_preserves_endpoints() {
    assert_eq!(guarded_paged_kv_row_coordinates(0, 0), None);
    assert_eq!(guarded_paged_kv_row_coordinates(usize::MAX, 0), None);
    assert_eq!(guarded_paged_kv_row_coordinates(0, 1), Some((0, 0)));
    assert_eq!(
        guarded_paged_kv_row_coordinates(usize::MAX, 1),
        Some((usize::MAX, 0))
    );
    assert_eq!(
        guarded_paged_kv_row_coordinates(usize::MAX, usize::MAX),
        Some((1, 0))
    );
}

#[test]
fn paged_kv_page_table_arithmetic_is_guarded_in_exact_order() {
    let compact = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let (_, kv) = compact
        .split_once("pubfnqwen3_paged_kv_write_v1(")
        .expect("paged-KV root is present");
    let logical_page_guard =
        "iflogical_page>=QWEN3_KV_PAGE_TABLE_ENTRIES_V1asusize{fe2o3_device::trap();}";
    let table_base = "lettable_base=ifsequence<32{sequence*QWEN3_KV_PAGE_TABLE_ENTRIES_V1asusize}else{fe2o3_device::trap()};";
    let table_index = "lettable_index=iflogical_page<=usize::MAX-table_base{table_base+logical_page}else{fe2o3_device::trap()};";
    let page_load = "memory::volatile_load(page_indices,table_index)";

    let logical_page_guard_position = kv
        .find(logical_page_guard)
        .expect("logical-page guard is present");
    let table_base_position = kv.find(table_base).expect("guarded table base is present");
    let table_index_position = kv
        .find(table_index)
        .expect("guarded table index is present");
    let page_load_position = kv.find(page_load).expect("page-table load is present");
    assert!(logical_page_guard_position < table_base_position);
    assert!(table_base_position < table_index_position);
    assert!(table_index_position < page_load_position);
    assert_eq!(kv.matches(logical_page_guard).count(), 1);
    assert_eq!(kv.matches(table_base).count(), 1);
    assert_eq!(kv.matches(table_index).count(), 1);
    assert_eq!(kv.matches(page_load).count(), 1);
}

fn guarded_paged_kv_table_index(sequence: usize, logical_page: usize) -> Option<usize> {
    const PAGE_TABLE_ENTRIES: usize = 512;

    if logical_page >= PAGE_TABLE_ENTRIES {
        return None;
    }
    let table_base = if sequence < 32 {
        sequence * PAGE_TABLE_ENTRIES
    } else {
        return None;
    };
    let table_index = if logical_page <= usize::MAX - table_base {
        table_base + logical_page
    } else {
        return None;
    };
    Some(table_index)
}

#[test]
fn paged_kv_page_table_model_preserves_endpoints_and_rejects_hostile_inputs() {
    assert_eq!(guarded_paged_kv_table_index(0, 0), Some(0));
    assert_eq!(guarded_paged_kv_table_index(31, 511), Some(16_383));
    assert_eq!(guarded_paged_kv_table_index(32, 0), None);
    assert_eq!(guarded_paged_kv_table_index(usize::MAX, 0), None);
    assert_eq!(guarded_paged_kv_table_index(0, 512), None);
    assert_eq!(guarded_paged_kv_table_index(0, usize::MAX), None);
}

fn eager_paged_kv_profile_is_admitted(
    active_tokens: u32,
    sequences: u32,
    context_tokens: u32,
) -> bool {
    let common = (((active_tokens == 128) & ((sequences == 1) | (sequences == 8)))
        & (context_tokens == 128))
        | (((active_tokens == 512) & (sequences == 1)) & (context_tokens == 512))
        | (((active_tokens == 2_048) & (sequences == 1)) & (context_tokens == 2_048))
        | (((active_tokens == 1) & (((sequences == 1) | (sequences == 8)) | (sequences == 32)))
            & (context_tokens == 8_192));
    let target = (context_tokens == 8_192)
        & ((((active_tokens == 5) & ((sequences == 1) | (sequences == 8)))
            | ((active_tokens == 9) & (sequences == 1)))
            | ((active_tokens == 17) & (sequences == 1)));
    let draft = (context_tokens == 8_192)
        & ((((active_tokens == 4) & ((sequences == 1) | (sequences == 8)))
            | ((active_tokens == 8) & (sequences == 1)))
            | ((active_tokens == 16) & (sequences == 1)));
    (common | target) | draft
}

#[test]
fn paged_kv_profile_classifier_is_eager_and_exactly_equivalent() {
    use ferric_qwen3_rope_kv_device_v1::qwen3_paged_kv_write_profile_is_admitted_v1;

    let compact = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let (_, kv) = compact
        .split_once("pubfnqwen3_paged_kv_write_v1(")
        .expect("paged-KV root is present");
    let profile_start = kv
        .find("letcommon_profile_is_admitted=")
        .expect("paged-KV profile classifier is present");
    let profile_end = kv
        .find("letactive_tokens=active_tokensasusize;")
        .expect("paged-KV profile classifier precedes scalar conversion");
    let profile = &kv[profile_start..profile_end];
    assert!(!profile.contains("&&"));
    assert!(!profile.contains("||"));
    assert_eq!(profile.matches('&').count(), 16);
    assert_eq!(profile.matches('|').count(), 14);
    assert_eq!(
        profile
            .matches("if!profile_is_admitted{fe2o3_device::trap();}")
            .count(),
        1
    );

    let active_tokens = [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        15,
        16,
        17,
        18,
        127,
        128,
        129,
        511,
        512,
        513,
        2_047,
        2_048,
        2_049,
        u32::MAX,
    ];
    let sequences = [0, 1, 2, 7, 8, 9, 31, 32, 33, u32::MAX];
    let context_tokens = [
        0,
        127,
        128,
        129,
        511,
        512,
        513,
        2_047,
        2_048,
        2_049,
        8_191,
        8_192,
        8_193,
        u32::MAX,
    ];
    for active_tokens in active_tokens {
        for sequences in sequences {
            for context_tokens in context_tokens {
                assert_eq!(
                    eager_paged_kv_profile_is_admitted(active_tokens, sequences, context_tokens,),
                    qwen3_paged_kv_write_profile_is_admitted_v1(
                        active_tokens,
                        sequences,
                        context_tokens,
                    ),
                    "classifier mismatch for ({active_tokens}, {sequences}, {context_tokens})"
                );
            }
        }
    }
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
    assert_eq!(kv.sig.inputs.len(), 10);
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
            "row_components",
        ]
    );
    assert_eq!(compact_type(&kv.sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&kv.sig.inputs[1]), "&[u16]");
    assert_eq!(compact_type(&kv.sig.inputs[2]), "&[u32]");
    assert_eq!(compact_type(&kv.sig.inputs[3]), "&[u32]");
    assert_eq!(
        compact_type(&kv.sig.inputs[4]),
        "WriteOnlyDisjointSlice<u16,GridExclusive>"
    );
    assert_eq!(
        compact_type(&kv.sig.inputs[5]),
        "WriteOnlyDisjointSlice<u16,GridExclusive>"
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
        32_768,
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

fn grid_exclusive_cache_address(
    physical_page: usize,
    token_in_page: usize,
    component: usize,
) -> Option<usize> {
    if physical_page >= 16_384 || token_in_page >= 16 || component >= 1_024 {
        return None;
    }
    physical_page
        .checked_mul(16_384)?
        .checked_add(token_in_page.checked_mul(1_024)?)?
        .checked_add(component)
}

#[test]
fn paged_cache_uses_one_grid_exclusive_work_proportional_scatter() {
    let compact: String = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let kv = compact
        .split_once("pubfnqwen3_paged_kv_write_v1(")
        .expect("paged-KV root is present")
        .1;
    let flattened_extent = "letrow_components=row_componentsasusize;";
    let flattened_bound = "ifrow_components>32_768{fe2o3_device::trap();}";
    let flattened_geometry =
        "ifrow_components%16!=0||row_components/16!=rows{fe2o3_device::trap();}";
    let owner = "letleader;ifletSome(current_leader)=thread::grid_leader(){leader=current_leader;}else{return;}";
    let row_header = "letmutrow=0_usize;whilerow<rows{";
    let row_coordinates = "let(sequence,local_token)=ifactive_tokens!=0{(row/active_tokens,row%active_tokens)}else{fe2o3_device::trap()};";
    let input_base = "letinput_base=row*kv_columns;";
    let cache_page_base = "letcache_page_base=physical_page*16_384;";
    let cache_token_base = "letcache_token_base=cache_page_base+token_in_page*kv_columns;";
    let component_header = "letmutcomponent=0_usize;whilecomponent<kv_columns{";
    let input_index = "letinput_index=input_base+component;";
    let cache_index = "letcache_index=cache_token_base+component;";
    let key_load = "letkey_component=memory::volatile_load(rotated_key_bf16,input_index);";
    let value_load = "letvalue_component=memory::volatile_load(value_bf16,input_index);";
    let key_write = "if!key_cache_bf16.write_exclusive(&leader,cache_index,key_component){fe2o3_device::trap();}";
    let value_write = "if!value_cache_bf16.write_exclusive(&leader,cache_index,value_component){fe2o3_device::trap();}";
    let component_latch = "component+=1;";
    let row_latch = "row+=1;";
    let markers = [
        owner,
        flattened_extent,
        flattened_bound,
        flattened_geometry,
        row_header,
        row_coordinates,
        input_base,
        cache_page_base,
        cache_token_base,
        component_header,
        input_index,
        cache_index,
        key_load,
        value_load,
        key_write,
        value_write,
        component_latch,
        row_latch,
    ];
    let mut prior = 0_usize;
    for marker in markers {
        let position = kv
            .find(marker)
            .unwrap_or_else(|| panic!("missing paged-KV marker {marker}"));
        assert!(prior <= position, "marker is out of order: {marker}");
        assert_eq!(
            kv.matches(marker).count(),
            1,
            "marker is duplicated: {marker}"
        );
        prior = position;
    }
    assert_eq!(kv.matches(".write_block(").count(), 0);
    assert_eq!(kv.matches(".write_row_striped_2d(").count(), 0);
    assert_eq!(kv.matches("thread::grid_leader()").count(), 1);
    assert_eq!(
        kv.matches(".write_exclusive(&leader,cache_index,").count(),
        2
    );
    assert!(!kv.contains("get_mut_exclusive"));
    assert_eq!(kv.matches("whilerow<rows{").count(), 1);
    assert_eq!(kv.matches("whilecomponent<kv_columns{").count(), 1);
    assert!(!kv.contains("owned_physical_page"));
    assert!(!kv.contains("whilerow_component<row_components{"));
    assert!(!kv.contains("checked_row_striped_2d::<64,256>()"));
    assert!(!SOURCE.contains("get_mut_at"));
    assert!(!kv.contains("unsafe{"));
    assert!(!SOURCE.contains("key_cache_bf16["));
    assert!(!SOURCE.contains("value_cache_bf16["));
}
#[test]
fn grid_exclusive_cache_mapping_preserves_every_component_address() {
    for token_in_page in 0..16 {
        for physical_page in [0, 1, 8_191, 16_383] {
            for component in [0, 1, 63, 64, 1_023] {
                assert_eq!(
                    grid_exclusive_cache_address(physical_page, token_in_page, component),
                    Some((physical_page * 16 + token_in_page) * 1_024 + component),
                );
            }
        }
    }
    assert_eq!(grid_exclusive_cache_address(0, 0, 0), Some(0));
    assert_eq!(
        grid_exclusive_cache_address(16_383, 15, 1_023),
        Some(268_435_455)
    );
    assert_eq!(grid_exclusive_cache_address(16_384, 0, 0), None);
    assert_eq!(grid_exclusive_cache_address(0, 16, 0), None);
    assert_eq!(grid_exclusive_cache_address(0, 0, 1_024), None);
    assert_eq!(grid_exclusive_cache_address(usize::MAX, 0, 0), None);
}

#[test]
fn launch_and_numerical_source_contracts_are_explicit() {
    let compact: String = SOURCE
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    assert_eq!(compact.matches("max_grid=[2048,1,1]").count(), 1);
    assert_eq!(compact.matches("max_grid=[1,1,1]").count(), 1);
    assert_eq!(
        SOURCE
            .matches("thread::launch_extent_1d() != rows * 64")
            .count(),
        1
    );
    assert!(SOURCE.contains("thread::launch_extent_1d() != QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1"));
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
