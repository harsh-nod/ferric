use super::*;
use std::string::ToString;
use std::vec;
use syn::{FnArg, GenericArgument, Item, PathArguments, Type};

const N: usize = 151936;
const SOURCE: &str = include_str!("../logits.rs");
const BASELINE: &str = include_str!("../../../qwen3-tp-batch-kernels-v2/src/logits.rs");

// Independent oracle: order finite BF16 encodings as integers, canonicalizing both zeros.
fn scalar(values: &[u16]) -> Result<u32, ()> {
    assert_eq!(values.len(), N);
    let mut winner = 0;
    let mut maximum = 0_u16;
    for (token, &raw) in values.iter().enumerate() {
        if raw & 0x7f80 == 0x7f80 {
            return Err(());
        }
        let raw = if raw & 0x7fff == 0 { 0 } else { raw };
        let key = if raw & 0x8000 != 0 {
            !raw
        } else {
            raw ^ 0x8000
        };
        if token == 0 || key > maximum {
            maximum = key;
            winner = token as u32;
        }
    }
    Ok(winner)
}

fn wave_max(mut values: [f32; 64]) -> f32 {
    for offset in [1, 2, 4, 8, 16, 32] {
        let previous = values;
        for lane in 0..64 {
            values[lane] = previous[lane].max(previous[lane ^ offset]);
        }
    }
    assert!(values.iter().all(|&value| value == values[0]));
    values[0]
}

// Numeric fixture only; no host emulation of a device capability or kernel launch.
fn cooperative(values: &[u16], base: usize) -> Result<u32, ()> {
    let logits_view = StridedReadView2D::from_shared_slice(values, base, 1, N, N).unwrap();
    let row = 0;
    let lanes: [(f32, u32, f32); 64] =
        core::array::from_fn(|lane| lane_argmax!(logits_view, row, lane));
    if wave_max(core::array::from_fn(|lane| lanes[lane].2)) != 0.0 {
        return Err(());
    }
    let maximum = wave_max(core::array::from_fn(|lane| lanes[lane].0));
    let key = wave_max(core::array::from_fn(|lane| {
        let (value, winner, _) = lanes[lane];
        stable_key!(value, maximum, winner)
    }));
    Ok(decode_key!(key))
}

fn abi(source: &str) -> (std::string::String, std::vec::Vec<std::string::String>, u32) {
    let roots: std::vec::Vec<_> = syn::parse_file(source)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(root) if root.attrs.iter().any(|attr| attr.path().is_ident("kernel")) => {
                Some(root)
            }
            _ => None,
        })
        .collect();
    assert_eq!(roots.len(), 1);
    let root = &roots[0];
    let mut types = vec![];
    let mut bytes = 0;
    for argument in &root.sig.inputs {
        let FnArg::Typed(argument) = argument else {
            panic!("receiver")
        };
        match argument.ty.as_ref() {
            Type::Reference(reference) => {
                assert!(reference.mutability.is_none());
                let Type::Slice(slice) = reference.elem.as_ref() else {
                    panic!("not a slice")
                };
                let Type::Path(path) = slice.elem.as_ref() else {
                    panic!("slice element")
                };
                assert!(path.path.is_ident("u16"));
                types.push(std::string::String::from("&[u16]"));
                bytes += 16;
            }
            Type::Path(path) if path.path.is_ident("u32") => {
                types.push(std::string::String::from("u32"));
                bytes += 4;
            }
            Type::Path(path) => {
                let last = path.path.segments.last().unwrap();
                assert_eq!(last.ident, "WriteOnlyDisjointSlice");
                let PathArguments::AngleBracketed(arguments) = &last.arguments else {
                    panic!("output")
                };
                let Some(GenericArgument::Type(Type::Path(element))) = arguments.args.first()
                else {
                    panic!("element")
                };
                assert!(element.path.is_ident("u32"));
                types.push(std::string::String::from("WriteOnlyDisjointSlice<u32>"));
                bytes += 16;
            }
            _ => panic!("unsupported argument"),
        }
    }
    (root.sig.ident.to_string(), types, bytes)
}

