use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::{fixed_host, host};

fn geometry() -> host::KernelGeometry {
    host::KernelGeometry {
        rows: 1,
        n: 12_288,
        k: 4096,
        world_size: 1,
        projection: 4,
        activation_words: 2048,
        weight_words: 12_288 * 2048,
        output_elements: 12_288,
        launch_threads: 12_288 * 64,
    }
}

#[test]
fn admits_only_gate_up_and_preserves_capacity_tails() {
    for role in [4, 5] {
        for capacity in [1, 32] {
            let mut value = geometry();
            value.projection = role;
            value.activation_words *= capacity;
            value.output_elements *= capacity;
            assert_eq!(fixed_host::validate(value), Ok(()));
        }
    }
}

#[test]
fn rejects_wrong_geometry_before_any_index_is_formed() {
    for value in [
        host::KernelGeometry { rows: 0, ..geometry() },
        host::KernelGeometry { rows: 2, ..geometry() },
        host::KernelGeometry { n: 4096, projection: 1, ..geometry() },
        host::KernelGeometry { n: 1024, projection: 2, ..geometry() },
        host::KernelGeometry { n: 12_287, ..geometry() },
        host::KernelGeometry { k: 12_288, ..geometry() },
        host::KernelGeometry { world_size: 2, ..geometry() },
        host::KernelGeometry { projection: 0, ..geometry() },
        host::KernelGeometry { projection: 6, ..geometry() },
    ] {
        assert_eq!(fixed_host::validate(value), Err(host::PackingError::Shape));
    }
}

#[test]
fn rejects_short_excess_or_mismatched_storage_and_launch() {
    for value in [
        host::KernelGeometry { activation_words: 2047, ..geometry() },
        host::KernelGeometry { activation_words: 65_537, ..geometry() },
        host::KernelGeometry { weight_words: 12_288 * 2048 - 1, ..geometry() },
        host::KernelGeometry { weight_words: 12_288 * 2048 + 1, ..geometry() },
        host::KernelGeometry { output_elements: 12_287, ..geometry() },
        host::KernelGeometry { output_elements: 32 * 12_288 + 1, ..geometry() },
        host::KernelGeometry { launch_threads: 12_288 * 64 - 1, ..geometry() },
        host::KernelGeometry { launch_threads: 12_288 * 64 + 64, ..geometry() },
    ] {
        assert_eq!(fixed_host::validate(value), Err(host::PackingError::Length));
    }
}

#[test]
fn full_shape_indices_stay_in_bounds_without_overflow() {
    for column in 0..12_288 {
        for group in 0..32 {
            for lane in 0..64 {
                let [left, right] = fixed_host::read_indices(column, group, lane).unwrap();
                assert!(left < 2048 && right < 12_288 * 2048);
                assert_eq!(right, host::word_index(12_288, 4096, column, group, lane).unwrap());
            }
        }
    }
    assert_eq!(fixed_host::read_indices(12_287, 31, 63), Ok([2047, 12_288 * 2048 - 1]));
}

#[test]
fn invalid_coordinates_return_without_touching_storage() {
    for (column, group, lane) in [(12_288, 0, 0), (0, 32, 0), (0, 0, 64), (usize::MAX, 0, 0)] {
        assert_eq!(fixed_host::read_indices(column, group, lane), Err(host::PackingError::Coordinate));
    }
}

#[test]
fn every_bf16_pattern_keeps_exact_lane_time_packing() {
    for start in (0..65_536_u32).step_by(4096) {
        let source = (start..start + 4096)
            .map(|value| u16::try_from(value).unwrap()).collect::<Vec<_>>();
        let packed = host::pack_rows(&source, 1, 4096).unwrap();
        assert_eq!(host::unpack_rows(&packed, 1, 4096).unwrap(), source);
        for group in 0..32 {
            for lane in 0..64 {
                let pair = host::unpack_word(packed[group * 64 + lane]);
                assert_eq!(pair, [source[group * 128 + lane], source[group * 128 + lane + 64]]);
            }
        }
    }
}

