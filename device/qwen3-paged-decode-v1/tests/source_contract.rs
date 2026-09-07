use ferric_qwen3_paged_decode_device_v1::{
    QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1, QWEN3_PAGED_DECODE_ATTENTION_SCALE_V1,
    QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1, QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1,
    QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1, QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1,
    QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1, QWEN3_PAGED_DECODE_KV_HEADS_V1,
    QWEN3_PAGED_DECODE_MAX_GRID_WORKGROUPS_V1, QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1,
    QWEN3_PAGED_DECODE_PAGE_TOKENS_V1,
};
use syn::{Expr, FnArg, Item, ItemFn, Meta, Stmt};

const SOURCE: &str = include_str!("../../qwen3-all-kernels-v1/src/paged_decode.rs");

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
    assert_eq!(kernel.sig.ident, "qwen3_paged_gqa_decode_bf16_f32_v1");
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
fn signature_retains_exact_six_slice_abi_and_write_only_pair_authority() {
    let kernel = kernel();
    assert_eq!(kernel.sig.inputs.len(), 6);
    assert_eq!(compact_type(&kernel.sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[1]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[2]), "&[u16]");
    assert_eq!(compact_type(&kernel.sig.inputs[3]), "&[u32]");
    assert_eq!(compact_type(&kernel.sig.inputs[4]), "&[u32]");
    assert_eq!(
        compact_type(&kernel.sig.inputs[5]),
        "WriteOnlyDisjointSlice<u16,Blocked<Index1D,1,2>>"
    );
    assert!(matches!(kernel.sig.output, syn::ReturnType::Default));
}

