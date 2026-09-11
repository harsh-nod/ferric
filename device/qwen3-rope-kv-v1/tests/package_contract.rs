const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCK: &str = include_str!("../Cargo.lock");
const TOOLCHAIN: &str = include_str!("../rust-toolchain.toml");
const README: &str = include_str!("../README.md");

const FE2O3_REVISION: &str = "4fbc0a34c7938474406d37a45e7972ea8b1ec277";
const FE2O3_LOCK_SOURCE: &str = "git+https://github.com/harsh-nod/fe2o3.git?rev=4fbc0a34c7938474406d37a45e7972ea8b1ec277#4fbc0a34c7938474406d37a45e7972ea8b1ec277";

#[test]
fn manifest_and_complete_lock_closure_pin_exact_reviewed_fe2o3_source() {
    assert_eq!(MANIFEST.matches(FE2O3_REVISION).count(), 2);
    assert!(MANIFEST.contains("fe2o3-device = { git ="));
    assert!(MANIFEST.contains("fe2o3-host = { git ="));
    assert!(!MANIFEST.contains("fe2o3-device = { path ="));
    assert!(!MANIFEST.contains("fe2o3-host = { path ="));
    assert!(!MANIFEST.contains("branch ="));

    let fe2o3_sources = LOCK
        .lines()
        .filter(|line| line.contains("git+https://github.com/harsh-nod/fe2o3.git"))
        .collect::<Vec<_>>();
    let expected_source = format!("source = \"{FE2O3_LOCK_SOURCE}\"");
    assert!(fe2o3_sources.len() > 20);
    assert!(fe2o3_sources
        .iter()
        .all(|line| line.trim() == expected_source));
    assert_eq!(
        LOCK.matches("?rev=4fbc0a34c7938474406d37a45e7972ea8b1ec277#")
            .count(),
        fe2o3_sources.len()
    );
}

#[test]
fn package_retains_the_reviewed_nightly_and_truthful_nonclaims() {
    assert!(TOOLCHAIN.contains("nightly-2026-04-03"));
    assert!(TOOLCHAIN.contains("rustc-dev"));
    assert!(README.contains("revision `2d275684d7a2`"));
    assert!(README.contains("initial work-proportional schedule"));
    assert!(README.contains("unique grid leader"));
    assert!(README.contains("other 63 lanes"));
    assert!(README.contains("does not claim"));
    assert!(README.contains("hardware measurement"));
    assert!(README.contains("both fixed 512 MiB cache"));
    assert!(README.contains("buffers in each direction"));
    assert!(README.contains("profile binds the same single-workgroup geometry"));
    assert!(!README.contains("one Wave64 workgroup for each"));
    assert!(README.contains("not establish extracted host-plan binding"));
}
