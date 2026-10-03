#[path = "support/attention_expansion.rs"]
mod attention_expansion;
use attention_expansion::expand as actual;

const BEFORE: &str = include_str!("fixtures/attention_before_composition.rs");
const WRAPPER: &str = include_str!("../src/attention.rs");
const ONLINE: &str = include_str!("../src/attention_online.rs");

fn replace_once(source: &mut String, before: &str, after: &str) {
    assert_eq!(source.matches(before).count(), 1, "{before}");
    *source = source.replacen(before, after, 1);
}

fn expected() -> syn::File {
    let mut before = BEFORE.to_owned();
    // Exhaustive permitted extraction changes. Everything else, including the
    // entire admission/ownership wrapper, must remain AST-identical.
    replace_once(
        &mut before,
        "let (first, second) = {",
        "let (first, second, finite) = {",
    );
    replace_once(
        &mut before,
        "finite &= product_0.is_finite() & product_1.is_finite() & partial.is_finite();",
        "let product_finite = product_0.is_finite() & product_1.is_finite() & partial.is_finite();",
    );
    replace_once(
        &mut before,
        "let table_index =",
        "let (score, value_0, value_1, product_finite) = { let table_index =",
    );
    replace_once(
        &mut before,
        "finite &= score.is_finite() & value_0.is_finite() & value_1.is_finite();",
        "(score, value_0, value_1, product_finite) }; finite &= product_finite; finite &= score.is_finite() & value_0.is_finite() & value_1.is_finite();",
    );
    for name in ["0", "1"] {
        replace_once(
            &mut before,
            &format!("let narrowed_{name} = Bf16::from_f32(output_{name});"),
            &format!("let narrowed_{name} = fe2o3_device::Bf16::from_f32(output_{name});"),
        );
    }
    let start = before.find("        // Numeric failures").unwrap();
    let end = before.find("    let Some(stripe)").unwrap();
    before.replace_range(start..end,
        "finite &= output_0.is_finite() & output_1.is_finite() & narrowed_0.is_finite() & narrowed_1.is_finite();
        (narrowed_0.to_bits(), narrowed_1.to_bits(), finite)
    }; if !finite { fe2o3_device::trap(); }
");
    syn::parse_file(&before).unwrap()
}

#[test]
fn complete_expanded_source_preserves_the_frozen_production_wrapper() {
    assert_eq!(actual(WRAPPER, ONLINE), expected());
}

#[test]
fn macro_routing_attributes_and_opaque_statements_are_rejected() {
    for (in_wrapper, before, after) in [
        (
            true,
            "qwen_attention_online_pair_v1!(",
            "#[cfg(not(target_arch = \"amdgpu\"))] qwen_attention_online_pair_v1!(",
        ),
        (
            false,
            "macro_rules! qwen_attention_online_pair_v1",
            "#[cfg(not(target_arch = \"amdgpu\"))] macro_rules! qwen_attention_online_pair_v1",
        ),
        (false, "$($prelude)* { $($body)* };", "$($prelude)* { 0 };"),
        (
            true,
            "let math = Math::current();",
            "let math = Math::current(); opaque!();",
        ),
    ] {
        let mut wrapper = WRAPPER.to_owned();
        let mut online = ONLINE.to_owned();
        replace_once(
            if in_wrapper {
                &mut wrapper
            } else {
                &mut online
            },
            before,
            after,
        );
        assert!(std::panic::catch_unwind(|| actual(&wrapper, &online)).is_err());
    }
}

#[test]
fn arithmetic_bounds_load_order_and_ownership_mutations_are_detected() {
    let expected = expected();
    for (in_wrapper, before, after) in [
        (false, "$token <= $position", "$token < $position"),
        (false, "$token += 1", "$token += 2"),
        (false, "score > maximum", "score < maximum"),
        (false, "maximum - next_maximum", "next_maximum - maximum"),
        (false, "score - next_maximum", "next_maximum - score"),
        (
            false,
            "denominator * previous_weight + current_weight",
            "denominator + current_weight",
        ),
        (
            false,
            "value_0 * current_weight",
            "value_1 * current_weight",
        ),
        (
            false,
            "finite &= product_finite",
            "finite |= product_finite",
        ),
        (
            false,
            "Bf16::from_f32(output_0)",
            "Bf16::from_f32(output_1)",
        ),
        (true, "0x3db5_04f3", "0x3db5_04f4"),
        (true, "query_head / 4", "query_head / 2"),
        (true, "head_row, lane + 64", "head_row, lane + 63"),
        (true, "product_0 + product_1", "product_0 - product_1"),
        (
            true,
            "physical_page < physical_pages",
            "physical_page <= physical_pages",
        ),
        (
            true,
            "value_view.load_or(cache_row, cache_column + 64, 0)",
            "value_view.load_or(cache_row, cache_column + 63, 0)",
        ),
        (true, "if !dot.is_finite()", "if dot.is_finite()"),
        (
            true,
            "&stripe, 1, head_rows, 128, 128, second",
            "&stripe, 0, head_rows, 128, 128, second",
        ),
    ] {
        let mut wrapper = WRAPPER.to_owned();
        let mut online = ONLINE.to_owned();
        replace_once(
            if in_wrapper {
                &mut wrapper
            } else {
                &mut online
            },
            before,
            after,
        );
        assert_ne!(
            actual(&wrapper, &online),
            expected,
            "undetected mutation: {before}"
        );
    }
}