#[test]
fn finite_fp32_checkpoints_reduction_and_bf16_rounding_are_identical() {
    let values = [0x0000, 0x8000, 0x0001, 0x8001, 0x0080, 0x3f80, 0xbf80, 0x3e00];
    for shift in [0, 1, 3, 7] {
        let a = (0..4096).map(|i| values[i % values.len()]).collect::<Vec<_>>();
        let w = (0..4096).map(|i| values[(i + shift) % values.len()]).collect::<Vec<_>>();
        let packed_a = host::pack_rows(&a, 1, 4096).unwrap();
        let packed_w = host::pack_rows(&w, 1, 4096).unwrap();
        let baseline = (0..64).map(|lane| host::baseline_lane(&a, &w, lane).unwrap()).collect::<Vec<_>>();
        let candidate = (0..64).map(|lane| host::packed_lane(&packed_a, &packed_w, lane).unwrap()).collect::<Vec<_>>();
        assert_eq!(baseline, candidate);
        let result = host::finish_row(&baseline).unwrap();
        assert!(result.accepted);
        assert_eq!(result, host::finish_row(&candidate).unwrap());
    }
}

#[test]
fn nonfinite_input_and_overflow_keep_the_same_rejection() {
    for bits in [0x7f80, 0xff80, 0x7fc1, 0x7f7f] {
        let a = vec![bits; 4096];
        let w = vec![0x4000; 4096];
        let packed_a = host::pack_rows(&a, 1, 4096).unwrap();
        let packed_w = host::pack_rows(&w, 1, 4096).unwrap();
        let baseline = (0..64).map(|lane| host::baseline_lane(&a, &w, lane).unwrap()).collect::<Vec<_>>();
        let candidate = (0..64).map(|lane| host::packed_lane(&packed_a, &packed_w, lane).unwrap()).collect::<Vec<_>>();
        assert_eq!(baseline, candidate);
        assert!(!host::finish_row(&candidate).unwrap().accepted);
    }
}

#[test]
fn cancellation_keeps_low_then_high_accumulation_order() {
    let mut a = vec![0_u16; 4096];
    a[0] = 0x4b80;
    a[64] = 0x3f80;
    a[128] = 0xcb80;
    let w = vec![0x3f80; 4096];
    let packed_a = host::pack_rows(&a, 1, 4096).unwrap();
    let packed_w = host::pack_rows(&w, 1, 4096).unwrap();
    let baseline = host::baseline_lane(&a, &w, 0).unwrap();
    let candidate = host::packed_lane(&packed_a, &packed_w, 0).unwrap();
    assert_eq!(baseline, candidate);
    assert_eq!(candidate.steps[0].partial_bits, 0x4b80_0000);
    assert_eq!(candidate.steps[1].partial_bits, 0x4b80_0000);
    assert_eq!(candidate.steps[2].partial_bits, 0);
}

#[test]
fn a_nonfinite_nonzero_lane_rejects_the_whole_row_model() {
    let mut a = vec![0x3f80; 4096];
    a[7] = 0x7fc1;
    let w = vec![0x3f80; 4096];
    let packed_a = host::pack_rows(&a, 1, 4096).unwrap();
    let packed_w = host::pack_rows(&w, 1, 4096).unwrap();
    let lanes = (0..64)
        .map(|lane| host::packed_lane(&packed_a, &packed_w, lane).unwrap())
        .collect::<Vec<_>>();
    assert!(lanes[0].finite);
    assert!(!lanes[7].finite);
    assert!(!host::finish_row(&lanes).unwrap().accepted);
}

#[test]
fn dependency_free_reference_preserves_all_finite_bf16_patterns() {
    use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::reference_bf16::Bf16;
    for bits in 0..=u16::MAX {
        let value = Bf16::from_bits(bits);
        assert_eq!(value.to_f32().to_bits(), u32::from(bits) << 16);
        if value.is_finite() {
            assert_eq!(Bf16::from_f32(value.to_f32()).to_bits(), bits);
        }
    }
}

