//! Explicit all36-layer typed548 four-forward route, not a comparison fallback.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
use crate::finite_setup_wire_v1 as setup;
use crate::finite_tiles_decode_wire_v1::{
    self as wire, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget, InputMode,
    LayerObservation, Response,
};
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::tiles_decode_v1::{Mode, Owner, Run};
use crate::native_setup::{PreparedSetup, PreparedTilesSetup};
use crate::resident_layer::mlp_tiles_v2::Image;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
    mode: InputMode,
    admission: wire::KernelAdmission,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let a = args
        .iter()
        .map(|s| {
            s.to_str()
                .ok_or_else(|| io::Error::other("tiles non-UTF8 option"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if !matches!(a.len(), 8 | 10)
        || a[0] != "--engineering-native-tiles-decode-v1"
        || a[1] != "--allow-unauthenticated-machine-code"
        || a[2] != "--devices"
        || a[4] != "--timeout-ms"
        || a[6] != "--mode"
    {
        return Err(io::Error::other(
            "tiles exact engineering invocation required",
        ));
    }
    let admission = if a.len() == 8 {
        wire::KernelAdmission::Baseline
    } else if a[8] == "--kernel-admission" && a[9] == "cached-immutable" {
        wire::KernelAdmission::CachedImmutable
    } else {
        return Err(io::Error::other("tiles exact cache-only admission option"));
    };
    fn decimal(s: &str) -> io::Result<u64> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(io::Error::other("tiles decimal option"));
        }
        s.parse().map_err(io::Error::other)
    }
    let ids = a[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("tiles two devices"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(a[5])?).map_err(io::Error::other)?;
    let mode = match a[7] {
        "teacher-forced" => InputMode::TeacherForced,
        "autoregressive" => InputMode::Autoregressive,
        _ => return Err(io::Error::other("tiles explicit input mode")),
    };
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10000).contains(&timeout_ms)
    {
        return Err(io::Error::other("tiles device/timeout option"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
        mode,
        admission,
    })
}
fn mode(b: &Bootstrap) -> io::Result<Mode> {
    Ok(match b.mode {
        InputMode::TeacherForced => Mode::TeacherForced(
            b.input_tokens
                .as_slice()
                .try_into()
                .map_err(io::Error::other)?,
        ),
        InputMode::Autoregressive => Mode::Autoregressive {
            first: *b
                .input_tokens
                .first()
                .ok_or_else(|| io::Error::other("tiles first token"))?,
        },
    })
}
fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
) -> io::Result<(Bootstrap, PreparedTilesSetup)> {
    let (b, bytes) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("tiles bootstrap missing"))?;
    b.validate(
        options.devices,
        options.timeout_ms,
        std::process::id(),
        options.mode,
    )?;
    if b.profile.admission() != options.admission {
        return Err(io::Error::other(
            "tiles argv/bootstrap admission differs before open",
        ));
    }
    let (request, payload) =
        setup::read_request(r)?.ok_or_else(|| io::Error::other("tiles Begin missing"))?;
    let setup::Command::Begin(begin) = &request.command else {
        return Err(io::Error::other("tiles expected Begin"));
    };
    if begin.registration.sha256 != b.registration || begin.tail_image.is_none() {
        return Err(io::Error::other(
            "tiles source registration/tail differs before open",
        ));
    }
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.scope)
        .map_err(io::Error::other)?;
    let image = Image::new(bytes, &b.tiles_image).map_err(io::Error::other)?;
    let prepared = PreparedTilesSetup::new_with_admission(
        setup,
        image,
        mode(&b)?,
        options.timeout_ms,
        options.admission,
    )
    .map_err(io::Error::other)?;
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other(
            "tiles wire/backend profile encoding differs",
        ));
    }
    Ok((b, prepared))
}
/// # Safety
/// Trusted parent authenticates original model/root recipes and separately
/// reviews the supplied V2 image. Exact current process/device ownership,
/// visibility assumptions and bounded teardown remain mandatory. This explicit
/// engineering route does not grant production authority from image hashes.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (b, prepared) = prepare(&options, r, &mut incoming)?;
    // SAFETY: complete CPU scope/image/profile checks precede the unique opener.
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    let applied_admission = setup.applied_admission().map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Ok(());
    }
    let owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    serve(
        &mut Native {
            owner: Some(owner),
            profile: b.sha256()?,
            applied_admission,
        },
        r,
        w,
        &b,
        &mut incoming,
    )
}
trait Backend {
    fn applied_admission(&self) -> io::Result<Option<wire::AdmissionReceipt>> {
        Ok(None)
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run>;
    fn close(&mut self) -> io::Result<()>;
}
struct Native {
    owner: Option<Owner>,
    profile: [u8; 32],
    applied_admission: Option<wire::AdmissionReceipt>,
}
impl Backend for Native {
    fn applied_admission(&self) -> io::Result<Option<wire::AdmissionReceipt>> {
        Ok(self.applied_admission)
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("tiles owner consumed"))?
            .run(self.profile, input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("tiles owner consumed"))?
            .close()
            .map_err(io::Error::other)
    }
}
fn response(r: &wire::Request, event: Event, closed: bool) -> Response {
    Response {
        protocol: wire::PROTOCOL,
        id: r.id,
        device_ids: r.device_ids,
        session: r.session,
        registration: r.registration,
        profile_sha256: r.profile_sha256,
        event,
        native_closed: closed,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        applied_admission: None,
    }
}
fn observation(
    run: Run,
    request: &wire::Request,
    chain: &mut Chain,
) -> io::Result<(Response, Control, Vec<u8>)> {
    let c = run.completion;
    let Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err(io::Error::other("tiles observation non-forward"));
    };
    if c.profile_sha256 != request.profile_sha256
        || c.generation != *generation
        || c.position != request.id as u32 - 1
        || c.input_token != *token
        || c.output_token >= 151936
        || c.layers.len() != 36
        || run.layer_hidden.len() != 36
        || run.layer_hidden.iter().any(|r| r.len() != 8192)
        || run.final_normalized.len() != 8192
        || run.logits.len() != 303872
    {
        return Err(io::Error::other("tiles actual observation identity/extent"));
    }
    let control = Control {
        embedding_ns: c.embedding_ns,
        tail_ns: c.tail_ns,
        layers: c
            .layers
            .into_iter()
            .map(|l| LayerObservation {
                prefix_states: l.prefix_states,
                tiles_states: l.tiles_states,
                paired_ns: l.paired_ns,
            })
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| io::Error::other("tiles control layer count"))?,
    };
    control.validate()?;
    let mut bytes = Vec::with_capacity(OBSERVATION_BYTES);
    for row in run.layer_hidden {
        bytes.extend_from_slice(&row);
    }
    bytes.extend_from_slice(&run.final_normalized);
    bytes.extend_from_slice(&run.logits);
    let mut completed = Completion {
        generation: c.generation,
        position: c.position,
        input_token: c.input_token,
        output_token: c.output_token,
        control: part(&control.encode()),
        observation: part(&bytes),
        capture: Payload::from_bytes(&bytes)?,
        chain: [0; 32],
    };
    completed.chain = chain.advance(&completed);
    Ok((
        response(request, Event::Completed(completed), false),
        control,
        bytes,
    ))
}
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut FrameBudget,
) -> io::Result<()> {
    let applied_admission = backend.applied_admission()?;
    if applied_admission != b.profile.admission().receipt() {
        return Err(io::Error::other(
            "tiles requested/applied admission differs",
        ));
    }
    let profile = b.sha256()?;
    let mut outgoing = FrameBudget::new();
    let mut chain = Chain::new(b.registration, profile);
    let mut previous = None;
    for id in 1..=5 {
        let request = wire::read_request(r, incoming)?
            .ok_or_else(|| io::Error::other("tiles EOF before Close"))?;
        if request.id != id
            || request.device_ids != b.device_ids
            || request.session != b.scope.session
            || request.registration != b.registration
            || request.profile_sha256 != profile
        {
            return Err(io::Error::other("tiles request profile/session/order"));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                if id > 4 || *token != b.input(id as u32 - 1, previous)? {
                    return Err(io::Error::other("tiles request mode trajectory"));
                }
                let input = ForwardInput {
                    registration: b.registration,
                    generation: *generation,
                    token: *token,
                    cache_metadata: cache_metadata
                        .as_slice()
                        .try_into()
                        .map_err(io::Error::other)?,
                    rotary_bits: rotary_bits
                        .as_slice()
                        .try_into()
                        .map_err(io::Error::other)?,
                };
                let run = backend.run(&input)?;
                let output = run.completion.output_token;
                let (mut reply, control, bytes) = observation(run, &request, &mut chain)?;
                reply.applied_admission = applied_admission;
                wire::write_response(w, &mut outgoing, &reply, Some(&control), &bytes)?;
                previous = Some(output);
            }
            Command::Close => {
                if id != 5 {
                    return Err(io::Error::other("tiles early Close"));
                }
                backend.close()?;
                let mut reply = response(
                    &request,
                    Event::Closed {
                        completed_forwards: 4,
                        transcript_sha256: chain.digest(),
                    },
                    true,
                );
                reply.applied_admission = applied_admission;
                wire::write_response(w, &mut outgoing, &reply, None, &[])?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("tiles missing Close"))
}
#[cfg(test)]
#[path = "native_tiles_decode_cli_v1_tests.rs"]
mod tests;
