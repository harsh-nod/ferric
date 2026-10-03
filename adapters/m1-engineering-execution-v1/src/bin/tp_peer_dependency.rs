//! Parent-only ownership, negotiation and all-or-terminal receipt binding.

use super::{
    Connection, Incoming, LoadedKernel, Outgoing, RuntimeOptions, dependency_wire as wire,
};
use super::{
    EngineeringTp2CollectiveReceiptV1 as Receipt, EngineeringTp2CollectiveRequestV1 as Request,
};
use super::{EngineeringTpArgumentV1 as Arg, EngineeringTpDispatchV1, TpResult, pack_dispatch};
use fe2o3_kfd::engineering_wire::{CommandV1, SequenceDispatchV1};
use ferric_engine::tensor_parallel::Qwen3TensorParallelCollectiveV1;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const COLLECTIVE_IPC_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct State {
    ids: [u64; 2],
    next_generation: u64,
    last_frontiers: [u64; 2],
    pub(super) kernels: [BTreeMap<String, LoadedKernel>; 2],
}

impl State {
    pub(super) fn new(ids: [u64; 2]) -> Self {
        Self {
            ids,
            next_generation: 1,
            last_frontiers: [0; 2],
            kernels: std::array::from_fn(|_| BTreeMap::new()),
        }
    }
}

pub(super) fn validate_options(ids: &[u64], options: RuntimeOptions) -> TpResult<()> {
    if ids.len() != 2
        || ids.contains(&0)
        || ids[0] == ids[1]
        || options.operational
        || options.sequences
        || options.ordered_batches
        || options.full_forward
        || options.rollover
        || options.shared_full_currentness
        || options.profile
    {
        return Err(
            "TP2 dependency requires exact two devices and full-currentness serial setup".into(),
        );
    }
    Ok(())
}

pub(super) fn valid_ready(header: &wire::Response, ids: &[u64], pid: u32) -> bool {
    matches!(header,wire::Response::Ready {protocol,mode,target,unique_ids,process_id,authority,
        producer_root,consumer_root,currentness,control_allocation_flags,collective_timeout_ms}
        if ids.len()==2 && ids[0]!=0 && ids[1]!=0 && ids[0]!=ids[1]
            && *protocol==wire::PROTOCOL && mode==wire::MODE && target=="gfx950:xnack-"
            && unique_ids==ids && *process_id==pid && authority=="none"
            && producer_root==wire::PRODUCER_ROOT && consumer_root==wire::CONSUMER_ROOT
            && currentness=="full" && *control_allocation_flags==wire::CONTROL_ALLOCATION_FLAGS
            && *collective_timeout_ms==wire::TIMEOUT_MS)
}

fn identity(request: &Request, request_id: u64, generation: u64) -> wire::Identity {
    wire::Identity {
        request_id,
        generation,
        group_id: request.key.group_id,
        model_role: wire::ModelRole::Target8b,
        epoch: request.key.epoch,
        layer: request.key.layer,
        operation: match request.key.operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => {
                wire::Operation::AttentionOutputSum
            }
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => {
                wire::Operation::FeedForwardDownSum
            }
        },
    }
}

fn pack(
    connection: &Connection,
    rank: usize,
    dispatch: &EngineeringTpDispatchV1,
    producer: bool,
) -> TpResult<(SequenceDispatchV1, Vec<u8>)> {
    let state = connection
        .dependency
        .as_ref()
        .ok_or("TP2 dependency not negotiated")?;
    let expected = if producer {
        wire::PRODUCER_ROOT
    } else {
        wire::CONSUMER_ROOT
    };
    if dispatch.kernel != expected {
        return Err("TP2 dependency native profile is MFMA-only".into());
    }
    validate_buffers(connection, rank, dispatch, producer)?;
    let kernel = state.kernels[rank]
        .get(expected)
        .ok_or("TP2 dependency kernel is not admitted")?;
    let other = state.kernels[1 - rank]
        .get(expected)
        .ok_or("TP2 dependency peer kernel is not admitted")?;
    if kernel.image != other.image {
        return Err("TP2 dependency producer/consumer image differs across ranks".into());
    }
    let (command, bytes) = pack_dispatch(kernel, dispatch, &connection.capacities)?;
    let CommandV1::Dispatch {
        kernel,
        payload_bytes,
        workgroup,
        grid,
        pointers,
        ..
    } = command
    else {
        return Err("TP2 dependency dispatch packing drift".into());
    };
    Ok((
        SequenceDispatchV1 {
            kernel,
            payload_bytes,
            workgroup,
            grid,
            pointers,
            timeout_ms: wire::TIMEOUT_MS,
        },
        bytes,
    ))
}

fn validate_buffers(
    connection: &Connection,
    rank: usize,
    dispatch: &EngineeringTpDispatchV1,
    producer: bool,
) -> TpResult<()> {
    for (index, argument) in dispatch.arguments.iter().enumerate() {
        if let Arg::Buffer {
            id,
            elements,
            element_bytes,
            ..
        } = argument
        {
            let (owner, peers) = *connection
                .ownership
                .get(id)
                .ok_or("TP2 dependency buffer owner missing")?;
            let (expected_owner, needs_peer, expected_capacity) = if producer {
                (
                    rank,
                    index == 2,
                    elements.checked_mul(*element_bytes as usize),
                )
            } else if index < 8 {
                (if index < 2 { index } else { 0 }, true, Some(16 * 4096 * 4))
            } else {
                (rank, false, Some(16 * 4096 * 2))
            };
            if owner != expected_owner
                || needs_peer && !peers
                || expected_capacity.is_none()
                || connection.capacities.get(id).copied() != expected_capacity
            {
                return Err(
                    "TP2 dependency exact allocation owner, capacity or peer mapping mismatch"
                        .into(),
                );
            }
        }
    }
    Ok(())
}

