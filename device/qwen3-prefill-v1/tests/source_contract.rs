use ferric_qwen3_prefill_device_v1::{
    QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1, QWEN3_PREFILL_ATTENTION_SCALE_V1,
    QWEN3_PREFILL_CACHE_ELEMENTS_V1, QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1,
    QWEN3_PREFILL_CACHE_POOL_PAGES_V1, QWEN3_PREFILL_HEAD_DIMENSION_V1, QWEN3_PREFILL_KV_HEADS_V1,
    QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1, QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1,
    QWEN3_PREFILL_PAGE_TOKENS_V1,
};
use syn::{Expr, FnArg, Item, ItemFn, Meta, Stmt};

const SOURCE: &str = include_str!("../../qwen3-all-kernels-v1/src/prefill.rs");

fn kernel() -> ItemFn {
    let kernels: Vec<_> = syn::parse_file(SOURCE)
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
        .collect();
    assert_eq!(kernels.len(), 1);
    kernels.into_iter().next().unwrap()
}

fn compact_tokens(tokens: impl quote::ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn compact_type(argument: &FnArg) -> String {
    let FnArg::Typed(argument) = argument else {
        panic!("device roots cannot have a receiver");
    };
    compact_tokens(argument.ty.as_ref())
}

#[test]
fn source_has_one_exact_ferric_kernel_and_no_escape_hatch() {
    let kernel = kernel();
    assert_eq!(kernel.sig.ident, "qwen3_gqa_prefill_causal_bf16_f32_v1");
    let lowercase = SOURCE.to_ascii_lowercase();
    for forbidden in [
        "compilerhandoff",
        "pinnedworker",
        "std::process",
        "command::new",
        "include_bytes!",
        "llvm assembly",
        "macro_rules!",
        "unsafe {",
        "unsafe fn",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "found forbidden marker {forbidden}"
        );
    }
}

#[test]
fn signature_retains_exact_five_slice_abi_and_write_only_pair_authority() {
    let kernel = kernel();
    assert_eq!(kernel.sig.inputs.len(), 5);
    assert_eq!(compact_type(&kernel.sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[1]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[2]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[3]), "&[u32]");
    assert_eq!(
        compact_type(&kernel.sig.inputs[4]),
        "WriteOnlyDisjointSlice<u16,Blocked<Index1D,1,2>>"
    );
    assert!(matches!(kernel.sig.output, syn::ReturnType::Default));
}

#[test]
fn host_build_exposes_exact_read_read_read_read_write_kfd_adapter() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_prefill_device_v1::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::{
        Arguments, Marker,
    };

    fn assert_kfd_adapter<'allocation, K, A>()
    where
        K: CompilerGeneratedKernelExpectationV1,
        A: CompilerGeneratedKfdArguments<'allocation, K>,
    {
    }

    assert_kfd_adapter::<
        Marker,
        Arguments<
            'static,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdReadSlice<'static, u16>,
            GeneratedKfdReadSlice<'static, u32>,
            GeneratedKfdWriteSlice<'static, u16>,
        >,
    >();

    let q = [0_u16; 128];
    let k = [0_u16; 128];
    let v = [0_u16; 128];
    let pages = [0_u32; 1];
    let mut output = [0_u16; 128];
    let _arguments = Arguments::new(
        GeneratedKfdReadSlice::new(&q),
        GeneratedKfdReadSlice::new(&k),
        GeneratedKfdReadSlice::new(&v),
        GeneratedKfdReadSlice::new(&pages),
        GeneratedKfdWriteSlice::new(&mut output),
    );
}

#[test]
fn attribute_pins_wave64_grid_and_two_bounded_loops() {
    let kernel = kernel();
    let attribute = kernel
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .unwrap();
    let Meta::List(arguments) = &attribute.meta else {
        panic!("kernel attribute must carry the typed contract");
    };
    let tokens = arguments.tokens.to_string();
    assert!(tokens.contains("typed"));
    assert!(tokens.contains("required = [64 , 1 , 1]"));
    assert!(tokens.contains("max = [64 , 1 , 1]"));
    assert!(tokens.contains("max_grid = [65536 , 1 , 1]"));
    assert!(tokens.contains("loop_bounds (2048 , 128)"));
    assert!(!tokens.contains("integer_switches"));
}

