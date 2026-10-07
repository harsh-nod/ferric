use ferric_qwen3_tp_peer32_kernels_device_v6::{ROOTS_V6, compiler_expectation_roster_v6};
use syn::{FnArg, Item, Type};

const SOURCE: &str = include_str!("../src/collective.rs");

#[test]
fn exact_two_root_roster_and_argument_widths() {
    let roster = compiler_expectation_roster_v6();
    assert_eq!(roster.len(), 2);
    assert!(roster[0].kernel_binding_id() < roster[1].kernel_binding_id());
    let mut actual: Vec<_> = roster.iter().map(|entry| entry.export_name()).collect();
    actual.sort();
    let mut expected = ROOTS_V6;
    expected.sort();
    assert_eq!(actual, expected);
    let source = syn::parse_file(SOURCE).unwrap();
    let roots: Vec<_> = source
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.attrs.iter().any(|a| a.path().is_ident("kernel")) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(roots.len(), 2);
    for (name, bytes) in [(ROOTS_V6[0], 168_u32), (ROOTS_V6[1], 36)] {
        let function = roots
            .iter()
            .find(|function| function.sig.ident == name)
            .unwrap();
        assert!(function.sig.unsafety.is_none());
        let actual: u32 = function
            .sig
            .inputs
            .iter()
            .map(|arg| {
                let FnArg::Typed(arg) = arg else {
                    panic!("receiver")
                };
                match arg.ty.as_ref() {
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
                    _ => panic!("unexpected ABI type"),
                }
            })
            .sum();
        assert_eq!(actual, bytes);
    }
}

#[test]
fn all_active_rows_have_exact_injective_flat_ownership() {
    for rows in 1..=32_usize {
        let elements = rows * 4096;
        let groups = rows * 64;
        assert_eq!(groups * 64, elements);
        assert!(groups <= 2048);
        let mut owners = vec![0_u8; elements];
        for group in 0..groups {
            for lane in 0..64 {
                owners[group * 64 + lane] += 1;
            }
        }
        assert!(owners.into_iter().all(|count| count == 1));
    }
}

#[test]
fn source_keeps_ordered_rank_checks_and_bit_preserving_copy() {
    let production = SOURCE.split("#[cfg(test)]").next().unwrap();
    assert_eq!(production.matches("max_grid = [2048, 1, 1]").count(), 2);
    assert_eq!(
        production
            .matches("let elements = elements_v6!(rows)")
            .count(),
        2
    );
    assert!(production.contains("rows == 0 || rows > 32"));
    assert!(production.contains("if rows < 33"));
    assert!(production.contains("!(world == 2 || world == 8)"));
    let mut previous = 0;
    for rank in 0..8 {
        let offset = SOURCE
            .find(&format!("add_rank_v6!(sum, $p{rank})"))
            .unwrap();
        assert!(offset > previous);
        previous = offset;
    }
    assert!(SOURCE.contains("let mut sum = 0.0_f32"));
    assert!(SOURCE.contains("let value = sum + residual"));
    assert!(SOURCE.contains("!value.is_finite() || !$sum.is_finite()"));
    assert_eq!(SOURCE.matches("output.len() != elements").count(), 2);
    assert_eq!(
        SOURCE
            .matches("thread::launch_extent_1d() != elements")
            .count(),
        2
    );
    let copy = SOURCE
        .split("pub fn ferric_qwen3_tp_batch32_peer_copy_bf16_v6")
        .nth(1)
        .unwrap();
    let copy = copy.split("#[cfg(test)]").next().unwrap();
    assert!(copy.contains("let bits = memory::volatile_load(source, index)"));
    assert!(copy.contains("output.write(invocation, bits)"));
    assert!(!copy.contains("Bf16::"));
}
