#[allow(dead_code, unexpected_cfgs)]
#[path = "../qwen3-all-kernels-v1/src/target.rs"]
mod target;
#[path = "../qwen3-all-kernels-v1/build/target_contract.rs"]
mod target_contract;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"gfx942\"))");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo target architecture");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    target_contract::validate_device_build(
        &arch,
        &flags,
        target::QWEN3_DEVICE_CPU_V1,
        target::QWEN3_DEVICE_RUSTC_FEATURES_V1,
    )
    .expect("Qwen3 Draft06B target differs from its source contract");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_CHECK_WRAPPER_MODE_V1");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_WRAPPER_MODE_V1");
    if std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some()
    {
        return;
    }
    // Host-test fixture only; managed device builds supply their measured binding.
    println!(
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=a3b50886acc34cd4e46aaa6d3c5f27f998d864ad718c57d824fd130e0b631b29"
    );
}