#[test]
fn dependency_free_reference_rounds_ties_to_even_and_keeps_nonfinite() {
    use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::reference_bf16::Bf16;
    for (fp32, bf16) in [
        (0x3f80_7fff, 0x3f80),
        (0x3f80_8000, 0x3f80),
        (0x3f80_8001, 0x3f81),
        (0x3f81_8000, 0x3f82),
        (0xbf80_8000, 0xbf80),
        (0xbf81_8000, 0xbf82),
        (0x0000_8000, 0x0000),
        (0x0001_8000, 0x0002),
        (0x8000_8000, 0x8000),
        (0x7f7f_8000, 0x7f80),
        (0x7f80_0000, 0x7f80),
        (0xff80_0000, 0xff80),
        (0x7f80_0001, 0x7fc0),
        (0xff80_0001, 0xffc0),
    ] {
        assert_eq!(Bf16::from_f32(f32::from_bits(fp32)).to_bits(), bf16);
    }
}

fn arithmetic(source: &str) -> String {
    let start = source.find("        {\n            let left = Bf16::from_bits(left_bits").unwrap();
    let end = source.find("    if lane == 0 {").unwrap();
    source[start..end].lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| !matches!(line.trim(),
            "finite &= product.is_finite() & partial.is_finite();"
            | "let finite = partial.is_finite();"))
        .flat_map(str::chars).filter(|ch| !ch.is_whitespace()).collect()
}

#[test]
fn device_arithmetic_and_rounding_match_r2_except_finite_check_placement() {
    assert_eq!(arithmetic(include_str!("../src/projection.rs")), arithmetic(include_str!("fixtures/r2-projection.rs")));
}

#[test]
fn device_checks_lane_finiteness_once_without_early_collective_exit() {
    let source = include_str!("../src/projection.rs");
    let start = source.find("    while group < 32 {").unwrap();
    let check = source.find("    let finite = partial.is_finite();").unwrap();
    let reduction = source.find("    let sum = subgroup.reduce_sum_f32::<64>(partial);").unwrap();
    let rejection = source.find("    if !finite || !sum.is_finite() || !narrowed.is_finite() {").unwrap();
    let store = source.find("    if lane == 0 {").unwrap();
    assert!(start < check && check < reduction && reduction < rejection && rejection < store);
    assert_eq!(source.matches("let finite = partial.is_finite();").count(), 1);
    assert!(!source.contains("let mut finite") && !source.contains("finite &= "));
    let body = &source[start..check];
    assert!(!body.contains("is_finite()"));
    assert!(!body.contains("trap()") && !body.contains("return") && !body.contains("break"));
    assert_eq!(body.matches("let product = left * right;").count(), 2);
    assert_eq!(body.matches("partial += product;").count(), 2);
}

#[path = "fixed_shape/finite_hoist.rs"]
mod finite_hoist;

#[test]
fn fixed_device_source_has_distinct_identity_and_only_safe_reads() {
    let source = include_str!("../src/projection.rs");
    assert!(source.contains("pub fn ferric_qwen3_c1_gate_up_fixed_safe_u32_bf16_r1("));
    assert_eq!(source.matches("StridedReadView2D::from_shared_slice(").count(), 2);
    assert_eq!(source.matches(".load_or(").count(), 2);
    assert!(!source.contains("unsafe") && !source.contains("get_unchecked"));
    assert!(!source.contains("a[packed_inner]") && !source.contains("weights["));
    let left = source.find("let left_bits = left_view.load_or(0, packed_inner, 0);").unwrap();
    let right = source.find("let right_bits = right_view.load_or(column, packed_inner, 0);").unwrap();
    let first_product = source.find("let product = left * right;").unwrap();
    assert!(left < right && right < first_product);
    assert!(source.find("if column >= 12_288").unwrap() < left);
    assert!(source.find("weights.len() != 12_288 * 2048").unwrap() < left);
}

#[test]
fn fixed_view_construction_is_uniform_and_precedes_the_collective_loop() {
    let source = include_str!("../src/projection.rs");
    let invocation = source.find("let invocation = thread::index_1d();").unwrap();
    for view in [
        "StridedReadView2D::from_shared_slice(a, 0, 1, 2048, 2048)",
        "StridedReadView2D::from_shared_slice(weights, 0, 12_288, 2048, 2048)",
    ] {
        assert!(source.find(view).unwrap() < invocation);
    }
    let start = source.find("    while group < 32 {").unwrap();
    let reduction = source.find("    let sum = subgroup.reduce_sum_f32::<64>(partial);").unwrap();
    let body = &source[start..reduction];
    assert!(!body.contains("trap()") && !body.contains("return") && !body.contains("break"));
}
