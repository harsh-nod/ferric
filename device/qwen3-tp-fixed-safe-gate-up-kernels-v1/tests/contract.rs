#[path = "../build/target_contract.rs"]
mod target_contract;

const CPU: &str = "gfx950";
const FEATURES: &str = "-wavefrontsize32,+wavefrontsize64,-xnack";

fn validate(flags: &str) -> Result<(), &'static str> {
    target_contract::validate_device_build("amdgpu", flags, CPU, FEATURES)
}

#[test]
fn device_flags_require_one_exact_cpu_and_wave64_feature_set() {
    for flags in [
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack",
        "-C\u{1f}target-cpu=gfx950\u{1f}--codegen=target-feature=-wavefrontsize32,+wavefrontsize64,-xnack",
    ] {
        assert_eq!(validate(flags), Ok(()));
    }
    for flags in [
        "",
        "-Ctarget-cpu=gfx942\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack",
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=+wavefrontsize32,-wavefrontsize64,-xnack",
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,+xnack",
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack",
        "-Ctarget-cpu=gfx950\u{1f}-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack\u{1f}-Ctarget-feature=-xnack",
    ] {
        assert!(validate(flags).is_err(), "accepted {flags:?}");
    }
    assert_eq!(
        target_contract::validate_device_build("x86_64", "", CPU, FEATURES),
        Ok(())
    );
}

#[test]
fn package_is_default_off_and_pins_both_sdk_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("default = []"));
    assert!(manifest.contains("fixed-safe-gate-up-r1 = [\"gfx950\", \"dep:fe2o3-host\"]"));
    assert_eq!(
        manifest.matches("rev = \"1a5999f6e1c5f2363bc2d525af65e84c46502ce6\"").count(),
        2
    );
    assert!(manifest.contains("optional = true"));
    let library = include_str!("../src/lib.rs");
    for module in ["activation_pack", "projection"] {
        assert!(library.contains(&format!(
            "#[cfg(feature = \"fixed-safe-gate-up-r1\")]\npub mod {module};"
        )));
    }
    assert!(library.contains("#[cfg(all(target_arch = \"amdgpu\", not(feature = \"fixed-safe-gate-up-r1\")))]"));
    assert!(library.contains("#[cfg(all(feature = \"fixed-safe-gate-up-r1\", not(target_arch = \"amdgpu\"), not(test)))]"));
}

#[test]
fn device_rejects_unmanaged_binding_before_host_fixture_identity() {
    let build = include_str!("../build.rs");
    let rejection = build.find("binding_route::select_route(").unwrap();
    let fixture = build.find("cargo:rustc-env=FE2O3_CRATE_BINDING_ID_V1=").unwrap();
    assert!(rejection < fixture);
}

#[test]
fn all_fixed_admission_checks_precede_first_device_read() {
    let source = include_str!("../src/projection.rs");
    let first_read = source.find("let left_bits = left_view.load_or(0, packed_inner, 0);").unwrap();
    for admission in [
        "rows != 1",
        "n != 12_288",
        "k != 4096",
        "world_size != 1",
        "!(projection == 4 || projection == 5)",
        "a.len() < 2048",
        "a.len() > 32 * 2048",
        "weights.len() != 12_288 * 2048",
        "output.len() < 12_288",
        "output.len() > 32 * 12_288",
        "thread::launch_extent_1d() != 12_288 * 64",
        "if column >= 12_288",
    ] {
        assert!(source.find(admission).unwrap() < first_read, "{admission}");
    }
    assert!(source.contains("let lane = invocation.get() % 64;"));
    assert!(source.contains("StridedReadView2D::from_shared_slice(a, 0, 1, 2048, 2048)"));
    assert!(source.contains("StridedReadView2D::from_shared_slice(weights, 0, 12_288, 2048, 2048)"));
    assert!(source.contains("while group < 32 {"));
    assert!(source.contains("let packed_inner = group * 64 + lane;"));
}

#[test]
fn device_uses_sdk_bf16_not_the_dependency_free_host_model() {
    for source in [
        include_str!("../src/projection.rs"),
        include_str!("../src/activation_pack.rs"),
    ] {
        assert!(source.contains("use fe2o3_device::{"));
        assert!(!source.contains("reference_bf16"));
        assert!(!source.contains("get_unchecked"));
        assert!(!source.contains("unsafe"));
    }
}
