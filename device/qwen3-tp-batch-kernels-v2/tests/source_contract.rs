use ferric_qwen3_tp_batch_kernels_device_v2::{compiler_expectation_roster_v2, contract};
use syn::{FnArg, GenericArgument, Item, Pat, PathArguments, Type};

const SOURCES: [&str; 6] = [
    include_str!("../src/embedding.rs"),
    include_str!("../src/projection.rs"),
    include_str!("../src/activation.rs"),
    include_str!("../src/rope_kv.rs"),
    include_str!("../src/attention.rs"),
    include_str!("../src/logits.rs"),
];

fn element(ty: &Type) -> String {
    match ty {
        Type::Reference(reference) => {
            assert!(reference.mutability.is_none());
            let Type::Slice(slice) = reference.elem.as_ref() else {
                panic!("not a slice")
            };
            format!("ro:{}", element(&slice.elem))
        }
        Type::Path(path) => {
            let segment = path.path.segments.last().unwrap();
            if segment.ident == "WriteOnlyDisjointSlice" {
                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    panic!("output arguments")
                };
                let GenericArgument::Type(ty) = args.args.first().unwrap() else {
                    panic!("output element")
                };
                format!("wo:{}", element(ty))
            } else {
                assert!(matches!(segment.arguments, PathArguments::None));
                segment.ident.to_string()
            }
        }
        _ => panic!("unrecognized argument type"),
    }
}