#[test]
fn all_immutable_kernel_reads_are_bounded_volatile_loads() {
    let body = compact_tokens(kernel().block);
    for forbidden in [
        "q[",
        "k[",
        "v[",
        "pages[",
        "q.get(",
        "k.get(",
        "v.get(",
        "pages.get(",
        ".get_unchecked(",
        ".as_ptr(",
        "read_volatile(",
        "core::ptr",
    ] {
        assert!(
            !body.contains(forbidden),
            "found direct read marker {forbidden}"
        );
    }
    assert_eq!(body.matches("memory::volatile_load(q,").count(), 1);
    assert_eq!(body.matches("memory::volatile_load(k,").count(), 1);
    assert_eq!(body.matches("memory::volatile_load(v,").count(), 2);
    assert_eq!(body.matches("memory::volatile_load(pages,").count(), 1);
}

#[test]
fn validation_and_page_guards_precede_every_dependent_read_or_store() {
    let kernel = kernel();
    let body = compact_tokens(&kernel.block);
    let shape_trap = body
        .find("if!(target||draft)||k.len()!=QWEN3_PREFILL_CACHE_ELEMENTS_V1")
        .unwrap();
    let global_trap = body.find("ifglobal>=q.len()/2").unwrap();
    let key_limit_guard = body
        .find("ifquery_token<2_048{}else{fe2o3_device::trap();}")
        .unwrap();
    let key_limit = body.find("letkey_limit=query_token+1").unwrap();
    let query_vector_guard = body
        .find("ifvector<65_536{}else{fe2o3_device::trap();}")
        .unwrap();
    let query_base = body
        .find("letquery_base=vector*QWEN3_PREFILL_HEAD_DIMENSION_V1")
        .unwrap();
    let local_guard = body
        .find("iflocal<64{}else{fe2o3_device::trap();}")
        .unwrap();
    let column_0 = body.find("letcolumn_0=local*2").unwrap();
    let column_0_guard = body
        .find("ifcolumn_0<128{}else{fe2o3_device::trap();}")
        .unwrap();
    let column_1 = body.find("letcolumn_1=column_0+1").unwrap();
    let column_1_guard = body
        .find("ifcolumn_1<128{}else{fe2o3_device::trap();}")
        .unwrap();
    let key_loop = body.find("whilekey_token<2_048{").unwrap();
    let key_limit_effect_guard = body.find("ifkey_token<key_limit{").unwrap();
    let logical_page_guard = body
        .find("iflogical_page<QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1{}else{fe2o3_device::trap();}")
        .unwrap();
    let page_table_multiply_guard = body
        .find("ifsequence<8{}else{fe2o3_device::trap();}")
        .unwrap();
    let page_table_base = body
        .find("letpage_table_base=sequence*QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1")
        .unwrap();
    let page_table_add_guard = body
        .find("iflogical_page<=usize::MAX-page_table_base{}else{fe2o3_device::trap();}")
        .unwrap();
    let page_table_index = body
        .find("letpage_table_index=page_table_base+logical_page")
        .unwrap();
    let page_table_bounds = body
        .find("ifpage_table_index<pages.len(){}else{fe2o3_device::trap();}")
        .unwrap();
    let page_load = body
        .find("memory::volatile_load(pages,page_table_index)")
        .unwrap();
    let physical_page_guard = body
        .find("ifphysical_page>=QWEN3_PREFILL_CACHE_POOL_PAGES_V1")
        .unwrap();
    let page_token_multiply_guard = body
        .find(
            "ifphysical_page<=usize::MAX/QWEN3_PREFILL_PAGE_TOKENS_V1{}else{fe2o3_device::trap();}",
        )
        .unwrap();
    let page_token_base = body
        .find("letpage_token_base=physical_page*QWEN3_PREFILL_PAGE_TOKENS_V1")
        .unwrap();
    let page_token_add_guard = body
        .find("iftoken_in_page<=usize::MAX-page_token_base{}else{fe2o3_device::trap();}")
        .unwrap();
    let page_token = body
        .find("letpage_token=page_token_base+token_in_page")
        .unwrap();
    let cache_head_multiply_guard = body
        .find("ifpage_token<=usize::MAX/QWEN3_PREFILL_KV_HEADS_V1{}else{fe2o3_device::trap();}")
        .unwrap();
    let cache_head_base = body
        .find("letcache_head_base=page_token*QWEN3_PREFILL_KV_HEADS_V1")
        .unwrap();
    let cache_head_add_guard = body
        .find("ifkv_head<=usize::MAX-cache_head_base{}else{fe2o3_device::trap();}")
        .unwrap();
    let cache_head = body.find("letcache_head=cache_head_base+kv_head").unwrap();
    let cache_base_multiply_guard = body
        .find(
            "ifcache_head<=usize::MAX/QWEN3_PREFILL_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        )
        .unwrap();
    let cache_head_capacity_guard = body
        .find("ifcache_head<2_097_152{}else{fe2o3_device::trap();}")
        .unwrap();
    let cache_base = body
        .find("letcache_base=cache_head*QWEN3_PREFILL_HEAD_DIMENSION_V1")
        .unwrap();
    let cache_len_marker = "letcache_len=k.len()";
    let cache_len = body.find(cache_len_marker).unwrap();
    let cache_remaining_marker = "letcache_remaining=ifcache_base<=cache_len{cache_len-cache_base}else{fe2o3_device::trap();}";
    let cache_remaining = body.find(cache_remaining_marker).unwrap();
    let cache_extent_guard = body
        .find("ifQWEN3_PREFILL_HEAD_DIMENSION_V1<=cache_remaining{}else{fe2o3_device::trap();}")
        .unwrap();
    let feature_loop = body
        .find("whilefeature<QWEN3_PREFILL_HEAD_DIMENSION_V1{")
        .unwrap();
    let query_index = body.find("letquery_index=query_base+feature").unwrap();
    let key_index = body.find("letkey_index=cache_base+feature").unwrap();
    let query_load = body.find("memory::volatile_load(q,query_index)").unwrap();
    let key_load = body.find("memory::volatile_load(k,key_index)").unwrap();
    let value_index_0 = body.find("letvalue_index_0=cache_base+column_0").unwrap();
    let value_load = body.find("memory::volatile_load(v,value_index_0)").unwrap();
    let value_index_1 = body.find("letvalue_index_1=cache_base+column_1").unwrap();
    let second_value_load = body.find("memory::volatile_load(v,value_index_1)").unwrap();
    let key_token_increment_guard = body
        .find("ifkey_token<2_048{key_token+=1;}else{fe2o3_device::trap();}")
        .unwrap();
    let first_store = body
        .find("output.write_block(&output_pair,0,output_0.to_bits())")
        .unwrap();

    assert!(shape_trap < global_trap);
    assert!(global_trap < key_limit_guard);
    assert!(key_limit_guard < key_limit);
    assert!(key_limit < query_vector_guard);
    assert!(query_vector_guard < query_base);
    assert!(query_base < local_guard);
    assert!(local_guard < column_0);
    assert!(column_0 < column_0_guard);
    assert!(column_0_guard < column_1);
    assert!(column_1 < column_1_guard);
    assert!(column_1_guard < key_loop);
    assert!(key_loop < key_limit_effect_guard);
    assert!(key_limit_effect_guard < logical_page_guard);
    assert!(logical_page_guard < page_table_multiply_guard);
    assert!(page_table_multiply_guard < page_table_base);
    assert!(page_table_base < page_table_add_guard);
    assert!(page_table_add_guard < page_table_index);
    assert!(page_table_index < page_table_bounds);
    assert!(page_table_bounds < page_load);
    assert!(page_load < physical_page_guard);
    assert!(physical_page_guard < page_token_multiply_guard);
    assert!(page_token_multiply_guard < page_token_base);
    assert!(page_token_base < page_token_add_guard);
    assert!(page_token_add_guard < page_token);
    assert!(page_token < cache_head_multiply_guard);
    assert!(cache_head_multiply_guard < cache_head_base);
    assert!(cache_head_base < cache_head_add_guard);
    assert!(cache_head_add_guard < cache_head);
    assert!(cache_head < cache_base_multiply_guard);
    assert!(cache_base_multiply_guard < cache_head_capacity_guard);
    assert!(cache_head_capacity_guard < cache_base);
    assert!(cache_base < cache_len);
    assert!(cache_len < cache_remaining);
    assert!(cache_remaining < cache_extent_guard);
    assert!(cache_extent_guard < feature_loop);
    assert!(feature_loop < query_index);
    assert!(query_index < query_load);
    assert!(query_load < key_index);
    assert!(key_index < key_load);
    assert!(query_load < value_index_0);
    assert!(value_index_0 < value_load);
    assert!(value_load < value_index_1);
    assert!(value_index_1 < second_value_load);
    assert!(second_value_load < key_token_increment_guard);
    assert!(key_token_increment_guard < first_store);
    for marker in [
        "iflogical_page<QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1{}else{fe2o3_device::trap();}",
        "ifsequence<8{}else{fe2o3_device::trap();}",
        "iflogical_page<=usize::MAX-page_table_base{}else{fe2o3_device::trap();}",
        "ifpage_table_index<pages.len(){}else{fe2o3_device::trap();}",
        "ifphysical_page<=usize::MAX/QWEN3_PREFILL_PAGE_TOKENS_V1{}else{fe2o3_device::trap();}",
        "iftoken_in_page<=usize::MAX-page_token_base{}else{fe2o3_device::trap();}",
        "ifpage_token<=usize::MAX/QWEN3_PREFILL_KV_HEADS_V1{}else{fe2o3_device::trap();}",
        "ifkv_head<=usize::MAX-cache_head_base{}else{fe2o3_device::trap();}",
        "ifcache_head<=usize::MAX/QWEN3_PREFILL_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        "ifcache_head<2_097_152{}else{fe2o3_device::trap();}",
        cache_len_marker,
        cache_remaining_marker,
        "ifQWEN3_PREFILL_HEAD_DIMENSION_V1<=cache_remaining{}else{fe2o3_device::trap();}",
        "letkey_index=cache_base+feature",
        "letvalue_index_0=cache_base+column_0",
        "letvalue_index_1=cache_base+column_1",
        "ifkey_token<2_048{key_token+=1;}else{fe2o3_device::trap();}",
        "ifvector<65_536{}else{fe2o3_device::trap();}",
        "iflocal<64{}else{fe2o3_device::trap();}",
        "ifcolumn_0<128{}else{fe2o3_device::trap();}",
        "ifcolumn_1<128{}else{fe2o3_device::trap();}",
        "letquery_index=query_base+feature",
        "ifquery_token<2_048{}else{fe2o3_device::trap();}",
        "letkey_limit=query_token+1",
        "whilekey_token<2_048{",
        "ifkey_token<key_limit{",
    ] {
        assert_eq!(
            body.matches(marker).count(),
            1,
            "proof guard must occur exactly once: {marker}"
        );
    }
    for marker in [
        "memory::volatile_load(pages,page_table_index)",
        "memory::volatile_load(q,query_index)",
        "memory::volatile_load(k,key_index)",
        "memory::volatile_load(v,value_index_0)",
        "memory::volatile_load(v,value_index_1)",
    ] {
        assert_eq!(
            body.matches(marker).count(),
            1,
            "dependent load must occur exactly once: {marker}"
        );
    }
    assert_eq!(body.matches("letcache_len=k.len()").count(), 1);
    assert_eq!(body.matches("ifcache_base<=cache_len").count(), 1);
    assert_eq!(body.matches("cache_len-cache_base").count(), 1);
    assert_eq!(body.matches("k.len()").count(), 2);
    assert!(!body.contains("usize::MAX-query_base"));
    assert!(!body.contains("usize::MAX-cache_base"));
    assert!(!body.contains("cache_base+QWEN3_PREFILL_HEAD_DIMENSION_V1"));
    assert!(!body.contains("cache_base.checked_add("));
    assert!(!body.contains("cache_base.wrapping_add("));
    assert!(!body.contains("cache_base.overflowing_add("));

    let uniform_loop = kernel
        .block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            Stmt::Expr(Expr::While(expression), _)
                if compact_tokens(&expression.cond) == "key_token<2_048" =>
            {
                Some(expression)
            }
            _ => None,
        })
        .expect("missing exact uniform key-token loop");
    assert_eq!(uniform_loop.body.stmts.len(), 2);
    let Stmt::Expr(Expr::If(effect_region), _) = &uniform_loop.body.stmts[0] else {
        panic!("first uniform-loop statement must be the semantic effect guard");
    };
    assert_eq!(compact_tokens(&effect_region.cond), "key_token<key_limit");
    assert!(effect_region.else_branch.is_none());
    let effects = compact_tokens(&effect_region.then_branch);
    for marker in [
        "memory::volatile_load(pages,page_table_index)",
        "memory::volatile_load(q,query_index)",
        "memory::volatile_load(k,key_index)",
        "memory::volatile_load(v,value_index_0)",
        "memory::volatile_load(v,value_index_1)",
        "letnext_sum=running_sum*previous_weight+current_weight",
    ] {
        assert!(
            effects.contains(marker),
            "effect escaped semantic guard: {marker}"
        );
    }
    assert_eq!(
        compact_tokens(&uniform_loop.body.stmts[1]),
        "ifkey_token<2_048{key_token+=1;}else{fe2o3_device::trap();}"
    );
}

