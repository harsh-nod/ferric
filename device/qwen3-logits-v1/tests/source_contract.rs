use quote::ToTokens as _;
use syn::{FnArg, Item, ItemFn, Meta};

const SOURCE: &str = include_str!("../../qwen3-all-kernels-v1/src/logits.rs");

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

fn compact_body(function: &ItemFn) -> String {
    function
        .block
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn launch_tokens(function: &ItemFn) -> String {
    let attribute = function
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("kernel"))
        .expect("kernel attribute");
    let Meta::List(arguments) = &attribute.meta else {
        panic!("kernel attribute must carry the typed contract");
    };
    arguments.tokens.to_string()
}

#[test]
fn source_has_exact_three_root_roster_and_no_worker_escape_hatch() {
    let kernels = kernels();
    assert_eq!(kernels.len(), 3);
    assert_eq!(
        kernels
            .iter()
            .map(|function| function.sig.ident.to_string())
            .collect::<Vec<_>>(),
        [
            "ferric_qwen3_lowest_id_argmax_bf16_v1",
            "ferric_qwen3_compact_completion_v1",
            "ferric_qwen3_speculative_token_assembly_v1",
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
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "found forbidden marker {forbidden}"
        );
    }
}

#[test]
fn signatures_preserve_exact_slice_effects_carriers_and_scalar_order() {
    let kernels = kernels();
    let token_output = "WriteOnlyDisjointSlice<u32,RowStriped2D<Index1D,64,1>>";
    let record_output = "WriteOnlyDisjointSlice<u8,RowStriped2D<Index1D,64,2>>";

    assert_eq!(kernels[0].sig.inputs.len(), 4);
    assert_eq!(compact_type(&kernels[0].sig.inputs[0]), "&[u16]");
    assert_eq!(compact_type(&kernels[0].sig.inputs[1]), token_output);
    assert_eq!(compact_type(&kernels[0].sig.inputs[2]), "u32");
    assert_eq!(compact_type(&kernels[0].sig.inputs[3]), "u32");

    assert_eq!(kernels[1].sig.inputs.len(), 11);
    for (argument, expected) in kernels[1].sig.inputs.iter().zip([
        "&[u32]",
        "&[u32]",
        "&[u32]",
        "&[u32]",
        "&[u32]",
        "&[u64]",
        "&[u8]",
        record_output,
        "u32",
        "u32",
        "u32",
    ]) {
        assert_eq!(compact_type(argument), expected);
    }

    assert_eq!(kernels[2].sig.inputs.len(), 5);
    assert_eq!(compact_type(&kernels[2].sig.inputs[0]), "&[u32]");
    assert_eq!(compact_type(&kernels[2].sig.inputs[1]), "&[u32]");
    assert_eq!(compact_type(&kernels[2].sig.inputs[2]), token_output);
    assert_eq!(compact_type(&kernels[2].sig.inputs[3]), "u32");
    assert_eq!(compact_type(&kernels[2].sig.inputs[4]), "u32");
}

#[test]
fn launch_attributes_pin_wave64_and_exact_grid_caps() {
    let kernels = kernels();
    for kernel in &kernels {
        let tokens = launch_tokens(kernel);
        assert!(tokens.contains("typed"));
        assert!(tokens.contains("required = [64 , 1 , 1]"));
        assert!(tokens.contains("max = [64 , 1 , 1]"));
    }
    assert!(launch_tokens(&kernels[0]).contains("max_grid = [2048 , 1 , 1]"));
    assert!(launch_tokens(&kernels[0]).contains("loop_bounds (151936)"));
    assert!(launch_tokens(&kernels[1]).contains("max_grid = [32 , 1 , 1]"));
    assert!(launch_tokens(&kernels[1]).contains("loop_bounds (32 , 16 , 2)"));
    assert!(launch_tokens(&kernels[2]).contains("max_grid = [8 , 1 , 1]"));
    assert!(!launch_tokens(&kernels[2]).contains("control_flow"));
}

#[test]
fn immutable_reads_are_bounded_volatile_and_outputs_are_write_only() {
    let kernels = kernels();
    let argmax = compact_body(&kernels[0]);
    let compact = compact_body(&kernels[1]);
    let assembly = compact_body(&kernels[2]);

    assert_eq!(argmax.matches("memory::volatile_load(").count(), 2);
    assert_eq!(compact.matches("memory::volatile_load(").count(), 10);
    assert_eq!(assembly.matches("memory::volatile_load(").count(), 2);
    for body in [&argmax, &compact, &assembly] {
        assert!(!body.contains('['), "kernel body contains direct indexing");
        assert!(!body.contains("get_unchecked"));
        assert!(!body.contains("DisjointSlice<"));
        assert!(body.contains("write_row_striped_2d("));
    }
    assert_eq!(argmax.matches("write_row_striped_2d(").count(), 1);
    assert_eq!(compact.matches("write_row_striped_2d(").count(), 3);
    assert_eq!(assembly.matches("write_row_striped_2d(").count(), 1);
}

