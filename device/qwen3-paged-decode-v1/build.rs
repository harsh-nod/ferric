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
        "2324f26234c79bdb101f167281d2867442b5178292a82984047a5e65a85375f5"
    );
}
