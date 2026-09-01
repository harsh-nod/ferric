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
        "4f200831b9d9a21dc910b5dc09ccbc0556e4a695ced6a5c75b24eb55fcde4605"
    );
}
