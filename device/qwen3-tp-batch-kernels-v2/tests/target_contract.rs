#[path = "../../qwen3-all-kernels-v1/build/target_contract.rs"]
mod build_contract;

use ferric_qwen3_tp_batch_kernels_device_v2::{compiler_expectation_roster_v2, rmsnorm, target};

#[test]
fn all_nine_roots_share_the_exact_target_contract() {
    assert_eq!(compiler_expectation_roster_v2().len(), 9);
    assert_eq!(target::QWEN3_DEVICE_CODE_OBJECT_VERSION_V1, 6);
    assert_eq!(
        rmsnorm::QWEN3_RMSNORM_TARGET_V1,
        target::QWEN3_DEVICE_TARGET_V1
    );
    assert_eq!(
        target::QWEN3_DEVICE_CPU_V1,
        if cfg!(feature = "gfx950") {
            "gfx950"
        } else {
            "gfx942"
        }
    );
    assert_eq!(
        target::QWEN3_DEVICE_TARGET_V1,
        format!("{}:xnack-", target::QWEN3_DEVICE_CPU_V1)
    );
}

#[test]
fn source_target_and_codegen_settings_must_agree() {
    for cpu in ["gfx942", "gfx950"] {
        let flags = format!(
            "-Ctarget-cpu={cpu}\u{1f}-Ctarget-feature={}",
            target::QWEN3_DEVICE_RUSTC_FEATURES_V1
        );
        assert_eq!(
            build_contract::validate_device_build(
                "amdgpu",
                &flags,
                cpu,
                target::QWEN3_DEVICE_RUSTC_FEATURES_V1
            ),
            Ok(())
        );
        let other = if cpu == "gfx942" { "gfx950" } else { "gfx942" };
        assert!(
            build_contract::validate_device_build(
                "amdgpu",
                &flags,
                other,
                target::QWEN3_DEVICE_RUSTC_FEATURES_V1
            )
            .is_err()
        );
    }
    assert!(
        build_contract::validate_device_build(
            "amdgpu",
            "",
            "gfx950",
            target::QWEN3_DEVICE_RUSTC_FEATURES_V1
        )
        .is_err()
    );
}
