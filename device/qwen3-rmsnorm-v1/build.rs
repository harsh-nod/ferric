fn main() {
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_CHECK_WRAPPER_MODE_V1");
    println!("cargo:rerun-if-env-changed=FE2O3_BINDING_WRAPPER_MODE_V1");
    if std::env::var_os("FE2O3_BINDING_CHECK_WRAPPER_MODE_V1").is_some()
        || std::env::var_os("FE2O3_BINDING_WRAPPER_MODE_V1").is_some()
    {
        return;
    }
    println!(
        "cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1={}",
        "23628d5cdb792f8ce6cfe60158728174c6fc655264d23ea3aaa7eee1485f6006"
    );
}