fn prepare(connection: &Connection, request: &Request) -> TpResult<(wire::Collective, Vec<u8>)> {
    request.validate()?;
    let state = connection
        .dependency
        .as_ref()
        .ok_or("TP2 dependency support was not negotiated")?;
    if connection.failed
        || connection.exited
        || connection.closed.iter().any(|closed| *closed)
        || connection.pending.iter().any(Option::is_some)
        || connection.completed.iter().any(Option::is_some)
        || !connection.order.is_empty()
        || !connection.queued_round.is_empty()
        || connection.concurrent_rounds
    {
        return Err("TP2 dependency requires an idle live whole group".into());
    }
    let (p0, b0) = pack(connection, 0, &request.producers[0], true)?;
    let (p1, b1) = pack(connection, 1, &request.producers[1], true)?;
    let (c0, b2) = pack(connection, 0, &request.consumers[0], false)?;
    let (c1, b3) = pack(connection, 1, &request.consumers[1], false)?;
    let collective = wire::Collective {
        identity: identity(request, connection.next_request, state.next_generation),
        producers: [p0, p1],
        consumers: [c0, c1],
    };
    let expected = collective
        .payload_bytes()
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(expected);
    for part in [b0, b1, b2, b3] {
        bytes.extend(part);
    }
    if bytes.len() != expected {
        return Err("TP2 dependency aggregate payload mismatch".into());
    }
    Ok((collective, bytes))
}

fn validate_receipt(
    state: &State,
    request: &Request,
    expected: wire::Identity,
    actual: &wire::Receipt,
) -> TpResult<Receipt> {
    actual.validate().map_err(|error| error.to_string())?;
    if actual.identity != expected
        || actual.queues.iter().enumerate().any(|(rank, queue)| {
            queue.unique_id != state.ids[rank] || queue.first_packet < state.last_frontiers[rank]
        })
    {
        return Err(
            "TP2 dependency completion key, generation, device or frontier is stale".into(),
        );
    }
    let receipt = Receipt {
        key: request.key,
        generation: actual.identity.generation,
        kernel_dispatches: actual.kernel_counts,
        barrier_packets: actual.barrier_counts,
        packet_counts: actual.packet_counts,
        completion_values: std::array::from_fn(|slot| actual.completion_values[slot / 3][slot % 3]),
        queue_epochs: actual.queues.map(|queue| queue.queue_epoch),
        first_packet_ids: actual.queues.map(|queue| queue.first_packet),
        frontiers: actual.final_frontiers,
    };
    receipt.validate_for(request)?;
    Ok(receipt)
}

pub(super) fn execute(connection: &mut Connection, request: &Request) -> TpResult<Receipt> {
    let (collective, bytes) = match prepare(connection, request) {
        Ok(value) => value,
        Err(error) => return connection.reject(error),
    };
    transact(connection, request, collective, bytes)
}

fn transact(
    connection: &mut Connection,
    request: &Request,
    collective: wire::Collective,
    bytes: Vec<u8>,
) -> TpResult<Receipt> {
    let identity = collective.identity;
    let Some(next_request) = connection.next_request.checked_add(1) else {
        return connection.reject("TP2 dependency request counter exhausted");
    };
    let Some(next_generation) = identity.generation.checked_add(1) else {
        return connection.reject("TP2 dependency generation exhausted");
    };
    let deadline = Instant::now() + COLLECTIVE_IPC_TIMEOUT.min(connection.timeout);
    connection.pending.fill(Some(identity.request_id));
    connection.next_request = next_request;
    let mut ticket = connection
        .timing
        .request(None, "tp2_dependency_collective", bytes.len(), 4);
    let Some(writer) = &connection.writer else {
        return connection.reject("TP2 dependency writer closed");
    };
    if writer
        .try_send(Outgoing::Dependency(
            wire::Request::Collective { collective },
            bytes,
        ))
        .is_err()
    {
        return connection.reject("TP2 dependency writer unavailable");
    }
    match connection
        .written
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
        Ok(Ok(())) => ticket.sent(true),
        Ok(Err(error)) => return connection.reject(error),
        Err(error) => {
            return connection.reject(format!("TP2 dependency write deadline/disconnect: {error}"));
        }
    }
    let incoming = match connection
        .reader
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return connection.reject(error),
        Err(error) => {
            return connection.reject(format!(
                "TP2 dependency completion deadline/disconnect: {error}"
            ));
        }
    };
    let Incoming::Dependency(wire::Response::CollectiveCompleted { receipt: actual }, payload) =
        incoming
    else {
        return connection.reject("TP2 dependency expected one whole-collective receipt");
    };
    if !payload.is_empty() {
        return connection.reject("TP2 dependency completion unexpectedly carried bytes");
    }
    let Some(state) = connection.dependency.as_ref() else {
        return connection.reject("TP2 dependency negotiation disappeared");
    };
    let receipt = match validate_receipt(state, request, identity, &actual) {
        Ok(receipt) => receipt,
        Err(error) => return connection.reject(error),
    };
    // Commit transport progress only after the entire identity and six-packet receipt.
    let state = connection
        .dependency
        .as_mut()
        .expect("validated dependency state");
    state.next_generation = next_generation;
    state.last_frontiers = actual.queues.map(|queue| queue.next_packet);
    connection.pending.fill(None);
    ticket.finish(0, true);
    Ok(receipt)
}

#[cfg(test)]
#[path = "tp_peer_dependency_tests.rs"]
mod tests;
