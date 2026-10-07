use fe2o3_device::{Bf16, Bf16MfmaAMatrix, Bf16MfmaBMatrix};
use ferric_qwen3_tp_prefill32_m2_gate_up_kernels_device_v1::{
    EXPERIMENT_ENABLED, ROOT,
    contract::{
        GROUPS, HALF_BYTES, HALF_ELEMENTS, K, LANES, LaunchContract, N, OUTPUT_BYTES, ROWS,
        output_coordinate,
    },
};

#[path = "../build/target_contract.rs"]
mod target_contract;

#[test]
fn exact_gate_and_up_are_admitted() {
    for projection in [4, 5] {
        assert!(LaunchContract::exact(projection, 17).admitted());
    }
}

#[test]
fn geometry_and_row_tails_are_refused() {
    let original = LaunchContract::exact(4, 17);
    for rows in [0, 1, 16, 17, 31, 33, u32::MAX] {
        assert!(!LaunchContract { rows, ..original }.admitted());
    }
    for n in [0, 4096, 12287, 12289, u32::MAX] {
        assert!(!LaunchContract { n, ..original }.admitted());
    }
    for k in [0, 4095, 4097, 12288, u32::MAX] {
        assert!(!LaunchContract { k, ..original }.admitted());
    }
    for world_size in [0, 2, 8, u32::MAX] {
        assert!(
            !LaunchContract {
                world_size,
                ..original
            }
            .admitted()
        );
    }
    for projection in [0, 1, 2, 3, 6, 7, u32::MAX] {
        assert!(
            !LaunchContract {
                projection,
                ..original
            }
            .admitted()
        );
    }
}

#[test]
fn every_slice_requires_exact_not_padded_extent() {
    let original = LaunchContract::exact(5, 17);
    for a_elements in [0, ROWS * K - 1, ROWS * K + 1, usize::MAX] {
        assert!(
            !LaunchContract {
                a_elements,
                ..original
            }
            .admitted()
        );
    }
    for weights_elements in [0, K * N - 1, K * N + 1, usize::MAX] {
        assert!(
            !LaunchContract {
                weights_elements,
                ..original
            }
            .admitted()
        );
    }
    for half in 0..2 {
        for length in [0, HALF_ELEMENTS - 1, HALF_ELEMENTS + 1, usize::MAX] {
            let mut changed = original;
            changed.output_elements[half] = length;
            assert!(!changed.admitted());
        }
    }
}

#[test]
fn incomplete_and_excess_launches_are_refused() {
    let original = LaunchContract::exact(4, 17);
    for grid_x in [0, GROUPS - 1, GROUPS + 1, 2 * GROUPS, usize::MAX] {
        assert!(!LaunchContract { grid_x, ..original }.admitted());
    }
    for block_x in [0, 32, 63, 65, 128, usize::MAX] {
        assert!(
            !LaunchContract {
                block_x,
                ..original
            }
            .admitted()
        );
    }
}

#[test]
fn adjacent_output_spans_reject_aliases_gaps_and_wrong_allocation() {
    let original = LaunchContract::exact(4, 17);
    assert_eq!(
        original.output_offsets[0] + HALF_BYTES,
        original.output_offsets[1]
    );
    assert_eq!(original.output_offsets[1] + HALF_BYTES, OUTPUT_BYTES);
    for output_offsets in [
        [0, 0],
        [0, HALF_BYTES - 2],
        [0, HALF_BYTES + 2],
        [2, HALF_BYTES],
        [HALF_BYTES, 0],
        [0, usize::MAX],
    ] {
        assert!(
            !LaunchContract {
                output_offsets,
                ..original
            }
            .admitted()
        );
    }
    assert!(
        !LaunchContract {
            output_buffers: [17, 18],
            ..original
        }
        .admitted()
    );
    for output_allocation_bytes in [0, OUTPUT_BYTES - 2, OUTPUT_BYTES + 2, usize::MAX] {
        assert!(
            !LaunchContract {
                output_allocation_bytes,
                ..original
            }
            .admitted()
        );
    }
}

#[test]
fn complete_output_ownership_has_no_overlap_or_hole() {
    let mut seen = vec![false; ROWS * N];
    for raw in 0..GROUPS * LANES {
        for half in 0..2 {
            for component in 0..4 {
                let (row, column) = output_coordinate(raw, half, component).unwrap();
                assert!(row < ROWS && column < N);
                let index = row * N + column;
                assert!(!seen[index], "duplicate output {row},{column}");
                seen[index] = true;
                let view_index = (row - half * 16) * N + column;
                assert!(view_index < HALF_ELEMENTS);
                assert_eq!(half * HALF_BYTES + view_index * 2, index * 2);
            }
        }
    }
    assert!(seen.into_iter().all(|owned| owned));
}

#[test]
fn row_major_inverse_matches_each_old_v5_m16_tile() {
    for row in 0..ROWS {
        for column in 0..N {
            let half = row / 16;
            let group = column / 16;
            let lane = ((row % 16) / 4) * 16 + column % 16;
            let component = row % 4;
            let raw = group * LANES + lane;
            assert_eq!(output_coordinate(raw, half, component), Some((row, column)));
            let old_raw = (half * GROUPS + group) * LANES + lane;
            let old_tile = old_raw / LANES;
            let old_row = (old_tile / GROUPS) * 16 + (lane / 16) * 4 + component;
            let old_column = (old_tile % GROUPS) * 16 + lane % 16;
            assert_eq!((old_row, old_column), (row, column));
        }
    }
}

