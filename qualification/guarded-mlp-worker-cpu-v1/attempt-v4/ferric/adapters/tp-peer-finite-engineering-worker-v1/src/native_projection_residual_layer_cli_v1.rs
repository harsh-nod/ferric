//! Separate one-layer projection-materialization diagnostic; no default fallback.
use crate::finite_projection_residual_layer_wire_v1::{self as wire, Bootstrap, Budget};
use crate::native_prefix_layer_cli_v1 as original;
use crate::native_setup::{PreparedSetup, prefix_layer_v6::PreparedPrefixLayerSetup};
use crate::resident_layer::{
    mlp_tiles_v2::Image as MlpImage,
    prefix_tiles_v6::{artifacts::Image as PrefixImage, projection_residual::Image},
};
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
};

#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let a = args
        .iter()
        .map(|v| {
            v.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 projection option"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if a.len() != 6
        || a[0] != "--engineering-native-projection-residual-layer-v1"
        || a[1] != "--allow-unauthenticated-machine-code"
        || a[2] != "--devices"
        || a[4] != "--timeout-ms"
    {
        return Err(io::Error::other(
            "exact projection layer invocation required",
        ));
    }
    fn decimal(v: &str) -> io::Result<u64> {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
            return Err(io::Error::other("decimal projection option"));
        }
        v.parse().map_err(io::Error::other)
    }
    let ids = a[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("two projection devices required"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(a[5])?).map_err(io::Error::other)?;
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10000).contains(&timeout_ms)
    {
        return Err(io::Error::other("projection devices/deadline"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
    })
}
fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    budget: &mut Budget,
) -> io::Result<(Bootstrap, PreparedPrefixLayerSetup)> {
    let (b, mlp, prefix, projection) = wire::read_bootstrap(r, budget)?;
    if b.layer.device_ids != options.devices
        || b.layer.timeout_ms != options.timeout_ms
        || b.layer.begin.scope.child_identity != std::process::id()
    {
        return Err(io::Error::other(
            "projection bootstrap/argv/process mismatch",
        ));
    }
    let (request, payload) = wire::read_begin(r, budget, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.layer.begin.scope)
        .map_err(io::Error::other)?;
    let mlp = MlpImage::new(mlp, &b.layer.mlp_image).map_err(io::Error::other)?;
    let prefix = PrefixImage::new(
        prefix,
        &b.layer
            .prefix_image
            .ok_or_else(|| io::Error::other("prefix image missing"))?,
    )
    .map_err(io::Error::other)?;
    let projection =
        Image::new(projection, &b.projection_residual_image).map_err(io::Error::other)?;
    let prepared = PreparedPrefixLayerSetup::new_projection(
        setup,
        prefix,
        mlp,
        projection,
        &original::input(&b.layer)?,
        options.timeout_ms,
    )
    .map_err(io::Error::other)?;
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other("projection wire/backend profile differs"));
    }
    Ok((b, prepared))
}
/// # Safety
/// A trusted parent independently reviews this candidate image for both O and
/// Down residuals, all unchanged source/model roles and the owned native lifetime.
/// Every uncertain failure requires disposable-child exit and parent reap/audit.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    let mut budget = Budget::new();
    let (b, prepared) = prepare(&options, r, &mut budget)?;
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Ok(());
    }
    let owner = unsafe { setup.into_layer() }.map_err(io::Error::other)?;
    original::serve_projection_owner(owner, r, w, &b, &mut budget)
}

#[cfg(test)]
#[path = "native_projection_residual_layer_cli_v1_tests.rs"]
mod tests;
