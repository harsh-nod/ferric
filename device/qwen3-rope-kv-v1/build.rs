fn main() {
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
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1={}",
        "7f875e4529cfd9ff2abce28d185dcdc7ccb0565194c993978dca2c8544ea82b4"
    );
}