fn guarded_cache_end(cache_base: usize, cache_len: usize) -> Option<usize> {
    if cache_base <= cache_len {
    } else {
        return None;
    }
    let cache_remaining = cache_len - cache_base;
    if QWEN3_PREFILL_HEAD_DIMENSION_V1 <= cache_remaining {
    } else {
        return None;
    }
    Some(cache_base + QWEN3_PREFILL_HEAD_DIMENSION_V1)
}

fn guarded_cache_base(physical_page: usize, token_in_page: usize, kv_head: usize) -> Option<usize> {
    if physical_page <= usize::MAX / QWEN3_PREFILL_PAGE_TOKENS_V1 {
    } else {
        return None;
    }
    let page_token_base = physical_page * QWEN3_PREFILL_PAGE_TOKENS_V1;
    if token_in_page <= usize::MAX - page_token_base {
    } else {
        return None;
    }
    let page_token = page_token_base + token_in_page;
    if page_token <= usize::MAX / QWEN3_PREFILL_KV_HEADS_V1 {
    } else {
        return None;
    }
    let cache_head_base = page_token * QWEN3_PREFILL_KV_HEADS_V1;
    if kv_head <= usize::MAX - cache_head_base {
    } else {
        return None;
    }
    let cache_head = cache_head_base + kv_head;
    guarded_cache_base_from_head(cache_head)
}

