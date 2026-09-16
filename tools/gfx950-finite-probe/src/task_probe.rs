use std::time::Instant;

use fe2o3_kfd::engineering_wire::{CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1};
use serde::Serialize;

use crate::artifact;
use crate::probe::{read, request};
use crate::session::Transport;
use crate::task_artifact::{self as abi, BUFFER_COUNT, EPOCH_COUNT, GUARD, STATE_WORDS};
use crate::Result;

#[derive(Debug, Serialize)]
pub struct EpochObservation {
    pub expected_epoch: u32,
    pub initialized_epoch: u32,
    pub stale_epoch_negative: bool,
    pub state: [u32; STATE_WORDS],
    pub task_owners: [u32; 7],
    pub distinct_observed_workgroups: u32,
    pub cross_workgroup_dependency_edges: Vec<[u32; 2]>,
    pub worker_dispatch_interval_ns: u64,
    pub host_dispatch_roundtrip_ns: u64,
    pub host_epoch_reset_dispatch_readback_ns: u64,
}

pub fn elapsed(started: Instant) -> Result<u64> {
    u64::try_from(started.elapsed().as_nanos()).map_err(|_| "host timing exceeds u64".into())
}

fn write(transport: &mut impl Transport, buffer: u64, bytes: Vec<u8>) -> Result<()> {
    let written = request(
        transport,
        CommandV1::Write {
            buffer,
            offset: 0,
            payload_bytes: u32::try_from(bytes.len()).expect("bounded fixture"),
        },
        bytes,
    )?;
    if !matches!(written.response, ResponseV1::Written) {
        return Err("worker did not acknowledge task buffer initialization".into());
    }
    Ok(())
}

pub fn guarded(bytes: &[u8]) -> Vec<u8> {
    let mut storage = vec![0xa5; bytes.len() + 2 * GUARD];
    storage[GUARD..GUARD + bytes.len()].copy_from_slice(bytes);
    storage
}

pub fn initial_state(epoch: u32) -> [u32; STATE_WORDS] {
    let mut state = [0; STATE_WORDS];
    state[0] = epoch;
    state[1] = 1;
    state
}

pub fn owner_evidence(packed: u32) -> ([u32; 7], u32, Vec<[u32; 2]>) {
    let owners = std::array::from_fn(|task| (packed >> (2 * task)) & 3);
    let distinct = u32::from(owners.contains(&1)) + u32::from(owners.contains(&2));
    let edges = abi::EDGES
        .into_iter()
        .filter(|[producer, consumer]| {
            let producer = owners[usize::try_from(*producer).expect("seven tasks")];
            let consumer = owners[usize::try_from(*consumer).expect("seven tasks")];
            producer != 0 && consumer != 0 && producer != consumer
        })
        .collect();
    (owners, distinct, edges)
}

fn initialize_epoch(
    transport: &mut impl Transport,
    buffers: &[u64; BUFFER_COUNT],
    expected_epoch: u32,
    actual_epoch: u32,
) -> Result<()> {
    write(
        transport,
        buffers[1],
        guarded(&expected_epoch.to_le_bytes()),
    )?;
    for (index, value) in initial_state(actual_epoch).iter().enumerate() {
        write(transport, buffers[index + 2], guarded(&value.to_le_bytes()))?;
    }
    Ok(())
}

pub fn validate_state(state: &[u32; STATE_WORDS], expected: u32, actual: u32) -> Result<()> {
    if expected != actual {
        let mut rejected = initial_state(actual);
        rejected[5] = 1;
        if state != &rejected {
            return Err("stale epoch did not reject with only its exact error flag".into());
        }
    } else if state[0] != actual
        || state[1] != 0
        || state[2] != 127
        || state[3] != 127
        || state[5] != 0
        || state[4] >> 14 != 0
        || (0..7).any(|task| !matches!((state[4] >> (2 * task)) & 3, 1 | 2))
    {
        return Err("task graph completion, ownership or epoch state mismatch".into());
    }
    Ok(())
}

fn read_epoch(
    transport: &mut impl Transport,
    buffers: &[u64; BUFFER_COUNT],
    inputs: &[u8],
    expected: u32,
    actual: u32,
) -> Result<[u32; STATE_WORDS]> {
    let mut state = [0; STATE_WORDS];
    for (index, buffer) in buffers.iter().enumerate() {
        let bytes = abi::buffer_bytes(index);
        let storage = read(transport, *buffer, bytes + 2 * GUARD)?;
        if storage[..GUARD]
            .iter()
            .chain(&storage[GUARD + bytes..])
            .any(|byte| *byte != 0xa5)
        {
            return Err("task graph buffer guard corruption".into());
        }
        let data = &storage[GUARD..GUARD + bytes];
        if (index == 0 && data != inputs) || (index == 1 && data != expected.to_le_bytes()) {
            return Err("task graph immutable input or configuration changed".into());
        }
        if index >= 2 {
            state[index - 2] = u32::from_le_bytes(data.try_into().expect("one state word"));
        }
    }
    validate_state(&state, expected, actual)?;
    Ok(state)
}

