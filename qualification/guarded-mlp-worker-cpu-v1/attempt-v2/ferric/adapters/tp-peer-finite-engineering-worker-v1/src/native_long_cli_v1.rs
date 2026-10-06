//! Separate closed long CLI. Bootstrap is trusted-parent custody, not admission.

use crate::finite_forward_wire_v1::{LayerObservation, OBSERVATION_BYTES, Payload, part};
use crate::finite_long_wire_v1::{
    self as wire, Bootstrap, Chain, Command, Completion, Control, Event, FrameBudget, Response,
};
use crate::finite_setup_wire_v1 as setup;
use crate::forward_sequence::ForwardInput;
use crate::native_catalog::forward::{ForwardRun, long_v1::LongForwardOwnerV1};
use crate::native_setup::PreparedSetup;
use fe2o3_kfd::Gfx950EngineeringPeerGroupV1 as Group;
use std::ffi::OsString;
use std::io::{self, Read, Write};

#[derive(Debug, Eq, PartialEq)]
pub struct NativeOptions {
    devices: [u64; 2],
    timeout_ms: u32,
}

pub fn parse_args(args: &[OsString]) -> io::Result<NativeOptions> {
    let args = args
        .iter()
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| io::Error::other("non-UTF8 long argument"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    if args.len() != 6
        || args[0] != "--engineering-native-long-v1"
        || args[1] != "--allow-unauthenticated-machine-code"
        || args[2] != "--devices"
        || args[4] != "--timeout-ms"
    {
        return Err(io::Error::other(
            "unsupported long invocation or missing opt-in",
        ));
    }
    fn decimal(value: &str) -> io::Result<u64> {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(io::Error::other("long unsigned decimal argument"));
        }
        value.parse().map_err(io::Error::other)
    }
    let ids = args[3].split(',').collect::<Vec<_>>();
    if ids.len() != 2 {
        return Err(io::Error::other("long requires exactly two IDs"));
    }
    let devices = [decimal(ids[0])?, decimal(ids[1])?];
    let timeout_ms = u32::try_from(decimal(args[5])?).map_err(io::Error::other)?;
    if devices[0] == 0
        || devices[1] == 0
        || devices[0] == devices[1]
        || !(1..=10_000).contains(&timeout_ms)
    {
        return Err(io::Error::other("long IDs or per-dispatch timeout"));
    }
    Ok(NativeOptions {
        devices,
        timeout_ms,
    })
}

/// # Safety
/// Same captured, single-threaded, exclusively parent-owned engineering process
/// contract as the two-forward CLI, with a separate authentic2048-token prompt
/// and closed long-profile review. Parent enforces the unchanged deadline and
/// fatal teardown on every protocol/transport error, never retries. The parent
/// must not launch a workload that cannot fit its reviewed bounded resources.
/// Original setup Registration/source bytes are descriptive custody, not a
/// production or long-forward proof. All saved native commands stay private.
#[allow(unsafe_code)]
pub unsafe fn run_native(
    options: NativeOptions,
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> io::Result<()> {
    let mut incoming = FrameBudget::new();
    let (bootstrap, registration, prepared) = prepare_native(&options, reader, &mut incoming)?;
    // SAFETY: sole opener for this explicit profile, after actual PID/scope and
    // complete Begin/image validation. No Ready is sent before acknowledged setup.
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
            "long sealed registration differs from Begin",
        ));
    }
    // SAFETY: distinct bootstrap selection and same acknowledged exact owner;
    // this constructor consumes the fresh banks, never a prior two-forward run.
    let owner =
        unsafe { LongForwardOwnerV1::from_sealed(owner, &bootstrap) }.map_err(io::Error::other)?;
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
        .ok_or_else(|| io::Error::other("missing long bootstrap"))?;
    bootstrap.validate(options.devices, options.timeout_ms, std::process::id())?;
    let (request, payload) =
        setup::read_request(reader)?.ok_or_else(|| io::Error::other("missing long setup Begin"))?;
    let registration = match &request.command {
        setup::Command::Begin(begin) => begin.registration.sha256,
        _ => return Err(io::Error::other("long setup must begin with Begin")),
    };
    let prepared = PreparedSetup::prepare(request, payload, options.devices, &bootstrap.scope)
        .map_err(io::Error::other)?;
    Ok((bootstrap, registration, prepared))
}

