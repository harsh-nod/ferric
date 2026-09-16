use fe2o3_kfd::engineering_wire::{CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1};

use crate::atomic_artifact::{self as abi, BYTES, GUARD};
use crate::probe::{read, request};
use crate::session::Transport;
use crate::{artifact, Result};

pub struct Observation {
    pub channels: Vec<u8>,
    pub output: Vec<u8>,
    pub elapsed_ns: u64,
}

fn initialized(inputs: &[u8], index: usize) -> Vec<u8> {
    let mut storage = vec![0xa5; BYTES + 2 * GUARD];
    // Every expected word differs from both poisons, including zero and all ones.
    let mask = [0x5a, 0, 0xc3][index];
    for (dest, value) in storage[GUARD..GUARD + BYTES].iter_mut().zip(inputs) {
        *dest = *value ^ mask;
    }
    storage
}

fn read_buffers(
    transport: &mut impl Transport,
    buffers: &[u64; 3],
    inputs: &[u8],
) -> Result<[Vec<u8>; 3]> {
    let mut data = std::array::from_fn(|_| Vec::new());
    for (index, buffer) in buffers.iter().enumerate() {
        let storage = read(transport, *buffer, BYTES + 2 * GUARD)?;
        if storage[..GUARD]
            .iter()
            .chain(&storage[GUARD + BYTES..])
            .any(|byte| *byte != 0xa5)
        {
            return Err("atomic channel allocation guard corruption".into());
        }
        data[index] = storage[GUARD..GUARD + BYTES].to_vec();
    }
    if data[1] != inputs {
        return Err("atomic channel immutable input changed".into());
    }
    Ok(data)
}

/// One fixed dispatch. Errors terminate the owning worker; they never retry.
pub fn run(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    inputs: &[u8],
) -> Result<Observation> {
    abi::validate_metadata(metadata)?;
    abi::validate_inputs(inputs)?;
    if artifact::digest(&object) != metadata.object_sha256 {
        return Err("atomic channel object hash mismatch".into());
    }
    let loaded = request(
        transport,
        CommandV1::LoadKernel {
            payload_bytes: u32::try_from(object.len()).map_err(|_| "atomic object too large")?,
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
        return Err("worker did not return atomic channel metadata".into());
    };
    if kernel == 0 || observed != *metadata {
        return Err("worker atomic channel metadata differs from inspected artifact".into());
    }
    let mut buffers = [0; 3];
    for index in 0..3 {
        let bytes = (BYTES + 2 * GUARD) as u64;
        let allocated = request(transport, CommandV1::Allocate { bytes }, vec![])?;
        let ResponseV1::Allocated {
            buffer,
            bytes: observed,
        } = allocated.response
        else {
            return Err("worker did not return atomic channel allocation".into());
        };
        if buffer == 0 || buffers.contains(&buffer) || observed != bytes {
            return Err("atomic channel allocation identity or extent mismatch".into());
        }
        buffers[index] = buffer;
        let storage = initialized(inputs, index);
        let written = request(
            transport,
            CommandV1::Write {
                buffer,
                offset: 0,
                payload_bytes: u32::try_from(storage.len()).expect("bounded fixture"),
            },
            storage,
        )?;
        if !matches!(written.response, ResponseV1::Written) {
            return Err("worker did not acknowledge atomic buffer initialization".into());
        }
    }
    let args = abi::kernarg(metadata)?;
    let pointers = buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| PointerFixupV1 {
            kernarg_offset: u32::try_from(index * 16).expect("three pointers"),
            buffer: *buffer,
            buffer_offset: GUARD as u64,
            extent_bytes: BYTES as u64,
            access: abi::ACCESS[index],
        })
        .collect();
    let dispatched = request(
        transport,
        CommandV1::Dispatch {
            kernel,
            payload_bytes: u32::try_from(args.len()).expect("bounded ABI"),
            workgroup: abi::WORKGROUP,
            grid: abi::GRID,
            pointers,
            timeout_ms: 10_000,
        },
        args,
    )?;
    let ResponseV1::Dispatched { elapsed_ns } = dispatched.response else {
        return Err("worker did not observe atomic channel dispatch completion".into());
    };
    let [channels, _, output] = read_buffers(transport, &buffers, inputs)?;
    for buffer in buffers.into_iter().rev() {
        if !matches!(
            request(transport, CommandV1::Free { buffer }, vec![])?.response,
            ResponseV1::Freed
        ) {
            return Err("worker did not acknowledge atomic buffer release".into());
        }
    }
    if !matches!(
        request(transport, CommandV1::Close, vec![])?.response,
        ResponseV1::Closed
    ) {
        return Err("worker did not acknowledge atomic channel cleanup".into());
    }
    Ok(Observation {
        channels,
        output,
        elapsed_ns,
    })
}
