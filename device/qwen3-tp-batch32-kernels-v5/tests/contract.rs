use ferric_qwen3_tp_batch32_kernels_device_v5::{compiler_expectation_roster_v5, contract};
use syn::{FnArg, Item, Type};

const PROJECTION: &str = include_str!("../src/projection.rs");
const ATTENTION: &str = include_str!("../src/attention.rs");
const COLLECTIVE: &str = include_str!("../src/collective.rs");

#[test]
fn independent_row32_profile_retains_page_and_model_geometry() {
    assert_eq!(contract::MAX_ROWS, 32);
    assert_eq!(contract::PAGE_TOKENS, 16);
    assert_eq!(contract::MAX_PAGES, 512);
    for rows in 0..=64 {
        assert_eq!(contract::rows_are_supported(rows), (1..=32).contains(&rows));
    }
    for source in [
        PROJECTION,
        ATTENTION,
        COLLECTIVE,
        include_str!("../src/baseline_projection.rs"),
        include_str!("../src/baseline_attention.rs"),
        include_str!("../src/activation.rs"),
        include_str!("../src/embedding.rs"),
        include_str!("../src/logits.rs"),
        include_str!("../src/rope_kv.rs"),
    ] {
        assert!(!source.contains("rows > 16"));
        assert!(!source.contains("rows < 17"));
        assert!(source.contains("rows > 32"));
        assert!(source.contains("rows < 33"));
        assert!(!source.contains("fn ferric_qwen3_tp_batch_"));
        assert!(!source.contains("fn ferric_qwen3_tp_wave_"));
    }
    for source in [PROJECTION, include_str!("../src/baseline_projection.rs")] {
        assert!(source.contains("tile_row * 16"));
        assert!(source.contains("((rows + 15) / 16)"));
        assert!(source.contains("tile_row < 2"));
    }
    assert_eq!(PROJECTION.matches("left.load_m16k16(&lane, tile_row * 16,").count(), 7);
    assert!(include_str!("../src/rope_kv.rs").contains("loop_bounds(32, 32, 32, 1024)"));
}

#[test]
fn exact_closed_roster_keeps_all_baseline_symbols() {
    let roster = compiler_expectation_roster_v5();
    assert_eq!(roster.len(), if cfg!(feature = "mfma") { 15 } else { 13 });
    assert!(
        roster
            .windows(2)
            .all(|p| p[0].kernel_binding_id() < p[1].kernel_binding_id())
    );
    let mut expected = contract::NEW_ROOTS.to_vec();
    expected.extend(
        contract::PERFORMANCE_ROOTS
            .into_iter()
            .filter(|root| cfg!(feature = "mfma") || !root.contains("mfma")),
    );
    expected.push("qwen3_rmsnorm_v1");
    expected.sort();
    let mut actual: Vec<_> = roster
        .iter()
        .map(|entry| {
            assert_ne!(entry.generated_host_contract_identity(), [0; 32]);
            entry.export_name()
        })
        .collect();
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn exact_six_new_typed_abi_widths() {
    let mut roots = Vec::new();
    for source in [PROJECTION, ATTENTION, COLLECTIVE] {
        for item in syn::parse_file(source).unwrap().items {
            if let Item::Fn(function) = item
                && function.attrs.iter().any(|a| a.path().is_ident("kernel"))
            {
                roots.push(function);
            }
        }
    }
    assert_eq!(roots.len(), 6);
    for (index, name) in contract::PERFORMANCE_ROOTS.iter().enumerate() {
        let root = roots.iter().find(|f| f.sig.ident == *name).unwrap();
        assert!(root.sig.unsafety.is_none());
        let bytes: u32 = root
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
                    _ => panic!("unexpected argument"),
                }
            })
            .sum();
        assert_eq!(
            bytes,
            if index < 4 {
                68
            } else if index == 4 {
                116
            } else {
                52
            }
        );
    }
}

