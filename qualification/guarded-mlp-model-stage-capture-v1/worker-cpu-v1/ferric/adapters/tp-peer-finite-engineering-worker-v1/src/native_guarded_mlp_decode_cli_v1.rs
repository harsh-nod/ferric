//! Explicit bounded guarded real-model TF4/AR4 entry. No legacy or performance fallback.
use crate::finite_forward_wire_v1::{OBSERVATION_BYTES, Payload, part};
use crate::finite_guarded_mlp_decode_wire_v1::{
    self as wire, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget, InputMode,
    LayerObservation, Response,
};
use crate::forward_sequence::ForwardInput;
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
pub const CAPTURE_FLAG: &str = "--engineering-native-guarded-mlp-stage-capture-v1";
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
fn prepare(
    options: &NativeOptions,
    r: &mut impl Read,
    incoming: &mut FrameBudget,
) -> io::Result<(Bootstrap, PreparedGuardedDecodeSetup)> {
    let (b, [mlp, prefix, projection, guarded]) = wire::read_bootstrap(r, incoming)?
        .ok_or_else(|| io::Error::other("guarded bootstrap absent"))?;
    let mode = admitted_mode(options, &b)?;
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
    unsafe { run_selected(options, r, w, None) }
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
    unsafe { run_selected(options, r, w, Some(diagnostic)) }
}

#[allow(unsafe_code)]
unsafe fn run_selected(
    options: NativeOptions,
    r: &mut impl Read,
    w: &mut impl Write,
    diagnostic: Option<&mut dyn Write>,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (b, prepared) = prepare(&options, r, &mut incoming)?;
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut setup = prepared.into_processor(group).map_err(io::Error::other)?;
    setup.serve(r, w).map_err(io::Error::other)?;
    if setup.is_closed() {
        return Err(io::Error::other("guarded setup closed before forwards"));
    }
    let mut owner = unsafe { setup.into_decode() }.map_err(io::Error::other)?;
    if diagnostic.is_some() {
        owner.enable_layer0_capture().map_err(io::Error::other)?;
    }
    let mut backend = Native {
        owner: Some(owner),
        profile: b.sha256()?,
        capture_enabled: diagnostic.is_some(),
        closed_capture: None,
    };
    serve(&mut backend, r, w, &b, &mut incoming)?;
    if let Some(diagnostic) = diagnostic {
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
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run>;
    fn close(&mut self) -> io::Result<()>;
    fn failed(&mut self);
}
struct Native {
    owner: Option<Owner>,
    profile: [u8; 32],
    capture_enabled: bool,
    closed_capture: Option<crate::resident_layer::capture_v1::Layer0CaptureV1>,
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("guarded owner consumed"))?
            .run(self.profile, input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        let owner = self
            .owner
            .take()
            .ok_or_else(|| io::Error::other("guarded owner consumed"))?;
        if self.capture_enabled {
            self.closed_capture = Some(owner.close_with_capture().map_err(io::Error::other)?);
            Ok(())
        } else {
            owner.close().map_err(io::Error::other)
        }
    }
    fn failed(&mut self) {
        drop(self.owner.take());
        self.closed_capture = None;
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
