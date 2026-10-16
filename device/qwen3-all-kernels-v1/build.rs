#[allow(dead_code)]
#[path = "src/target.rs"]
mod target;
#[path = "build/target_contract.rs"]
mod target_contract;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("Cargo must identify the compilation target architecture");
    let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    target_contract::validate_device_build(
        &target_arch,
        &rustflags,
        target::QWEN3_DEVICE_CPU_V1,
        target::QWEN3_DEVICE_RUSTC_FEATURES_V1,
    )
    .expect("Qwen3 aggregate compilation target does not match its source contract");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_CHECK_WRAPPER_MODE_V1");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_WRAPPER_MODE_V1");
    if std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some()
    {
        return;
    }
    // Direct host tests use a non-authoritative fallback namespace. cargo-fe2o3
    // replaces it with the compiler-derived binding for managed builds.
    println!(
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=423280f43dabf0a3eb2162b5b0779fa050960ec0b57cd7d6813ea967ee8273b4"
    );
}
