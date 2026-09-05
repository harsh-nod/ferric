use syn::Item;

const ROOT: &str = include_str!("../src/lib.rs");
const FAMILY_SOURCES: [(&str, &str); 7] = [
    ("gemm", include_str!("../src/gemm.rs")),
    ("logits", include_str!("../src/logits.rs")),
    ("paged_decode", include_str!("../src/paged_decode.rs")),
    ("prefill", include_str!("../src/prefill.rs")),
    ("rmsnorm", include_str!("../src/rmsnorm.rs")),
    ("rope_kv", include_str!("../src/rope_kv.rs")),
    ("swiglu", include_str!("../src/swiglu.rs")),
];

#[test]
fn aggregate_root_owns_the_seven_canonical_sources() {
    let root = syn::parse_file(ROOT).expect("aggregate root parses as ordinary Rust");
    let modules = root
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if module.content.is_none() => {
                assert!(
                    module.attrs.is_empty(),
                    "aggregate modules must use package-local default paths"
                );
                Some(module.ident.to_string())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        modules,
        vec![
            "gemm",
            "logits",
            "paged_decode",
            "prefill",
            "rmsnorm",
            "rope_kv",
            "swiglu"
        ]
    );
}

#[test]
fn shared_family_sources_expose_exactly_twelve_kernel_roots() {
    let mut kernels = Vec::new();
    for (family, source) in FAMILY_SOURCES {
        assert!(source.starts_with("#![forbid(unsafe_op_in_unsafe_fn)]\n"));
        assert!(!source.contains("#![no_std]"));
        let parsed = syn::parse_file(source).expect("family source parses as ordinary Rust");
        kernels.extend(parsed.items.into_iter().filter_map(|item| {
            match item {
                Item::Fn(function)
                    if function
                        .attrs
                        .iter()
                        .any(|attribute| attribute.path().is_ident("kernel")) =>
                {
                    Some((family, function.sig.ident.to_string()))
                }
                _ => None,
            }
        }));
    }
    assert_eq!(
        kernels,
        vec![
            (
                "gemm",
                "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1".to_owned()
            ),
            (
                "gemm",
                "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1".to_owned()
            ),
            (
                "gemm",
                "ferric_qwen3_token_embedding_bf16_copy_v1".to_owned()
            ),
            ("logits", "ferric_qwen3_lowest_id_argmax_bf16_v1".to_owned()),
            ("logits", "ferric_qwen3_compact_completion_v1".to_owned()),
            (
                "logits",
                "ferric_qwen3_speculative_token_assembly_v1".to_owned()
            ),
            (
                "paged_decode",
                "qwen3_paged_gqa_decode_bf16_f32_v1".to_owned()
            ),
            ("prefill", "qwen3_gqa_prefill_causal_bf16_f32_v1".to_owned()),
            ("rmsnorm", "qwen3_rmsnorm_v1".to_owned()),
            ("rope_kv", "qwen3_rope_v1".to_owned()),
            ("rope_kv", "qwen3_paged_kv_write_v1".to_owned()),
            ("swiglu", "qwen3_swiglu_bf16_f32_v1".to_owned()),
        ]
    );
}

