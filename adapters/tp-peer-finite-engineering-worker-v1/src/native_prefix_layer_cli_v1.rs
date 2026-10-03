//! Disposable-child one-layer route. No cached-admission or full-forward fallback.
use crate::finite_prefix_layer_wire_v1::{self as wire, Bootstrap, Budget, Command, Profile};
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::prefix_tiles_layer_v6::{ClosedRun, Owner};
use crate::native_setup::{PreparedSetup, prefix_layer_v6::PreparedPrefixLayerSetup};
use crate::resident_layer::{
    mlp_tiles_v2::Image as MlpImage,
    prefix_tiles_v6::{PrefixObservation, artifacts::Image as PrefixImage},
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
    profile: Profile,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let a = args
        .iter()
        .map(|v| {
            v.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 layer option"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if a.len() != 8
        || a[0] != "--engineering-native-prefix-layer-v1"
        || a[1] != "--allow-unauthenticated-machine-code"
        || a[2] != "--devices"
        || a[4] != "--timeout-ms"
        || a[6] != "--profile"
    {
        return Err(io::Error::other("exact layer invocation required"));
    }
    fn decimal(v: &str) -> io::Result<u64> {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
            return Err(io::Error::other("decimal layer option"));
        }
        v.parse().map_err(io::Error::other)
    }
    let ids = a[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("two devices required"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(a[5])?).map_err(io::Error::other)?;
    let profile = match a[7] {
        "baseline22-mlp548" => Profile::Baseline22Mlp548,
        "prefix284-mlp548" => Profile::Prefix284Mlp548,
        _ => return Err(io::Error::other("closed layer profile")),
    };
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10000).contains(&timeout_ms)
    {
        return Err(io::Error::other("layer devices/deadline"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
        profile,
    })
}
fn input(b: &Bootstrap) -> io::Result<ForwardInput> {
    b.input.validate()?;
    Ok(ForwardInput {
        registration: b.begin.registration.sha256,
        generation: 1,
        token: b.input.token,
        cache_metadata: b
            .input
            .cache_metadata
            .as_slice()
            .try_into()
            .map_err(io::Error::other)?,
        rotary_bits: b
            .input
            .rotary_bits
            .as_slice()
            .try_into()
            .map_err(io::Error::other)?,
    })
}
fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    budget: &mut Budget,
) -> io::Result<(Bootstrap, PreparedPrefixLayerSetup)> {
    let (b, mlp, prefix) = wire::read_bootstrap(r, budget)?;
    if b.device_ids != options.devices
        || b.timeout_ms != options.timeout_ms
        || b.profile != options.profile
        || b.begin.scope.child_identity != std::process::id()
    {
        return Err(io::Error::other("layer bootstrap/argv/process mismatch"));
    }
    let (request, payload) = wire::read_begin(r, budget, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.begin.scope)
        .map_err(io::Error::other)?;
    let mlp = MlpImage::new(mlp, &b.mlp_image).map_err(io::Error::other)?;
    let prefix = match (prefix, b.prefix_image) {
        (Some(bytes), Some(pin)) => Some(PrefixImage::new(bytes, &pin).map_err(io::Error::other)?),
        (None, None) => None,
        _ => return Err(io::Error::other("prefix image presence")),
    };
    let prepared =
        PreparedPrefixLayerSetup::new(setup, prefix, mlp, &input(&b)?, options.timeout_ms)
            .map_err(io::Error::other)?;
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other("wire/backend layer profile differs"));
    }
    Ok((b, prepared))
}
/// # Safety
/// The parent independently authenticates the original model and source roles,
/// actual images and engineering premises. Any uncertain error requires this
/// disposable child to exit and its owned parent to reap/audit; never retry Close.
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
    serve(
        &mut Native {
            owner: Some(owner),
            profile: b.sha256()?,
        },
        r,
        w,
        &b,
        &mut budget,
    )
}
trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<()>;
    fn close(&mut self) -> io::Result<ClosedRun>;
}
struct Native {
    owner: Option<Owner>,
    profile: [u8; 32],
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<()> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("layer owner consumed"))?
            .run(self.profile, input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<ClosedRun> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("layer owner consumed"))?
            .close()
            .map_err(io::Error::other)
    }
}
fn encode_closed(b: &Bootstrap, closed: ClosedRun) -> io::Result<(wire::Control, Vec<u8>)> {
    if closed.profile_sha256 != b.sha256()?
        || closed.generation != 1
        || closed.position != 0
        || closed.input_token != b.input.token
    {
        return Err(io::Error::other("actual closed layer identity"));
    }
    let prefix = match closed.layer.completion.prefix {
        PrefixObservation::Baseline22(words) if b.profile == Profile::Baseline22Mlp548 => {
            words.map(|v| v.to_vec())
        }
        PrefixObservation::Tiles284(words) if b.profile == Profile::Prefix284Mlp548 => {
            words.map(|v| v.to_vec())
        }
        _ => return Err(io::Error::other("actual prefix completion kind")),
    };
    let control = wire::Control {
        prefix,
        mlp: closed.layer.completion.mlp.map(|v| v.to_vec()),
        embedding_ns: closed.embedding_ns,
        paired_ns: closed.layer.completion.paired_ns,
    };
    control.validate(b.profile)?;
    let capture = closed.layer.capture;
    let mut bytes = Vec::with_capacity(wire::CAPTURE_BYTES);
    for rank in 0..2 {
        let rows = capture.prefix[rank]
            .iter()
            .chain([&capture.first_residual[rank]])
            .chain(capture.mlp[rank].iter())
            .chain([&capture.final_hidden[rank]]);
        for (row, (_, extent, _)) in rows.zip(wire::STAGES) {
            if row.len() != extent {
                return Err(io::Error::other("actual layer stage extent"));
            }
            bytes.extend_from_slice(row);
        }
    }
    wire::capture_rows(&bytes)?;
    Ok((control, bytes))
}
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut Budget,
) -> io::Result<()> {
    let sha = b.sha256()?;
    let mut outgoing = Budget::new();
    for id in 1..=2 {
        let request = wire::read_request(r, incoming)?;
        if request.id != id || request.profile_sha256 != sha {
            return Err(io::Error::other("layer request profile/order"));
        }
        let (control, body) = match request.command {
            Command::Run => {
                backend.run(&input(b)?)?;
                (None, Vec::new())
            }
            Command::Close => {
                let closed = backend.close()?;
                let (c, bytes) = encode_closed(b, closed)?;
                (Some(c), bytes)
            }
        };
        let response = wire::Response {
            protocol: wire::PROTOCOL,
            id,
            profile_sha256: sha,
            profile: b.profile,
            native_closed: id == 2,
            completed_layers: 1,
            control,
            capture: if id == 2 {
                Some(wire::part(&body))
            } else {
                None
            },
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        };
        wire::write_response(w, &mut outgoing, &response, &body)?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "native_prefix_layer_cli_v1_tests.rs"]
mod tests;
