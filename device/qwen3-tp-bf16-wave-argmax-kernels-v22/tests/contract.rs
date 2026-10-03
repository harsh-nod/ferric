use ferric_qwen3_tp_bf16_wave_argmax_kernels_device_v22::{
    ROOTS_V22, compiler_expectation_roster_v22,
};
use syn::{FnArg, Item, Type};

const LOGITS: &str = include_str!("../src/logits.rs");

#[test]
fn closed_single_root_roster_and_unchanged_bf16_argmax_abi() {
    let roster = compiler_expectation_roster_v22();
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].export_name(), ROOTS_V22[0]);
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
    assert_eq!(function.sig.ident, ROOTS_V22[0]);
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
                    assert!(matches!(slice.elem.as_ref(), Type::Path(p) if p.path.is_ident("u16")));
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
fn three_convergent_reductions_precede_the_only_store() {
    assert_eq!(LOGITS.matches("reduce_max_f32::<64>").count(), 3);
    assert_eq!(LOGITS.matches("Gfx950Subgroup::current()").count(), 1);
    assert!(!LOGITS.contains("return;"));
    let invalid = LOGITS.find("reduce_max_f32::<64>(invalid)").unwrap();
    let reject = LOGITS.find("if any_invalid != 0.0").unwrap();
    let maximum = LOGITS.find("reduce_max_f32::<64>(value)").unwrap();
    let key = LOGITS.find("reduce_max_f32::<64>(key)").unwrap();
    let lane_zero = LOGITS.find("if lane == 0").unwrap();
    let store = LOGITS.find("choices.write_row_striped_2d").unwrap();
    assert!(invalid < maximum && maximum < key && key < reject);
    assert!(reject < lane_zero && lane_zero < store);
    assert_eq!(LOGITS.matches("choices.write_row_striped_2d").count(), 1);
    assert!(!LOGITS.contains("Bf16"));
    assert!(!LOGITS.contains("to_f32"));
    assert!(!LOGITS.contains("65535 -"));
    assert!(!LOGITS.contains("32768 +"));
    assert_eq!(LOGITS.matches("raw ^ 0xffff").count(), 3);
    assert_eq!(LOGITS.matches("raw | 0x8000").count(), 3);
}

#[test]
fn shape_and_launch_contract_are_closed_and_bounded() {
    for guard in [
        "rows != 1",
        "logits_len != 16 * 151936",
        "choices_len != 16",
        "launch_extent != 64",
        "required = [64, 1, 1]",
        "max = [64, 1, 1]",
        "max_grid = [1, 1, 1]",
        "control_flow(loop_bounds(2374))",
        "checked_row_striped_2d::<64, 1>()",
        "winning_key < 151937",
        "winning_key == 0",
        "let row = thread::block_idx_x() as usize",
        "StridedReadView2D::from_shared_slice(logits, 0, rows, 151936, 151936)",
    ] {
        assert!(LOGITS.contains(guard), "missing {guard}");
    }
}

#[test]
fn active_output_has_one_writer_and_fifteen_capacity_outputs_have_none() {
    let mut owners = [0_u8; 16];
    for raw in 0..64 {
        let row = raw / 64;
        let lane = raw % 64;
        if lane == 0 {
            owners[row] += 1;
        }
    }
    assert_eq!(owners[0], 1);
    assert!(owners[1..].iter().all(|&owner| owner == 0));
}

#[test]
fn host_numeric_macros_match_inline_emitted_bodies() {
    for name in [
        "invalid_shape",
        "ordered_bits",
        "lane_argmax",
        "stable_key",
        "decode_key",
    ] {
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
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .flat_map(str::chars)
                .filter(|c| !c.is_whitespace() && *c != '$')
                .collect::<String>()
        };
        assert_eq!(normalize(macro_body), normalize(inline), "macro {name}");
    }
}
