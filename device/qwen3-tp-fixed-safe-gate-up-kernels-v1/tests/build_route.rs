#[path = "../build/binding_route.rs"]
mod binding_route;

use binding_route::{BuildRoute, select_route};
use std::ffi::OsStr;

const CRATE_NAME: &str = "ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1";
const HANDOFF: &str = "/private/compiler-handoff-v2";

fn route(
    arch: &str,
    managed_wrapper: bool,
    selected_crate: Option<&str>,
    handoff_path: Option<&str>,
) -> Result<BuildRoute, &'static str> {
    select_route(
        arch,
        managed_wrapper,
        selected_crate.map(OsStr::new),
        handoff_path.map(OsStr::new),
    )
}

#[test]
fn raw_amdgpu_build_is_rejected() {
    assert!(route("amdgpu", false, None, None).is_err());
}

#[test]
fn selected_engineering_handoff_defers_binding_to_extractor() {
    assert_eq!(
        route("amdgpu", false, Some(CRATE_NAME), Some(HANDOFF)),
        Ok(BuildRoute::EngineeringHandoff)
    );
}

#[test]
fn engineering_handoff_requires_this_exact_crate() {
    for selected in [None, Some(""), Some("other_crate"), Some("build_script_build")] {
        assert!(route("amdgpu", false, selected, Some(HANDOFF)).is_err());
    }
}

#[test]
fn engineering_handoff_requires_nonempty_absolute_output() {
    for output in [None, Some(""), Some("compiler-handoff-v2")] {
        assert!(route("amdgpu", false, Some(CRATE_NAME), output).is_err());
    }
}

#[test]
fn existing_managed_wrapper_route_is_preserved() {
    for arch in ["amdgpu", "x86_64"] {
        assert_eq!(
            route(arch, true, None, None),
            Ok(BuildRoute::ManagedWrapper)
        );
    }
}

#[test]
fn fixture_binding_is_only_a_host_route() {
    assert_eq!(
        route("x86_64", false, None, None),
        Ok(BuildRoute::HostFixture)
    );
    assert_eq!(
        route("x86_64", false, Some(CRATE_NAME), Some(HANDOFF)),
        Ok(BuildRoute::HostFixture)
    );
}