fn guarded_cache_base_from_head(cache_head: usize) -> Option<usize> {
    if cache_head < QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1 {
        Some(cache_head * QWEN3_PREFILL_HEAD_DIMENSION_V1)
    } else {
        None
    }
}

fn guarded_page_table_index(sequence: usize, logical_page: usize) -> Option<usize> {
    if logical_page < QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 {
    } else {
        return None;
    }
    if sequence < 8 {
    } else {
        return None;
    }
    let page_table_base = sequence * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1;
    if logical_page <= usize::MAX - page_table_base {
    } else {
        return None;
    }
    Some(page_table_base + logical_page)
}

fn geometry_value_columns(
    cache_head: usize,
    column_0: usize,
    column_1: usize,
) -> Option<(usize, usize)> {
    let cache_base = guarded_cache_base_from_head(cache_head)?;
    if column_0 >= QWEN3_PREFILL_HEAD_DIMENSION_V1 || column_1 >= QWEN3_PREFILL_HEAD_DIMENSION_V1 {
        return None;
    }
    Some((cache_base + column_0, cache_base + column_1))
}

fn geometry_cache_feature_index(cache_head: usize, feature: usize) -> Option<usize> {
    let cache_base = guarded_cache_base_from_head(cache_head)?;
    if feature >= QWEN3_PREFILL_HEAD_DIMENSION_V1 {
        return None;
    }
    Some(cache_base + feature)
}

