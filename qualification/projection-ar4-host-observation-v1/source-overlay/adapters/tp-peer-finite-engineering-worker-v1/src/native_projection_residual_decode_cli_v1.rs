//! Explicit plain TF4/AR4 projection route; no old/default/clock fallback.
use crate::finite_projection_residual_decode_wire_v1::{
    self as wire, Bootstrap, FrameBudget, InputMode,
};
use crate::native_catalog::forward::prefix_tiles_decode_v6::Mode;
use crate::native_prefix_decode_cli_v1 as ordinary;
use crate::native_setup::{PreparedSetup, prefix_decode_v6::PreparedPrefixDecodeSetup};
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
    pub(crate) mode: InputMode,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let a = args
        .iter()
        .map(|s| {
            s.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 projection option"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if a.len() != 8
        || a[0] != "--engineering-native-projection-residual-decode-v1"
        || a[1] != "--allow-unauthenticated-machine-code"
        || a[2] != "--devices"
        || a[4] != "--timeout-ms"
        || a[6] != "--mode"
    {
        return Err(io::Error::other("exact plain projection decode invocation"));
    }
    let mode = match a[7] {
        "teacher-forced" => InputMode::TeacherForced,
        "autoregressive" => InputMode::Autoregressive,
        _ => return Err(io::Error::other("explicit projection decode mode")),
    };
    fn decimal(s: &str) -> io::Result<u64> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(io::Error::other("decimal projection option"));
        }
        s.parse().map_err(io::Error::other)
    }
    let ids = a[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("two projection devices"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(a[5])?).map_err(io::Error::other)?;
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10000).contains(&timeout_ms)
    {
        return Err(io::Error::other("projection decode device/deadline"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
        mode,
    })
}
fn admitted_mode(options: &NativeOptions, b: &Bootstrap) -> io::Result<Mode> {
    b.validate(
        options.devices,
        options.timeout_ms,
        std::process::id(),
        options.mode,
    )?;
    Ok(match options.mode {
        InputMode::TeacherForced => Mode::TeacherForced(
            b.decode
                .input_tokens
                .as_slice()
                .try_into()
                .map_err(io::Error::other)?,
        ),
        InputMode::Autoregressive => Mode::Autoregressive {
            first: b.decode.input_tokens[0],
        },
    })
}
pub(crate) fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
) -> io::Result<(Bootstrap, PreparedPrefixDecodeSetup)> {
    let (b, mlp, prefix, projection) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("projection bootstrap absent"))?;
    let mode = admitted_mode(options, &b)?;
    let (request, payload) = wire::read_begin(r, incoming, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.decode.scope)
        .map_err(io::Error::other)?;
    let prepared = PreparedPrefixDecodeSetup::new_projection(
        setup,
        PrefixImage::new(prefix, &b.decode.prefix_image).map_err(io::Error::other)?,
        MlpImage::new(mlp, &b.decode.tiles_image).map_err(io::Error::other)?,
        Image::new(projection, &b.projection_residual_image).map_err(io::Error::other)?,
        mode,
        options.timeout_ms,
    )
    .map_err(io::Error::other)?;
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other(
            "projection decode wire/backend profile mismatch",
        ));
    }
    Ok((b, prepared))
}
/// # Safety
/// A trusted parent authenticates all actual source/model/images and reviews the
/// candidate for both residuals of every layer. Owned child exit/reap and audits
/// remain mandatory; an image digest is not arithmetic or runtime proof.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (b, prepared) = prepare(&options, r, &mut incoming)?;
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Err(io::Error::other(
            "projection setup closed before four forwards",
        ));
    }
    let owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    ordinary::serve_projection_owner(owner, r, w, &b, &mut incoming)
}

#[cfg(test)]
#[path = "native_projection_residual_decode_cli_v1_tests.rs"]
mod tests;
