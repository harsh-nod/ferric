use crate::probe::{read, request};
use crate::publication_artifact::{self as abi, BYTES, GUARD};
use crate::session::Transport;
use crate::{artifact, Result};
use fe2o3_kfd::engineering_wire::{CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1};

pub struct Observation {
    pub data: [Vec<u8>; 5],
    pub elapsed_ns: u64,
}

fn initialized(inputs: &[u8], flags: &[u8], index: usize) -> Vec<u8> {
    let mut storage = vec![0xa5; BYTES[index] + 2 * GUARD];
    let body = &mut storage[GUARD..GUARD + BYTES[index]];
    match index {
        0 => body.fill(0xa7),
        1 => body.copy_from_slice(flags),
        2 => body.copy_from_slice(inputs),
        3 => body.fill(0xff),
        _ => body.fill(0xd3),
    }
    storage
}

fn read_buffers(
    transport: &mut impl Transport,
    buffers: &[u64; 5],
    inputs: &[u8],
) -> Result<[Vec<u8>; 5]> {
    let mut data = std::array::from_fn(|_| Vec::new());
    for (index, buffer) in buffers.iter().enumerate() {
        let storage = read(transport, *buffer, BYTES[index] + 2 * GUARD)?;
        if storage[..GUARD]
            .iter()
            .chain(&storage[GUARD + BYTES[index]..])
            .any(|byte| *byte != 0xa5)
        {
            return Err("publication allocation guard corruption".into());
        }
        data[index] = storage[GUARD..GUARD + BYTES[index]].to_vec();
    }
    if data[2] != inputs {
        return Err("publication immutable input changed".into());
    }
    Ok(data)
}

/// Exactly one attempt. No host access or cleanup follows uncertain completion.
pub fn run(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    case: &abi::Case,
    inputs: &[u8],
    flags: &[u8],
) -> Result<Observation> {
    abi::validate_metadata(metadata)?;
    abi::validate_inputs(case, inputs, flags)?;
    if artifact::digest(&object) != metadata.object_sha256 {
        return Err("publication object hash mismatch".into());
    }
    let loaded = request(
        transport,
        CommandV1::LoadKernel {
            payload_bytes: u32::try_from(object.len())
                .map_err(|_| "publication object too large")?,
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
        return Err("worker did not return publication metadata".into());
    };
    if kernel == 0 || observed != *metadata {
        return Err("worker publication metadata mismatch".into());
    }
    let mut buffers = [0; 5];
    for index in 0..5 {
        let bytes = (BYTES[index] + 2 * GUARD) as u64;
        let allocated = request(transport, CommandV1::Allocate { bytes }, vec![])?;
        let ResponseV1::Allocated {
            buffer,
            bytes: observed,
        } = allocated.response
        else {
            return Err("worker did not return publication allocation".into());
        };
        if buffer == 0 || buffers.contains(&buffer) || observed != bytes {
            return Err("publication allocation identity or extent mismatch".into());
        }
        buffers[index] = buffer;
        let storage = initialized(inputs, flags, index);
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
            return Err("publication initialization not acknowledged".into());
        }
    }
    let args = abi::kernarg(metadata, case)?;
    let lengths = case.lengths();
    let pointers = buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| PointerFixupV1 {
            kernarg_offset: u32::try_from(index * 16).expect("five pointers"),
            buffer: *buffer,
            buffer_offset: GUARD as u64,
            extent_bytes: lengths[index] * 4,
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
        return Err("worker did not observe publication completion".into());
    };
    let data = read_buffers(transport, &buffers, inputs)?;
    for buffer in buffers.into_iter().rev() {
        if !matches!(
            request(transport, CommandV1::Free { buffer }, vec![])?.response,
            ResponseV1::Freed
        ) {
            return Err("publication release not acknowledged".into());
        }
    }
    if !matches!(
        request(transport, CommandV1::Close, vec![])?.response,
        ResponseV1::Closed
    ) {
        return Err("publication cleanup not acknowledged".into());
    }
    Ok(Observation { data, elapsed_ns })
}
