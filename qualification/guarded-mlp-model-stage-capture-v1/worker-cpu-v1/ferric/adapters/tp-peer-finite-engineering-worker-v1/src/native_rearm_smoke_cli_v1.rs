//! Explicit four-forward smoke CLI; independent of old and long completion rules.

use crate::finite_forward_wire_v1::{LayerObservation, OBSERVATION_BYTES, Payload, part};
use crate::finite_rearm_smoke_wire_v1::{
    self as wire, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget, Response,
};
use crate::finite_setup_wire_v1 as setup;
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::rearm_smoke_v1::{SmokeOwner, SmokeRun};
use crate::native_setup::PreparedSetup;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
    capture_layer0: bool,
}
pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let args = args
        .iter()
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 smoke argument"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if !(args.len() == 6 || args.len() == 7 && args[6] == "--capture-layer0")
        || args[0] != "--engineering-native-rearm-smoke-v1"
        || args[1] != "--allow-unauthenticated-machine-code"
        || args[2] != "--devices"
        || args[4] != "--timeout-ms"
    {
        return Err(io::Error::other(
            "unsupported smoke invocation or missing opt-in",
        ));
    }
    fn decimal(value: &str) -> io::Result<u64> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(io::Error::other("smoke decimal argument"));
        }
        value.parse().map_err(io::Error::other)
    }
    let ids = args[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("smoke requires exactly two IDs"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(args[5])?).map_err(io::Error::other)?;
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10_000).contains(&timeout_ms)
    {
        return Err(io::Error::other("smoke IDs or timeout"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
        capture_layer0: args.len() == 7,
    })
}

/// # Safety
/// Trusted owning parent must retain authenticated model/source, exact reviewed
/// images and first-four prompt custody, captured single-threaded child and
/// unchanged bounded deadline/owned teardown. No stale state pointer or command
/// may escape the synchronous owner. This opt-in does not admit a long workload,
/// arbitrary machine code, a model identity or production execution from hashes.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (bootstrap, registration, prepared) = prepare_native(&options, reader, &mut incoming)?;
    // SAFETY: exact closed opt-in and complete actual-PID/bootstrap/Begin/image
    // checks precede the one opener for this route. No early Ready is emitted.
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut processor = prepared.into_processor(group).map_err(io::Error::other)?;
    processor.serve(reader, writer).map_err(io::Error::other)?;
    if processor.is_closed() {
        return Ok(());
    }
    let owner = processor.into_forward_owner().map_err(io::Error::other)?;
    if owner.registration_sha256() != registration {
        return Err(io::Error::other(
            "smoke sealed registration differs from Begin",
        ));
    }
    // SAFETY: same trusted-parent custody and consumed fresh sealed owner.
    let owner = unsafe { SmokeOwner::from_sealed(owner, &bootstrap) }.map_err(io::Error::other)?;
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
    incoming: &mut FrameBudget,
) -> io::Result<(Bootstrap, [u8; 32], PreparedSetup)> {
    let bootstrap = wire::read_bootstrap(reader, incoming)?
        .ok_or_else(|| io::Error::other("missing smoke bootstrap"))?;
    bootstrap.validate(
        options.devices,
        options.timeout_ms,
        std::process::id(),
        options.capture_layer0,
    )?;
    let (request, payload) =
        setup::read_request(reader)?.ok_or_else(|| io::Error::other("missing smoke Begin"))?;
    let registration = match &request.command {
        setup::Command::Begin(begin) => begin.registration.sha256,
        _ => return Err(io::Error::other("smoke setup must begin with Begin")),
    };
    let prepared = PreparedSetup::prepare(request, payload, options.devices, &bootstrap.scope)
        .map_err(io::Error::other)?;
    Ok((bootstrap, registration, prepared))
}

trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<SmokeRun>;
    fn close(&mut self) -> io::Result<()>;
}
struct Native {
    owner: Option<SmokeOwner>,
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<SmokeRun> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("smoke owner consumed"))?
            .run(input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("smoke owner consumed"))?
            .close()
            .map_err(io::Error::other)
    }
}

struct StageJson(Vec<u8>);
impl Write for StageJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > wire::STAGE_JSON_BYTES)
        {
            return Err(io::Error::other("smoke stage serialization bound"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn serialize_stage(value: &impl serde::Serialize) -> io::Result<Vec<u8>> {
    let mut output = StageJson(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(io::Error::other)?;
    wire::validate_stage_capture(&output.0)?;
    Ok(output.0)
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
fn observation(
    run: SmokeRun,
    request: &wire::Request,
    capture_enabled: bool,
    chain: &mut Chain,
) -> io::Result<(Response, Control, Vec<u8>, Vec<u8>)> {
    let selected = capture_enabled && request.id == 1;
    if run.layer0.is_some() != selected {
        return Err(io::Error::other("actual smoke stage capture presence"));
    }
    let stage = match run.layer0 {
        Some(value) => serialize_stage(&value)?,
        None => Vec::new(),
    };
    let run = run.forward;
    let c = run.completion;
    let Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err(io::Error::other("smoke observation non-forward"));
    };
    if c.generation != *generation
        || c.position != request.id as u32 - 1
        || c.input_token != *token
        || c.output_token >= 151_936
        || c.layers.len() != 36
        || run.layer_hidden.len() != 36
        || run.layer_hidden.iter().any(|row| row.len() != 8192)
        || run.final_normalized.len() != 8192
        || run.logits.len() != 303_872
    {
        return Err(io::Error::other("actual smoke observation identity/shape"));
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
            .map_err(|_| io::Error::other("smoke control layers"))?,
    };
    control.validate()?;
    let mut bytes = Vec::with_capacity(OBSERVATION_BYTES);
    for row in run.layer_hidden {
        bytes.extend_from_slice(&row);
    }
    bytes.extend_from_slice(&run.final_normalized);
    bytes.extend_from_slice(&run.logits);
    let mut completion = Completion {
        generation: c.generation,
        position: c.position,
        input_token: c.input_token,
        output_token: c.output_token,
        control: part(&control.encode()),
        observation: part(&bytes),
        capture: Payload::from_bytes(&bytes)?,
        stage_capture: selected.then(|| part(&stage)),
        chain: [0; 32],
    };
    completion.chain = chain.advance(&completion);
    Ok((
        response(request, Event::Completed(completion), false),
        control,
        bytes,
        stage,
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
    for expected in 1..=5 {
        let request = wire::read_request(reader, incoming)?
            .ok_or_else(|| io::Error::other("EOF before explicit smoke Close"))?;
        if request.id != expected
            || request.device_ids != bootstrap.device_ids
            || request.session != bootstrap.scope.session
            || request.registration != registration
            || request.profile_sha256 != profile
        {
            return Err(io::Error::other("smoke sealed profile/session mismatch"));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                if expected > 4 || *token != bootstrap.prompt_tokens[expected as usize - 1] {
                    return Err(io::Error::other(
                        "smoke input differs from declared four-token prompt",
                    ));
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
                let (response, control, bytes, stage) =
                    observation(run, &request, bootstrap.capture_layer0, &mut chain)?;
                wire::write_response(
                    writer,
                    &mut outgoing,
                    &response,
                    Some(&control),
                    &bytes,
                    &stage,
                )?;
            }
            Command::Close => {
                if expected != 5 {
                    return Err(io::Error::other("premature smoke Close"));
                }
                backend.close()?;
                wire::write_response(
                    writer,
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
                    &[],
                )?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("smoke Close missing"))
}

#[cfg(test)]
#[path = "native_rearm_smoke_cli_v1_tests.rs"]
mod tests;