fn guarded_key_token_increment(mut key_token: usize) -> Option<usize> {
    if key_token < 2_048 {
        key_token += 1;
    } else {
        return None;
    }
    Some(key_token)
}

fn guarded_key_limit(query_token: usize) -> Option<usize> {
    if query_token < 2_048 {
        Some(query_token + 1)
    } else {
        None
    }
}

fn modeled_key_tokens(query_token: usize) -> Option<Vec<usize>> {
    let key_limit = guarded_key_limit(query_token)?;
    let mut key_token = 0;
    let mut visited = Vec::new();
    while key_token < 2_048 {
        if key_token >= key_limit {
            break;
        }
        visited.push(key_token);
        key_token = guarded_key_token_increment(key_token)?;
    }
    Some(visited)
}

fn guarded_query_base(vector: usize) -> Option<usize> {
    if vector < 65_536 {
    } else {
        return None;
    }
    Some(vector * QWEN3_PREFILL_HEAD_DIMENSION_V1)
}

fn guarded_column_0(local: usize) -> Option<usize> {
    if local < 64 {
    } else {
        return None;
    }
    Some(local * 2)
}

fn guarded_column_1(column_0: usize) -> Option<usize> {
    if column_0 < 128 {
    } else {
        return None;
    }
    let column_1 = column_0 + 1;
    if column_1 < 128 { Some(column_1) } else { None }
}

fn geometry_query_feature_index(vector: usize, feature: usize) -> Option<usize> {
    let query_base = guarded_query_base(vector)?;
    if feature >= QWEN3_PREFILL_HEAD_DIMENSION_V1 {
        return None;
    }
    Some(query_base + feature)
}