#[test]
fn original_symbol_36_byte_abi_launch_and_capacity_are_unchanged() {
    assert_eq!(abi(SOURCE), abi(BASELINE));
    assert_eq!(abi(SOURCE).0, "ferric_qwen3_tp_batch_argmax_bf16_v2");
    assert_eq!(abi(SOURCE).2, 36);
    for text in [
        "required = [64, 1, 1], max = [64, 1, 1], max_grid = [16, 1, 1]",
        "rows == 0 || rows > 16",
        "logits.len() < rows * 151936",
        "logits.len() > 16 * 151936",
        "choices.len() < rows",
        "choices.len() > 16",
        "thread::launch_extent_1d() != rows * 64",
        "RowStriped2D<Index1D, 64, 1>",
    ] {
        assert!(SOURCE.contains(text), "{text}");
        assert!(BASELINE.contains(text), "{text}");
    }
}

#[test]
fn source_equivalence_and_invalid_rejection_precede_the_only_output_store() {
    for name in ["lane_argmax", "stable_key", "decode_key"] {
        let fixture = SOURCE
            .split(&std::format!("macro_rules! {name}"))
            .nth(1)
            .unwrap()
            .split("=> {{")
            .nth(1)
            .unwrap()
            .split("}};")
            .next()
            .unwrap();
        let emitted = SOURCE
            .split(&std::format!("// BEGIN {name}"))
            .nth(1)
            .unwrap()
            .split(&std::format!("// END {name}"))
            .next()
            .unwrap();
        let normalize = |value: &str| {
            value
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '$')
                .collect::<std::string::String>()
        };
        assert_eq!(normalize(fixture), normalize(emitted));
    }
    assert_eq!(SOURCE.matches("choices.write_row_striped_2d").count(), 1);
    let body = SOURCE
        .split("pub fn ferric_qwen3_tp_batch_argmax_bf16_v2")
        .nth(1)
        .unwrap();
    assert!(!body.contains("return;"));
    assert!(body.contains("if any_invalid != 0.0 {\n        fe2o3_device::trap();"));
    assert!(body.find("if any_invalid != 0.0").unwrap() < body.find("if lane == 0").unwrap());
}

#[test]
fn all_bf16_bit_patterns_decode_exactly_and_classify_nonfinites() {
    for bits in 0..=u16::MAX {
        let actual = Bf16::from_bits(bits).to_f32();
        assert_eq!(actual.to_bits(), u32::from(bits) << 16);
        assert_eq!(actual.is_finite(), bits & 0x7f80 != 0x7f80);
    }
}

#[test]
fn every_logit_and_active_output_has_exactly_one_owner() {
    let mut owners = vec![0_u8; N];
    for lane in 0..64 {
        for step in 0..2374 {
            owners[step * 64 + lane] += 1;
        }
    }
    assert!(owners.iter().all(|&count| count == 1));
    for rows in 1..=16 {
        let mut outputs = [0_u8; 16];
        for raw in 0..rows * 64 {
            if raw % 64 == 0 {
                outputs[raw / 64] += 1;
            }
        }
        assert!(outputs[..rows].iter().all(|&count| count == 1));
        assert!(outputs[rows..].iter().all(|&count| count == 0));
    }
}

#[test]
fn exact_nonzero_keys_preserve_lowest_id_and_reject_invalid_decodes() {
    for token in 0..N as u32 {
        let key = stable_key!(1.0, 1.0, token);
        assert_eq!(key as u32, N as u32 - token);
        assert_eq!(decode_key!(key), token);
    }
    for key in [0.0, -0.0, -1.0, 151937.0, f32::NAN, f32::INFINITY] {
        assert!(std::panic::catch_unwind(|| decode_key!(key)).is_err());
    }
}

#[test]
fn winner_in_every_lane_and_scan_boundary_matches_independent_reference() {
    let mut values = vec![0xc000; N];
    for lane in 0..64 {
        for step in [0, 1, 1187, 2373] {
            let token = step * 64 + lane;
            values[token] = 0x4040;
            assert_eq!(cooperative(&values, 0), Ok(token as u32));
            assert_eq!(cooperative(&values, 0), scalar(&values));
            values[token] = 0xc000;
        }
    }
}

