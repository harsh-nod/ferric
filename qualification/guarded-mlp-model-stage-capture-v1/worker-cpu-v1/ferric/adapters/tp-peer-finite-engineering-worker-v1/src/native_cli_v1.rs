//! Explicit trusted-parent engineering entry, never independent model admission.

use crate::finite_forward_wire_v1::{
    self as wire, Command, Completion, Event, InputMode, LayerObservation, Payload, Response,
};
use crate::finite_setup_wire_v1 as setup;
use crate::forward_sequence::{self, ForwardInput};
use crate::native_catalog::forward::{ForwardOwner, ForwardRun};
use crate::native_setup::PreparedSetup;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, Eq, PartialEq)]
pub enum Invocation {
    CheckWire,
    Native(NativeOptions),
}

/// Constructible only by the closed argument parser, not a native capability.
#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    mode: InputMode,
    timeout_ms: u32,
}

pub fn parse_args(args: &[OsString]) -> io::Result<Invocation> {
    if args.len() == 1 && args[0] == "--check-wire" {
        return Ok(Invocation::CheckWire);
    }
    let args = args
        .iter()
        .map(|value| {
            value
                .to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 finite argument"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if args.len() != 8
        || args[0] != "--engineering-native"
        || args[1] != "--allow-unauthenticated-machine-code"
        || args[2] != "--devices"
        || args[4] != "--input-mode"
        || args[6] != "--timeout-ms"
    {
        return Err(io::Error::other(
            "unsupported finite invocation or missing engineering opt-in",
        ));
    }
    let values = args[3].split(',').collect::<Vec<_>>();
    if values.len() != 2 {
        return Err(io::Error::other("exactly two device IDs required"));
    }
    let devices = [decimal(values[0])?, decimal(values[1])?];
    if devices[0] == 0 || devices[1] == 0 || devices[0] == devices[1] {
        return Err(io::Error::other("two distinct nonzero device IDs required"));
    }
    let mode = match args[5] {
        "teacher-forced" => InputMode::TeacherForced,
        "autoregressive" => InputMode::Autoregressive,
        _ => return Err(io::Error::other("explicit finite input mode required")),
    };
    let timeout_ms = u32::try_from(decimal(args[7])?).map_err(io::Error::other)?;
    if !(1..=10_000).contains(&timeout_ms) {
        return Err(io::Error::other("finite timeout bound"));
    }
    Ok(Invocation::Native(NativeOptions {
        devices,
        mode,
        timeout_ms,
    }))
}

fn decimal(value: &str) -> io::Result<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(io::Error::other("finite argument must be unsigned decimal"));
    }
    value.parse().map_err(io::Error::other)
}

/// # Safety
/// The caller must own a captured, single-threaded child and its parent-controlled
/// stdin, devices and lifetime. The parent supplies authenticated model/source/
/// upload custody and reviews the exact retained machine code. Bootstrap/Begin
/// hashes do not independently prove those premises. The owning parent must
/// enforce a process-group deadline and kill/reap on EOF, protocol failure or
/// transport loss; no retry is allowed. This grants no production admission.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> io::Result<()> {
    let (bootstrap, registration, prepared) = prepare_native(&options, reader)?;
    // SAFETY: explicit unsafe caller contract and closed CLI opt-in. Bootstrap,
    // complete Begin bytes/images, frozen scope and actual PID were checked
    // before this sole opener. No Ready or owner has yet been published.
    let group =
        unsafe { Group::open_unchecked(&prepared.device_ids()) }.map_err(io::Error::other)?;
    let mut processor = prepared.into_processor(group).map_err(io::Error::other)?;
    processor.serve(reader, writer).map_err(io::Error::other)?;
    if processor.is_closed() {
        return Ok(());
    }
    let owner = processor.into_forward_owner().map_err(io::Error::other)?;
    if owner.registration_sha256() != registration {
        return Err(io::Error::other("sealed registration differs from Begin"));
    }
    let mode = match options.mode {
        InputMode::TeacherForced => forward_sequence::InputMode::TeacherForced,
        InputMode::Autoregressive => forward_sequence::InputMode::Autoregressive,
    };
    // SAFETY: same parent custody; acknowledged setup consumed the only owner
    // and fixed all retained roots/images/states before this transition.
    let owner = unsafe { ForwardOwner::from_sealed(owner, mode, options.timeout_ms) }
        .map_err(io::Error::other)?;
    serve_forwards(
        &mut Native { owner: Some(owner) },
        reader,
        writer,
        options.devices,
        bootstrap.scope.session,
        registration,
    )
}