#[test]
fn compact_completion_uses_one_header_exit_and_an_inert_mismatch_tail() {
    let source = FAMILY_SOURCES
        .iter()
        .find_map(|(family, source)| (*family == "logits").then_some(*source))
        .expect("logits source is in the aggregate family roster");
    let compact = source
        .split_once("pub fn ferric_qwen3_compact_completion_v1(")
        .and_then(|(_, tail)| {
            tail.split_once("pub fn ferric_qwen3_speculative_token_assembly_v1(")
                .map(|(body, _)| body)
        })
        .expect("compact completion source boundaries are present");
    let compact_no_whitespace = compact
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    for marker in [
        "let mut candidate = 0;",
        "let mut matching_prefix = true;",
        "let speculative_k_usize = speculative_k as usize;",
        "while candidate < speculative_k_usize",
        "if matching_prefix",
        "if candidate < speculative_k_usize {",
        "if candidate < QWEN3_LOGITS_MAX_SPECULATIVE_K_V1 {",
        "if candidate < active_tokens {",
        "let draft_row = candidate * sequences;",
        "let draft_index = draft_row + sequence;",
        "let target_index = choice_base + candidate;",
        "if draft_token == target_token",
        "if accepted < QWEN3_LOGITS_MAX_SPECULATIVE_K_V1 {",
        "matching_prefix = false;",
        "candidate += 1;",
        "if accepted >= active_tokens {",
        "if direct_offset >= active_tokens {",
        "let direct_offset = live - 1;",
        "choice_base + direct_offset",
        "if accepted < QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1 {",
        "if byte >= 4 {",
        "let generation_byte = byte - 4;",
        "if generation_byte < 4 {",
        "if generation_byte == 0 {",
        "else if generation_byte == 1 {",
        "else if generation_byte == 2 {",
        "(generation >> 24) as u8",
        "if byte >= 8 {",
        "let epoch_byte = byte - 8;",
        "if epoch_byte < 8 {",
        "let epoch_word = memory::volatile_load(completion_epochs, sequence);",
        "if epoch_byte == 0 {",
        "else if epoch_byte == 1 {",
        "else if epoch_byte == 2 {",
        "else if epoch_byte == 3 {",
        "else if epoch_byte == 4 {",
        "else if epoch_byte == 5 {",
        "else if epoch_byte == 6 {",
        "(epoch_word >> 56) as u8",
        "if byte >= 16 {",
        "let plan_offset = byte - 16;",
        "if plan_offset < 32 {",
        "if byte >= 52 {",
        "if token < speculative_k_usize {",
        "if token < QWEN3_LOGITS_MAX_SPECULATIVE_K_V1 {",
        "if sequences < QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1 {",
        "if sequence < sequences {",
        "let token_draft_row = token * sequences;",
        "let token_draft_index = token_draft_row + sequence;",
    ] {
        assert!(compact.contains(marker), "missing compact marker {marker}");
    }
    assert!(
        compact_no_whitespace
            .contains("matchspeculative_k{0=>{}_=>{whilecandidate<speculative_k_usize")
    );
    let optional_entry = compact_no_whitespace
        .split_once("letmutaccepted=0")
        .and_then(|(_, tail)| {
            tail.split_once("letcorrection_index=")
                .map(|(entry, _)| entry)
        })
        .expect("optional compact entry is bounded by accepted and correction");
    assert!(!optional_entry.contains("matchspeculative_k_usize"));
    assert!(!optional_entry.contains("ifdirect{"));
    assert!(!optional_entry.contains("if!direct{"));
    assert!(!compact.contains("while accepted < speculative_k"));
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
        compact_no_whitespace
            .matches("ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(compact_no_whitespace.contains(
        "ifmatching_prefix{ifcandidate<speculative_k_usize{}else{fe2o3_device::trap();}ifcandidate<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}ifcandidate<active_tokens{}else{fe2o3_device::trap();}ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}ifsequence<sequences{}else{fe2o3_device::trap();}letdraft_row=candidate*sequences;letdraft_index=draft_row+sequence;lettarget_index=choice_base+candidate;letdraft_token=memory::volatile_load(draft,draft_index);lettarget_token=memory::volatile_load(choices,target_index);"
    ));
    assert_eq!(
        compact_no_whitespace
            .matches("letdraft_row=candidate*sequences;")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("letdraft_index=draft_row+sequence;")
            .count(),
        1
    );
    assert!(!compact_no_whitespace.contains("letdraft_index=candidate*sequences+sequence"));
    assert_eq!(
        compact_no_whitespace
            .matches("ifcandidate<active_tokens{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact_no_whitespace.contains("ifmatching_prefix{letdraft_index="));
    let accepted_increment_guard = compact_no_whitespace
        .find("ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
        .expect("accepted count is bounded immediately before increment");
    let accepted_increment = compact_no_whitespace
        .find("accepted+=1;")
        .expect("matching target increments the accepted count");
    assert!(accepted_increment_guard < accepted_increment);
    assert_eq!(
        compact_no_whitespace
            .matches("ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(compact_no_whitespace.contains(
        "ifdraft_token==target_token{ifaccepted<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}accepted+=1;}else{matching_prefix=false;}"
    ));
    let (_, after_accepted_bound) = compact_no_whitespace
        .split_once("ifaccepted>=active_tokens{fe2o3_device::trap();}")
        .expect("accepted bound guard follows optional prefix matching");
    assert!(
        after_accepted_bound.starts_with(
            "ifdirect_offset>=active_tokens{fe2o3_device::trap();}letcorrection_index="
        ),
        "accepted bound guard must precede the direct-offset guard and correction indexing"
    );
    assert_eq!(
        compact_no_whitespace
            .matches("ifaccepted>=active_tokens")
            .count(),
        1
    );
    let emitted_guard = compact_no_whitespace
        .find("ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}")
        .expect("accepted token count is bounded before record packing");
    let record_loop = compact_no_whitespace
        .find("letmutcomponent=0;whilecomponent<2")
        .expect("record packing follows the emitted-token guard");
    let emitted_count = compact_no_whitespace
        .find("(accepted+1)asu8")
        .expect("record metadata encodes the guarded emitted-token count");
    assert!(emitted_guard < record_loop && record_loop < emitted_count);
    assert_eq!(
        compact_no_whitespace
            .matches("ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}",)
            .count(),
        1
    );
    assert!(compact_no_whitespace.contains(
        "ifaccepted<QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1{}else{fe2o3_device::trap();}letmutcomponent=0;whilecomponent<2"
    ));
    let zero_exit = compact_no_whitespace
        .find("iflive==0{")
        .expect("empty live rows exit before direct indexing");
    let direct_offset = compact_no_whitespace
        .find("letdirect_offset=live-1;")
        .expect("direct indexing subtracts only from authenticated nonzero live");
    let zero_return = compact_no_whitespace[zero_exit..]
        .find("return;}")
        .map(|offset| zero_exit + offset)
        .expect("empty live rows return before direct indexing");
    let direct_use = compact_no_whitespace
        .find("choice_base+direct_offset")
        .expect("direct correction uses the authenticated offset");
    let direct_bound = compact_no_whitespace
        .find("ifdirect_offset>=active_tokens{fe2o3_device::trap();}")
        .expect("direct correction offset is reauthenticated against its row width");
    assert!(zero_exit < zero_return);
    assert!(
        zero_return < direct_offset && direct_offset < direct_bound && direct_bound < direct_use
    );
    assert_eq!(
        compact_no_whitespace
            .matches("letdirect_offset=live-1;")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("ifdirect_offset>=active_tokens{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact_no_whitespace.contains("choice_base+live-1"));
    let generation_lower_guard = compact_no_whitespace
        .find("ifbyte>=4{}else{fe2o3_device::trap();}")
        .expect("generation byte lower bound is authenticated before subtraction");
    let generation_guard = compact_no_whitespace
        .find("ifgeneration_byte<4{}else{fe2o3_device::trap();}")
        .expect("generation byte is authenticated before literal shift selection");
    let generation_load = compact_no_whitespace
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
        assert_eq!(
            compact_no_whitespace.matches(marker).count(),
            1,
            "count for {marker}"
        );
    }
    assert!(compact_no_whitespace.contains(
        "elseifbyte<8{ifbyte>=4{}else{fe2o3_device::trap();}letgeneration_byte=byte-4;ifgeneration_byte<4{}else{fe2o3_device::trap();}ifgeneration_byte==0{generationasu8}elseifgeneration_byte==1{(generation>>8)asu8}elseifgeneration_byte==2{(generation>>16)asu8}else{(generation>>24)asu8}"
    ));
    assert!(!compact_no_whitespace.contains(">>((byte-4)*8)"));
    assert!(!compact_no_whitespace.contains("letgeneration_shift=(byte-4)*8;"));
    assert!(!compact_no_whitespace.contains("letgeneration_shift="));
    assert!(!compact_no_whitespace.contains("generation>>generation_"));
    let epoch_lower_guard = compact_no_whitespace
        .find("ifbyte>=8{}else{fe2o3_device::trap();}")
        .expect("epoch byte lower bound is authenticated before subtraction");
    let epoch_byte = compact_no_whitespace
        .find("letepoch_byte=byte-8;")
        .expect("epoch byte offset is materialized after its lower bound");
    let epoch_guard = compact_no_whitespace
        .find("ifepoch_byte<8{}else{fe2o3_device::trap();}")
        .expect("epoch byte is authenticated before selection");
    let epoch_word_load = compact_no_whitespace
        .find("letepoch_word=memory::volatile_load(completion_epochs,sequence);")
        .expect("epoch word is loaded once before literal shift selection");
    let epoch_selection = compact_no_whitespace
        .find("ifepoch_byte==0{epoch_wordasu8}elseifepoch_byte==1{(epoch_word>>8)asu8}elseifepoch_byte==2{(epoch_word>>16)asu8}elseifepoch_byte==3{(epoch_word>>24)asu8}elseifepoch_byte==4{(epoch_word>>32)asu8}elseifepoch_byte==5{(epoch_word>>40)asu8}elseifepoch_byte==6{(epoch_word>>48)asu8}else{(epoch_word>>56)asu8}")
        .expect("epoch bytes use only literal little-endian shifts");
    assert!(
        epoch_lower_guard < epoch_byte
            && epoch_byte < epoch_guard
            && epoch_guard < epoch_word_load
            && epoch_word_load < epoch_selection
    );
    assert_eq!(
        compact_no_whitespace
            .matches("ifbyte>=8{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("letepoch_byte=byte-8;")
            .count(),
        1
    );
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
        assert_eq!(
            compact_no_whitespace.matches(marker).count(),
            1,
            "count for {marker}"
        );
    }
    assert_eq!(
        compact_no_whitespace
            .matches("letepoch_word=memory::volatile_load(completion_epochs,sequence);")
            .count(),
        1
    );
    assert!(compact_no_whitespace.contains(
        "elseifbyte<16{ifbyte>=8{}else{fe2o3_device::trap();}letepoch_byte=byte-8;ifepoch_byte<8{}else{fe2o3_device::trap();}letepoch_word=memory::volatile_load(completion_epochs,sequence);ifepoch_byte==0{epoch_wordasu8}elseifepoch_byte==1{(epoch_word>>8)asu8}elseifepoch_byte==2{(epoch_word>>16)asu8}elseifepoch_byte==3{(epoch_word>>24)asu8}elseifepoch_byte==4{(epoch_word>>32)asu8}elseifepoch_byte==5{(epoch_word>>40)asu8}elseifepoch_byte==6{(epoch_word>>48)asu8}else{(epoch_word>>56)asu8}"
    ));
    assert!(!compact_no_whitespace.contains(">>((byte-8)*8)"));
    assert!(!compact_no_whitespace.contains("letepoch_shift="));
    assert!(!compact_no_whitespace.contains("epoch_word>>epoch_shift"));
    let plan_lower_guard = compact_no_whitespace
        .find("ifbyte>=16{}else{fe2o3_device::trap();}")
        .expect("plan byte lower bound is authenticated before subtraction");
    let plan_offset = compact_no_whitespace
        .find("letplan_offset=byte-16;")
        .expect("plan byte offset is separated from its row base");
    let plan_guard = compact_no_whitespace
        .find("ifplan_offset<32{}else{fe2o3_device::trap();}")
        .expect("plan byte offset is authenticated before use");
    let plan_load = compact_no_whitespace
        .find("memory::volatile_load(plan_identities,plan_base+plan_offset)")
        .expect("plan read uses only the authenticated row offset");
    assert!(plan_lower_guard < plan_offset && plan_offset < plan_guard && plan_guard < plan_load);
    assert_eq!(
        compact_no_whitespace
            .matches("ifbyte>=16{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(compact_no_whitespace.contains(
        "elseifbyte<48{ifbyte>=16{}else{fe2o3_device::trap();}letplan_offset=byte-16;ifplan_offset<32{}else{fe2o3_device::trap();}memory::volatile_load(plan_identities,plan_base+plan_offset)"
    ));
    assert_eq!(
        compact_no_whitespace
            .matches("letplan_offset=byte-16;")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("ifplan_offset<32{}else{fe2o3_device::trap();}")
            .count(),
        1
    );
    assert!(!compact_no_whitespace.contains("plan_base+byte-16"));
    assert!(
        compact_no_whitespace
            .contains("else{ifbyte>=52{}else{fe2o3_device::trap();}lettoken_byte=byte-52;")
    );
    assert!(compact_no_whitespace.contains(
        "iftoken<accepted{iftoken<speculative_k_usize{}else{fe2o3_device::trap();}iftoken<QWEN3_LOGITS_MAX_SPECULATIVE_K_V1{}else{fe2o3_device::trap();}ifsequences<QWEN3_LOGITS_COMPACT_GRID_BOUND_EXCLUSIVE_V1{}else{fe2o3_device::trap();}ifsequence<sequences{}else{fe2o3_device::trap();}lettoken_draft_row=token*sequences;lettoken_draft_index=token_draft_row+sequence;memory::volatile_load(draft,token_draft_index)"
    ));
    let token_record = compact_no_whitespace
        .find("iftoken<accepted{")
        .map(|start| &compact_no_whitespace[start..])
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
        compact_no_whitespace
            .matches("lettoken_draft_row=token*sequences;")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("lettoken_draft_index=token_draft_row+sequence;")
            .count(),
        1
    );
    assert_eq!(
        compact_no_whitespace
            .matches("memory::volatile_load(draft,token_draft_index)")
            .count(),
        1
    );
    assert!(
        !compact_no_whitespace.contains("memory::volatile_load(draft,token*sequences+sequence)")
    );
    assert!(!compact_no_whitespace.contains("lettoken_draft_index=token*sequences+sequence"));
    assert!(
        !compact_no_whitespace.contains("memory::volatile_load(draft,token_draft_row+sequence)")
    );
    assert!(!compact_no_whitespace.contains("lettoken_draft_index=candidate*"));
    assert!(!compact_no_whitespace.contains("letdraft_index=token*"));
}

#[test]
fn aggregate_gemm_roots_fail_closed_before_column_arithmetic_and_b_reads() {
    let source = FAMILY_SOURCES
        .iter()
        .find_map(|(family, source)| (*family == "gemm").then_some(*source))
        .expect("gemm source is in the aggregate family roster");
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    assert_eq!(
        compact
            .matches("iftile_column<tiles_per_row{}else{fe2o3_device::trap();}")
            .count(),
        2
    );
    assert_eq!(
        compact
            .matches("ifcolumn<n{}else{fe2o3_device::trap();}")
            .count(),
        2
    );
}

#[test]
fn aggregate_paged_decode_authenticates_every_coordinate_divisor() {
    let source = FAMILY_SOURCES
        .iter()
        .find_map(|(family, source)| (*family == "paged_decode").then_some(*source))
        .expect("paged-decode source is in the aggregate family roster");
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    for guard in [
        "ifquery_heads==0{fe2o3_device::trap();}",
        "ifactive_tokens==0{fe2o3_device::trap();}",
        "ifgqa_group_size==0{fe2o3_device::trap();}",
    ] {
        assert_eq!(compact.matches(guard).count(), 1, "guard count for {guard}");
    }
}
