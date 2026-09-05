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
        "5caff9ab773627b8503aff12d2d2a2a984db67668d8c4add7be0b6d55fc252d2"
    );
}