fn roots() -> Vec<syn::ItemFn> {
    SOURCES
        .into_iter()
        .flat_map(|source| syn::parse_file(source).unwrap().items)
        .filter_map(|item| match item {
            Item::Fn(function)
                if function
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("kernel")) =>
            {
                Some(function)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn exact_eight_new_signatures_and_element_units() {
    let expected = [
        (
            contract::NEW_ROOTS[0],
            "tokens:ro:u32,weight:ro:u16,output:wo:u16,rows:u32",
            contract::EMBEDDING_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[1],
            "a:ro:u16,weights:ro:u16,output:wo:u16,rows:u32,n:u32,k:u32,world_size:u32,projection:u32",
            contract::GEMM_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[2],
            "a:ro:u16,weights:ro:u16,output:wo:f32,rows:u32,n:u32,k:u32,world_size:u32,projection:u32",
            contract::GEMM_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[3],
            "gate:ro:u16,up:ro:u16,output:wo:u16,rows:u32,world_size:u32",
            contract::SWIGLU_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[4],
            "query:ro:u16,key:ro:u16,cos:ro:f32,sin:ro:f32,positions:ro:u32,rotated_query:wo:u16,rotated_key:wo:u16,rows:u32,world_size:u32",
            contract::ROPE_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[5],
            "key:ro:u16,value:ro:u16,positions:ro:u32,page_table:ro:u32,key_cache:wo:u16,value_cache:wo:u16,rows:u32,world_size:u32,max_pages_per_sequence:u32,physical_pages:u32",
            contract::APPEND_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[6],
            "query:ro:u16,key_cache:ro:u16,value_cache:ro:u16,positions:ro:u32,page_table:ro:u32,output:wo:u16,rows:u32,world_size:u32,max_pages_per_sequence:u32,physical_pages:u32,max_context_tokens:u32",
            contract::ATTENTION_EXPLICIT_BYTES,
        ),
        (
            contract::NEW_ROOTS[7],
            "logits:ro:u16,choices:wo:u32,rows:u32",
            contract::ARGMAX_EXPLICIT_BYTES,
        ),
    ];
    let functions = roots();
    assert_eq!(functions.len(), expected.len());
    for (name, signature, bytes) in expected {
        let function = functions
            .iter()
            .find(|function| function.sig.ident == name)
            .unwrap();
        assert!(
            !function
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("cfg"))
        );
        let mut actual_bytes = 0;
        let actual = function
            .sig
            .inputs
            .iter()
            .map(|argument| {
                let FnArg::Typed(argument) = argument else {
                    panic!("receiver")
                };
                let Pat::Ident(name) = argument.pat.as_ref() else {
                    panic!("argument pattern")
                };
                let ty = element(&argument.ty);
                actual_bytes += if ty.contains(':') { 16 } else { 4 };
                format!("{}:{ty}", name.ident)
            })
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(actual, signature);
        assert_eq!(actual_bytes, bytes);
    }
}

#[test]
fn all_nine_compiler_markers_are_current_nonzero_and_unique() {
    let roster = compiler_expectation_roster_v2();
    assert_eq!(roster.len(), 9);
    assert!(
        roster
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id())
    );
    let mut actual = roster
        .iter()
        .map(|entry| {
            assert_ne!(entry.kernel_binding_id(), [0; 32]);
            assert_ne!(entry.generated_host_contract_identity(), [0; 32]);
            assert!(!entry.logical_name().is_empty());
            println!(
                "logical={} entry={} descriptor={}.kd",
                entry.logical_name(),
                entry.export_name(),
                entry.export_name()
            );
            entry.export_name()
        })
        .collect::<Vec<_>>();
    actual.sort();
    let mut expected = contract::NEW_ROOTS.to_vec();
    expected.push("qwen3_rmsnorm_v1");
    expected.sort();
    assert_eq!(actual, expected);
}

fn normalized(source: &str) -> String {
    syn::parse_str::<syn::Macro>(&format!("tokens!{{{source}}}"))
        .unwrap()
        .tokens
        .to_string()
}

fn macro_body(source: &str, name: &str) -> String {
    let start = source.find(&format!("macro_rules! {name}")).unwrap();
    let definition = &source[start..];
    let arrow = definition.find("=>").unwrap();
    let opening = arrow + definition[arrow..].find('{').unwrap();
    let mut depth = 0_u32;
    let mut closing = None;
    for (offset, value) in definition[opening..].char_indices() {
        if value == '{' {
            depth += 1;
        }
        if value == '}' {
            depth -= 1;
            if depth == 0 {
                closing = Some(opening + offset);
                break;
            }
        }
    }
    let body = definition[opening + 1..closing.unwrap()].trim();
    body.strip_prefix('{')
        .and_then(|body| body.strip_suffix('}'))
        .unwrap_or(body)
        .trim()
        .to_owned()
}

fn instantiate(source: &str, name: &str, replacements: &[(&str, &str)]) -> String {
    let mut body = macro_body(source, name);
    for (parameter, value) in replacements {
        body = body.replace(parameter, value);
    }
    // The sole nested helper has five identifier arguments, checked below.
    while let Some(start) = body.find("batch_paged_slot_v2!(") {
        let opening = start + "batch_paged_slot_v2!(".len();
        let closing = opening + body[opening..].find(')').unwrap();
        let arguments = body[opening..closing]
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        assert_eq!(arguments.len(), 5);
        for argument in &arguments {
            syn::parse_str::<syn::Ident>(argument).unwrap();
        }
        let nested = instantiate(
            source,
            "batch_paged_slot_v2",
            &[
                ("$positions", arguments[0]),
                ("$tables", arguments[1]),
                ("$row", arguments[2]),
                ("$stride", arguments[3]),
                ("$pages", arguments[4]),
            ],
        );
        body.replace_range(start..=closing, &format!("{{ {nested} }}"));
    }
    body
}

fn check_regions(source: &str, name: &str, replacements: &[(&str, &str)], count: usize) {
    syn::parse_file(source).unwrap();
    let expected = normalized(&instantiate(source, name, replacements));
    let mut actual_count = 0;
    for region in source.split(&format!("// BEGIN {name}")).skip(1) {
        let actual = region.split_once(&format!("// END {name}")).unwrap().0;
        assert_eq!(
            normalized(actual),
            expected,
            "device/test body drift: {name}"
        );
        actual_count += 1;
    }
    assert_eq!(actual_count, count, "missing actual-body regions: {name}");
}

#[test]
fn explicit_projection_bodies_match_the_host_tested_four_row_accumulation() {
    check_regions(
        include_str!("../src/projection.rs"),
        "batch_four_dots_v2",
        &[
            ("$row_base", "row_base"),
            ("$weights", "weights"),
            ("$column", "column"),
            ("$rows", "rows"),
            ("$a", "a"),
            ("$k", "k"),
        ],
        2,
    );
}

#[test]
fn explicit_paged_attention_matches_the_host_tested_causal_body() {
    check_regions(
        include_str!("../src/attention.rs"),
        "batch_paged_attention_pair_v2",
        &[
            ("$query_base", "query_base"),
            ("$query", "query"),
            ("$keys", "key_cache"),
            ("$values", "value_cache"),
            ("$tables", "page_table"),
            ("$row", "row"),
            ("$kv_head", "kv_head"),
            ("$columns", "columns"),
            ("$lane", "lane"),
            ("$position", "position"),
            ("$context", "max_context_tokens"),
            ("$stride", "max_pages_per_sequence"),
            ("$pages", "physical_pages"),
            ("$math", "math"),
        ],
        1,
    );
}

#[test]
fn append_validates_all_slots_before_entering_the_write_loop() {
    let source = include_str!("../src/rope_kv.rs");
    check_regions(
        source,
        "batch_distinct_slots_v2",
        &[
            ("$positions", "positions"),
            ("$tables", "page_table"),
            ("$rows", "rows"),
            ("$stride", "max_pages_per_sequence"),
            ("$pages", "physical_pages"),
        ],
        1,
    );
    let validation_end = source.find("// END batch_distinct_slots_v2").unwrap();
    assert!(validation_end < source.find(".write_exclusive").unwrap());
}

#[test]
fn explicit_argmax_matches_the_host_tested_lowest_id_scan() {
    check_regions(
        include_str!("../src/logits.rs"),
        "batch_argmax_v2",
        &[("$row_base", "row_base"), ("$logits", "logits")],
        1,
    );
}

#[test]
fn actual_numerical_and_causal_source_mutations_are_rejected() {
    let source = include_str!("../src/projection.rs");
    let split = source.find("// BEGIN batch_four_dots_v2").unwrap();
    let changed = source[..split].to_owned()
        + &source[split..].replacen("sum_0 += product", "sum_0 -= product", 1);
    assert_ne!(source, changed);
    assert!(
        std::panic::catch_unwind(|| check_regions(
            &changed,
            "batch_four_dots_v2",
            &[
                ("$row_base", "row_base"),
                ("$weights", "weights"),
                ("$column", "column"),
                ("$rows", "rows"),
                ("$a", "a"),
                ("$k", "k"),
            ],
            2
        ))
        .is_err()
    );
    let source = include_str!("../src/attention.rs");
    let split = source
        .find("// BEGIN batch_paged_attention_pair_v2")
        .unwrap();
    let changed = source[..split].to_owned()
        + &source[split..].replacen("token <= position", "token < position", 1);
    assert_ne!(source, changed);
    assert!(
        std::panic::catch_unwind(|| check_regions(
            &changed,
            "batch_paged_attention_pair_v2",
            &[
                ("$query_base", "query_base"),
                ("$query", "query"),
                ("$keys", "key_cache"),
                ("$values", "value_cache"),
                ("$tables", "page_table"),
                ("$row", "row"),
                ("$kv_head", "kv_head"),
                ("$columns", "columns"),
                ("$lane", "lane"),
                ("$position", "position"),
                ("$context", "max_context_tokens"),
                ("$stride", "max_pages_per_sequence"),
                ("$pages", "physical_pages"),
                ("$math", "math"),
            ],
            1
        ))
        .is_err()
    );
}

#[test]
fn host_geometry_limits_are_exact() {
    assert_eq!(
        (
            contract::MAX_ROWS,
            contract::MAX_REQUESTS,
            contract::PAGE_TOKENS,
            contract::MAX_PAGES,
            contract::MAX_CONTEXT_TOKENS
        ),
        (16, 32, 16, 512, 8192)
    );
    assert!(!contract::rows_are_supported(0));
    assert!(!contract::rows_are_supported(17));
    for rows in 1..=16 {
        assert!(contract::rows_are_supported(rows));
    }
    for world in [1, 2, 8] {
        assert_eq!(contract::local_query_heads(world), Some(32 / world));
        assert_eq!(contract::local_kv_heads(world), Some(8 / world));
        assert_eq!(contract::local_intermediate(world), Some(12288 / world));
    }
    for world in [0, 3, 4, 16] {
        assert!(!contract::world_is_supported(world));
        assert_eq!(contract::local_query_heads(world), None);
        assert_eq!(contract::local_kv_heads(world), None);
        assert_eq!(contract::local_intermediate(world), None);
    }
}
