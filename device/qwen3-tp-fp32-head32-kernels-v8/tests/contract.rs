use ferric_qwen3_tp_fp32_head32_kernels_device_v8::{ROOTS_V8, compiler_expectation_roster_v8};
use syn::{FnArg, Item, Type};

const PROJECTION: &str = include_str!("../src/projection.rs");
const LOGITS: &str = include_str!("../src/logits.rs");

#[test]
fn closed_three_root_roster_and_exact_argument_widths() {
    let roster = compiler_expectation_roster_v8();
    assert_eq!(roster.len(), 3);
    assert!(
        roster
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id())
    );
    let mut actual: Vec<_> = roster.iter().map(|entry| entry.export_name()).collect();
    actual.sort();
    let mut expected = ROOTS_V8;
    expected.sort();
    assert_eq!(actual, expected);
    let sources = [
        syn::parse_file(PROJECTION).unwrap(),
        syn::parse_file(LOGITS).unwrap(),
    ];
    let functions: Vec<_> = sources
        .iter()
        .flat_map(|source| &source.items)
        .filter_map(|item| match item {
            Item::Fn(function) if function.attrs.iter().any(|a| a.path().is_ident("kernel")) => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(functions.len(), 3);
    for (name, expected_bytes) in [(ROOTS_V8[0], 68_u32), (ROOTS_V8[1], 68), (ROOTS_V8[2], 36)] {
        let function = functions.iter().find(|f| f.sig.ident == name).unwrap();
        assert!(function.sig.unsafety.is_none());
        let bytes: u32 = function
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
                        assert!(matches!(r.elem.as_ref(), Type::Slice(_)));
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
            .sum();
        assert_eq!(bytes, expected_bytes);
    }
}

#[test]
fn every_active_output_has_one_owner_and_all_tail_rows_have_none() {
    for rows in 1..=32_usize {
        let mut owners = vec![0_u8; 32 * 151936];
        for group in 0..rows.div_ceil(16) * 9496 {
            for lane in 0..64 {
                let column = group % 9496 * 16 + lane % 16;
                for component in 0..4 {
                    let row = group / 9496 * 16 + lane / 16 * 4 + component;
                    if row < rows {
                        owners[row * 151936 + column] += 1;
                    }
                }
            }
        }
        assert!(owners[..rows * 151936].iter().all(|&owner| owner == 1));
        assert!(owners[rows * 151936..].iter().all(|&owner| owner == 0));
    }
}

#[test]
fn source_preserves_input_guards_fp32_stores_and_zero_filled_mfma_rows() {
    assert_eq!(PROJECTION.matches("world_size != 1").count(), 2);
    assert_eq!(PROJECTION.matches("projection != 6").count(), 2);
    assert_eq!(PROJECTION.matches("n != 151936").count(), 2);
    assert_eq!(PROJECTION.matches("k != 4096").count(), 2);
    assert_eq!(PROJECTION.matches("WriteOnlyDisjointSlice<f32").count(), 2);
    assert_eq!(PROJECTION.matches("rows > 32").count(), 2);
    assert_eq!(PROJECTION.matches("tile_row < 2").count(), 2);
    assert_eq!(PROJECTION.matches("tile_row as u8 as usize").count(), 2);
    let mfma = PROJECTION
        .split("pub fn ferric_qwen3_tp_batch32_mfma_head_f32_v8")
        .nth(1)
        .unwrap()
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    assert!(mfma.contains("Bf16MfmaAMatrix::row_major(a, 0, rows, 4096, 4096)"));
    assert!(mfma.contains("Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 151936, 151936)"));
    assert!(!mfma.contains("Bf16::from_f32"));
    assert!(mfma.contains("step < 256"));
    assert!(mfma.contains("left.load_m16k16(&lane, tile_row * 16, step * 16)"));
    assert!(mfma.contains("((rows + 15) / 16) * 9496"));
    for index in 0..4 {
        assert!(mfma.contains(&format!("!value_{index}.is_finite()")));
        assert!(mfma.contains(&format!(
            "&tile, {index}, rows, 151936, 151936, value_{index}"
        )));
    }
    let argmax = LOGITS
        .split("pub fn ferric_qwen3_tp_batch32_argmax_f32_v8")
        .nth(1)
        .unwrap()
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    assert!(argmax.contains("logits: &[f32]"));
    assert!(argmax.contains("candidate > value"));
    assert!(!argmax.contains("candidate >= value"));
    assert!(!argmax.contains("Bf16"));
    assert!(argmax.contains("rows > 32"));
    assert!(argmax.contains("logits.len() > 32 * 151936"));
    assert!(argmax.contains("choices.len() > 32"));
}

#[test]
fn host_numeric_macros_match_the_inline_emitted_bodies() {
    for (source, name) in [(PROJECTION, "four_dots"), (LOGITS, "fp32_argmax")] {
        let macro_body = source
            .split("=> {{")
            .nth(1)
            .unwrap()
            .split("}};")
            .next()
            .unwrap();
        let inline = source
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
