//! Explicit all36-layer Prefix284 + MLP548 four-forward route, with no fallback.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
use crate::finite_prefix_decode_wire_v1::{
    self as wire, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget, InputMode,
    LayerObservation, Response,
};
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::prefix_tiles_decode_v6::{Mode, Owner, Run};
use crate::native_setup::{PreparedSetup, prefix_decode_v6::PreparedPrefixDecodeSetup};
use crate::resident_layer::mlp_tiles_v2::Image;
use crate::resident_layer::prefix_tiles_v6::artifacts::Image as PrefixImage;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
    mode: InputMode,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let a = args
        .iter()
        .map(|s| {
            s.to_str()
                .ok_or_else(|| io::Error::other("tiles non-UTF8 option"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if a.len() != 8
        || a[0] != "--engineering-native-prefix-decode-v1"
        || a[1] != "--allow-unauthenticated-machine-code"
        || a[2] != "--devices"
        || a[4] != "--timeout-ms"
        || a[6] != "--mode"
    {
        return Err(io::Error::other(
            "tiles exact engineering invocation required",
        ));
    }
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
) -> io::Result<(Bootstrap, PreparedPrefixDecodeSetup)> {
    let (b, bytes, prefix) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("tiles bootstrap missing"))?;
    b.validate(
        options.devices,
        options.timeout_ms,
        std::process::id(),
        options.mode,
    )?;
    let (request, payload) = wire::read_begin(r, incoming, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.scope)
        .map_err(io::Error::other)?;
    let image = Image::new(bytes, &b.tiles_image).map_err(io::Error::other)?;
    let prefix = PrefixImage::new(prefix, &b.prefix_image).map_err(io::Error::other)?;
    let prepared =
        PreparedPrefixDecodeSetup::new(setup, prefix, image, mode(&b)?, options.timeout_ms)
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
    unsafe { run_observed(options, r, w, None) }
}
#[allow(unsafe_code)]
pub(crate) unsafe fn run_observed(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: Option<(std::path::PathBuf, [u8; 32])>,
) -> io::Result<()> {
    unsafe {
        run_with_observer(
            options,
            r,
            w,
            diagnostic.map(|(path, worker)| Diagnostic::V1(path, worker)),
        )
    }
}
#[allow(unsafe_code)]
pub(crate) unsafe fn run_policy_observed(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    path: std::path::PathBuf,
    worker: [u8; 32],
    policy: crate::prefix_decode_host_observation_v2::Policy,
) -> io::Result<()> {
    unsafe { run_with_observer(options, r, w, Some(Diagnostic::V2(path, worker, policy))) }
}
#[allow(unsafe_code)]
pub(crate) unsafe fn run_device_observed(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    path: std::path::PathBuf,
    worker: [u8; 32],
) -> io::Result<()> {
    unsafe { run_with_observer(options, r, w, Some(Diagnostic::Device(path, worker))) }
}
#[allow(unsafe_code)]
pub(crate) unsafe fn run_device_clock_observed(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    path: std::path::PathBuf,
    worker: [u8; 32],
) -> io::Result<()> {
    unsafe { run_with_observer(options, r, w, Some(Diagnostic::DeviceClocks(path, worker))) }
}
enum Diagnostic {
    V1(std::path::PathBuf, [u8; 32]),
    V2(
        std::path::PathBuf,
        [u8; 32],
        crate::prefix_decode_host_observation_v2::Policy,
    ),
    Device(std::path::PathBuf, [u8; 32]),
    DeviceClocks(std::path::PathBuf, [u8; 32]),
}
enum Observer {
    V1(crate::native_prefix_decode_host_v1::Recorder),
    V2(crate::native_prefix_decode_host_v2::Recorder),
}
impl Observer {
    fn record(&mut self, raw: fe2o3_kfd::Gfx950EngineeringPeerHostObservationV1) -> io::Result<()> {
        match self {
            Self::V1(v) => v.record(raw),
            Self::V2(v) => v.record(raw),
        }
    }
    fn forward(
        &mut self,
        raw: fe2o3_kfd::Gfx950EngineeringPeerHostObservationV1,
        elapsed: std::time::Duration,
    ) -> io::Result<()> {
        match self {
            Self::V1(v) => v.forward(raw, elapsed),
            Self::V2(v) => v.forward(raw, elapsed),
        }
    }
    fn completed(&mut self, value: &Completion) -> io::Result<()> {
        match self {
            Self::V1(v) => v.completed(value),
            Self::V2(v) => v.completed(value),
        }
    }
    fn closed(&mut self, elapsed: std::time::Duration) -> io::Result<()> {
        match self {
            Self::V1(v) => v.closed(elapsed),
            Self::V2(v) => v.closed(elapsed),
        }
    }
    fn finish(self, path: &std::path::Path) -> io::Result<()> {
        match self {
            Self::V1(v) => v.finish(path),
            Self::V2(v) => v.finish(path),
        }
    }
}
#[allow(unsafe_code)]
unsafe fn run_with_observer(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: Option<Diagnostic>,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (b, prepared) = prepare(&options, r, &mut incoming)?;
    // SAFETY: complete CPU scope/image/profile checks precede the unique opener.
    let mut group = if matches!(
        &diagnostic,
        Some(Diagnostic::Device(..) | Diagnostic::DeviceClocks(..))
    ) {
        unsafe { Group::open_raw_timestamps_unchecked(&prepared.device_ids()) }
    } else {
        unsafe { Group::open_unchecked(&prepared.device_ids()) }
    }
    .map_err(io::Error::other)?;
    let mut observer = match &diagnostic {
        Some(Diagnostic::V1(_, worker)) => Some(Observer::V1(
            crate::native_prefix_decode_host_v1::Recorder::enable(&mut group, &b, *worker)?,
        )),
        Some(Diagnostic::V2(_, worker, policy)) => Some(Observer::V2(
            crate::native_prefix_decode_host_v2::Recorder::enable(
                &mut group, &b, *worker, *policy,
            )?,
        )),
        Some(Diagnostic::Device(..) | Diagnostic::DeviceClocks(..)) | None => None,
    };
    let mut device = match &diagnostic {
        Some(Diagnostic::Device(_, worker)) => Some(
            crate::native_prefix_device_recorder_v1::Recorder::enable(&mut group, &b, *worker)?,
        ),
        _ => None,
    };
    let mut clocks = match &diagnostic {
        Some(Diagnostic::DeviceClocks(_, worker)) => Some(
            crate::native_prefix_device_recorder_v1::clocks::Recorder::enable(
                &mut group, &b, *worker,
            )?,
        ),
        _ => None,
    };
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        if observer.is_some() || device.is_some() || clocks.is_some() {
            return Err(io::Error::other(
                "host diagnostic setup closed before four forwards",
            ));
        }
        return Ok(());
    }
    let mut owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    if let Some(observer) = observer.as_mut() {
        observer.record(owner.host_observation().map_err(io::Error::other)?)?;
    }
    if let Some(device) = device.as_mut() {
        owner.bind_device_images(device).map_err(io::Error::other)?;
    }
    if let Some(clocks) = clocks.as_mut() {
        clocks.bind_images(&mut owner)?;
    }
    let mut backend = Native {
        owner: Some(owner),
        profile: b.sha256()?,
        observer,
        device,
        closed_device: None,
        clocks,
        closed_clocks: None,
    };
    serve_and_finish(
        &mut backend,
        r,
        w,
        &b,
        &mut incoming,
        |backend| match diagnostic {
            Some(Diagnostic::DeviceClocks(path, _)) => {
                crate::native_prefix_decode_device_clock_v2::publish(
                    backend
                        .closed_clocks
                        .take()
                        .ok_or_else(|| io::Error::other("closed clock report missing"))?,
                    &path,
                )
            }
            Some(Diagnostic::Device(path, _)) => crate::native_prefix_decode_device_v1::publish(
                backend
                    .closed_device
                    .take()
                    .ok_or_else(|| io::Error::other("closed device report missing"))?,
                &path,
            ),
            Some(Diagnostic::V1(path, _) | Diagnostic::V2(path, _, _)) => backend
                .observer
                .take()
                .ok_or_else(|| io::Error::other("host observer missing"))?
                .finish(&path),
            None => Ok(()),
        },
    )
}
trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run>;
    fn close(&mut self) -> io::Result<()>;
    fn control(&mut self, _control: &Control) -> io::Result<()> {
        Ok(())
    }
    fn completed(&mut self, _completion: &Completion) -> io::Result<()> {
        Ok(())
    }
    fn failed(&mut self) {}
}
struct Native {
    owner: Option<Owner>,
    profile: [u8; 32],
    observer: Option<Observer>,
    device: Option<crate::native_prefix_device_recorder_v1::Recorder>,
    closed_device: Option<crate::native_prefix_device_recorder_v1::ClosedReport>,
    clocks: Option<crate::native_prefix_device_recorder_v1::clocks::Recorder>,
    closed_clocks: Option<crate::native_prefix_device_recorder_v1::clocks::ClosedReport>,
}
impl Backend for Native {
    fn failed(&mut self) {
        if let Some(clocks) = self.clocks.as_mut() {
            clocks.poison();
            if let Some(owner) = self.owner.as_mut() {
                owner.poison_device_recording();
            }
        }
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        let started = self.observer.as_ref().map(|_| std::time::Instant::now());
        let owner = self
            .owner
            .as_mut()
            .ok_or_else(|| io::Error::other("tiles owner consumed"))?;
        if let Some(clocks) = self.clocks.as_mut() {
            return clocks.run(owner, self.profile, input);
        }
        let run = match self.device.as_mut() {
            Some(device) => owner.run_recorded(self.profile, input, device),
            None => owner.run(self.profile, input),
        }
        .map_err(io::Error::other)?;
        if let Some(observer) = self.observer.as_mut() {
            let elapsed = started
                .ok_or_else(|| io::Error::other("host run timer missing"))?
                .elapsed();
            observer.forward(owner.host_observation().map_err(io::Error::other)?, elapsed)?;
        }
        Ok(run)
    }
    fn close(&mut self) -> io::Result<()> {
        let mut owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("tiles owner consumed"))?;
        if let Some(clocks) = self.clocks.take() {
            self.closed_clocks = Some(clocks.close(owner)?);
            return Ok(());
        }
        if let Some(device) = self.device.take() {
            self.closed_device = Some(device.close(owner)?);
            return Ok(());
        }
        if let Some(observer) = self.observer.as_mut() {
            observer.record(owner.host_observation().map_err(io::Error::other)?)?;
        }
        let started = self.observer.as_ref().map(|_| std::time::Instant::now());
        owner.close().map_err(io::Error::other)?;
        if let Some(observer) = self.observer.as_mut() {
            observer.closed(
                started
                    .ok_or_else(|| io::Error::other("host Close timer missing"))?
                    .elapsed(),
            )?;
        }
        Ok(())
    }
    fn completed(&mut self, completion: &Completion) -> io::Result<()> {
        if let Some(clocks) = self.clocks.as_mut() {
            return clocks.completed(
                self.owner
                    .as_mut()
                    .ok_or_else(|| io::Error::other("clock owner consumed"))?,
                completion,
            );
        }
        if let Some(observer) = self.observer.as_mut() {
            observer.completed(completion)?;
        }
        if let Some(device) = self.device.as_mut() {
            device.completed(completion)?;
        }
        Ok(())
    }
    fn control(&mut self, control: &Control) -> io::Result<()> {
        if let Some(clocks) = self.clocks.as_mut() {
            return clocks.control(
                self.owner
                    .as_mut()
                    .ok_or_else(|| io::Error::other("clock owner consumed"))?,
                control,
            );
        }
        if let Some(device) = self.device.as_mut() {
            device.control(control)?;
        }
        Ok(())
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
                tiles_states: l.mlp_states,
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
fn serve_and_finish<B: Backend>(
    backend: &mut B,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut FrameBudget,
    finish: impl FnOnce(&mut B) -> io::Result<()>,
) -> io::Result<()> {
    let result = serve(backend, r, w, b, incoming).and_then(|()| finish(backend));
    if result.is_err() {
        backend.failed();
    }
    result
}
fn serve(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut FrameBudget,
) -> io::Result<()> {
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
                let (reply, control, bytes) = observation(run, &request, &mut chain)?;
                backend.control(&control)?;
                wire::write_response(w, &mut outgoing, &reply, Some(&control), &bytes)?;
                if let Event::Completed(completion) = &reply.event {
                    backend.completed(completion)?;
                }
                previous = Some(output);
            }
            Command::Close => {
                if id != 5 {
                    return Err(io::Error::other("tiles early Close"));
                }
                backend.close()?;
                let reply = response(
                    &request,
                    Event::Closed {
                        completed_forwards: 4,
                        transcript_sha256: chain.digest(),
                    },
                    true,
                );
                wire::write_response(w, &mut outgoing, &reply, None, &[])?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("tiles missing Close"))
}
#[cfg(test)]
#[path = "native_prefix_decode_cli_v1_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_prefix_decode_host_cli_tests.rs"]
mod host_tests;

#[cfg(test)]
#[path = "native_prefix_decode_device_cli_tests.rs"]
mod device_tests;

#[cfg(test)]
#[path = "native_prefix_decode_clock_cli_tests.rs"]
mod clock_tests;
