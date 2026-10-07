//! Explicit bounded guarded real-model TF4/AR4 entry. No legacy or performance fallback.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
use crate::finite_guarded_mlp_decode_wire_v1::{
    self as wire, ArenaRoute, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget,
    InputMode, LayerObservation, Response,
};
use crate::forward_sequence::ForwardInput;
use crate::guarded_mlp_host_observation_v1::Policy as HostPolicy;
use crate::native_catalog::forward::guarded_mlp_decode_v1::{Mode, Owner, Run};
use crate::native_setup::{PreparedSetup, guarded_mlp_decode_v1::PreparedGuardedDecodeSetup};
use crate::resident_layer::{
    guarded_mlp_decode_v1::{Image, Images},
    mlp_tiles_v2::Image as MlpImage,
    prefix_tiles_v6::{
        artifacts::Image as PrefixImage, projection_residual::Image as ProjectionImage,
    },
};
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
};

pub const FLAG: &str = "--engineering-native-guarded-mlp-decode-v1";
pub const REUSE_FLAG: &str = wire::REUSE_WORKER_FLAG;
pub const PAIRED_TERMINAL_FLAG: &str = wire::PAIRED_TERMINAL_WORKER_FLAG;
pub const CAPTURE_FLAG: &str = "--engineering-native-guarded-mlp-stage-capture-v1";
pub const HOST_FLAG: &str = "--engineering-native-guarded-mlp-host-observation-v1";
pub const HOST_SHARED_FLAG: &str = HostPolicy::SharedFull.worker_flag();
pub const HOST_PAIRED_READ_FLAG: &str = HostPolicy::SharedFullPairedRead.worker_flag();
pub use crate::native_projection_residual_decode_cli_v1::NativeOptions;

pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(FLAG) {
        return Err(io::Error::other("explicit guarded decode invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = "--engineering-native-projection-residual-decode-v1".into();
    crate::native_projection_residual_decode_cli_v1::parse_args(&delegated)
}
pub fn parse_capture_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(CAPTURE_FLAG) {
        return Err(io::Error::other(
            "explicit guarded stage capture invocation",
        ));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    parse_args(&delegated)
}
pub fn parse_reuse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(REUSE_FLAG) {
        return Err(io::Error::other("explicit reusable guarded AR4 invocation"));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    let options = parse_args(&delegated)?;
    if options.mode != InputMode::Autoregressive {
        return Err(io::Error::other("reusable guarded entry requires AR4"));
    }
    Ok(options)
}
pub fn parse_paired_terminal_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(PAIRED_TERMINAL_FLAG) {
        return Err(io::Error::other(
            "explicit warm paired terminal AR4 invocation",
        ));
    }
    let mut delegated = args.to_vec();
    delegated[0] = REUSE_FLAG.into();
    parse_reuse_args(&delegated)
}
pub fn parse_host_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(HOST_FLAG) {
        return Err(io::Error::other(
            "explicit guarded host observation invocation",
        ));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    parse_args(&delegated)
}
pub fn parse_host_shared_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(HOST_SHARED_FLAG) {
        return Err(io::Error::other(
            "explicit guarded shared-full host invocation",
        ));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    parse_args(&delegated)
}
pub fn parse_host_paired_read_args(args: &[OsString]) -> io::Result<NativeOptions> {
    if args.first().and_then(|s| s.to_str()) != Some(HOST_PAIRED_READ_FLAG) {
        return Err(io::Error::other(
            "explicit paired hidden-read host invocation",
        ));
    }
    let mut delegated = args.to_vec();
    delegated[0] = FLAG.into();
    let options = parse_args(&delegated)?;
    admitted_host_read_mode(Some(HostPolicy::SharedFullPairedRead), options.mode, false)?;
    Ok(options)
}

fn admitted_host_read_mode(
    policy: Option<HostPolicy>,
    mode: InputMode,
    reusable: bool,
) -> io::Result<()> {
    if policy.is_some_and(|value| value.paired_read())
        && (mode != InputMode::Autoregressive || reusable)
    {
        return Err(io::Error::other(
            "paired hidden-read host mode requires fresh AR4",
        ));
    }
    Ok(())
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
fn admitted_arena_mode(options: &NativeOptions, b: &Bootstrap, reusable: bool) -> io::Result<Mode> {
    admitted_arena_route(options, b, ArenaRoute::from_reusable(reusable))
}
fn admitted_arena_route(
    options: &NativeOptions,
    b: &Bootstrap,
    route: ArenaRoute,
) -> io::Result<Mode> {
    let mode = admitted_mode(options, b)?;
    if b.schema != route.schema() {
        return Err(io::Error::other(
            "guarded invocation/bootstrap arena policy mismatch",
        ));
    }
    Ok(mode)
}
fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
    reusable: bool,
) -> io::Result<(Bootstrap, PreparedGuardedDecodeSetup)> {
    prepare_route(options, r, incoming, ArenaRoute::from_reusable(reusable))
}
fn prepare_route(
    options: &NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
    route: ArenaRoute,
) -> io::Result<(Bootstrap, PreparedGuardedDecodeSetup)> {
    let (b, [mlp, prefix, projection, guarded]) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("guarded bootstrap absent"))?;
    let mode = admitted_arena_route(options, &b, route)?;
    let (request, payload) = wire::read_begin(r, incoming, &b)?;
    let setup = PreparedSetup::prepare(request, payload, options.devices, &b.decode.scope)
        .map_err(io::Error::other)?;
    let prepared = PreparedGuardedDecodeSetup::new(
        setup,
        PrefixImage::new(prefix, &b.decode.prefix_image).map_err(io::Error::other)?,
        MlpImage::new(mlp, &b.decode.tiles_image).map_err(io::Error::other)?,
        Images {
            projection: ProjectionImage::new(projection, &b.projection_image)
                .map_err(io::Error::other)?,
            guarded: Image::new(guarded, &b.guarded_image).map_err(io::Error::other)?,
        },
        mode,
        options.timeout_ms,
    )
    .map_err(io::Error::other)?;
    let prepared = if route.reusable() {
        prepared.with_reusable_arenas().map_err(io::Error::other)?
    } else {
        prepared
    };
    let prepared = if route.paired_terminal() {
        prepared.with_paired_terminal().map_err(io::Error::other)?
    } else {
        prepared
    };
    if prepared.profile_sha256() != b.sha256()? {
        return Err(io::Error::other("guarded wire/backend profile mismatch"));
    }
    Ok((b, prepared))
}

