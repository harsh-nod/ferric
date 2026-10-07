use fe2o3_device::{Bf16MfmaAMatrix, Bf16MfmaBMatrix};
use ferric_qwen3_tp_prefill32_down_k2_kernels_device_v1::{
    CONTROL_ROOT, EXPERIMENT_ENABLED, PAIRED_ROOT,
    contract::{GROUPS, K, LANES, LaunchContract, N, OUTPUT_BYTES, ROWS, output_coordinate},
};

#[path = "../build/target_contract.rs"]
mod target_contract;

#[test]
fn only_exact_tp1_prefill32_down_geometry_is_admitted() {
    let exact = LaunchContract::exact();
    assert!(exact.admitted());
    for rows in [0, 1, 16, 17, 31, 33, u32::MAX] {
        assert!(!LaunchContract { rows, ..exact }.admitted());
    }
    for n in [0, 1024, 4095, 4097, 12288, u32::MAX] {
        assert!(!LaunchContract { n, ..exact }.admitted());
    }
    for k in [0, 4096, 6144, 12287, 12289, u32::MAX] {
        assert!(!LaunchContract { k, ..exact }.admitted());
    }
    for world_size in [0, 2, 8, u32::MAX] {
        assert!(
            !LaunchContract {
                world_size,
                ..exact
            }
            .admitted()
        );
    }
    for projection in [0, 1, 3, 4, 5, 6, u32::MAX] {
        assert!(
            !LaunchContract {
                projection,
                ..exact
            }
            .admitted()
        );
    }
}

#[test]
fn every_slice_requires_exact_not_padded_extent() {
    let exact = LaunchContract::exact();
    for a_elements in [0, ROWS * K - 1, ROWS * K + 1, usize::MAX] {
        assert!(
            !LaunchContract {
                a_elements,
                ..exact
            }
            .admitted()
        );
    }
    for weights_elements in [0, K * N - 1, K * N + 1, usize::MAX] {
        assert!(
            !LaunchContract {
                weights_elements,
                ..exact
            }
            .admitted()
        );
    }
    for output_elements in [0, ROWS * N - 1, ROWS * N + 1, usize::MAX] {
        assert!(
            !LaunchContract {
                output_elements,
                ..exact
            }
            .admitted()
        );
    }
    assert_eq!(OUTPUT_BYTES, 524288);
}

#[test]
fn incomplete_excess_and_multidimensional_launches_are_refused() {
    let exact = LaunchContract::exact();
    for grid in [
        [0, 1, 1],
        [511, 1, 1],
        [513, 1, 1],
        [512, 2, 1],
        [512, 1, 2],
    ] {
        assert!(!LaunchContract { grid, ..exact }.admitted());
    }
    for workgroup in [[32, 1, 1], [63, 1, 1], [65, 1, 1], [64, 2, 1], [64, 1, 2]] {
        assert!(!LaunchContract { workgroup, ..exact }.admitted());
    }
}

#[test]
fn each_fp32_output_is_owned_once_in_the_unchanged_v5_layout() {
    let mut seen = vec![false; ROWS * N];
    for raw in 0..GROUPS * LANES {
        for component in 0..4 {
            let (row, column) = output_coordinate(raw, component).unwrap();
            assert!(row < ROWS && column < N);
            let offset = row * N + column;
            assert!(!seen[offset], "duplicate output {row},{column}");
            seen[offset] = true;
            let group = raw / 64;
            let lane = raw % 64;
            let v5_row = (group / 256) * 16 + (lane / 16) * 4 + component;
            let v5_column = (group % 256) * 16 + lane % 16;
            assert_eq!((row, column), (v5_row, v5_column));
        }
    }
    assert!(seen.into_iter().all(|written| written));
}

#[test]
fn output_boundaries_and_rejected_coordinates_are_exact() {
    assert_eq!(output_coordinate(0, 0), Some((0, 0)));
    assert_eq!(output_coordinate(63, 3), Some((15, 15)));
    assert_eq!(output_coordinate(256 * 64, 0), Some((16, 0)));
    assert_eq!(output_coordinate(GROUPS * LANES - 1, 3), Some((31, 4095)));
    for (raw, component) in [
        (GROUPS * LANES, 0),
        (usize::MAX, 0),
        (0, 4),
        (0, usize::MAX),
    ] {
        assert_eq!(output_coordinate(raw, component), None);
    }
}