#[test]
fn actual_sources_keep_finite_causal_and_active_row_checks() {
    assert_eq!(PROJECTION.matches("Gfx950Subgroup::current()").count(), 2);
    assert_eq!(
        PROJECTION.matches("reduce_sum_f32::<64>(partial)").count(),
        2
    );
    assert_eq!(PROJECTION.matches("DeviceMatrix::current()").count(), 2);
    assert_eq!(
        PROJECTION
            .matches("matrix.multiply_accumulate(a_fragment, b_fragment, accumulator)")
            .count(),
        7
    );
    assert!(PROJECTION.contains("Bf16MfmaAMatrix::row_major(a, 0, rows, 4096, 4096)"));
    assert!(PROJECTION.contains("Bf16MfmaAMatrix::row_major(a, 0, rows, k, k)"));
    assert!(!PROJECTION.contains("wrapping_"));
    assert!(ATTENTION.contains("token <= position"));
    assert!(ATTENTION.contains("physical_page < physical_pages"));
    assert!(ATTENTION.contains("physical_page < 512"));
    assert!(ATTENTION.contains("product_0 + product_1"));
    assert!(ATTENTION.contains("reduce_sum_f32::<64>(partial)"));
    assert!(!ATTENTION.contains("while dimension"));
    assert!(ATTENTION.contains("product_0.is_finite() & product_1.is_finite()"));
    assert!(ATTENTION.contains("& denominator.is_finite()"));
    assert!(ATTENTION.contains("if !finite"));
    assert_eq!(ATTENTION.matches("broadcast_f32::<64>").count(), 2);
    assert!(ATTENTION.contains("!narrowed_1.is_finite()"));
}

#[test]
fn tiled_output_and_wave_leader_ownership_cover_each_active_element_once() {
    for rows in 1..=32_usize {
        for columns in [128, 512, 1536, 4096] {
            let mut counts = vec![0_u8; rows * columns];
            for tile in 0..rows.div_ceil(16) * (columns / 16) {
                for lane in 0..64 {
                    for component in 0..4 {
                        let row = (tile / (columns / 16)) * 16 + lane / 16 * 4 + component;
                        let column = (tile % (columns / 16)) * 16 + lane % 16;
                        if row < rows {
                            counts[row * columns + column] += 1;
                        }
                    }
                }
            }
            assert!(counts.iter().all(|count| *count == 1));
            for (index, count) in counts.iter_mut().enumerate() {
                let raw_leader = index * 64;
                assert_eq!(raw_leader / 64, index);
                *count -= 1;
            }
            assert!(counts.iter().all(|count| *count == 0));
        }
    }
}

fn wave_sum(mut values: [f32; 64]) -> [f32; 64] {
    for offset in [1, 2, 4, 8, 16, 32] {
        let previous = values;
        for lane in 0..64 {
            values[lane] = previous[lane] + previous[lane ^ offset];
        }
    }
    values
}

#[test]
fn cooperative_dense_dot_handles_all_supported_reduction_widths() {
    for width in [128, 512, 1536, 2048, 4096, 6144, 12288] {
        let mut partials = [0_f32; 64];
        let mut reference = 0_f64;
        for dimension in 0..width {
            let left = ((dimension * 3 % 31) as f32 - 15.0) / 32.0;
            let right = ((dimension * 7 % 23) as f32 - 11.0) / 64.0;
            partials[dimension % 64] += left * right;
            reference += f64::from(left) * f64::from(right);
        }
        for result in wave_sum(partials) {
            assert_eq!(f64::from(result), reference);
        }
    }
}

#[test]
fn attention_cooperative_128_dimensions_do_not_drop_the_second_half() {
    let mut partials = [0_f32; 64];
    for (lane, partial) in partials.iter_mut().enumerate() {
        let product_0 = (lane as f32 - 7.0) * 0.0625;
        let product_1 = (lane as f32 + 64.0) * -0.03125;
        *partial = product_0 + product_1;
    }
    let result = wave_sum(partials);
    assert!(result.iter().all(|value| *value == -93.0));
}

#[test]
fn deferred_projection_nonfinite_values_reach_every_lane_before_store() {
    for bad_lane in 0..64 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX * 2.0] {
            let mut partials = [0.25_f32; 64];
            partials[bad_lane] = invalid;
            assert!(wave_sum(partials).iter().all(|sum| !sum.is_finite()));
        }
    }
}

#[test]
fn metadata_float_carrier_preserves_valid_values_and_rejects_invalid_bounds() {
    for value in 0..8192_u32 {
        assert_eq!(value as f32 as usize, value as usize);
    }
    for limit in [1_u32, 16, 128, 512, 8192] {
        for value in [limit, limit + 1, 65535, 1 << 24, (1 << 24) + 1, u32::MAX] {
            assert!(value as f32 as usize >= limit as usize);
        }
    }
}
