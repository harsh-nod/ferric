#[path = "../build/target_contract.rs"]
mod build_contract;

use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::{
    M1AllKernelsWorkerV3RosterV1,
    rmsnorm::{QWEN3_RMSNORM_CODE_OBJECT_VERSION_V1, QWEN3_RMSNORM_TARGET_V1},
    target::{
        QWEN3_DEVICE_CODE_OBJECT_VERSION_V1, QWEN3_DEVICE_CPU_V1, QWEN3_DEVICE_RUSTC_FEATURES_V1,
        QWEN3_DEVICE_TARGET_V1,
    },
};

#[test]
fn all_twelve_entrypoints_share_the_selected_target_contract() {
    assert_eq!(M1AllKernelsWorkerV3RosterV1::ENTRIES.len(), 12);
    assert_eq!(QWEN3_RMSNORM_TARGET_V1, QWEN3_DEVICE_TARGET_V1);
    assert_eq!(
        QWEN3_RMSNORM_CODE_OBJECT_VERSION_V1,
        QWEN3_DEVICE_CODE_OBJECT_VERSION_V1
    );
    assert_eq!(QWEN3_DEVICE_CODE_OBJECT_VERSION_V1, 6);
    assert_eq!(
        QWEN3_DEVICE_TARGET_V1,
        format!("{QWEN3_DEVICE_CPU_V1}:xnack-")
    );
    assert_eq!(
        QWEN3_DEVICE_CPU_V1,
        if cfg!(feature = "gfx950") {
            "gfx950"
        } else {
            "gfx942"
        }
    );
}

fn validate(flags: &str, cpu: &str) -> Result<(), &'static str> {
    build_contract::validate_device_build("amdgpu", flags, cpu, QWEN3_DEVICE_RUSTC_FEATURES_V1)
}

#[test]
fn both_targets_accept_only_their_own_canonical_device_invocation() {
    for cpu in ["gfx942", "gfx950"] {
        for prefix in ["-C", "-C\u{1f}", "--codegen=", "--codegen\u{1f}"] {
            let flags = format!(
                "-Zalways-encode-mir\u{1f}{prefix}target-cpu={cpu}\u{1f}{prefix}target-feature={QWEN3_DEVICE_RUSTC_FEATURES_V1}"
            );
            assert_eq!(validate(&flags, cpu), Ok(()));
            let other = if cpu == "gfx942" { "gfx950" } else { "gfx942" };
            assert!(validate(&flags, other).is_err());
        }
    }
}

#[test]
fn missing_duplicate_or_conflicting_device_settings_are_rejected() {
    let features = QWEN3_DEVICE_RUSTC_FEATURES_V1;
    for flags in [
        String::new(),
        "-Ctarget-cpu=gfx950".into(),
        format!("-Ctarget-feature={features}"),
        format!("-Ctarget-cpu=gfx950\u{1f}-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature={features}"),
        format!("-Ctarget-cpu=gfx950\u{1f}-Ctarget-cpu=gfx942\u{1f}-Ctarget-feature={features}"),
        format!(
            "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature={features}\u{1f}-Ctarget-feature=+xnack"
        ),
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=+wavefrontsize32,-wavefrontsize64,-xnack".into(),
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,+xnack".into(),
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64".into(),
    ] {
        assert!(validate(&flags, "gfx950").is_err(), "accepted {flags:?}");
    }
}

#[test]
fn host_checks_do_not_require_an_amdgpu_rustc_invocation() {
    assert_eq!(
        build_contract::validate_device_build(
            "x86_64",
            "",
            "gfx950",
            QWEN3_DEVICE_RUSTC_FEATURES_V1
        ),
        Ok(())
    );
}
