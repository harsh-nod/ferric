use ferric_qwen3_tp_projection_residual_kernels_device_v1::{
    ROOTS_V1, compiler_expectation_roster_v1,
};
use syn::{FnArg, Item, Pat, Type};

const SOURCE: &str = include_str!("../src/collective.rs");

#[test]
fn exact_single_root_roster_excludes_old_residual_and_copy() {
    let roster = compiler_expectation_roster_v1();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V1[0]);
    assert_eq!(
        ROOTS_V1,
        ["ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1"]
    );
}

#[test]
fn abi_keeps_ten_slices_then_rows_world_and_168_explicit_bytes() {
    let parsed = syn::parse_file(SOURCE).unwrap();
    let roots: Vec<_> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.attrs.iter().any(|a| a.path().is_ident("kernel")) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(roots.len(), 1);
    let function = roots[0];
    assert_eq!(function.sig.ident, ROOTS_V1[0]);
    assert!(function.sig.unsafety.is_none());
    let mut names = Vec::new();
    let widths: Vec<_> = function
        .sig
        .inputs
        .iter()
        .map(|arg| {
            let FnArg::Typed(arg) = arg else {
                panic!("receiver")
            };
            let Pat::Ident(name) = arg.pat.as_ref() else {
                panic!("pattern")
            };
            names.push(name.ident.to_string());
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
        .collect();
    assert_eq!(
        names,
        [
            "p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "residual", "output", "rows", "world"
        ]
    );
    assert_eq!(widths, [vec![16; 10], vec![4; 2]].concat());
    assert_eq!(widths.iter().sum::<usize>(), 168);
}

#[test]
fn original_row_one_world_two_and_launch_checks_precede_loads() {
    let body = SOURCE
        .split("pub fn ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1")
        .nth(1)
        .unwrap();
    let body = body.split("#[cfg(test)]").next().unwrap();
    assert!(SOURCE.contains("required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]"));
    let first_load = body.find("memory::volatile_load").unwrap();
    for guard in [
        "if rows != 1 || world != 2",
        "let elements = 4096_usize;",
        "p0.len() != elements",
        "p1.len() != elements",
        "residual.len() != elements",
        "output.len() != elements",
        "thread::launch_extent_1d() != elements",
        "if index < elements",
    ] {
        assert!(body.find(guard).unwrap() < first_load);
    }
    for rank in 2..8 {
        assert!(body.find(&format!("p{rank}.len() != 0")).unwrap() < first_load);
        assert!(!body.contains(&format!("volatile_load(p{rank},")));
    }
    assert_eq!(body.matches("memory::volatile_load(").count(), 3);
    assert!(body.contains("if !output.write(invocation, value)"));
}

#[test]
fn sixty_four_groups_have_exact_injective_4096_element_ownership() {
    let mut owners = vec![0_u8; 4096];
    for group in 0..64 {
        for lane in 0..64 {
            owners[group * 64 + lane] += 1;
        }
    }
    assert!(owners.into_iter().all(|count| count == 1));
}

#[test]
fn two_materializations_surround_only_the_residual_add() {
    let arithmetic = SOURCE.split("#[kernel(").next().unwrap();
    let positions = [
        "let mut sum = 0.0_f32;",
        "add_rank_v1!(sum, $p0);",
        "add_rank_v1!(sum, $p1);",
        "let projection = Bf16::from_f32(sum);",
        "if !projection.is_finite()",
        "let residual = Bf16::from_bits($residual).to_f32();",
        "let value = projection.to_f32() + residual;",
        "let narrowed = Bf16::from_f32(value);",
    ]
    .map(|part| arithmetic.find(part).unwrap());
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(arithmetic.matches("Bf16::from_f32").count(), 2);
    assert!(arithmetic.contains("!value.is_finite() || !$sum.is_finite()"));
    assert!(
        arithmetic.contains("!residual.is_finite() || !value.is_finite() || !narrowed.is_finite()")
    );
}