trait Backend {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ForwardRun>;
    fn close(&mut self) -> io::Result<()>;
}
struct Native {
    owner: Option<LongForwardOwnerV1>,
}
impl Backend for Native {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ForwardRun> {
        self.owner
            .as_mut()
            .ok_or_else(|| io::Error::other("long owner consumed"))?
            .run(input)
            .map_err(io::Error::other)
    }
    fn close(&mut self) -> io::Result<()> {
        self.owner
            .take()
            .ok_or_else(|| io::Error::other("long owner consumed"))?
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

fn observation(
    run: ForwardRun,
    request: &wire::Request,
    chain: &mut Chain,
) -> io::Result<(Response, Control, Vec<u8>)> {
    let c = run.completion;
    let Command::Forward {
        generation, token, ..
    } = &request.command
    else {
        return Err(io::Error::other("long observation non-forward"));
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
        return Err(io::Error::other(
            "actual long observation shape or identity",
        ));
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
            .map_err(|_| io::Error::other("long control layer count"))?,
    };
    control.validate()?;
    // Native Active already checked every rank row, all finite values and the
    // full-logit argmax on every forward, including those not selected for IPC.
    let mut bytes = Vec::with_capacity(OBSERVATION_BYTES);
    for row in run.layer_hidden {
        bytes.extend_from_slice(&row);
    }
    bytes.extend_from_slice(&run.final_normalized);
    bytes.extend_from_slice(&run.logits);
    let captured = wire::capture_position(c.position);
    let mut completion = Completion {
        generation: c.generation,
        position: c.position,
        input_token: c.input_token,
        output_token: c.output_token,
        control: part(&control.encode()),
        observation: part(&bytes),
        capture: if captured {
            Some(Payload::from_bytes(&bytes)?)
        } else {
            None
        },
        chain: [0; 32],
    };
    completion.chain = chain.advance(&completion);
    if !captured {
        bytes.clear();
    }
    Ok((
        response(request, Event::Completed(completion), false),
        control,
        bytes,
    ))
}

fn healthy_close(
    backend: &mut impl Backend,
    writer: &mut impl Write,
    outgoing: &mut FrameBudget,
    request: &wire::Request,
    chain: &Chain,
) -> io::Result<()> {
    backend.close()?;
    wire::write_response(
        writer,
        outgoing,
        &response(
            request,
            Event::Closed {
                completed_forwards: wire::FORWARDS,
                transcript_sha256: chain.digest(),
            },
            true,
        ),
        None,
        &[],
    )
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
    let mut last = None;
    for expected in 1..=u64::from(wire::FORWARDS) + 1 {
        let request = wire::read_request(reader, incoming)?
            .ok_or_else(|| io::Error::other("EOF before explicit long Close"))?;
        if request.id != expected
            || request.device_ids != bootstrap.device_ids
            || request.session != bootstrap.scope.session
            || request.registration != registration
            || request.profile_sha256 != profile
        {
            return Err(io::Error::other(
                "long request differs from sealed profile/session",
            ));
        }
        match &request.command {
            Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } => {
                let wanted = if expected <= wire::PROMPT_TOKENS as u64 {
                    bootstrap.prompt_tokens[expected as usize - 1]
                } else {
                    last.ok_or_else(|| io::Error::other("long predecessor missing"))?
                };
                if *token != wanted {
                    return Err(io::Error::other(
                        "long input differs from prompt/committed output",
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
                let (response, control, payload) = observation(run, &request, &mut chain)?;
                let Event::Completed(c) = &response.event else {
                    unreachable!()
                };
                last = Some(c.output_token);
                // Lost response is fatal after commit; no resubmission path exists.
                wire::write_response(writer, &mut outgoing, &response, Some(&control), &payload)?;
            }
            Command::Close => {
                if expected != u64::from(wire::FORWARDS) + 1 {
                    return Err(io::Error::other("premature long Close"));
                }
                return healthy_close(backend, writer, &mut outgoing, &request, &chain);
            }
        }
    }
    Err(io::Error::other("long Close missing"))
}

#[cfg(test)]
#[path = "native_long_cli_v1_tests.rs"]
mod tests;
