use syn::{FnArg, Item, ItemFn, Meta};

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
        "active_tokens>QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1-committed_tokens",
        "letquery_position=committed_tokens+query_token",
        "whilekey_token<=query_position",
        "letpage_table_index=sequence*QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1+logical_page",
        "letphysical_page=memory::volatile_load(pages,page_table_index)asusize",
        "ifphysical_page>=QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1",
        "((physical_page*QWEN3_PAGED_DECODE_PAGE_TOKENS_V1+token_in_page)*QWEN3_PAGED_DECODE_KV_HEADS_V1+kv_head)*QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1",
    ] {
        assert!(body.contains(marker), "missing coordinate marker {marker}");
    }
    assert!(!body.contains("cache_base+sequence"));
}

#[test]
fn recurrence_is_ascending_d128_online_softmax_with_exact_scale() {
    let body = compact_tokens(kernel().block);
    for marker in [
        "whilefeature<QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1",
        "letproduct=query_value.to_f32()*key_value.to_f32()",
        "letnext_dot=dot+product",
        "letscale=f32::from_bits(QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1)",
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