/// # Safety
/// The trusted parent authenticates source, images and model; establishes the
/// reviewed TP2 peer/coherence and queue-lifetime premises; and owns bounded child
/// cleanup. Success records observations, not model acceptance or runtime authority.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_selected(options, r, w, None, None, false) }
}

/// # Safety
/// Same authenticated images, coherent producers and owned-child contract as
/// run_native; the runtime additionally verifies private retired arena reuse.
#[allow(unsafe_code)]
pub unsafe fn run_native_reuse(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut dyn Write,
) -> io::Result<()> {
    unsafe { run_selected(options, r, w, Some(diagnostic), None, true) }
}

/// # Safety
/// Same model, coherent producers and owned-child contract as run_native_reuse.
/// Only genuinely retired generation-two pairs use the opt-in terminal API.
#[allow(unsafe_code)]
pub unsafe fn run_native_paired_terminal(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut dyn Write,
) -> io::Result<()> {
    unsafe {
        run_selected_route(
            options,
            r,
            w,
            Some(diagnostic),
            None,
            ArenaRoute::PairedTerminal,
        )
    }
}

/// # Safety
/// Identical image/model/peer and owned-child requirements as `run_native`.
/// The extra output is a bounded diagnostic only, emitted after actual Close.
#[allow(unsafe_code)]
pub unsafe fn run_native_capture(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut impl Write,
) -> io::Result<()> {
    unsafe { run_selected(options, r, w, Some(diagnostic), None, false) }
}

/// # Safety
/// All `run_native` image/model/peer and owned-child obligations are unchanged.
/// Enables host observation only, never a different performance policy.
#[allow(unsafe_code)]
pub unsafe fn run_native_host(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_selected(
            options,
            r,
            w,
            Some(diagnostic),
            Some(HostPolicy::DefaultFull),
            false,
        )
    }
}

/// # Safety
/// All `run_native` image, coherence, ownership, and child-lifetime obligations
/// remain mandatory. Only fresh full observations are shared within each fence;
/// operational checks, admission caching, and raw timestamps remain disabled.
#[allow(unsafe_code)]
pub unsafe fn run_native_host_shared(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_selected(
            options,
            r,
            w,
            Some(diagnostic),
            Some(HostPolicy::SharedFull),
            false,
        )
    }
}

