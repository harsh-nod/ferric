use syn::{FnArg, Item, ItemFn, Meta};

const SOURCE: &str = include_str!("../src/lib.rs");

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
    let body = compact_tokens(kernel().block);
    let shape_trap = body
        .find("if!(target||draft)||k.len()!=QWEN3_PREFILL_CACHE_ELEMENTS_V1")
        .unwrap();
    let global_trap = body.find("ifglobal>=q.len()/2").unwrap();
    let page_bounds = body
        .find("iflogical_page>=QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1")
        .unwrap();
    let page_load = body
        .find("memory::volatile_load(pages,page_table_index)")
        .unwrap();
    let physical_page_guard = body
        .find("ifphysical_page>=QWEN3_PREFILL_CACHE_POOL_PAGES_V1")
        .unwrap();
    let cache_extent_guard = body
        .find("ifcache_base+QWEN3_PREFILL_HEAD_DIMENSION_V1>k.len()")
        .unwrap();
    let query_load = body
        .find("memory::volatile_load(q,query_base+feature)")
        .unwrap();
    let value_load = body
        .find("memory::volatile_load(v,cache_base+column_0)")
        .unwrap();
    let first_store = body
        .find("output.write_block(&output_pair,0,output_0.to_bits())")
        .unwrap();

    assert!(shape_trap < global_trap);
    assert!(global_trap < page_bounds);
    assert!(page_bounds < page_load);
    assert!(page_load < physical_page_guard);
    assert!(physical_page_guard < cache_extent_guard);
    assert!(cache_extent_guard < query_load);
    assert!(query_load < value_load);
    assert!(value_load < first_store);
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
        "letpage_table_index=sequence*QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1+logical_page",
        "letphysical_page=memory::volatile_load(pages,page_table_index)asusize",
        "ifphysical_page>=QWEN3_PREFILL_CACHE_POOL_PAGES_V1",
        "((physical_page*QWEN3_PREFILL_PAGE_TOKENS_V1+token_in_page)*QWEN3_PREFILL_KV_HEADS_V1+kv_head)*QWEN3_PREFILL_HEAD_DIMENSION_V1",
    ] {
        assert!(body.contains(marker), "missing coordinate marker {marker}");
    }
    assert!(!body.contains("cache_base+sequence"));
}

#[test]
fn recurrence_is_ascending_d128_online_softmax_with_exact_scale() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "whilekey_token<=query_token",
        "whilefeature<QWEN3_PREFILL_HEAD_DIMENSION_V1",
        "letproduct=query_value.to_f32()*key_value.to_f32()",
        "letnext_dot=dot+product",
        "letscale=f32::from_bits(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1)",
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
