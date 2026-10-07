#[path = "build/target_contract.rs"]
mod target_contract;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo target architecture");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    target_contract::validate_device_build(
        &arch,
        &flags,
        "gfx950",
        "-wavefrontsize32,+wavefrontsize64,-xnack",
    )
    .expect("Prefill32 down requires exact gfx950 Wave64 xnack-disabled emission");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_CHECK_WRAPPER_MODE_V1");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_WRAPPER_MODE_V1");
    if std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some()
    {
        return;
    }
    // Host fixture only; managed emission supplies the actual source binding.
    println!(
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=3232323232323232323232323232323232323232323232323232323232323232"
    );
}
