//! Explicit one-forward engineering comparison, never an old-profile fallback.

use crate::finite_forward_wire_v1::{LayerObservation, OBSERVATION_BYTES, Payload, part};
use crate::finite_mlp_tiles_comparison_wire_v1::{
    self as wire, Bootstrap, Chain, Command, ComparisonReport, Completion, Control, Event,
    FrameBudget, Response,
};
use crate::finite_setup_wire_v1 as setup;
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::mlp_tiles_comparison_v1::{ComparisonOwner, ComparisonRun};
use crate::native_setup::PreparedSetup;
use crate::resident_layer::mlp_tiles_v2 as tiles;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, PartialEq, Eq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let args = args
        .iter()
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 comparison argument"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if args.len() != 6
        || args[0] != "--engineering-native-mlp-tiles-comparison-v1"
        || args[1] != "--allow-unauthenticated-machine-code"
        || args[2] != "--devices"
        || args[4] != "--timeout-ms"
    {
        return Err(io::Error::other(
            "unsupported V1/V2 comparison invocation or missing opt-in",
        ));
    }
    fn decimal(value: &str) -> io::Result<u64> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(io::Error::other("comparison decimal argument"));
        }
        value.parse().map_err(io::Error::other)
    }
    let ids = args[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("comparison requires exactly two IDs"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(args[5])?).map_err(io::Error::other)?;
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10_000).contains(&timeout_ms)
    {
        return Err(io::Error::other("comparison IDs or timeout"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
    })
}

/// # Safety
/// The trusted owning parent retains authentic model/source/first-token custody,
/// exact reviewed images, captured single-threaded child and bounded owned
/// process teardown. Bootstrap/Begin do not independently authenticate a model
/// or grant production admission. The separate mode runs both MLP routes and
/// is not a whole-model speedup or V2 full-model admission.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (bootstrap, registration, prepared, images) =
        prepare_native(&options, reader, &mut incoming)?;
    // SAFETY: full actual-PID/bootstrap/Begin/image validation precedes the sole
    // opener. No Ready or execution acknowledgement is emitted before setup.
    let mut group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    if group
        .preflight_additional_allocations_v1(&[715, 711])
        .map_err(io::Error::other)?
        != [0, 0]
    {
        return Err(io::Error::other(
            "V2 comparison requires fresh group before any allocation",
        ));
    }
    let mut processor = prepared.into_processor(group).map_err(io::Error::other)?;
    processor.serve(reader, writer).map_err(io::Error::other)?;
    if processor.is_closed() {
        return Ok(());
    }
    let owner = processor.into_forward_owner().map_err(io::Error::other)?;
    if owner.registration_sha256() != registration {
        return Err(io::Error::other(
            "comparison sealed registration differs from Begin",
        ));
    }
    // SAFETY: the same consumed owner and exact immutable objects, with the
    // additional entries loaded only through that owner's group.
    let owner = unsafe { ComparisonOwner::from_sealed(owner, &bootstrap, images) }
        .map_err(io::Error::other)?;
    serve_forwards(
        &mut Native { owner: Some(owner) },
        reader,
        writer,
        &bootstrap,
        registration,
        &mut incoming,
    )
}

fn prepare_native(
    options: &NativeOptions,
    reader: &mut impl Read,
    budget: &mut FrameBudget,
) -> io::Result<(Bootstrap, [u8; 32], PreparedSetup, tiles::Image)> {
    let (bootstrap, image_bytes) = wire::read_bootstrap(reader, budget)?
        .ok_or_else(|| io::Error::other("missing comparison bootstrap"))?;
    bootstrap.validate(options.devices, options.timeout_ms, std::process::id())?;
    let (request, payload) =
        setup::read_request(reader)?.ok_or_else(|| io::Error::other("missing comparison Begin"))?;
    let registration = match &request.command {
        setup::Command::Begin(begin) if begin.tail_image.is_some() => begin.registration.sha256,
        setup::Command::Begin(_) => {
            return Err(io::Error::other(
                "V2 comparison requires complete tail setup before open",
            ));
        }
        _ => return Err(io::Error::other("comparison setup must start with Begin")),
    };
    let images =
        tiles::Image::new(image_bytes, &bootstrap.tiles_image).map_err(io::Error::other)?;
    let prepared = PreparedSetup::prepare(request, payload, options.devices, &bootstrap.scope)
        .map_err(io::Error::other)?;
    Ok((bootstrap, registration, prepared, images))
}

trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ComparisonRun>;
    fn close(&mut self) -> io::Result<()>;
}
struct Native {
    owner: Option<ComparisonOwner>,
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ComparisonRun> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("comparison owner consumed"))?
            .run(input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("comparison owner consumed"))?
            .close()
            .map_err(io::Error::other)
    }
}
fn response(request: &wire::Request, event: Event, closed: bool) -> Response {
    Response {
        protocol: wire::PROTOCOL,
        id: request.id,
        device_ids: request.device_ids,
        session: request.session,
        registration: request.registration,
        profile_sha256: request.profile_sha256,
        event,
        native_closed: closed,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
fn report(value: tiles::Comparison, control: &Control) -> io::Result<Vec<u8>> {
    let report = ComparisonReport {
        schema: "FerricFiniteMlpTilesComparisonV1".into(),
        generation: 1,
        position: 0,
        layer: 0,
        finite_states: value.finite_states,
        finite_queue_host_ns: value.finite_queue_host_ns,
        tiles_states: value.tiles_states.map(Vec::from),
        tiles_queue_host_ns: value.tiles_queue_host_ns,
        equality: value.equality,
        v2_full_model: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    report.validate(control)?;
    let bytes = serde_json::to_vec(&report).map_err(io::Error::other)?;
    wire::validate_comparison(&bytes, control)?;
    Ok(bytes)
}
fn observation(
    run: ComparisonRun,
    request: &wire::Request,
    chain: &mut Chain,
) -> io::Result<(Response, Control, Vec<u8>, Vec<u8>)> {
    let forward = run.forward;
    let c = forward.completion;
    if c.generation != 1
        || c.position != 0
        || c.input_token != wire::TOKEN
        || c.output_token >= 151_936
        || c.layers.len() != 36
        || forward.layer_hidden.len() != 36
        || forward.layer_hidden.iter().any(|row| row.len() != 8192)
        || forward.final_normalized.len() != 8192
        || forward.logits.len() != 303872
    {
        return Err(io::Error::other("actual comparison forward identity/shape"));
    }
    let control = Control {
        embedding_ns: c.embedding_ns,
        tail_ns: c.tail_ns,
        layers: c
            .layers
            .into_iter()
            .map(|layer| LayerObservation {
                prefix_states: layer.prefix_states,
                mlp_states: layer.mlp_states,
                paired_ns: layer.paired_ns,
            })
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| io::Error::other("comparison control layer count"))?,
    };
    control.validate()?;
    let report = report(run.comparison, &control)?;
    let mut main = Vec::with_capacity(OBSERVATION_BYTES);
    for row in forward.layer_hidden {
        main.extend_from_slice(&row);
    }
    main.extend_from_slice(&forward.final_normalized);
    main.extend_from_slice(&forward.logits);
    let mut completion = Completion {
        generation: 1,
        position: 0,
        input_token: c.input_token,
        output_token: c.output_token,
        control: part(&control.encode()),
        observation: part(&main),
        capture: Payload::from_bytes(&main)?,
        comparison: part(&report),
        chain: [0; 32],
    };
    completion.chain = chain.advance(&completion);
    Ok((
        response(request, Event::Completed(completion), false),
        control,
        main,
        report,
    ))
}
fn serve_forwards(
    backend: &mut impl Backend,
    reader: &mut impl Read,
    writer: &mut impl Write,
    bootstrap: &Bootstrap,
    registration: [u8; 32],
    incoming: &mut FrameBudget,
) -> io::Result<()> {
    let profile = bootstrap.sha256()?;
    let mut outgoing = FrameBudget::new();
    let mut chain = Chain::new(registration, profile);
    for expected in 1..=2 {
        let request = wire::read_request(reader, incoming)?
            .ok_or_else(|| io::Error::other("EOF before comparison Close"))?;
        if request.id != expected
            || request.device_ids != bootstrap.device_ids
            || request.session != bootstrap.scope.session
            || request.registration != registration
            || request.profile_sha256 != profile
        {
            return Err(io::Error::other("comparison session/profile mismatch"));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                if expected != 1 || *token != bootstrap.token {
                    return Err(io::Error::other("comparison declared token mismatch"));
                }
                let input = ForwardInput {
                    registration,
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
                let (value, control, main, report) = observation(run, &request, &mut chain)?;
                wire::write_response(
                    writer,
                    &mut outgoing,
                    &value,
                    Some(&control),
                    &main,
                    &report,
                )?;
            }
            Command::Close => {
                if expected != 2 {
                    return Err(io::Error::other("premature comparison Close"));
                }
                backend.close()?;
                wire::write_response(
                    writer,
                    &mut outgoing,
                    &response(
                        &request,
                        Event::Closed {
                            completed_forwards: 1,
                            transcript_sha256: chain.digest(),
                        },
                        true,
                    ),
                    None,
                    &[],
                    &[],
                )?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("comparison Close missing"))
}

#[cfg(test)]
#[path = "native_mlp_tiles_comparison_cli_v1_tests.rs"]
mod tests;
