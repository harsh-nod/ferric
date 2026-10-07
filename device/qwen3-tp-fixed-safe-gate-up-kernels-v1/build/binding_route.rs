use std::ffi::OsStr;
use std::path::Path;

const CRATE_NAME: &str = "ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1";

#[derive(Debug, Eq, PartialEq)]
pub enum BuildRoute {
    ManagedWrapper,
    EngineeringHandoff,
    HostFixture,
}

// Routing only, not authentication. The selected extractor installs the binding
// before macro expansion; engineering output grants no production authority.
pub fn select_route(
    arch: &str,
    managed_wrapper: bool,
    selected_crate: Option<&OsStr>,
    handoff_path: Option<&OsStr>,
) -> Result<BuildRoute, &'static str> {
    if managed_wrapper {
        return Ok(BuildRoute::ManagedWrapper);
    }
    if arch != "amdgpu" {
        return Ok(BuildRoute::HostFixture);
    }
    if selected_crate == Some(OsStr::new(CRATE_NAME))
        && handoff_path.is_some_and(|path| !path.is_empty() && Path::new(path).is_absolute())
    {
        return Ok(BuildRoute::EngineeringHandoff);
    }
    Err("device emission requires managed source binding or a selected engineering handoff")
}