#[test]
fn cache_extent_proof_accepts_exact_endpoints_and_rejects_hostile_extents() {
    let cache_len = QWEN3_PREFILL_CACHE_ELEMENTS_V1;
    let width = QWEN3_PREFILL_HEAD_DIMENSION_V1;
    assert_eq!(guarded_cache_end(0, cache_len), Some(width));
    assert_eq!(
        guarded_cache_end(cache_len - width, cache_len),
        Some(cache_len)
    );
    assert_eq!(guarded_cache_end(cache_len - (width - 1), cache_len), None);
    assert_eq!(guarded_cache_end(cache_len, cache_len), None);
    assert_eq!(
        guarded_cache_end(usize::MAX - (width - 1), usize::MAX),
        None
    );
    assert_eq!(guarded_cache_end(usize::MAX, usize::MAX), None);
}

#[test]
fn cache_coordinate_proof_preserves_valid_endpoints_and_rejects_overflow() {
    assert_eq!(
        QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1 * QWEN3_PREFILL_HEAD_DIMENSION_V1,
        QWEN3_PREFILL_CACHE_ELEMENTS_V1
    );
    assert_eq!(
        guarded_cache_base_from_head(QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1 - 1),
        Some(QWEN3_PREFILL_CACHE_ELEMENTS_V1 - QWEN3_PREFILL_HEAD_DIMENSION_V1)
    );
    assert_eq!(
        guarded_cache_base_from_head(QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1),
        None
    );
    assert_eq!(guarded_cache_base_from_head(usize::MAX), None);
    assert_eq!(guarded_cache_base(0, 0, 0), Some(0));
    let last_base = guarded_cache_base(
        QWEN3_PREFILL_CACHE_POOL_PAGES_V1 - 1,
        QWEN3_PREFILL_PAGE_TOKENS_V1 - 1,
        QWEN3_PREFILL_KV_HEADS_V1 - 1,
    )
    .unwrap();
    assert_eq!(
        last_base + QWEN3_PREFILL_HEAD_DIMENSION_V1,
        QWEN3_PREFILL_CACHE_ELEMENTS_V1
    );
    assert_eq!(guarded_cache_base(usize::MAX, 0, 0), None);
    assert_eq!(
        guarded_cache_base(
            usize::MAX / QWEN3_PREFILL_PAGE_TOKENS_V1,
            QWEN3_PREFILL_PAGE_TOKENS_V1 - 1,
            0,
        ),
        None
    );
    assert_eq!(
        guarded_cache_base(
            usize::MAX / QWEN3_PREFILL_HEAD_DIMENSION_V1,
            0,
            QWEN3_PREFILL_KV_HEADS_V1 - 1,
        ),
        None
    );
}

#[test]
fn page_table_coordinate_proof_preserves_valid_endpoints_and_rejects_overflow() {
    assert_eq!(guarded_page_table_index(0, 0), Some(0));
    assert_eq!(
        guarded_page_table_index(7, QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 - 1),
        Some(8 * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 - 1)
    );
    assert_eq!(guarded_page_table_index(usize::MAX, 0), None);
    assert_eq!(
        guarded_page_table_index(8, QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 - 1),
        None
    );
    assert_eq!(
        guarded_page_table_index(0, QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1),
        None
    );
}

#[test]
fn value_column_proof_accepts_geometry_endpoint_and_rejects_first_invalid_coordinate() {
    let last_head = QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1 - 1;
    assert_eq!(
        geometry_value_columns(last_head, 126, 127),
        Some((
            QWEN3_PREFILL_CACHE_ELEMENTS_V1 - 2,
            QWEN3_PREFILL_CACHE_ELEMENTS_V1 - 1,
        ))
    );
    assert_eq!(
        geometry_value_columns(QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1, 0, 1),
        None
    );
    assert_eq!(
        geometry_value_columns(last_head, 127, QWEN3_PREFILL_HEAD_DIMENSION_V1),
        None
    );
    assert_eq!(geometry_value_columns(last_head, 128, 0), None);
}

#[test]
fn feature_index_proof_accepts_geometry_endpoint_and_rejects_first_invalid_coordinate() {
    let last_head = QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1 - 1;
    assert_eq!(
        geometry_cache_feature_index(last_head, QWEN3_PREFILL_HEAD_DIMENSION_V1 - 1),
        Some(QWEN3_PREFILL_CACHE_ELEMENTS_V1 - 1)
    );
    assert_eq!(
        geometry_cache_feature_index(QWEN3_PREFILL_CACHE_HEAD_CAPACITY_V1, 0),
        None
    );
    assert_eq!(geometry_cache_feature_index(last_head, 128), None);
    assert_eq!(geometry_cache_feature_index(usize::MAX, 0), None);
}