fn pointers(buffers: &[u64; BUFFER_COUNT]) -> Vec<PointerFixupV1> {
    buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| PointerFixupV1 {
            kernarg_offset: abi::pointer_offset(index),
            buffer: *buffer,
            buffer_offset: GUARD as u64,
            extent_bytes: abi::buffer_bytes(index) as u64,
            access: abi::buffer_access(index),
        })
        .collect()
}

fn epochs(
    transport: &mut impl Transport,
    kernel: u64,
    metadata: &KernelMetadataV1,
    buffers: &[u64; BUFFER_COUNT],
    inputs: &[u8],
) -> Result<Vec<EpochObservation>> {
    let mut observations = Vec::with_capacity(EPOCH_COUNT);
    for expected in 1..=u32::try_from(EPOCH_COUNT).expect("five epochs") {
        let actual = expected.min(4);
        let started = Instant::now();
        initialize_epoch(transport, buffers, expected, actual)?;
        let args = abi::kernarg(metadata)?;
        let dispatch_started = Instant::now();
        let packet = request(
            transport,
            CommandV1::Dispatch {
                kernel,
                payload_bytes: u32::try_from(args.len()).expect("bounded kernarg"),
                workgroup: abi::WORKGROUP,
                grid: abi::GRID,
                pointers: pointers(buffers),
                timeout_ms: 10_000,
            },
            args,
        )?;
        let roundtrip_ns = elapsed(dispatch_started)?;
        let ResponseV1::Dispatched { elapsed_ns } = packet.response else {
            return Err("worker did not observe task graph dispatch completion".into());
        };
        let state = read_epoch(transport, buffers, inputs, expected, actual)?;
        let (task_owners, distinct_observed_workgroups, cross_workgroup_dependency_edges) =
            owner_evidence(state[4]);
        observations.push(EpochObservation {
            expected_epoch: expected,
            initialized_epoch: actual,
            stale_epoch_negative: expected != actual,
            state,
            task_owners,
            distinct_observed_workgroups,
            cross_workgroup_dependency_edges,
            worker_dispatch_interval_ns: elapsed_ns,
            host_dispatch_roundtrip_ns: roundtrip_ns,
            host_epoch_reset_dispatch_readback_ns: elapsed(started)?,
        });
    }
    Ok(observations)
}

/// Reuse one finite worker queue and fifteen disjoint allocations for five epochs.
pub fn run(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    inputs: &[u8],
) -> Result<Vec<EpochObservation>> {
    abi::validate_metadata(metadata)?;
    abi::validate_inputs(inputs)?;
    if artifact::digest(&object) != metadata.object_sha256 {
        return Err("task graph object hash mismatch".into());
    }
    let loaded = request(
        transport,
        CommandV1::LoadKernel {
            payload_bytes: u32::try_from(object.len()).map_err(|_| "task object too large")?,
            object_sha256: metadata.object_sha256,
            symbol: abi::SYMBOL.into(),
        },
        object,
    )?;
    let ResponseV1::LoadedKernel {
        kernel,
        metadata: observed,
    } = loaded.response
    else {
        return Err("worker did not return task graph metadata".into());
    };
    if kernel == 0 || observed != *metadata {
        return Err("worker task graph metadata differs from inspected artifact".into());
    }
    let mut buffers = [0; BUFFER_COUNT];
    for index in 0..BUFFER_COUNT {
        let bytes = (abi::buffer_bytes(index) + 2 * GUARD) as u64;
        let allocated = request(transport, CommandV1::Allocate { bytes }, vec![])?;
        let ResponseV1::Allocated {
            buffer,
            bytes: observed,
        } = allocated.response
        else {
            return Err("worker did not return task graph allocation".into());
        };
        if buffer == 0 || buffers.contains(&buffer) || observed != bytes {
            return Err("task graph allocation identity or extent mismatch".into());
        }
        buffers[index] = buffer;
    }
    write(transport, buffers[0], guarded(inputs))?;
    let observations = epochs(transport, kernel, metadata, &buffers, inputs)?;
    for buffer in buffers.into_iter().rev() {
        if !matches!(
            request(transport, CommandV1::Free { buffer }, vec![])?.response,
            ResponseV1::Freed
        ) {
            return Err("worker did not acknowledge task graph buffer release".into());
        }
    }
    if !matches!(
        request(transport, CommandV1::Close, vec![])?.response,
        ResponseV1::Closed
    ) {
        return Err("worker did not acknowledge task graph cleanup".into());
    }
    Ok(observations)
}