#[test]
fn negative_uniform_extreme_and_subnormal_rows_match_independent_reference() {
    for bits in [
        0xff7f, 0xbf80, 0x8080, 0x8001, 0x8000, 0, 1, 0x7f, 0x80, 0x7f7f,
    ] {
        let mut values = vec![bits; N];
        assert_eq!(cooperative(&values, 0), Ok(0));
        assert_eq!(cooperative(&values, 0), scalar(&values));
        values[N - 1] = 0x7f7f;
        assert_eq!(cooperative(&values, 0), scalar(&values));
    }
    let mut values = vec![0x8001; N];
    values[64] = 0;
    values[129] = 1;
    assert_eq!(cooperative(&values, 0), Ok(129));
    assert_eq!(cooperative(&values, 0), scalar(&values));
}

#[test]
fn ties_bf16_rounding_and_signed_zeros_keep_the_original_lowest_id() {
    let mut values = vec![0xc000; N];
    for (first, second) in [(0, 63), (63, 64), (127, 128), (64, 128), (0, N - 1)] {
        for (left, right) in [(0xbf80, 0xbf80), (0x8000, 0), (0, 0x8000), (0x41c3, 0x41c3)] {
            values[first] = left;
            values[second] = right;
            assert_eq!(cooperative(&values, 0), Ok(first as u32));
            assert_eq!(cooperative(&values, 0), scalar(&values));
            values[first] = 0xc000;
            values[second] = 0xc000;
        }
    }
}

#[test]
fn exhaustive_finite_encodings_and_random_rows_match_integer_ordering() {
    let mut values = vec![0_u16; N];
    for (index, value) in values.iter_mut().enumerate() {
        let bits = index as u16;
        *value = if bits & 0x7f80 == 0x7f80 { 0 } else { bits };
    }
    assert_eq!(cooperative(&values, 0), scalar(&values));
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..16 {
        for value in &mut values {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let bits = state as u16;
            *value = if bits & 0x7f80 == 0x7f80 {
                bits ^ 0x0080
            } else {
                bits
            };
        }
        assert_eq!(cooperative(&values, 0), scalar(&values));
    }
}

#[test]
fn every_lane_and_all_nonfinite_encodings_reject_without_publishing() {
    let mut values = vec![0xbf80; N];
    values[0] = 0x7f7f;
    for lane in 0..64 {
        for step in [0, 1187, 2373] {
            let token = step * 64 + lane;
            let saved = values[token];
            for invalid in [0x7f80, 0xff80, 0x7fc1, 0x7f81, 0xff81] {
                values[token] = invalid;
                let mut published = 0xa5a5_a5a5;
                if let Ok(winner) = cooperative(&values, 0) {
                    published = winner;
                }
                assert_eq!(published, 0xa5a5_a5a5);
                assert_eq!(scalar(&values), Err(()));
            }
            values[token] = saved;
        }
    }
    for bits in 0..=u16::MAX {
        if bits & 0x7f80 == 0x7f80 {
            values[N - 1] = bits;
            assert_eq!(cooperative(&values, 0), Err(()));
        }
    }
}

#[test]
fn active_rows_ignore_invalid_capacity_tails_and_leave_input_and_output_tails_untouched() {
    for rows in [1, 2, 8, 16] {
        let mut values = vec![0x7fc1; 16 * N + 2];
        let mut choices = [0xa5a5_a5a5_u32; 18];
        values[1..1 + rows * N].fill(0xbf80);
        for row in 0..rows {
            values[1 + row * N + row * 997] = 0x3f80;
        }
        let before = values.clone();
        for row in 0..rows {
            choices[1 + row] = cooperative(&values, 1 + row * N).unwrap();
            assert_eq!(choices[1 + row], (row * 997) as u32);
        }
        assert_eq!(choices[0], 0xa5a5_a5a5);
        assert!(
            choices[1 + rows..]
                .iter()
                .all(|&value| value == 0xa5a5_a5a5)
        );
        assert_eq!(values, before);
    }
}
