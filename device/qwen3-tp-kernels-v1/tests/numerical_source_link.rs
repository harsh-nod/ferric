fn tokens(source: &str) -> String {
    syn::parse_str::<syn::Macro>(&format!("tokens!{{{source}}}"))
        .unwrap()
        .tokens
        .to_string()
}

fn checked_regions(
    source: &str,
    name: &str,
    replacements: &[(&str, &str)],
    tail: (&str, &str),
    count: usize,
) {
    syn::parse_file(source).unwrap();
    let macro_start = source.find(&format!("macro_rules! {name}")).unwrap();
    let body = source[macro_start..]
        .split_once("=> {{")
        .unwrap()
        .1
        .split_once("\n    }};")
        .unwrap()
        .0;
    let mut expected = body.to_owned();
    for (parameter, value) in replacements {
        expected = expected.replace(parameter, value);
    }
    let expected = expected.trim().strip_suffix(tail.0).unwrap().to_owned() + tail.1;
    let mut found = 0;
    for rest in source.split(&format!("// BEGIN {name}")).skip(1) {
        let actual = rest.split_once(&format!("// END {name}")).unwrap().0;
        assert_eq!(
            tokens(actual),
            tokens(&expected),
            "device region differs from the host-tested macro"
        );
        found += 1;
    }
    assert_eq!(found, count);
}

#[test]
fn both_projection_stores_use_the_host_tested_accumulation_body() {
    checked_regions(
        include_str!("../src/projection.rs"),
        "tp_gemv_sum_v1",
        &[
            ("$weights", "weights"),
            ("$column", "column"),
            ("$a", "a"),
            ("$k", "k"),
        ],
        ("sum", ""),
        2,
    );
}

#[test]
fn query_and_key_rotation_use_the_host_tested_pair_body() {
    checked_regions(
        include_str!("../src/rope_kv.rs"),
        "tp_rope_pair_v1",
        &[
            ("$first", "first"),
            ("$second", "second"),
            ("$cos", "cosine"),
            ("$sin", "sine"),
        ],
        (
            "(first_out.to_bits(), second_out.to_bits())",
            "let first_out = first_out.to_bits(); let second_out = second_out.to_bits();",
        ),
        2,
    );
}

#[test]
fn local_attention_uses_the_host_tested_online_softmax_body() {
    checked_regions(
        include_str!("../src/attention.rs"),
        "tp_attention_pair_v1",
        &[
            ("$query_head", "query_head"),
            ("$kv_head", "kv_head"),
            ("$keys", "key_cache"),
            ("$values", "value_cache"),
            ("$count", "count"),
            ("$columns", "columns"),
            ("$q", "query"),
            ("$lane", "lane"),
            ("$math", "math"),
        ],
        (
            "(narrowed_0.to_bits(), narrowed_1.to_bits())",
            "let first = narrowed_0.to_bits(); let second = narrowed_1.to_bits();",
        ),
        1,
    );
}