#[test]
fn compact_source_retains_canonical_record_and_direct_empty_draft_semantics() {
    let compact = compact_body(&kernels()[1]);
    for marker in [
        "draft.len()!=sequences*speculative_k",
        "records.len()!=sequences*QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1",
        "checked_row_striped_2d::<64,2>()",
        "iflive==0",
        "letslot=memory::volatile_load(request_slots,sequence)",
        "letgeneration=memory::volatile_load(request_generations,sequence)",
        "whileplan_byte<32",
        "if!plan_present",
        "if!direct{whileaccepted<speculative_k",
        "ifdraft_token!=target_token{break;}",
        "choice_base+live-1",
        "letbyte=component*64+lane",
        "ifbyte<QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1",
        "elseifbyte==48{acceptedasu8}",
        "elseifbyte==49{(accepted+1)asu8}",
        "lettoken_byte=byte-52",
    ] {
        assert!(compact.contains(marker), "missing compact marker {marker}");
    }
    assert!(!compact.contains("draft["));
    assert!(!compact.contains("records["));
}

#[test]
fn generated_host_adapters_retain_exact_kfd_read_write_effects() {
    use fe2o3_host::__generated::{
        CompilerGeneratedKernelExpectationV1, CompilerGeneratedKfdArguments, GeneratedKfdReadSlice,
        GeneratedKfdWriteSlice,
    };
    use ferric_qwen3_logits_device_v1::{
        ferric_qwen3_compact_completion_v1_gpu as compact,
        ferric_qwen3_lowest_id_argmax_bf16_v1_gpu as argmax,
        ferric_qwen3_speculative_token_assembly_v1_gpu as assembly,
    };

    fn assert_kfd_adapter<'allocation, K, A>()
    where
        K: CompilerGeneratedKernelExpectationV1,
        A: CompilerGeneratedKfdArguments<'allocation, K>,
    {
    }

    type ReadU8 = GeneratedKfdReadSlice<'static, u8>;
    type ReadU16 = GeneratedKfdReadSlice<'static, u16>;
    type ReadU32 = GeneratedKfdReadSlice<'static, u32>;
    type ReadU64 = GeneratedKfdReadSlice<'static, u64>;
    type WriteU8 = GeneratedKfdWriteSlice<'static, u8>;
    type WriteU32 = GeneratedKfdWriteSlice<'static, u32>;

    assert_kfd_adapter::<argmax::Marker, argmax::Arguments<'static, ReadU16, WriteU32>>();
    assert_kfd_adapter::<
        compact::Marker,
        compact::Arguments<
            'static,
            ReadU32,
            ReadU32,
            ReadU32,
            ReadU32,
            ReadU32,
            ReadU64,
            ReadU8,
            WriteU8,
        >,
    >();
    assert_kfd_adapter::<assembly::Marker, assembly::Arguments<'static, ReadU32, ReadU32, WriteU32>>(
    );

    let logits = [0_u16; 1];
    let mut choices_out = [0_u32; 1];
    let _argmax = argmax::Arguments::new(
        GeneratedKfdReadSlice::new(&logits),
        GeneratedKfdWriteSlice::new(&mut choices_out),
        1,
        151_936,
    );

    let choices = [0_u32; 1];
    let draft: [u32; 0] = [];
    let active_lengths = [1_u32];
    let slots = [0_u32];
    let generations = [1_u32];
    let epochs = [1_u64];
    let plans = [1_u8; 32];
    let mut records = [0_u8; 120];
    let _compact = compact::Arguments::new(
        GeneratedKfdReadSlice::new(&choices),
        GeneratedKfdReadSlice::new(&draft),
        GeneratedKfdReadSlice::new(&active_lengths),
        GeneratedKfdReadSlice::new(&slots),
        GeneratedKfdReadSlice::new(&generations),
        GeneratedKfdReadSlice::new(&epochs),
        GeneratedKfdReadSlice::new(&plans),
        GeneratedKfdWriteSlice::new(&mut records),
        1,
        1,
        0,
    );

    let anchors = [7_u32];
    let draft_choices = [11_u32; 4];
    let mut targets = [0_u32; 5];
    let _assembly = assembly::Arguments::new(
        GeneratedKfdReadSlice::new(&anchors),
        GeneratedKfdReadSlice::new(&draft_choices),
        GeneratedKfdWriteSlice::new(&mut targets),
        1,
        4,
    );
}
