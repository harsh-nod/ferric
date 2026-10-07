#[path = "build/target_contract.rs"]
mod target_contract;

fn main() {
    for key in [
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_ENCODED_RUSTFLAGS",
        "FE2O3_BINDING_CHECK_WRAPPER_MODE_V1",
        "FE2O3_BINDING_WRAPPER_MODE_V1",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo target architecture");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    target_contract::validate_device_build(&arch, &flags)
        .expect("fused gate/up requires exact gfx950 Wave64, xnack-disabled flags");
    if std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some()
    {
        return;
    }
    assert_ne!(
        arch, "amdgpu",
        "device emission requires managed source binding"
    );
    // Only a host fixture binding; managed emission supplies the actual identity.
    println!(
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1"
    );
}
