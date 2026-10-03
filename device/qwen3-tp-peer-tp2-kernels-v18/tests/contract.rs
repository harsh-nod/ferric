use ferric_qwen3_tp_peer_tp2_kernels_device_v18::{ROOTS_V18, compiler_expectation_roster_v18};
use syn::{FnArg, Item, Pat, Type};

const CONSUMER: &str = include_str!("../src/collective.rs");
const COPY: &str = include_str!("../src/copy.rs");

#[test]
fn exact_two_root_roster_is_closed_and_sorted_by_binding() {
    let roster = compiler_expectation_roster_v18();
    assert_eq!(roster.len(), 2);
    assert!(roster[0].kernel_binding_id() < roster[1].kernel_binding_id());
    let mut actual: Vec<_> = roster.iter().map(|entry| entry.export_name()).collect();
    actual.sort_unstable();
    assert_eq!(actual, ROOTS_V18);
    assert!(!actual.contains(&"ferric_qwen3_tp_peer_ordered_residual_bf16_v4"));
}

#[test]
fn exact_signatures_preserve_consumer_168_and_copy_36_explicit_bytes() {
    for (source, root, slice_count, scalar_count, bytes) in [
        (CONSUMER, ROOTS_V18[1], 10, 2, 168),
        (COPY, ROOTS_V18[0], 2, 1, 36),
    ] {
        let parsed = syn::parse_file(source).unwrap();
        let roots: Vec<_> = parsed
            .items
            .iter()
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
            .collect();
        assert_eq!(roots.len(), 1);
        let function = roots[0];
        assert_eq!(function.sig.ident, root);
        assert!(function.sig.unsafety.is_none());
        assert_eq!(function.sig.inputs.len(), slice_count + scalar_count);
        let widths: Vec<_> = function
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
                    _ => panic!("unexpected ABI type"),
                }
            })
            .collect();
        assert_eq!(
            widths,
            [vec![16; slice_count], vec![4; scalar_count]].concat()
        );
        assert_eq!(widths.iter().sum::<usize>(), bytes);
        if root == ROOTS_V18[1] {
            let names: Vec<_> = function
                .sig
                .inputs
                .iter()
                .map(|argument| {
                    let FnArg::Typed(argument) = argument else {
                        panic!("receiver")
                    };
                    let Pat::Ident(name) = argument.pat.as_ref() else {
                        panic!("pattern")
                    };
                    name.ident.to_string()
                })
                .collect();
            assert_eq!(
                names,
                [
                    "p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "residual", "output", "rows",
                    "world"
                ]
            );
        }
    }
}

#[test]
fn consumer_is_closed_to_row_one_world_two_and_4096_threads() {
    let kernel = CONSUMER
        .split("pub fn ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18")
        .nth(1)
        .unwrap();
    let kernel = kernel.split("#[cfg(test)]").next().unwrap();
    assert!(kernel.contains("if rows != 1 || world != 2"));
    assert!(kernel.contains("let elements = 4096_usize;"));
    assert!(CONSUMER.contains("max_grid = [64, 1, 1]"));
    assert!(CONSUMER.contains("required = [64, 1, 1], max = [64, 1, 1]"));
    let first_load = kernel.find("memory::volatile_load").unwrap();
    for condition in [
        "p0.len() != elements",
        "p1.len() != elements",
        "residual.len() != elements",
        "output.len() != elements",
        "thread::launch_extent_1d() != elements",
        "if index < elements",
    ] {
        assert!(kernel.find(condition).unwrap() < first_load);
    }
    assert_eq!(kernel.matches("memory::volatile_load(").count(), 3);
    for rank in 2..8 {
        assert!(kernel.find(&format!("p{rank}.len() != 0")).unwrap() < first_load);
        assert!(!kernel.contains(&format!("volatile_load(p{rank},")));
    }
    assert!(kernel.contains("if !output.write(invocation, value)"));
}

#[test]
fn consumer_has_exact_injective_4096_element_ownership() {
    let mut owners = vec![0_u8; 4096];
    for group in 0..64 {
        for lane in 0..64 {
            owners[group * 64 + lane] += 1;
        }
    }
    assert!(owners.into_iter().all(|count| count == 1));
}

#[test]
fn copy_keeps_generic_rows_launch_and_bit_preserving_body() {
    assert!(COPY.contains("if rows == 0 || rows > 16"));
    assert!(COPY.contains("let elements = rows * 4096;"));
    assert!(COPY.contains("max_grid = [1024, 1, 1]"));
    assert!(COPY.contains("thread::launch_extent_1d() != elements"));
    assert!(COPY.contains("let bits = memory::volatile_load(source, index)"));
    assert!(COPY.contains("output.write(invocation, bits)"));
    assert!(!COPY.contains("Bf16::"));
    for rows in 1..=16 {
        assert_eq!(rows * 64 * 64, rows * 4096);
    }
}

#[test]
fn consumer_retains_ordered_additions_checks_and_one_narrowing() {
    let arithmetic = CONSUMER.split("#[kernel(").next().unwrap();
    let initial = arithmetic.find("let mut sum = 0.0_f32").unwrap();
    let rank0 = arithmetic.find("add_rank_v18!(sum, $p0)").unwrap();
    let rank1 = arithmetic.find("add_rank_v18!(sum, $p1)").unwrap();
    let residual = arithmetic.find("let value = sum + residual").unwrap();
    assert!(initial < rank0 && rank0 < rank1 && rank1 < residual);
    assert_eq!(arithmetic.matches("Bf16::from_f32").count(), 1);
    assert!(arithmetic.contains("!value.is_finite() || !$sum.is_finite()"));
    assert!(
        arithmetic.contains("!residual.is_finite() || !value.is_finite() || !narrowed.is_finite()")
    );
}