#[test]
fn host_build_exposes_exact_five_read_one_write_kfd_adapter() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_paged_decode_device_v1::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::{
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
            GeneratedKfdReadSlice<'static, u32>,
            GeneratedKfdWriteSlice<'static, u16>,
        >,
    >();

    let q = [0_u16; 128];
    let k = [0_u16; 128];
    let v = [0_u16; 128];
    let pages = [0_u32; 1];
    let committed = [0_u32; 1];
    let mut output = [0_u16; 128];
    let _arguments = Arguments::new(
        GeneratedKfdReadSlice::new(&q),
        GeneratedKfdReadSlice::new(&k),
        GeneratedKfdReadSlice::new(&v),
        GeneratedKfdReadSlice::new(&pages),
        GeneratedKfdReadSlice::new(&committed),
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
    assert!(tokens.contains("max_grid = [1280 , 1 , 1]"));
    assert!(tokens.contains("loop_bounds (8192 , 128)"));
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
        "committed[",
        ".as_ptr(",
        "read_volatile(",
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
    assert_eq!(body.matches("memory::volatile_load(committed,").count(), 1);
}

#[test]
fn page_and_cache_arithmetic_is_guarded_before_every_dependent_read_or_store() {
    let kernel = kernel();
    let body = compact_tokens(&kernel.block);
    let committed_load = body
        .find("letcommitted_tokens=memory::volatile_load(committed,sequence)asusize")
        .unwrap();
    let active_capacity = body
        .find("letactive_capacity=ifcommitted_tokens<=QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1{QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1-committed_tokens}else{fe2o3_device::trap()}")
        .unwrap();
    let active_capacity_guard = body
        .find("ifactive_tokens>active_capacity{fe2o3_device::trap();}")
        .unwrap();
    let query_position = body
        .find("letquery_position=ifquery_token<active_capacity{committed_tokens+query_token}else{fe2o3_device::trap()}")
        .unwrap();
    let key_limit = body
        .find("letkey_limit=ifquery_position<8_192{query_position+1}else{fe2o3_device::trap()}")
        .unwrap();
    let query_base = body
        .find("letquery_base=ifvector<1_280{vector*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1}else{fe2o3_device::trap()}")
        .unwrap();
    let column_0 = body
        .find("letcolumn_0=iflocal<64{local*2}else{fe2o3_device::trap()}")
        .unwrap();
    let column_1 = body
        .find("letcolumn_1=ifcolumn_0<128{column_0+1}else{fe2o3_device::trap()}")
        .unwrap();
    let column_1_guard = body
        .find("ifcolumn_1<128{}else{fe2o3_device::trap();}")
        .unwrap();
    let key_loop = body.find("whilekey_token<8_192{").unwrap();
    let key_limit_effect_guard = body.find("ifkey_token<key_limit{").unwrap();
    let page_load = body
        .find("memory::volatile_load(pages,page_table_index)")
        .unwrap();
    let markers = [
        "iflogical_page<QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1{}else{fe2o3_device::trap();}",
        "letpage_table_base=ifsequence<32{sequence*QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1}else{fe2o3_device::trap()}",
        "letpage_table_index=iflogical_page<=usize::MAX-page_table_base{page_table_base+logical_page}else{fe2o3_device::trap()}",
        "ifpage_table_index<pages.len(){}else{fe2o3_device::trap();}",
    ];
    let mut prior = 0;
    for marker in markers {
        let position = body
            .find(marker)
            .unwrap_or_else(|| panic!("missing page-table proof marker {marker}"));
        assert!(
            prior < position,
            "page-table proof marker out of order {marker}"
        );
        prior = position;
    }
    assert!(committed_load < active_capacity);
    assert!(active_capacity < active_capacity_guard);
    assert!(active_capacity_guard < query_position);
    assert!(query_position < key_limit);
    assert!(key_limit < query_base);
    assert!(query_base < column_0);
    assert!(column_0 < column_1);
    assert!(column_1 < column_1_guard);
    assert!(column_1_guard < key_loop);
    assert!(key_loop < key_limit_effect_guard);
    assert!(key_limit_effect_guard < prior);
    assert!(prior < page_load);

    let physical_page_guard = body
        .find("ifphysical_page>=QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1{fe2o3_device::trap();}")
        .unwrap();
    let cache_len_marker = "letcache_len=k.len()";
    let cache_remaining_marker = "letcache_remaining=ifcache_base<=cache_len{cache_len-cache_base}else{fe2o3_device::trap();}";
    let cache_markers = [
        "letpage_token_base=ifphysical_page<=usize::MAX/QWEN3_PAGED_DECODE_PAGE_TOKENS_V1{physical_page*QWEN3_PAGED_DECODE_PAGE_TOKENS_V1}else{fe2o3_device::trap()}",
        "letpage_token=iftoken_in_page<=usize::MAX-page_token_base{page_token_base+token_in_page}else{fe2o3_device::trap()}",
        "letcache_head_base=ifpage_token<=usize::MAX/QWEN3_PAGED_DECODE_KV_HEADS_V1{page_token*QWEN3_PAGED_DECODE_KV_HEADS_V1}else{fe2o3_device::trap()}",
        "letcache_head=ifkv_head<=usize::MAX-cache_head_base{cache_head_base+kv_head}else{fe2o3_device::trap()}",
        "ifcache_head<=usize::MAX/QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        "letcache_base=ifcache_head<2_097_152{cache_head*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1}else{fe2o3_device::trap()}",
        cache_len_marker,
        cache_remaining_marker,
        "ifQWEN3_PAGED_DECODE_HEAD_DIMENSION_V1<=cache_remaining{}else{fe2o3_device::trap();}",
    ];
    prior = physical_page_guard;
    for marker in cache_markers {
        let position = body
            .find(marker)
            .unwrap_or_else(|| panic!("missing cache proof marker {marker}"));
        assert!(prior < position, "cache proof marker out of order {marker}");
        prior = position;
    }
    let query_load = body.find("memory::volatile_load(q,query_index)").unwrap();
    let feature_loop = body
        .find("whilefeature<QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1{")
        .unwrap();
    let query_index = body.find("letquery_index=query_base+feature").unwrap();
    let key_index = body.find("letkey_index=cache_base+feature").unwrap();
    let key_load = body.find("memory::volatile_load(k,key_index)").unwrap();
    let value_index_0 = body.find("letvalue_index_0=cache_base+column_0").unwrap();
    let value_load = body.find("memory::volatile_load(v,value_index_0)").unwrap();
    let value_index_1 = body.find("letvalue_index_1=cache_base+column_1").unwrap();
    let second_value_load = body.find("memory::volatile_load(v,value_index_1)").unwrap();
    let key_token_increment_guard = body
        .find("ifkey_token<8_192{key_token+=1;}else{fe2o3_device::trap();}")
        .unwrap();
    let first_store = body
        .find("output.write_block(&output_pair,0,output_0.to_bits())")
        .unwrap();
    assert!(prior < feature_loop);
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
        "iflogical_page<QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1{}else{fe2o3_device::trap();}",
        "letpage_table_base=ifsequence<32{sequence*QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1}else{fe2o3_device::trap()}",
        "letpage_table_index=iflogical_page<=usize::MAX-page_table_base{page_table_base+logical_page}else{fe2o3_device::trap()}",
        "ifpage_table_index<pages.len(){}else{fe2o3_device::trap();}",
        "letpage_token_base=ifphysical_page<=usize::MAX/QWEN3_PAGED_DECODE_PAGE_TOKENS_V1{physical_page*QWEN3_PAGED_DECODE_PAGE_TOKENS_V1}else{fe2o3_device::trap()}",
        "letpage_token=iftoken_in_page<=usize::MAX-page_token_base{page_token_base+token_in_page}else{fe2o3_device::trap()}",
        "letcache_head_base=ifpage_token<=usize::MAX/QWEN3_PAGED_DECODE_KV_HEADS_V1{page_token*QWEN3_PAGED_DECODE_KV_HEADS_V1}else{fe2o3_device::trap()}",
        "letcache_head=ifkv_head<=usize::MAX-cache_head_base{cache_head_base+kv_head}else{fe2o3_device::trap()}",
        "ifcache_head<=usize::MAX/QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        "letcache_base=ifcache_head<2_097_152{cache_head*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1}else{fe2o3_device::trap()}",
        cache_len_marker,
        cache_remaining_marker,
        "ifQWEN3_PAGED_DECODE_HEAD_DIMENSION_V1<=cache_remaining{}else{fe2o3_device::trap();}",
        "letkey_index=cache_base+feature",
        "letvalue_index_0=cache_base+column_0",
        "letvalue_index_1=cache_base+column_1",
        "ifkey_token<8_192{key_token+=1;}else{fe2o3_device::trap();}",
        "letquery_base=ifvector<1_280{vector*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1}else{fe2o3_device::trap()}",
        "letcolumn_0=iflocal<64{local*2}else{fe2o3_device::trap()}",
        "letcolumn_1=ifcolumn_0<128{column_0+1}else{fe2o3_device::trap()}",
        "ifcolumn_1<128{}else{fe2o3_device::trap();}",
        "letquery_index=query_base+feature",
        "letquery_position=ifquery_token<active_capacity{committed_tokens+query_token}else{fe2o3_device::trap()}",
        "letkey_limit=ifquery_position<8_192{query_position+1}else{fe2o3_device::trap()}",
        "whilekey_token<8_192{",
        "ifkey_token<key_limit{",
        "letactive_capacity=ifcommitted_tokens<=QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1{QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1-committed_tokens}else{fe2o3_device::trap()}",
        "ifactive_tokens>active_capacity{fe2o3_device::trap();}",
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
    assert!(!body.contains("cache_base+QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1"));
    assert!(!body.contains("cache_base.checked_add("));
    assert!(!body.contains("cache_base.wrapping_add("));
    assert!(!body.contains("cache_base.overflowing_add("));

    let uniform_loop = kernel
        .block
        .stmts
        .iter()
        .find_map(|statement| match statement {
            Stmt::Expr(Expr::While(expression), _)
                if compact_tokens(&expression.cond) == "key_token<8_192" =>
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
        "ifkey_token<8_192{key_token+=1;}else{fe2o3_device::trap();}"
    );
}

fn guarded_page_table_index(sequence: usize, logical_page: usize) -> Option<usize> {
    if logical_page < QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 {
    } else {
        return None;
    }
    if sequence < 32 {
    } else {
        return None;
    }
    let page_table_base = sequence * QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1;
    if logical_page <= usize::MAX - page_table_base {
    } else {
        return None;
    }
    Some(page_table_base + logical_page)
}

fn guarded_cache_base(physical_page: usize, token_in_page: usize, kv_head: usize) -> Option<usize> {
    if physical_page <= usize::MAX / QWEN3_PAGED_DECODE_PAGE_TOKENS_V1 {
    } else {
        return None;
    }
    let page_token_base = physical_page * QWEN3_PAGED_DECODE_PAGE_TOKENS_V1;
    if token_in_page <= usize::MAX - page_token_base {
    } else {
        return None;
    }
    let page_token = page_token_base + token_in_page;
    if page_token <= usize::MAX / QWEN3_PAGED_DECODE_KV_HEADS_V1 {
    } else {
        return None;
    }
    let cache_head_base = page_token * QWEN3_PAGED_DECODE_KV_HEADS_V1;
    if kv_head <= usize::MAX - cache_head_base {
    } else {
        return None;
    }
    let cache_head = cache_head_base + kv_head;
    guarded_cache_base_from_head(cache_head)
}

fn guarded_cache_base_from_head(cache_head: usize) -> Option<usize> {
    if cache_head < QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1 {
        Some(cache_head * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
    } else {
        None
    }
}

fn guarded_cache_end(cache_base: usize, cache_len: usize) -> Option<usize> {
    if cache_base <= cache_len {
    } else {
        return None;
    }
    let cache_remaining = cache_len - cache_base;
    if QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 <= cache_remaining {
    } else {
        return None;
    }
    Some(cache_base + QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
}

fn geometry_value_columns(
    cache_head: usize,
    column_0: usize,
    column_1: usize,
) -> Option<(usize, usize)> {
    let cache_base = guarded_cache_base_from_head(cache_head)?;
    if column_0 >= QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
        || column_1 >= QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
    {
        return None;
    }
    Some((cache_base + column_0, cache_base + column_1))
}

fn geometry_cache_feature_index(cache_head: usize, feature: usize) -> Option<usize> {
    let cache_base = guarded_cache_base_from_head(cache_head)?;
    if feature >= QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 {
        return None;
    }
    Some(cache_base + feature)
}

fn guarded_key_token_increment(mut key_token: usize) -> Option<usize> {
    if key_token < 8_192 {
        key_token += 1;
    } else {
        return None;
    }
    Some(key_token)
}

fn guarded_key_limit(query_position: usize) -> Option<usize> {
    if query_position < 8_192 {
        Some(query_position + 1)
    } else {
        None
    }
}

fn guarded_active_capacity(committed_tokens: usize, active_tokens: usize) -> Option<usize> {
    if active_tokens == 0 {
        return None;
    }
    if committed_tokens <= QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 {
    } else {
        return None;
    }
    let active_capacity = QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - committed_tokens;
    if active_tokens > active_capacity {
        return None;
    }
    Some(active_capacity)
}

fn guarded_query_position(
    committed_tokens: usize,
    active_tokens: usize,
    query_token: usize,
) -> Option<usize> {
    let active_capacity = guarded_active_capacity(committed_tokens, active_tokens)?;
    if query_token < active_capacity {
        Some(committed_tokens + query_token)
    } else {
        None
    }
}

fn modeled_key_tokens(query_position: usize) -> Option<Vec<usize>> {
    let key_limit = guarded_key_limit(query_position)?;
    let mut key_token = 0;
    let mut visited = Vec::new();
    while key_token < 8_192 {
        if key_token >= key_limit {
            break;
        }
        visited.push(key_token);
        key_token = guarded_key_token_increment(key_token)?;
    }
    Some(visited)
}

fn guarded_query_base(vector: usize) -> Option<usize> {
    if vector < 1_280 {
    } else {
        return None;
    }
    Some(vector * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
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
    if feature >= QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 {
        return None;
    }
    Some(query_base + feature)
}

#[test]
fn guarded_coordinate_models_accept_endpoints_and_reject_hostile_overflow() {
    assert_eq!(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1, 8_192);
    assert_eq!(
        QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1 * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
        QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1
    );
    assert_eq!(
        guarded_cache_base_from_head(QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1 - 1),
        Some(QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1 - QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
    );
    assert_eq!(
        guarded_cache_base_from_head(QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1),
        None
    );
    assert_eq!(guarded_cache_base_from_head(usize::MAX), None);
    assert_eq!(guarded_page_table_index(0, 0), Some(0));
    assert_eq!(
        guarded_page_table_index(31, QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 - 1),
        Some(32 * QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 - 1)
    );
    assert_eq!(guarded_page_table_index(usize::MAX, 0), None);
    assert_eq!(
        guarded_page_table_index(32, QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 - 1),
        None
    );
    assert_eq!(
        guarded_page_table_index(0, QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1),
        None
    );

    assert_eq!(guarded_cache_base(0, 0, 0), Some(0));
    let last_base = guarded_cache_base(
        QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1 - 1,
        QWEN3_PAGED_DECODE_PAGE_TOKENS_V1 - 1,
        QWEN3_PAGED_DECODE_KV_HEADS_V1 - 1,
    )
    .unwrap();
    assert_eq!(
        last_base + QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
        QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1
    );
    assert_eq!(guarded_cache_base(usize::MAX, 0, 0), None);
    assert_eq!(
        guarded_cache_base(
            usize::MAX / QWEN3_PAGED_DECODE_PAGE_TOKENS_V1,
            QWEN3_PAGED_DECODE_PAGE_TOKENS_V1 - 1,
            0,
        ),
        None
    );
    assert_eq!(
        guarded_cache_base(
            usize::MAX / QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
            0,
            QWEN3_PAGED_DECODE_KV_HEADS_V1 - 1,
        ),
        None
    );

    let width = QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1;
    let cache_len = QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1;
    assert_eq!(guarded_cache_end(0, cache_len), Some(width));
    assert_eq!(
        guarded_cache_end(cache_len - width, cache_len),
        Some(cache_len)
    );
    assert_eq!(guarded_cache_end(cache_len - (width - 1), cache_len), None);
    assert_eq!(
        guarded_cache_end(usize::MAX - (width - 1), usize::MAX),
        None
    );

    let last_head = QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1 - 1;
    assert_eq!(
        geometry_value_columns(last_head, width - 2, width - 1),
        Some((
            QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1 - 2,
            QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1 - 1,
        ))
    );
    assert_eq!(
        geometry_value_columns(QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1, 0, 1),
        None
    );
    assert_eq!(geometry_value_columns(last_head, 127, 128), None);
    assert_eq!(geometry_value_columns(last_head, 128, 0), None);

    assert_eq!(
        geometry_cache_feature_index(last_head, width - 1),
        Some(QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1 - 1)
    );
    assert_eq!(
        geometry_cache_feature_index(QWEN3_PAGED_DECODE_CACHE_HEAD_CAPACITY_V1, 0),
        None
    );
    assert_eq!(geometry_cache_feature_index(last_head, 128), None);
    assert_eq!(geometry_cache_feature_index(usize::MAX, 0), None);

    assert_eq!(guarded_key_limit(0), Some(1));
    assert_eq!(guarded_key_limit(8_191), Some(8_192));
    assert_eq!(guarded_key_limit(8_192), None);
    assert_eq!(guarded_key_limit(usize::MAX), None);
    assert_eq!(
        guarded_active_capacity(0, QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1),
        Some(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1)
    );
    assert_eq!(
        guarded_active_capacity(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - 1, 1),
        Some(1)
    );
    assert_eq!(
        guarded_active_capacity(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - 1, 2),
        None
    );
    assert_eq!(
        guarded_active_capacity(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - 1, 0),
        None
    );
    assert_eq!(
        guarded_active_capacity(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1, 0),
        None
    );
    assert_eq!(
        guarded_active_capacity(QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1, 1),
        None
    );
    assert_eq!(guarded_active_capacity(usize::MAX, 1), None);
    assert_eq!(
        guarded_query_position(0, QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1, 8_191),
        Some(8_191)
    );
    assert_eq!(guarded_query_position(8_191, 1, 0), Some(8_191));
    assert_eq!(guarded_query_position(8_191, 1, 1), None);
    assert_eq!(guarded_query_position(8_192, 0, 0), None);
    assert_eq!(guarded_query_position(usize::MAX, 0, usize::MAX), None);
    assert_eq!(modeled_key_tokens(0), Some(vec![0]));
    let full = modeled_key_tokens(8_191).unwrap();
    assert_eq!(full.len(), 8_192);
    assert_eq!(full.first(), Some(&0));
    assert_eq!(full.last(), Some(&8_191));
    assert_eq!(modeled_key_tokens(8_192), None);
    assert_eq!(modeled_key_tokens(usize::MAX), None);
    assert_eq!(guarded_key_token_increment(0), Some(1));
    assert_eq!(guarded_key_token_increment(8_191), Some(8_192));
    assert_eq!(guarded_key_token_increment(8_192), None);
    assert_eq!(guarded_key_token_increment(usize::MAX), None);

    assert_eq!(QWEN3_PAGED_DECODE_MAX_GRID_WORKGROUPS_V1 as usize, 1_280);
    assert_eq!(guarded_query_base(0), Some(0));
    assert_eq!(
        guarded_query_base(1_279),
        Some(1_279 * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
    );
    assert_eq!(guarded_query_base(1_280), None);
    assert_eq!(guarded_query_base(usize::MAX), None);

    assert_eq!(QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1, 128);
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
        geometry_query_feature_index(1_279, 127),
        Some(1_280 * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 - 1)
    );
    assert_eq!(geometry_query_feature_index(1_280, 0), None);
    assert_eq!(geometry_query_feature_index(1_279, 128), None);
    assert_eq!(geometry_query_feature_index(usize::MAX, 0), None);
}

#[test]
fn profile_guards_pin_all_fourteen_lengths_and_shared_cache_extents() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "q.len()==4_096&&pages.len()==512&&committed.len()==1",
        "q.len()==32_768&&pages.len()==4_096&&committed.len()==8",
        "q.len()==131_072&&pages.len()==16_384&&committed.len()==32",
        "q.len()==20_480&&pages.len()==512&&committed.len()==1",
        "q.len()==163_840&&pages.len()==4_096&&committed.len()==8",
        "q.len()==36_864&&pages.len()==512&&committed.len()==1",
        "q.len()==69_632&&pages.len()==512&&committed.len()==1",
        "q.len()==2_048&&pages.len()==512&&committed.len()==1",
        "q.len()==16_384&&pages.len()==4_096&&committed.len()==8",
        "q.len()==65_536&&pages.len()==16_384&&committed.len()==32",
        "q.len()==8_192&&pages.len()==512&&committed.len()==1",
        "q.len()==65_536&&pages.len()==4_096&&committed.len()==8",
        "q.len()==16_384&&pages.len()==512&&committed.len()==1",
        "q.len()==32_768&&pages.len()==512&&committed.len()==1",
        "k.len()!=QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1",
        "v.len()!=QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1",
        "output.len()!=q.len()",
    ] {
        assert!(body.contains(marker), "missing profile marker {marker}");
    }
}

#[test]
fn coordinate_divisors_are_authenticated_before_workitem_arithmetic() {
    let body = compact_tokens(kernel().block);
    let derivation = body
        .find("letactive_tokens=")
        .expect("active-token profile derivation is present");
    let query_heads_guard = body
        .find("ifquery_heads==0{fe2o3_device::trap();}")
        .expect("query-head divisor is authenticated");
    let active_tokens_guard = body
        .find("ifactive_tokens==0{fe2o3_device::trap();}")
        .expect("active-token divisor is authenticated");
    let gqa_guard = body
        .find("ifgqa_group_size==0{fe2o3_device::trap();}")
        .expect("GQA divisor is authenticated");
    let workitem = body
        .find("letworkitem=thread::index_1d();")
        .expect("workitem arithmetic follows profile authentication");

    assert!(derivation < query_heads_guard);
    assert!(query_heads_guard < active_tokens_guard);
    assert!(active_tokens_guard < gqa_guard && gqa_guard < workitem);
    for use_marker in [
        "%query_heads",
        "/query_heads",
        "%active_tokens",
        "/active_tokens",
        "/gqa_group_size",
    ] {
        let divisor_use = body
            .find(use_marker)
            .unwrap_or_else(|| panic!("missing divisor use {use_marker}"));
        assert!(
            gqa_guard < divisor_use,
            "guard follows divisor use {use_marker}"
        );
    }
    for guard in [
        "ifquery_heads==0{fe2o3_device::trap();}",
        "ifactive_tokens==0{fe2o3_device::trap();}",
        "ifgqa_group_size==0{fe2o3_device::trap();}",
    ] {
        assert_eq!(body.matches(guard).count(), 1, "guard count for {guard}");
    }
}

#[test]
fn coordinates_preserve_committed_causality_gqa_and_global_p16_mapping() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "letquery_heads=iftarget{32}else{16}",
        "letgqa_group_size=iftarget{4}else{2}",
        "letvector=global/64",
        "letlocal=global%64",
        "letquery_head=vector%query_heads",
        "letposition=vector/query_heads",
        "letquery_token=position%active_tokens",
        "letsequence=position/active_tokens",
        "letkv_head=query_head/gqa_group_size",
        "letcommitted_tokens=memory::volatile_load(committed,sequence)asusize",
        "letactive_capacity=ifcommitted_tokens<=QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1{QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1-committed_tokens}else{fe2o3_device::trap()}",
        "ifactive_tokens>active_capacity{fe2o3_device::trap();}",
        "letquery_position=ifquery_token<active_capacity{committed_tokens+query_token}else{fe2o3_device::trap()}",
        "whilekey_token<8_192",
        "ifkey_token<key_limit",
        "letpage_table_base=ifsequence<32{sequence*QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1}else{fe2o3_device::trap()}",
        "letpage_table_index=iflogical_page<=usize::MAX-page_table_base{page_table_base+logical_page}else{fe2o3_device::trap()}",
        "letphysical_page=memory::volatile_load(pages,page_table_index)asusize",
        "ifphysical_page>=QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1",
        "letpage_token_base=ifphysical_page<=usize::MAX/QWEN3_PAGED_DECODE_PAGE_TOKENS_V1{physical_page*QWEN3_PAGED_DECODE_PAGE_TOKENS_V1}else{fe2o3_device::trap()}",
        "letpage_token=iftoken_in_page<=usize::MAX-page_token_base{page_token_base+token_in_page}else{fe2o3_device::trap()}",
        "letcache_head_base=ifpage_token<=usize::MAX/QWEN3_PAGED_DECODE_KV_HEADS_V1{page_token*QWEN3_PAGED_DECODE_KV_HEADS_V1}else{fe2o3_device::trap()}",
        "letcache_head=ifkv_head<=usize::MAX-cache_head_base{cache_head_base+kv_head}else{fe2o3_device::trap()}",
        "ifcache_head<=usize::MAX/QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1{}else{fe2o3_device::trap();}",
        "letcache_base=ifcache_head<2_097_152{cache_head*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1}else{fe2o3_device::trap()}",
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
        QWEN3_PAGED_DECODE_ATTENTION_SCALE_V1.to_bits(),
        QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1
    );
    assert_eq!(
        source
            .matches("f32::from_bits(QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1)")
            .count(),
        1
    );
    assert!(!body.contains("f32::from_bits"));
    for marker in [
        "whilefeature<QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1",
        "letproduct=query_value.to_f32()*key_value.to_f32()",
        "letnext_dot=dot+product",
        "letscale=QWEN3_PAGED_DECODE_ATTENTION_SCALE_V1",
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
