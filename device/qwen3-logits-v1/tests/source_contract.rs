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
    assert!(launch_tokens(&kernels[1]).contains("integer_switches (u32)"));
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
        "letdirect_offset=live-1",
        "letslot=memory::volatile_load(request_slots,sequence)",
        "letgeneration=memory::volatile_load(request_generations,sequence)",
        "whileplan_byte<32",
        "if!plan_present",
        "letmutcandidate=0",
        "letmutmatching_prefix=true",
        "letspeculative_k_usize=speculative_kasusize",
        "matchspeculative_k{0=>{}_=>{whilecandidate<speculative_k_usize",
        "ifmatching_prefix",
        "ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}",
        "ifcandidate<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}",
        "ifcandidate<active_tokens{}else{fe2o3_device::trap();}",
        "letdraft_row=candidate*sequences",
        "letdraft_index=draft_row+sequence",
        "lettarget_index=choice_base+candidate",
        "ifdraft_token==target_token{ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}accepted+=1;}else{matching_prefix=false;}",
        "candidate+=1",
        "ifaccepted>=active_tokens{fe2o3_device::trap();}",
        "ifdirect_offset>=active_tokens{fe2o3_device::trap();}",
        "choice_base+direct_offset",
        "ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}",
        "letbyte=component*64+lane",
        "ifbyte<QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1",
        "elseifbyte==48{acceptedasu8}",
        "elseifbyte==49{(accepted+1)asu8}",
        "ifbyte>=4{}else{fe2o3_device::trap();}",
        "letgeneration_byte=byte-4",
        "ifgeneration_byte<4{}else{fe2o3_device::trap();}",
        "ifgeneration_byte==0{generationasu8}",
        "elseifgeneration_byte==1{(generation>>8)asu8}",
        "elseifgeneration_byte==2{(generation>>16)asu8}",
        "else{(generation>>24)asu8}",
        "ifbyte>=8{}else{fe2o3_device::trap();}",
        "letepoch_byte=byte-8",
        "ifepoch_byte<8{}else{fe2o3_device::trap();}",
        "letepoch_word=memory::volatile_load(completion_epochs,sequence)",
        "ifepoch_byte==0{epoch_wordasu8}",
        "elseifepoch_byte==1{(epoch_word>>8)asu8}",
        "elseifepoch_byte==2{(epoch_word>>16)asu8}",
        "elseifepoch_byte==3{(epoch_word>>24)asu8}",
        "elseifepoch_byte==4{(epoch_word>>32)asu8}",
        "elseifepoch_byte==5{(epoch_word>>40)asu8}",
        "elseifepoch_byte==6{(epoch_word>>48)asu8}",
        "else{(epoch_word>>56)asu8}",
        "ifbyte>=16{}else{fe2o3_device::trap();}",
        "letplan_offset=byte-16",
        "ifplan_offset<32{}else{fe2o3_device::trap();}",
        "lettoken_byte=byte-52",
        "ifbyte>=52{}else{fe2o3_device::trap();}",
        "iftoken<speculative_k_usize{}else{fe2o3_device::trap();}",
        "iftoken<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}",
        "ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}",
        "ifsequence<sequences{}else{fe2o3_device::trap();}",
        "lettoken_draft_row=token*sequences",
        "lettoken_draft_index=token_draft_row+sequence",
    ] {
        assert!(compact.contains(marker), "missing compact marker {marker}");
    }
    let optional_entry = compact
        .split_once("letmutaccepted=0")
        .and_then(|(_, tail)| {
            tail.split_once("letcorrection_index=")
                .map(|(entry, _)| entry)
        })
        .expect("optional compact entry is bounded by accepted and correction");
    assert!(!optional_entry.contains("ifdirect{"));
    assert!(!optional_entry.contains("if!direct{"));
    assert!(!optional_entry.contains("matchspeculative_k_usize"));
    assert!(!compact.contains("whileaccepted<speculative_k"));
    assert!(!compact.contains("break;"));
    let candidate_guard = optional_entry
        .find("ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}")
        .expect("candidate is reauthenticated at the draft read site");
    let candidate_cap_guard = optional_entry
        .find("ifcandidate<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
        .expect("candidate is bounded before draft row multiplication");
    let active_row_guard = optional_entry
        .find("ifcandidate<active_tokens{}else{fe2o3_device::trap();}")
        .expect("candidate is reauthenticated against the target row width");
    let candidate_sequence_count_guard = optional_entry
        .find(
            "ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}",
        )
        .expect("candidate draft row count is bounded before multiplication");
    let candidate_sequence_guard = optional_entry
        .find("ifsequence<sequences{}else{fe2o3_device::trap();}")
        .expect("candidate draft row offset is reauthenticated before addition");
    let draft_row = optional_entry
        .find("letdraft_row=candidate*sequences;")
        .expect("candidate draft row is materialized before its index");
    let draft_index = optional_entry
        .find("letdraft_index=draft_row+sequence;")
        .expect("draft index follows the candidate guard");
    let target_index = optional_entry
        .find("lettarget_index=choice_base+candidate;")
        .expect("target index follows the active-row guard");
    let draft_read = optional_entry
        .find("memory::volatile_load(draft,draft_index)")
        .expect("draft read uses the guarded index");
    let target_read = optional_entry
        .find("memory::volatile_load(choices,target_index)")
        .expect("target read uses the guarded index");
    assert!(
        candidate_guard < candidate_cap_guard
            && candidate_cap_guard < active_row_guard
            && active_row_guard < candidate_sequence_count_guard
            && candidate_sequence_count_guard < candidate_sequence_guard
            && candidate_sequence_guard < draft_row
            && draft_row < draft_index
            && draft_index < target_index
            && target_index < draft_read
            && draft_read < target_read
    );
    assert_eq!(
        compact
            .matches("ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(compact.contains(
        "ifmatching_prefix{ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}ifcandidate<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}ifcandidate<active_tokens{}else{fe2o3_device::trap();}ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}ifsequence<sequences{}else{fe2o3_device::trap();}letdraft_row=candidate*sequences;letdraft_index=draft_row+sequence;lettarget_index=choice_base+candidate;letdraft_token=memory::volatile_load(draft,draft_index);lettarget_token=memory::volatile_load(choices,target_index);"
    ));
    assert_eq!(
        compact.matches("letdraft_row=candidate*sequences;").count(),
        1
    );
    assert_eq!(
        compact
            .matches("letdraft_index=draft_row+sequence;")
            .count(),
        1
    );
    assert!(!compact.contains("letdraft_index=candidate*sequences+sequence"));
    assert_eq!(
        compact
            .matches("ifcandidate<active_tokens{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact.contains("ifmatching_prefix{letdraft_index="));
    let accepted_increment_guard = compact
        .find("ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
        .expect("accepted count is bounded immediately before increment");
    let accepted_increment = compact
        .find("accepted+=1;")
        .expect("matching target increments the accepted count");
    assert!(accepted_increment_guard < accepted_increment);
    assert_eq!(
        compact
            .matches("ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    let (_, after_accepted_bound) = compact
        .split_once("ifaccepted>=active_tokens{fe2o3_device::trap();}")
        .expect("accepted bound guard follows optional prefix matching");
    assert!(
        after_accepted_bound.starts_with(
            "ifdirect_offset>=active_tokens{fe2o3_device::trap();}letcorrection_index="
        ),
        "accepted bound guard must precede the direct-offset guard and correction indexing"
    );
    assert_eq!(compact.matches("ifaccepted>=active_tokens").count(), 1);
    let emitted_guard = compact
        .find("ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}")
        .expect("accepted token count is bounded before record packing");
    let record_loop = compact
        .find("letmutcomponent=0;whilecomponent<2")
        .expect("record packing follows the emitted-token guard");
    let emitted_count = compact
        .find("(accepted+1)asu8")
        .expect("record metadata encodes the guarded emitted-token count");
    assert!(emitted_guard < record_loop && record_loop < emitted_count);
    assert_eq!(
        compact
            .matches("ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}",)
            .count(),
        1
    );
    assert!(compact.contains(
        "ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}letmutcomponent=0;whilecomponent<2"
    ));
    let zero_exit = compact
        .find("iflive==0{")
        .expect("empty live rows exit before direct indexing");
    let direct_offset = compact
        .find("letdirect_offset=live-1;")
        .expect("direct indexing subtracts only from authenticated nonzero live");
    let zero_return = compact[zero_exit..]
        .find("return;}")
        .map(|offset| zero_exit + offset)
        .expect("empty live rows return before direct indexing");
    let direct_use = compact
        .find("choice_base+direct_offset")
        .expect("direct correction uses the authenticated offset");
    let direct_bound = compact
        .find("ifdirect_offset>=active_tokens{fe2o3_device::trap();}")
        .expect("direct correction offset is reauthenticated against its row width");
    assert!(zero_exit < zero_return);
    assert!(
        zero_return < direct_offset && direct_offset < direct_bound && direct_bound < direct_use
    );
    assert_eq!(compact.matches("letdirect_offset=live-1;").count(), 1);
    assert_eq!(
        compact
            .matches("ifdirect_offset>=active_tokens{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact.contains("choice_base+live-1"));
    let generation_lower_guard = compact
        .find("ifbyte>=4{}else{fe2o3_device::trap();}")
        .expect("generation byte lower bound is authenticated before subtraction");
    let generation_guard = compact
        .find("ifgeneration_byte<4{}else{fe2o3_device::trap();}")
        .expect("generation byte is authenticated before literal shift selection");
    let generation_load = compact
        .find("ifgeneration_byte==0{generationasu8}elseifgeneration_byte==1{(generation>>8)asu8}elseifgeneration_byte==2{(generation>>16)asu8}else{(generation>>24)asu8}")
        .expect("generation bytes use only literal shifts");
    assert!(generation_lower_guard < generation_guard && generation_guard < generation_load);
    for marker in [
        "ifbyte>=4{}else{fe2o3_device::trap();}",
        "letgeneration_byte=byte-4;",
        "ifgeneration_byte<4{}else{fe2o3_device::trap();}",
        "ifgeneration_byte==0{generationasu8}",
        "elseifgeneration_byte==1{(generation>>8)asu8}",
        "elseifgeneration_byte==2{(generation>>16)asu8}",
        "else{(generation>>24)asu8}",
    ] {
        assert_eq!(compact.matches(marker).count(), 1, "count for {marker}");
    }
    assert!(compact.contains(
        "elseifbyte<8{ifbyte>=4{}else{fe2o3_device::trap();}letgeneration_byte=byte-4;ifgeneration_byte<4{}else{fe2o3_device::trap();}ifgeneration_byte==0{generationasu8}elseifgeneration_byte==1{(generation>>8)asu8}elseifgeneration_byte==2{(generation>>16)asu8}else{(generation>>24)asu8}"
    ));
    assert!(!compact.contains(">>((byte-4)*8)"));
    assert!(!compact.contains("letgeneration_shift=(byte-4)*8;"));
    assert!(!compact.contains("letgeneration_shift="));
    assert!(!compact.contains("generation>>generation_"));
    let epoch_lower_guard = compact
        .find("ifbyte>=8{}else{fe2o3_device::trap();}")
        .expect("epoch byte lower bound is authenticated before subtraction");
    let epoch_byte = compact
        .find("letepoch_byte=byte-8;")
        .expect("epoch byte offset is materialized after its lower bound");
    let epoch_guard = compact
        .find("ifepoch_byte<8{}else{fe2o3_device::trap();}")
        .expect("epoch byte is authenticated before selection");
    let epoch_word_load = compact
        .find("letepoch_word=memory::volatile_load(completion_epochs,sequence);")
        .expect("epoch word is loaded once before literal shift selection");
    let epoch_selection = compact
        .find("ifepoch_byte==0{epoch_wordasu8}elseifepoch_byte==1{(epoch_word>>8)asu8}elseifepoch_byte==2{(epoch_word>>16)asu8}elseifepoch_byte==3{(epoch_word>>24)asu8}elseifepoch_byte==4{(epoch_word>>32)asu8}elseifepoch_byte==5{(epoch_word>>40)asu8}elseifepoch_byte==6{(epoch_word>>48)asu8}else{(epoch_word>>56)asu8}")
        .expect("epoch bytes use only literal little-endian shifts");
    assert!(
        epoch_lower_guard < epoch_byte
            && epoch_byte < epoch_guard
            && epoch_guard < epoch_word_load
            && epoch_word_load < epoch_selection
    );
    assert_eq!(
        compact
            .matches("ifbyte>=8{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert_eq!(compact.matches("letepoch_byte=byte-8;").count(), 1);
    for marker in [
        "ifepoch_byte<8{}else{fe2o3_device::trap();}",
        "ifepoch_byte==0{epoch_wordasu8}",
        "elseifepoch_byte==1{(epoch_word>>8)asu8}",
        "elseifepoch_byte==2{(epoch_word>>16)asu8}",
        "elseifepoch_byte==3{(epoch_word>>24)asu8}",
        "elseifepoch_byte==4{(epoch_word>>32)asu8}",
        "elseifepoch_byte==5{(epoch_word>>40)asu8}",
        "elseifepoch_byte==6{(epoch_word>>48)asu8}",
        "else{(epoch_word>>56)asu8}",
    ] {
        assert_eq!(compact.matches(marker).count(), 1, "count for {marker}");
    }
    assert_eq!(
        compact
            .matches("letepoch_word=memory::volatile_load(completion_epochs,sequence);")
            .count(),
        1
    );
    assert!(compact.contains(
        "elseifbyte<16{ifbyte>=8{}else{fe2o3_device::trap();}letepoch_byte=byte-8;ifepoch_byte<8{}else{fe2o3_device::trap();}letepoch_word=memory::volatile_load(completion_epochs,sequence);ifepoch_byte==0{epoch_wordasu8}elseifepoch_byte==1{(epoch_word>>8)asu8}elseifepoch_byte==2{(epoch_word>>16)asu8}elseifepoch_byte==3{(epoch_word>>24)asu8}elseifepoch_byte==4{(epoch_word>>32)asu8}elseifepoch_byte==5{(epoch_word>>40)asu8}elseifepoch_byte==6{(epoch_word>>48)asu8}else{(epoch_word>>56)asu8}"
    ));
    assert!(!compact.contains(">>((byte-8)*8)"));
    assert!(!compact.contains("letepoch_shift="));
    assert!(!compact.contains("epoch_word>>epoch_shift"));
    let plan_lower_guard = compact
        .find("ifbyte>=16{}else{fe2o3_device::trap();}")
        .expect("plan byte lower bound is authenticated before subtraction");
    let plan_offset = compact
        .find("letplan_offset=byte-16;")
        .expect("plan byte offset is separated from its row base");
    let plan_guard = compact
        .find("ifplan_offset<32{}else{fe2o3_device::trap();}")
        .expect("plan byte offset is authenticated before use");
    let plan_load = compact
        .find("memory::volatile_load(plan_identities,plan_base+plan_offset)")
        .expect("plan read uses only the authenticated row offset");
    assert!(plan_lower_guard < plan_offset && plan_offset < plan_guard && plan_guard < plan_load);
    assert_eq!(
        compact
            .matches("ifbyte>=16{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(compact.contains(
        "elseifbyte<48{ifbyte>=16{}else{fe2o3_device::trap();}letplan_offset=byte-16;ifplan_offset<32{}else{fe2o3_device::trap();}memory::volatile_load(plan_identities,plan_base+plan_offset)"
    ));
    assert_eq!(compact.matches("letplan_offset=byte-16;").count(), 1);
    assert_eq!(
        compact
            .matches("ifplan_offset<32{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact.contains("plan_base+byte-16"));
    assert!(compact.contains("else{ifbyte>=52{}else{fe2o3_device::trap();}lettoken_byte=byte-52;"));
    assert!(compact.contains(
        "iftoken<accepted{iftoken<speculative_k_usize{}else{fe2o3_device::trap();}iftoken<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}ifsequence<sequences{}else{fe2o3_device::trap();}lettoken_draft_row=token*sequences;lettoken_draft_index=token_draft_row+sequence;memory::volatile_load(draft,token_draft_index)"
    ));
    let token_record = compact
        .find("iftoken<accepted{")
        .map(|start| &compact[start..])
        .expect("record token branch follows compact metadata packing");
    let token_draft_guard = token_record
        .find("iftoken<speculative_k_usize{}else{fe2o3_device::trap();}")
        .expect("record token index is authenticated against the draft extent");
    let token_cap_guard = token_record
        .find("iftoken<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
        .expect("record token index is bounded before row multiplication");
    let sequence_count_guard = token_record
        .find(
            "ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}",
        )
        .expect("record token row count is bounded before multiplication");
    let sequence_guard = token_record
        .find("ifsequence<sequences{}else{fe2o3_device::trap();}")
        .expect("record token row offset is reauthenticated before addition");
    let token_draft_row = token_record
        .find("lettoken_draft_row=token*sequences;")
        .expect("record token draft row is materialized before its index");
    let token_draft_index = token_record
        .find("lettoken_draft_index=token_draft_row+sequence;")
        .expect("record token draft index is materialized before its read");
    let token_draft_read = token_record
        .find("memory::volatile_load(draft,token_draft_index)")
        .expect("record token draft read uses the materialized index");
    assert!(
        token_draft_guard < token_cap_guard
            && token_cap_guard < sequence_count_guard
            && sequence_count_guard < sequence_guard
            && sequence_guard < token_draft_row
            && token_draft_row < token_draft_index
            && token_draft_index < token_draft_read
    );
    assert_eq!(
        compact
            .matches("lettoken_draft_row=token*sequences;")
            .count(),
        1
    );
    assert_eq!(
        compact
            .matches("lettoken_draft_index=token_draft_row+sequence;")
            .count(),
        1
    );
    assert_eq!(
        compact
            .matches("memory::volatile_load(draft,token_draft_index)")
            .count(),
        1
    );
    assert!(!compact.contains("memory::volatile_load(draft,token*sequences+sequence)"));
    assert!(!compact.contains("lettoken_draft_index=token*sequences+sequence"));
    assert!(!compact.contains("memory::volatile_load(draft,token_draft_row+sequence)"));
    assert!(!compact.contains("lettoken_draft_index=candidate*"));
    assert!(!compact.contains("letdraft_index=token*"));
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