#[test]
fn row_half_boundaries_and_out_of_range_coordinates_are_exact() {
    assert_eq!(output_coordinate(0, 0, 0), Some((0, 0)));
    assert_eq!(output_coordinate(63, 0, 3), Some((15, 15)));
    assert_eq!(output_coordinate(0, 1, 0), Some((16, 0)));
    assert_eq!(
        output_coordinate(GROUPS * LANES - 1, 1, 3),
        Some((31, N - 1))
    );
    for (raw, half, component) in [
        (GROUPS * LANES, 0, 0),
        (usize::MAX, 0, 0),
        (0, 2, 0),
        (0, usize::MAX, 0),
        (0, 0, 4),
        (0, 0, usize::MAX),
    ] {
        assert_eq!(output_coordinate(raw, half, component), None);
    }
}

#[test]
fn fragment_lane_addresses_cover_both_a_halves_and_one_b_tile() {
    for step in [0, 255] {
        for group in [0, GROUPS - 1] {
            let mut a_seen = [false; 32 * 16];
            let mut b_seen = [false; 16 * 16];
            for lane in 0..LANES {
                for element in 0..4 {
                    let reduction = (lane / 16) * 4 + element;
                    let column = lane % 16;
                    let b_index = reduction * 16 + column;
                    assert!(!b_seen[b_index]);
                    b_seen[b_index] = true;
                    assert!((step * 16 + reduction) * N + group * 16 + column < K * N);
                    for half in 0..2 {
                        let row = half * 16 + lane % 16;
                        let a_index = row * 16 + reduction;
                        assert!(!a_seen[a_index]);
                        a_seen[a_index] = true;
                        assert!(row * K + step * 16 + reduction < ROWS * K);
                    }
                }
            }
            assert!(a_seen.into_iter().all(|loaded| loaded));
            assert!(b_seen.into_iter().all(|loaded| loaded));
        }
    }
}

#[test]
fn checked_matrix_views_reject_short_stride_extent_and_overflow() {
    let bits = [0_u16; 32];
    assert!(Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 16).is_ok());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 2).is_ok());
    assert!(Bf16MfmaAMatrix::row_major(&bits, 0, 2, 16, 15).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 0, 16, 2, 1).is_err());
    assert!(Bf16MfmaAMatrix::row_major(&bits[..31], 0, 2, 16, 16).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits[..31], 0, 16, 2, 2).is_err());
    assert!(Bf16MfmaAMatrix::row_major(&bits, 1, 2, 1, usize::MAX).is_err());
    assert!(Bf16MfmaBMatrix::row_major(&bits, 1, 2, 1, usize::MAX).is_err());
}

fn stored_bits(value: f32) -> Option<u16> {
    let narrowed = Bf16::from_f32(value);
    (value.is_finite() && narrowed.is_finite()).then_some(narrowed.to_bits())
}

#[test]
fn every_finite_bf16_value_retains_its_bits() {
    for bits in 0_u16..=u16::MAX {
        let value = Bf16::from_bits(bits);
        if value.is_finite() {
            assert_eq!(stored_bits(value.to_f32()), Some(bits));
        }
    }
}

#[test]
fn bf16_rounding_retains_ties_even_and_signed_zero() {
    for (input, output) in [
        (0x0000_0000, 0x0000),
        (0x8000_0000, 0x8000),
        (0x3f80_8000, 0x3f80),
        (0x3f81_8000, 0x3f82),
        (0xbf80_8000, 0xbf80),
        (0xbf81_8000, 0xbf82),
        (0x0000_8000, 0x0000),
        (0x0001_8000, 0x0002),
    ] {
        assert_eq!(stored_bits(f32::from_bits(input)), Some(output));
    }
}

#[test]
fn nonfinite_input_and_finite_to_bf16_overflow_are_refused() {
    for value in [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0001),
        f32::MAX,
        -f32::MAX,
    ] {
        assert_eq!(stored_bits(value), None);
    }
}

#[test]
fn exact_target_flags_and_non_device_fixture_are_accepted() {
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    for flags in [
        format!("-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature={features}"),
        format!("-C\u{1f}target-cpu=gfx950\u{1f}-C\u{1f}target-feature={features}"),
        format!("--codegen=target-cpu=gfx950\u{1f}--codegen=target-feature={features}"),
    ] {
        assert!(
            target_contract::validate_device_build("amdgpu", &flags, "gfx950", features).is_ok()
        );
    }
    assert!(target_contract::validate_device_build("x86_64", "", "gfx950", features).is_ok());
}

#[test]
fn wrong_duplicate_or_missing_target_flags_are_refused() {
    let features = "-wavefrontsize32,+wavefrontsize64,-xnack";
    let good = format!("-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature={features}");
    for flags in [
        String::new(),
        good.replace("gfx950", "gfx942"),
        good.replace("+wavefrontsize64", "-wavefrontsize64"),
        good.replace("-xnack", "+xnack"),
        format!("{good}\u{1f}-Ctarget-cpu=gfx950"),
        format!("{good}\u{1f}-Ctarget-feature={features}"),
    ] {
        assert!(
            target_contract::validate_device_build("amdgpu", &flags, "gfx950", features).is_err()
        );
    }
}

#[test]
fn opt_in_feature_and_single_root_remain_explicit() {
    assert_eq!(
        EXPERIMENT_ENABLED,
        cfg!(feature = "prefill32-m2-gate-up-r1")
    );
    assert_eq!(ROOT, "ferric_qwen3_prefill32_m2_gate_up_bf16_r1");
    assert!(include_str!("../Cargo.toml").contains("default = []"));
    #[cfg(feature = "prefill32-m2-gate-up-r1")]
    assert_eq!(
        ferric_qwen3_tp_prefill32_m2_gate_up_kernels_device_v1::compiler_expectation_roster().len(),
        1
    );
}
