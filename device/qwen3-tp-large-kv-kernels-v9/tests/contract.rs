use ferric_qwen3_tp_large_kv_kernels_device_v9::{
    ROOTS_V9, compiler_expectation_roster_v9, contract,
};
use syn::{FnArg, Item, Type};

const APPEND: &str = include_str!("../src/append.rs");
const ATTENTION: &str = include_str!("../src/attention.rs");
const OLD_APPEND: &str = include_str!("../../qwen3-tp-batch32-kernels-v5/src/rope_kv.rs");
const OLD_ATTENTION: &str =
    include_str!("../../qwen3-tp-batch32-kernels-v5/src/baseline_attention.rs");

fn normalized(source: &str) -> String {
    source.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[test]
fn closed_two_root_roster_preserves_explicit_abis_and_launch_geometry() {
    let roster = compiler_expectation_roster_v9();
    assert_eq!(roster.len(), 2);
    assert!(
        roster
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id())
    );
    let mut names: Vec<_> = roster
        .iter()
        .map(|entry| {
            assert_ne!(entry.generated_host_contract_identity(), [0; 32]);
            entry.export_name()
        })
        .collect();
    names.sort();
    let mut expected = ROOTS_V9;
    expected.sort();
    assert_eq!(names, expected);
    for (source, name, expected_bytes) in [
        (APPEND, ROOTS_V9[0], contract::APPEND_EXPLICIT_BYTES),
        (ATTENTION, ROOTS_V9[1], contract::ATTENTION_EXPLICIT_BYTES),
    ] {
        let parsed = syn::parse_file(source).unwrap();
        let functions: Vec<_> = parsed
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(f) if f.attrs.iter().any(|a| a.path().is_ident("kernel")) => Some(f),
                _ => None,
            })
            .collect();
        assert_eq!(functions.len(), 1);
        let function = functions[0];
        assert_eq!(function.sig.ident, name);
        assert!(function.sig.unsafety.is_none());
        let bytes: u32 = function
            .sig
            .inputs
            .iter()
            .map(|argument| {
                let FnArg::Typed(argument) = argument else {
                    panic!("receiver")
                };
                match argument.ty.as_ref() {
                    Type::Reference(reference) => {
                        assert!(reference.mutability.is_none());
                        assert!(matches!(reference.elem.as_ref(), Type::Slice(_)));
                        16
                    }
                    Type::Path(path)
                        if path.path.segments.last().unwrap().ident == "WriteOnlyDisjointSlice" =>
                    {
                        16
                    }
                    Type::Path(path) if path.path.is_ident("u32") => 4,
                    _ => panic!("unexpected ABI"),
                }
            })
            .sum();
        assert_eq!(bytes, expected_bytes);
        assert!(source.contains("required = [64, 1, 1], max = [64, 1, 1]"));
    }
    assert!(APPEND.contains("max_grid = [1, 1, 1]"));
    assert!(APPEND.contains("thread::launch_extent_1d() != 64"));
    assert!(ATTENTION.contains("max_grid = [1024, 1, 1]"));
    assert!(ATTENTION.contains("thread::launch_extent_1d() != head_rows * 64"));
}

#[test]
fn exact_v5_root_and_helper_copies_change_only_declared_scope() {
    let macros = OLD_APPEND
        .split("#[cfg(test)]\nmacro_rules! batch_rope_pair_v5")
        .next()
        .unwrap();
    let macros = &macros[macros.find("#[cfg(test)]").unwrap()..];
    let body = &OLD_APPEND[OLD_APPEND.find("/// One grid leader").unwrap()..];
    let old = format!(
        "use fe2o3_device::{{GridExclusive, WriteOnlyDisjointSlice, kernel, memory, thread}};\n\n{macros}{body}"
    );
    for (old, actual) in [(old.as_str(), APPEND), (OLD_ATTENTION, ATTENTION)] {
        let expected = old
            .replace("_v5", "_v9")
            .replace("ferric_qwen3_tp_batch32_paged_kv_append_v9", ROOTS_V9[0])
            .replace("ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v9", ROOTS_V9[1])
            .replace(
                "!(world_size == 1 || world_size == 2 || world_size == 8)",
                "world_size != 1",
            )
            .replace("physical_pages > 512", "physical_pages > 16384")
            .replace("physical_pages < 513", "physical_pages < 16385")
            .replace("physical_page < 512", "physical_page < 16384")
            .replace("slot < 8192", "slot < 262144")
            .replace("8388481", "268435329");
        let actual = actual.split("#[cfg(test)]\nmod tests;").next().unwrap();
        assert_eq!(normalized(&expected), normalized(actual));
    }
    for (source, marker, count) in [
        (APPEND, "batch_paged_slot_v9", 3),
        (APPEND, "batch_distinct_slots_v9", 1),
        (ATTENTION, "batch_paged_attention_pair_v9", 1),
    ] {
        assert_eq!(source.matches(&format!("// BEGIN {marker}")).count(), count);
        assert_eq!(source.matches(&format!("// END {marker}")).count(), count);
    }
}

#[test]
fn physical_and_logical_limits_are_independent_and_bounded() {
    for rows in [0, 1, 16, 17, 32, 33, u32::MAX] {
        for world in [0, 1, 2, 8, u32::MAX] {
            for stride in [0, 1, 512, 513, u32::MAX] {
                for pages in [0, 1, 511, 512, 513, 8192, 16384, 16385, u32::MAX] {
                    assert_eq!(
                        contract::supported(rows, world, stride, pages),
                        (1..=32).contains(&rows)
                            && world == 1
                            && (1..=512).contains(&stride)
                            && (1..=16384).contains(&pages)
                    );
                }
            }
        }
    }
    assert_eq!(contract::MAX_PHYSICAL_SLOTS, 262144);
    assert_eq!(contract::MAX_CACHE_ELEMENTS, 268435456);
    assert_eq!(u64::from(contract::MAX_CACHE_ELEMENTS) * 2, 536870912);
    assert_eq!(
        u64::from(contract::MAX_CACHE_ELEMENTS) * 2 * 2 * 36,
        38654705664
    );
    assert!(APPEND.contains("position < 8192"));
    assert!(ATTENTION.contains("max_context_tokens > 8192"));
    assert_eq!(ATTENTION.matches("physical_page < 16384").count(), 2);
    assert_eq!(ATTENTION.matches("cache_base < 268435329").count(), 6);
    assert_eq!(268435329_u32 - 1 + 127, contract::MAX_CACHE_ELEMENTS - 1);
}

#[test]
fn attention_active_output_ownership_leaves_tail_rows_untouched() {
    for rows in 1..=32_usize {
        let mut owners = vec![0_u8; 32 * 4096];
        for group in 0..rows * 32 {
            for lane in 0..64 {
                owners[group * 128 + lane] += 1;
                owners[group * 128 + lane + 64] += 1;
            }
        }
        assert!(owners[..rows * 4096].iter().all(|count| *count == 1));
        assert!(owners[rows * 4096..].iter().all(|count| *count == 0));
    }
}