// This entire function is CPU-only and precedes every native opener.
fn prepare_native(
    options: &NativeOptions,
    reader: &mut impl Read,
) -> io::Result<(wire::Bootstrap, [u8; 32], PreparedSetup)> {
    let bootstrap = wire::read_bootstrap(reader)?
        .ok_or_else(|| io::Error::other("missing finite bootstrap"))?;
    bootstrap.validate(
        options.devices,
        options.mode,
        options.timeout_ms,
        std::process::id(),
    )?;
    let (request, payload) =
        setup::read_request(reader)?.ok_or_else(|| io::Error::other("missing setup Begin"))?;
    let registration = match &request.command {
        setup::Command::Begin(begin) => begin.registration.sha256,
        _ => return Err(io::Error::other("finite setup must begin with Begin")),
    };
    let prepared = PreparedSetup::prepare(request, payload, options.devices, &bootstrap.scope)
        .map_err(io::Error::other)?;
    Ok((bootstrap, registration, prepared))
}

// Private seam for framing/failure/close tests; no test creates native handles.
trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ForwardRun>;
    fn close(&mut self) -> io::Result<()>;
}
struct Native {
    owner: Option<ForwardOwner>,
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ForwardRun> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("finite owner consumed"))?
            .run(input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("finite owner consumed"))?
            .close()
            .map_err(io::Error::other)
    }
}

fn observation(run: ForwardRun, request: &wire::Request) -> io::Result<(Response, Vec<u8>)> {
    let c = run.completion;
    let Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err(io::Error::other("observation for non-forward"));
    };
    if c.generation != *generation
        || c.position != *generation as u32 - 1
        || c.input_token != *token
        || c.layers.len() != wire::LAYERS
        || run.layer_hidden.len() != wire::LAYERS
        || run
            .layer_hidden
            .iter()
            .any(|row| row.len() != wire::ROW_BYTES)
        || run.final_normalized.len() != wire::ROW_BYTES
        || run.logits.len() != wire::LOGIT_BYTES
    {
        return Err(io::Error::other(
            "actual forward observation shape or identity",
        ));
    }
    let mut bytes = Vec::with_capacity(wire::OBSERVATION_BYTES);
    for row in run.layer_hidden {
        bytes.extend_from_slice(&row);
    }
    bytes.extend_from_slice(&run.final_normalized);
    bytes.extend_from_slice(&run.logits);
    let completion = Completion {
        generation: c.generation,
        position: c.position,
        input_token: c.input_token,
        output_token: c.output_token,
        embedding_ns: c.embedding_ns,
        layers: c
            .layers
            .into_iter()
            .map(|layer| LayerObservation {
                prefix_states: layer.prefix_states,
                mlp_states: layer.mlp_states,
                paired_ns: layer.paired_ns,
            })
            .collect(),
        tail_ns: c.tail_ns,
        payload: Payload::from_bytes(&bytes)?,
    };
    Ok((
        response(request, Event::Completed(completion), false),
        bytes,
    ))
}

fn response(request: &wire::Request, event: Event, native_closed: bool) -> Response {
    Response {
        protocol: wire::PROTOCOL,
        id: request.id,
        device_ids: request.device_ids,
        session: request.session,
        registration: request.registration,
        event,
        native_closed,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}

fn serve_forwards(
    backend: &mut impl Backend,
    reader: &mut impl Read,
    writer: &mut impl Write,
    devices: [u64; 2],
    session: [u8; 32],
    registration: [u8; 32],
) -> io::Result<()> {
    for expected in 1..=3 {
        let request = wire::read_request(reader)?
            .ok_or_else(|| io::Error::other("EOF before explicit finite close"))?;
        if request.id != expected
            || request.device_ids != devices
            || request.session != session
            || request.registration != registration
        {
            return Err(io::Error::other(
                "finite request differs from sealed session",
            ));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                if expected > 2 {
                    return Err(io::Error::other("too many finite forwards"));
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
                let (response, payload) = observation(run, &request)?;
                // Lost response is fatal even if device state already committed.
                wire::write_response(writer, &response, &payload)?;
            }
            Command::Close => {
                if expected != 3 {
                    return Err(io::Error::other("premature finite close"));
                }
                backend.close()?;
                wire::write_response(
                    writer,
                    &response(
                        &request,
                        Event::Closed {
                            completed_forwards: 2,
                        },
                        true,
                    ),
                    &[],
                )?;
                return Ok(());
            }
        }
    }
    Err(io::Error::other("finite protocol missing close"))
}

#[cfg(test)]
#[path = "native_cli_v1_tests.rs"]
mod tests;