#[test]
fn key_token_limit_and_increment_preserve_the_inclusive_endpoint() {
    assert_eq!(guarded_key_limit(0), Some(1));
    assert_eq!(guarded_key_limit(2_047), Some(2_048));
    assert_eq!(guarded_key_limit(2_048), None);
    assert_eq!(guarded_key_limit(usize::MAX), None);
    assert_eq!(modeled_key_tokens(0), Some(vec![0]));
    let full = modeled_key_tokens(2_047).unwrap();
    assert_eq!(full.len(), 2_048);
    assert_eq!(full.first(), Some(&0));
    assert_eq!(full.last(), Some(&2_047));
    assert_eq!(modeled_key_tokens(2_048), None);
    assert_eq!(modeled_key_tokens(usize::MAX), None);
    assert_eq!(guarded_key_token_increment(0), Some(1));
    assert_eq!(guarded_key_token_increment(2_047), Some(2_048));
    assert_eq!(guarded_key_token_increment(2_048), None);
    assert_eq!(guarded_key_token_increment(usize::MAX), None);
}

#[test]
fn query_base_proof_matches_exported_grid_bound_and_rejects_first_invalid_vector() {
    assert_eq!(QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1 as usize, 65_536);
    assert_eq!(guarded_query_base(0), Some(0));
    assert_eq!(
        guarded_query_base(65_535),
        Some(65_535 * QWEN3_PREFILL_HEAD_DIMENSION_V1)
    );
    assert_eq!(guarded_query_base(65_536), None);
    assert_eq!(guarded_query_base(usize::MAX), None);
}

#[test]
fn local_column_and_query_feature_proofs_pin_all_boundaries() {
    assert_eq!(QWEN3_PREFILL_HEAD_DIMENSION_V1, 128);
    assert_eq!(guarded_column_0(0), Some(0));
    assert_eq!(guarded_column_0(63), Some(126));
    assert_eq!(guarded_column_0(64), None);
    assert_eq!(guarded_column_0(usize::MAX), None);
    assert_eq!(guarded_column_1(0), Some(1));
    assert_eq!(guarded_column_1(126), Some(127));
    assert_eq!(guarded_column_1(127), None);
    assert_eq!(guarded_column_1(128), None);
    assert_eq!(guarded_column_1(usize::MAX), None);
    assert_eq!(
        geometry_query_feature_index(65_535, 127),
        Some(65_536 * QWEN3_PREFILL_HEAD_DIMENSION_V1 - 1)
    );
    assert_eq!(geometry_query_feature_index(65_536, 0), None);
    assert_eq!(geometry_query_feature_index(65_535, 128), None);
    assert_eq!(geometry_query_feature_index(usize::MAX, 0), None);
}

#[test]
fn profile_guards_pin_all_eight_lengths_and_disambiguating_page_extents() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "q.len()==524_288&&pages.len()==512",
        "q.len()==4_194_304&&pages.len()==4_096",
        "q.len()==2_097_152&&pages.len()==512",
        "q.len()==8_388_608&&pages.len()==512",
        "q.len()==262_144&&pages.len()==512",
        "q.len()==2_097_152&&pages.len()==4_096",
        "q.len()==1_048_576&&pages.len()==512",
        "q.len()==4_194_304&&pages.len()==512",
        "k.len()!=QWEN3_PREFILL_CACHE_ELEMENTS_V1",
        "v.len()!=QWEN3_PREFILL_CACHE_ELEMENTS_V1",
        "output.len()!=q.len()",
    ] {
        assert!(body.contains(marker), "missing profile marker {marker}");
    }
}

#[test]
fn each_profile_derived_divisor_has_one_dominating_zero_guard() {
    let body = compact_tokens(kernel().block);
    let tokens_derivation = body.find("lettokens=").unwrap();
    let query_heads_guard = body
        .find("ifquery_heads==0{fe2o3_device::trap();}")
        .unwrap();
    let tokens_guard = body.find("iftokens==0{fe2o3_device::trap();}").unwrap();
    let gqa_group_size_guard = body
        .find("ifgqa_group_size==0{fe2o3_device::trap();}")
        .unwrap();
    let first_query_heads_use = body.find("letquery_head=vector%query_heads").unwrap();
    let first_tokens_use = body.find("letquery_token=position%tokens").unwrap();
    let first_gqa_group_size_use = body.find("letkv_head=query_head/gqa_group_size").unwrap();

    assert!(tokens_derivation < query_heads_guard);
    assert!(query_heads_guard < tokens_guard);
    assert!(tokens_guard < gqa_group_size_guard);
    assert!(gqa_group_size_guard < first_query_heads_use);
    assert!(gqa_group_size_guard < first_tokens_use);
    assert!(gqa_group_size_guard < first_gqa_group_size_use);
    assert_eq!(body.matches("ifquery_heads==0").count(), 1);
    assert_eq!(body.matches("iftokens==0").count(), 1);
    assert_eq!(body.matches("ifgqa_group_size==0").count(), 1);
}

