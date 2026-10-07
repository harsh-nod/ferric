#[path = "build/binding_route.rs"]
mod binding_route;
#[path = "build/target_contract.rs"]
mod target_contract;

fn main() {
    for key in [
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_ENCODED_RUSTFLAGS",
        "FE2O3_BINDING_CHECK_WRAPPER_MODE_V1",
        "FE2O3_BINDING_WRAPPER_MODE_V1",
        "FE2O3_EXTRACT_CRATE_V1",
        "FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("Cargo target architecture");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    target_contract::validate_device_build(
        &arch,
        &flags,
        "gfx950",
        "-wavefrontsize32,+wavefrontsize64,-xnack",
    )
    .expect("fixed gate/up requires exact gfx950 Wave64, xnack-disabled flags");
    let managed_wrapper = std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some();
    let selected_crate = std::env::var_os("FE2O3_EXTRACT_CRATE_V1");
    let handoff_path = std::env::var_os("FE2O3_EXTRACT_AMDGPU_COMPILER_HANDOFF_PATH_V1");
    let route = binding_route::select_route(
        &arch,
        managed_wrapper,
        selected_crate.as_deref(),
        handoff_path.as_deref(),
    )
    .expect("fixed gate/up build route");
    if route != binding_route::BuildRoute::HostFixture {
        return;
    }
    // Host fixture only. Selected compiler routes install their own binding.
    println!("cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1");
}