/// # Safety
/// Same authenticated source, images, coherent producers and owned-child
/// requirements as run_native_host_shared. Only hidden-result reads use the
/// closed two-rank runtime transaction; this is not a new runtime permission.
#[allow(unsafe_code)]
pub unsafe fn run_native_host_paired_read(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: &mut impl Write,
) -> io::Result<()> {
    unsafe {
        run_selected(
            options,
            r,
            w,
            Some(diagnostic),
            Some(HostPolicy::SharedFullPairedRead),
            false,
        )
    }
}

#[allow(unsafe_code)]
unsafe fn run_selected(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: Option<&mut dyn Write>,
    host_policy: Option<HostPolicy>,
    reusable: bool,
) -> io::Result<()> {
    unsafe {
        run_selected_route(
            options,
            r,
            w,
            diagnostic,
            host_policy,
            ArenaRoute::from_reusable(reusable),
        )
    }
}
#[allow(unsafe_code)]
unsafe fn run_selected_route(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: Option<&mut dyn Write>,
    host_policy: Option<HostPolicy>,
    route: ArenaRoute,
) -> io::Result<()> {
    let reusable = route.reusable();
    if route.paired_terminal() && (host_policy.is_some() || diagnostic.is_none()) {
        return Err(io::Error::other(
            "paired terminal requires isolated close-only diagnostics",
        ));
    }
    admitted_host_read_mode(host_policy, options.mode, reusable)?;
    let mut incoming = FrameBudget::new();
    let (b, prepared) = prepare_route(&options, r, &mut incoming, route)?;
    let mut group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let observer = match host_policy {
        Some(HostPolicy::DefaultFull) => Some(crate::native_guarded_mlp_host_v1::Recorder::enable(
            &mut group, &b,
        )?),
        Some(HostPolicy::SharedFull) => Some(
            crate::native_guarded_mlp_host_v1::Recorder::enable_shared(&mut group, &b)?,
        ),
        Some(HostPolicy::SharedFullPairedRead) => {
            Some(crate::native_guarded_mlp_host_v1::Recorder::enable_paired_read(&mut group, &b)?)
        }
        None => None,
    };
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Err(io::Error::other("guarded setup closed before forwards"));
    }
    let mut owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    if diagnostic.is_some() && host_policy.is_none() && !reusable {
        owner.enable_layer0_capture().map_err(io::Error::other)?;
    }
    if let Some(observer) = observer {
        owner
            .enable_host_observation(observer)
            .map_err(io::Error::other)?;
    }
    let arena_census = if reusable {
        Some(vec![
            owner
                .reusable_allocation_census(0)
                .map_err(io::Error::other)?,
        ])
    } else {
        None
    };
    let mut backend = Native {
        owner: Some(owner),
        profile: b.sha256()?,
        capture_enabled: diagnostic.is_some() && host_policy.is_none() && !reusable,
        closed_capture: None,
        host_enabled: host_policy.is_some(),
        closed_host: None,
        arena_census,
        closed_arena_census: None,
        closed_terminal_dispatches: None,
    };
    serve(&mut backend, r, w, &b, &mut incoming)?;
    if let Some(diagnostic) = diagnostic {
        if reusable {
            let record = wire::ReuseArenaCensus {
                schema: "FerricGuardedMlpReusableAr4ArenaCensusV1".into(),
                profile_sha256: b.sha256()?,
                registration: b.decode.registration,
                session: b.decode.scope.session,
                device_ids: b.decode.device_ids,
                allocation_counts: backend.closed_arena_census.take().ok_or_else(|| {
                    io::Error::other("guarded reusable census absent after Close")
                })?,
                completed_forwards: 4,
                native_closed: true,
                performance_claim: false,
            };
            let mut bytes = if route.paired_terminal() {
                let terminal = wire::PairedTerminalCensus {
                    schema: "FerricGuardedMlpReusableAr4PairedTerminalCensusV1".into(),
                    arena: record,
                    paired_terminal_dispatches: backend
                        .closed_terminal_dispatches
                        .take()
                        .ok_or_else(|| {
                            io::Error::other("paired terminal census absent after Close")
                        })?,
                    first_use_legacy: true,
                    warm_retired_only: true,
                    global_currentness_policy_changed: false,
                    terminal_currentness_cadence_changed: true,
                    performance_claim: false,
                };
                terminal.validate(&b)?;
                serde_json::to_vec(&terminal).map_err(io::Error::other)?
            } else {
                record.validate(&b)?;
                serde_json::to_vec(&record).map_err(io::Error::other)?
            };
            bytes.push(b'\n');
            if bytes.len() > wire::REUSE_CENSUS_BYTES {
                return Err(io::Error::other("guarded reusable census output bound"));
            }
            diagnostic.write_all(&bytes)?;
            diagnostic.flush()?;
            return Ok(());
        }
        if let Some(policy) = host_policy {
            let report = backend
                .closed_host
                .take()
                .ok_or_else(|| io::Error::other("guarded host report absent after Close"))?;
            report.validate_policy(policy)?;
            let mut bytes = serde_json::to_vec(&report).map_err(io::Error::other)?;
            bytes.push(b'\n');
            if bytes.len() > crate::guarded_mlp_host_observation_v1::MAX_BYTES {
                return Err(io::Error::other("guarded host output bound"));
            }
            diagnostic.write_all(&bytes)?;
            diagnostic.flush()?;
            return Ok(());
        }
        let capture = backend
            .closed_capture
            .take()
            .ok_or_else(|| io::Error::other("guarded stage capture absent after Close"))?;
        let value = serde_json::json!({
            "schema": "FerricFiniteGuardedMlpLayerZeroCaptureV1",
            "profile_sha256": b.sha256()?, "registration_sha256": b.decode.registration,
            "session": b.decode.scope.session, "device_ids": b.decode.device_ids,
            "completed_forwards": 4, "native_closed": true,
            "sampling": "prefix-boundaries-and-post-paired-retained",
            "capture": capture, "numerical_acceptance": false,
            "performance_claim": false, "production_authority": false,
        });
        let mut bytes = serde_json::to_vec(&value).map_err(io::Error::other)?;
        bytes.push(b'\n');
        if bytes.len() > 1_100_000 {
            return Err(io::Error::other("guarded stage capture output bound"));
        }
        diagnostic.write_all(&bytes)?;
        diagnostic.flush()?;
    }
    Ok(())
}
trait Backend {
    fn completed(&mut self, _value: &Completion) -> io::Result<()> {
        Ok(())
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run>;
    fn close(&mut self) -> io::Result<()>;
    fn failed(&mut self);
}
struct Native {
    owner: Option<Owner>,
    profile: [u8; 32],
    capture_enabled: bool,
    closed_capture: Option<crate::resident_layer::capture_v1::Layer0CaptureV1>,
    host_enabled: bool,
    closed_host: Option<crate::guarded_mlp_host_observation_v1::Report>,
    arena_census: Option<Vec<[u64; 2]>>,
    closed_arena_census: Option<[[u64; 2]; 5]>,
    closed_terminal_dispatches: Option<[u32; 4]>,
}
fn finish_arena_census(
    samples: Vec<[u64; 2]>,
    close: impl FnOnce() -> io::Result<()>,
) -> io::Result<[[u64; 2]; 5]> {
    let samples = samples
        .try_into()
        .map_err(|_| io::Error::other("guarded reusable incomplete census before Close"))?;
    close()?;
    Ok(samples)
}
impl Backend for Native {
    fn completed(&mut self, value: &Completion) -> io::Result<()> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("guarded owner consumed"))?
            .host_completed(value)
            .map_err(io::Error::other)
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        let owner = self
            .owner
            .as_mut()
            .ok_or_else(|| io::Error::other("guarded owner consumed"))?;
        let run = owner.run(self.profile, input).map_err(io::Error::other)?;
        if let Some(samples) = self.arena_census.as_mut() {
            if samples.len() != input.generation as usize || samples.len() >= 5 {
                return Err(io::Error::other("guarded reusable sample order"));
            }
            samples.push(
                owner
                    .reusable_allocation_census(input.generation)
                    .map_err(io::Error::other)?,
            );
        }
        Ok(run)
    }
    fn close(&mut self) -> io::Result<()> {
        let mut owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("guarded owner consumed"))?;
        if let Some(samples) = self.arena_census.take() {
            let terminal = owner
                .paired_terminal_dispatches()
                .map_err(io::Error::other)?;
            self.closed_arena_census = Some(finish_arena_census(samples, || {
                owner.close().map_err(io::Error::other)
            })?);
            self.closed_terminal_dispatches = terminal;
            Ok(())
        } else if self.host_enabled {
            self.closed_host = Some(
                owner
                    .close_with_host_observation()
                    .map_err(io::Error::other)?,
            );
            Ok(())
        } else if self.capture_enabled {
            self.closed_capture = Some(owner.close_with_capture().map_err(io::Error::other)?);
            Ok(())
        } else {
            owner.close().map_err(io::Error::other)
        }
    }
    fn failed(&mut self) {
        drop(self.owner.take());
        self.closed_capture = None;
        self.closed_host = None;
        self.arena_census = None;
        self.closed_arena_census = None;
        self.closed_terminal_dispatches = None;
    }
}
fn response(r: &wire::Request, event: Event, closed: bool) -> Response {
    Response {
        schema: wire::RESPONSE_SCHEMA.into(),
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
        full_model_acceptance: false,
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
        return Err(io::Error::other("guarded observation non-forward"));
    };
    if c.profile_sha256 != request.profile_sha256
        || c.generation != *generation
        || c.position != request.id as u32 - 1
        || c.input_token != *token
        || c.output_token >= 151936
        || c.layers.len() != 36
        || run.layer_hidden.len() != 36
        || run.layer_hidden.iter().any(|v| v.len() != 8192)
        || run.final_normalized.len() != 8192
        || run.logits.len() != 303872
    {
        return Err(io::Error::other(
            "guarded actual observation identity/extent",
        ));
    }
    let control = Control {
        embedding_ns: c.embedding_ns,
        tail_ns: c.tail_ns,
        layers: c
            .layers
            .into_iter()
            .map(|l| LayerObservation {
                prefix_states: l.prefix_states,
                mlp_prefixes: l.guarded.prefixes,
                guards: l.guarded.guards,
                prefix_host_ns: l.prefix_ns,
                segment_host_ns: l.guarded.segment_host_ns,
                observed_queue_frontiers: l.guarded.observed_queue_frontiers,
            })
            .collect(),
    };
    control.validate(c.generation)?;
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
    let result = serve_inner(backend, r, w, b, incoming);
    if result.is_err() {
        backend.failed();
    }
    result
}
fn serve_inner(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &Bootstrap,
    incoming: &mut FrameBudget,
) -> io::Result<()> {
    let profile = b.sha256()?;
    let mut outgoing = FrameBudget::new();
    let mut chain = Chain::new(b.decode.registration, profile);
    let mut previous = None;
    let mut previous_frontiers = [(0, 0); 2];
    for id in 1..=5 {
        let request = wire::read_request(r, incoming)?
            .ok_or_else(|| io::Error::other("guarded EOF before Close"))?;
        if request.id != id
            || request.device_ids != b.decode.device_ids
            || request.session != b.decode.scope.session
            || request.registration != b.decode.registration
            || request.profile_sha256 != profile
        {
            return Err(io::Error::other("guarded request profile/session/order"));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                if id > 4 || *token != b.input(id as u32 - 1, previous)? {
                    return Err(io::Error::other("guarded request mode trajectory"));
                }
                let input = ForwardInput {
                    registration: b.decode.registration,
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
                for rank in 0..2 {
                    let first = control.layers[0].observed_queue_frontiers[rank];
                    if first.0 <= previous_frontiers[rank].0 || first.1 < previous_frontiers[rank].1
                    {
                        return Err(io::Error::other("guarded cross-forward queue frontier"));
                    }
                }
                previous_frontiers = control.layers[35].observed_queue_frontiers;
                if let Event::Completed(done) = &reply.event {
                    backend.completed(done)?;
                }
                wire::write_response(w, &mut outgoing, &reply, Some(&control), &bytes)?;
                previous = Some(output);
            }
            Command::Close => {
                if id != 5 {
                    return Err(io::Error::other("guarded early Close"));
                }
                backend.close()?;
                wire::write_response(
                    w,
                    &mut outgoing,
                    &response(
                        &request,
                        Event::Closed {
                            completed_forwards: 4,
                            transcript_sha256: chain.digest(),
                        },
                        true,
                    ),
                    None,
                    &[],
                )?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("guarded missing Close"))
}
#[cfg(test)]
#[path = "native_guarded_mlp_decode_cli_v1_tests.rs"]
mod tests;