#[test]
fn paired_updates_visit_all_768_k16_steps_in_control_order() {
    let control = (0..768).map(|step| step * 16).collect::<Vec<_>>();
    let paired = (0..384)
        .flat_map(|pair| [pair * 32, pair * 32 + 16])
        .collect::<Vec<_>>();
    assert_eq!(control, paired);
    assert_eq!(paired.first(), Some(&0));
    assert_eq!(paired.last(), Some(&(K - 16)));
}

#[test]
fn fragment_addresses_cover_exact_boundary_tiles_without_overread() {
    for pair in [0, 383] {
        for half in 0..2 {
            let k_base = pair * 32 + half * 16;
            for group in [0, 255, 256, 511] {
                let mut a_seen = [false; 256];
                let mut b_seen = [false; 256];
                for lane in 0..LANES {
                    for element in 0..4 {
                        let reduction = (lane / 16) * 4 + element;
                        let row = lane % 16;
                        let column = lane % 16;
                        let a_index = row * 16 + reduction;
                        let b_index = reduction * 16 + column;
                        assert!(!a_seen[a_index] && !b_seen[b_index]);
                        a_seen[a_index] = true;
                        b_seen[b_index] = true;
                        assert!(((group / 256) * 16 + row) * K + k_base + reduction < ROWS * K);
                        assert!((k_base + reduction) * N + (group % 256) * 16 + column < K * N);
                    }
                }
                assert!(a_seen.into_iter().all(|loaded| loaded));
                assert!(b_seen.into_iter().all(|loaded| loaded));
            }
        }
    }
}

#[test]
fn host_order_model_keeps_fp32_updates_without_intermediate_bf16_rounding() {
    // Order model only, not MFMA emulation or native numerical qualification.
    let contributions = [16777216.0_f32, 1.0, -16777216.0, 0.125, -0.25, 0.0625, 4.0];
    for offset in 0..contributions.len() {
        let contribution = |step: usize| contributions[(step + offset) % contributions.len()];
        let mut control = 0.0_f32;
        for step in 0..768 {
            control += contribution(step);
        }
        let mut paired = 0.0_f32;
        for pair in 0..384 {
            paired += contribution(pair * 2);
            paired += contribution(pair * 2 + 1);
        }
        assert_eq!(control.to_bits(), paired.to_bits());
    }
    let value = f32::from_bits(0x3f80_8000);
    assert_ne!(
        value.to_bits(),
        fe2o3_device::Bf16::from_f32(value).to_f32().to_bits()
    );
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
fn missing_duplicate_or_wrong_target_flags_are_refused() {
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
fn selection_is_default_off_with_two_explicit_roots() {
    assert_eq!(
        EXPERIMENT_ENABLED,
        cfg!(feature = "paired-prefill32-down-k16-r1")
    );
    assert_eq!(
        CONTROL_ROOT,
        "ferric_qwen3_prefill32_down_k16_control_f32_r1"
    );
    assert_eq!(PAIRED_ROOT, "ferric_qwen3_prefill32_down_k16_paired_f32_r1");
    assert!(include_str!("../Cargo.toml").contains("default = []"));
    assert!(
        include_str!("../src/lib.rs")
            .contains("#[cfg(feature = \"paired-prefill32-down-k16-r1\")]\npub mod projection;")
    );
}

#[cfg(feature = "paired-prefill32-down-k16-r1")]
#[test]
fn enabled_roster_has_two_distinct_source_bound_kernels() {
    let entries =
        ferric_qwen3_tp_prefill32_down_k2_kernels_device_v1::compiler_expectation_roster();
    assert_eq!(entries.len(), 2);
    assert_ne!(
        entries[0].kernel_binding_id(),
        entries[1].kernel_binding_id()
    );
}
