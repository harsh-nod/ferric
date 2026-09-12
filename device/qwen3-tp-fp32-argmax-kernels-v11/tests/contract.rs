use ferric_qwen3_tp_fp32_argmax_kernels_device_v11::{ROOTS_V11, compiler_expectation_roster_v11};
use syn::{FnArg, Item, Type};

const LOGITS: &str = include_str!("../src/logits.rs");

#[test]
fn closed_single_root_roster_and_unchanged_argmax_abi() {
    let roster = compiler_expectation_roster_v11();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V11[0]);
    let source = syn::parse_file(LOGITS).unwrap();
    let functions: Vec<_> = source
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.attrs.iter().any(|a| a.path().is_ident("kernel")) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(functions.len(), 1);
    let function = functions[0];
    assert_eq!(function.sig.ident, ROOTS_V11[0]);
    assert!(function.sig.unsafety.is_none());
    let widths: Vec<_> = function
        .sig
        .inputs
        .iter()
        .map(|arg| {
            let FnArg::Typed(arg) = arg else {
                panic!("receiver")
            };
            match arg.ty.as_ref() {
                Type::Reference(r) => {
                    assert!(r.mutability.is_none());
                    let Type::Slice(slice) = r.elem.as_ref() else {
                        panic!("not a slice")
                    };
                    assert!(matches!(slice.elem.as_ref(), Type::Path(p) if p.path.is_ident("f32")));
                    16
                }
                Type::Path(p)
                    if p.path.segments.last().unwrap().ident == "WriteOnlyDisjointSlice" =>
                {
                    16
                }
                Type::Path(p) if p.path.is_ident("u32") => 4,
                _ => panic!("unexpected ABI type"),
            }
        })
        .collect();
    assert_eq!(widths, [16, 16, 4]);
}

#[test]
fn all_lanes_participate_in_three_ordered_reductions_before_the_only_store() {
    assert_eq!(LOGITS.matches("reduce_max_f32::<64>").count(), 3);
    assert_eq!(LOGITS.matches("Gfx950Subgroup::current()").count(), 1);
    assert!(!LOGITS.contains("return;"));
    let invalid = LOGITS.find("reduce_max_f32::<64>(invalid)").unwrap();
    let reject = LOGITS.find("if any_invalid != 0.0").unwrap();
    let maximum = LOGITS.find("reduce_max_f32::<64>(value)").unwrap();
    let key = LOGITS.find("reduce_max_f32::<64>(key)").unwrap();
    let lane_zero = LOGITS.find("if lane == 0").unwrap();
    let store = LOGITS.find("choices.write_row_striped_2d").unwrap();
    assert!(invalid < reject && reject < maximum && maximum < key);
    assert!(key < lane_zero && lane_zero < store);
    assert_eq!(LOGITS.matches("choices.write_row_striped_2d").count(), 1);
    assert!(!LOGITS.contains("Bf16"));
}

#[test]
fn shape_contract_and_distinct_root_remain_bounded() {
    for guard in [
        "rows == 0 || rows > 32",
        "logits.len() < rows * 151936",
        "logits.len() > 32 * 151936",
        "choices.len() < rows",
        "choices.len() > 32",
        "thread::launch_extent_1d() != rows * 64",
        "required = [64, 1, 1]",
        "max_grid = [32, 1, 1]",
        "control_flow(loop_bounds(2374))",
        "checked_row_striped_2d::<64, 1>()",
        "winning_key < 151937",
        "winning_key == 0",
    ] {
        assert!(LOGITS.contains(guard), "missing {guard}");
    }
    assert!(!LOGITS.contains("lane_argmax!(logits, base, lane)"));
    assert!(!LOGITS.contains("ferric_qwen3_tp_batch32_argmax_f32_v8"));
}

#[test]
fn active_output_has_one_writer_and_capacity_tail_has_none() {
    for rows in 1..=32 {
        let mut owners = [0_u8; 32];
        for raw in 0..rows * 64 {
            let row = raw / 64;
            let lane = raw % 64;
            if lane == 0 {
                owners[row] += 1;
            }
        }
        assert!(owners[..rows].iter().all(|&owner| owner == 1));
        assert!(owners[rows..].iter().all(|&owner| owner == 0));
    }
}

#[test]
fn host_numeric_macros_match_inline_emitted_bodies() {
    for name in ["lane_argmax", "stable_key", "decode_key"] {
        let macro_body = LOGITS
            .split(&format!("macro_rules! {name}"))
            .nth(1)
            .unwrap()
            .split("=> {{")
            .nth(1)
            .unwrap()
            .split("}};")
            .next()
            .unwrap();
        let inline = LOGITS
            .split(&format!("// BEGIN {name}"))
            .nth(1)
            .unwrap()
            .split(&format!("// END {name}"))
            .next()
            .unwrap();
        let normalize = |value: &str| {
            value
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '$')
                .collect::<String>()
        };
        assert_eq!(normalize(macro_body), normalize(inline));
    }
}