#[test]
fn coordinates_preserve_vector_lane_gqa_and_global_p16_mapping() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "letvector=global/64",
        "letlocal=global%64",
        "letquery_head=vector%query_heads",
        "letposition=vector/query_heads",
        "letquery_token=position%tokens",
        "letsequence=position/tokens",
        "letkv_head=query_head/gqa_group_size",
        "letquery_base=vector*QWEN3_PREFILL_HEAD_DIMENSION_V1",
        "letcolumn_0=local*2",
        "letcolumn_1=column_0+1",
        "letpage_table_base=sequence*QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1",
        "letpage_table_index=page_table_base+logical_page",
        "letphysical_page=memory::volatile_load(pages,page_table_index)asusize",
        "ifphysical_page>=QWEN3_PREFILL_CACHE_POOL_PAGES_V1",
        "letpage_token_base=physical_page*QWEN3_PREFILL_PAGE_TOKENS_V1",
        "letpage_token=page_token_base+token_in_page",
        "letcache_head_base=page_token*QWEN3_PREFILL_KV_HEADS_V1",
        "letcache_head=cache_head_base+kv_head",
        "ifcache_head<=usize::MAX/QWEN3_PREFILL_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        "ifcache_head<2_097_152{}else{fe2o3_device::trap();}",
        "letcache_base=cache_head*QWEN3_PREFILL_HEAD_DIMENSION_V1",
    ] {
        assert!(body.contains(marker), "missing coordinate marker {marker}");
    }
    assert!(!body.contains("cache_base+sequence"));
}

#[test]
fn recurrence_is_ascending_d128_online_softmax_with_exact_scale() {
    let body = compact_tokens(kernel().block);
    let source = compact_tokens(syn::parse_file(SOURCE).expect("device source parses"));
    assert_eq!(
        QWEN3_PREFILL_ATTENTION_SCALE_V1.to_bits(),
        QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1
    );
    assert_eq!(
        source
            .matches("f32::from_bits(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1)")
            .count(),
        1
    );
    assert!(!body.contains("f32::from_bits"));
    for marker in [
        "whilekey_token<2_048",
        "ifkey_token<key_limit",
        "whilefeature<QWEN3_PREFILL_HEAD_DIMENSION_V1",
        "letproduct=query_value.to_f32()*key_value.to_f32()",
        "letnext_dot=dot+product",
        "letscale=QWEN3_PREFILL_ATTENTION_SCALE_V1",
        "letscore=dot*scale",
        "ifkey_token==0{running_max=score;running_sum=1.0;numerator_0=value_0;numerator_1=value_1;}",
        "letprevious_weight=math.exp_f32(running_max-next_max)",
        "letcurrent_weight=math.exp_f32(score-next_max)",
        "letnext_sum=running_sum*previous_weight+current_weight",
        "numerator_0*previous_weight+value_0*current_weight",
        "numerator_1*previous_weight+value_1*current_weight",
        "letoutput_0=numerator_0/running_sum",
        "letoutput_1=numerator_1/running_sum",
        "Bf16::from_f32(output_0)",
        "Bf16::from_f32(output_1)",
    ] {
        assert!(body.contains(marker), "missing recurrence marker {marker}");
    }
    assert_eq!(body.matches("math.exp_f32(").count(), 2);
    assert_eq!(body.matches("key_token+=1").count(), 1);
    assert_eq!(body.matches("feature+=1").count(), 1);
}

#[test]
fn output_witness_pins_adjacent_pair_and_two_constant_owned_stores() {
    let body = compact_tokens(kernel().block);
    assert!(body.contains("workitem.checked_block::<1,2>()"));
    assert_eq!(body.matches("output.write_block(&output_pair,").count(), 2);
    assert!(body.contains("output.write_block(&output_pair,0,output_0.to_bits())"));
    assert!(body.contains("output.write_block(&output_pair,1,output_1.to_bits())"));
    assert!(!body.contains("output.write("));
    assert!(!body.contains("get_mut"));
}
