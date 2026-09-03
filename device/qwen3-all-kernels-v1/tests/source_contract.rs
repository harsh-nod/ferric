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
        "let draft_index = candidate * sequences + sequence;",
        "let target_index = choice_base + candidate;",
        "if draft_token == target_token",
        "matching_prefix = false;",
        "candidate += 1;",
        "if accepted >= active_tokens {",
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
    assert!(!optional_entry.contains("ifdirect"));
    assert!(!optional_entry.contains("if!direct"));
    assert!(!compact.contains("while accepted < speculative_k"));
    assert!(!compact.contains("break;"));
    let (_, after_accepted_bound) = compact_no_whitespace
        .split_once("ifaccepted>=active_tokens{fe2o3_device::trap();}")
        .expect("accepted bound guard follows optional prefix matching");
    assert!(
        after_accepted_bound.starts_with("letcorrection_index="),
        "accepted bound guard must immediately precede correction indexing"
    );
    assert_eq!(
        compact_no_whitespace
            .matches("ifaccepted>=active_tokens")
            .count(),
        1
    );
}
