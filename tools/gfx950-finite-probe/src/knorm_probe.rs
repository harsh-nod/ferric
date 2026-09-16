use fe2o3_kfd::engineering_wire::{
    BufferAccessV1, CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1,
};

use crate::{
    artifact, knorm_artifact as abi, kproj_artifact,
    probe::{read, request},
    session::Transport,
    Result,
};

pub struct Observation {
    pub projection: Vec<u8>,
    pub quantized: Vec<u8>,
    pub output: Vec<u8>,
    pub elapsed_ns: [u64; 2],
}

pub struct Kernel<'a> {
    pub object: Vec<u8>,
    pub metadata: &'a KernelMetadataV1,
}

pub struct Inputs<'a> {
    pub hidden: &'a [u8],
    pub weights: &'a [u8],
    pub norm_weights: &'a [u8],
}

fn load(transport: &mut impl Transport, kernel: Kernel<'_>) -> Result<u64> {
    if artifact::digest(&kernel.object) != kernel.metadata.object_sha256 {
        return Err("Qwen3 chain object digest mismatch".into());
    }
    let loaded = request(
        transport,
        CommandV1::LoadKernel {
            payload_bytes: u32::try_from(kernel.object.len())
                .map_err(|_| "code object too large")?,
            object_sha256: kernel.metadata.object_sha256,
            symbol: kernel.metadata.symbol.clone(),
        },
        kernel.object,
    )?;
    let ResponseV1::LoadedKernel {
        kernel: id,
        metadata,
    } = loaded.response
    else {
        return Err("worker did not return Qwen3 chain kernel metadata".into());
    };
    if id == 0 || metadata != *kernel.metadata {
        return Err("worker Qwen3 chain artifact metadata mismatch".into());
    }
    Ok(id)
}

fn guarded(bytes: &[u8]) -> Vec<u8> {
    let mut result = vec![0xa5; bytes.len() + 2 * abi::GUARD];
    result[abi::GUARD..abi::GUARD + bytes.len()].copy_from_slice(bytes);
    result
}

fn initialized(inputs: &Inputs<'_>) -> [Vec<u8>; 6] {
    [
        guarded(inputs.hidden),
        guarded(inputs.weights),
        guarded(&0x7fc0_1234_u32.to_le_bytes().repeat(1024)),
        guarded(inputs.norm_weights),
        guarded(&0x7fc1_u16.to_le_bytes().repeat(1024)),
        guarded(&0x7fc2_u16.to_le_bytes().repeat(1024)),
    ]
}

fn read_checked(
    transport: &mut impl Transport,
    buffers: &[u64; 6],
    index: usize,
) -> Result<Vec<u8>> {
    let data = read(
        transport,
        buffers[index],
        abi::BYTES[index] + 2 * abi::GUARD,
    )?;
    if data[..abi::GUARD]
        .iter()
        .chain(&data[abi::GUARD + abi::BYTES[index]..])
        .any(|byte| *byte != 0xa5)
    {
        return Err("Qwen3 chain allocation guard corruption".into());
    }
    Ok(data[abi::GUARD..abi::GUARD + abi::BYTES[index]].to_vec())
}

fn finite(bytes: &[u8], wide: bool) -> Result<()> {
    let bad = if wide {
        bytes
            .chunks_exact(4)
            .any(|word| !f32::from_le_bytes(word.try_into().expect("f32 width")).is_finite())
    } else {
        bytes
            .chunks_exact(2)
            .any(|word| u16::from_le_bytes(word.try_into().expect("bf16 width")) & 0x7f80 == 0x7f80)
    };
    if bad {
        return Err("Qwen3 chain contains nonfinite or unwritten values".into());
    }
    Ok(())
}

fn dispatch(
    transport: &mut impl Transport,
    kernel: u64,
    args: Vec<u8>,
    buffers: &[u64; 6],
    consumer: bool,
) -> Result<u64> {
    let indices: &[usize] = if consumer { &[2, 3, 4, 5] } else { &[0, 1, 2] };
    let pointers = indices
        .iter()
        .enumerate()
        .map(|(argument, index)| PointerFixupV1 {
            kernarg_offset: u32::try_from(argument * 16).expect("four arguments"),
            buffer: buffers[*index],
            buffer_offset: abi::GUARD as u64,
            extent_bytes: abi::BYTES[*index] as u64,
            access: if argument < 2 {
                BufferAccessV1::Read
            } else {
                BufferAccessV1::Write
            },
        })
        .collect();
    let packet = request(
        transport,
        CommandV1::Dispatch {
            kernel,
            payload_bytes: u32::try_from(args.len()).expect("bounded ABI"),
            workgroup: abi::WORKGROUP,
            grid: if consumer {
                abi::GRID
            } else {
                kproj_artifact::WAVE64_GRID
            },
            pointers,
            timeout_ms: 10_000,
        },
        args,
    )?;
    let ResponseV1::Dispatched { elapsed_ns } = packet.response else {
        return Err("Qwen3 chain did not observe exact dispatch completion".into());
    };
    Ok(elapsed_ns)
}

/// A fixed completion-ordered graph, not an in-dispatch publication protocol.
pub fn run(
    transport: &mut impl Transport,
    producer: Kernel<'_>,
    consumer: Kernel<'_>,
    inputs: &Inputs<'_>,
) -> Result<Observation> {
    kproj_artifact::validate_abi(producer.metadata)?;
    if kproj_artifact::variant(producer.metadata)? != kproj_artifact::Variant::Wave64 {
        return Err("Qwen3 chain requires its exact wave64 projection".into());
    }
    abi::validate_consumer(consumer.metadata)?;
    if inputs.hidden.len() != abi::BYTES[0]
        || inputs.weights.len() != abi::BYTES[1]
        || inputs.norm_weights.len() != abi::BYTES[3]
    {
        return Err("Qwen3 chain input extent mismatch".into());
    }
    for data in [inputs.hidden, inputs.weights, inputs.norm_weights] {
        finite(data, false)?;
    }
    let producer_args = kproj_artifact::kernarg(producer.metadata)?;
    let consumer_args = abi::kernarg(consumer.metadata)?;
    let producer_id = load(transport, producer)?;
    let consumer_id = load(transport, consumer)?;
    if producer_id == consumer_id {
        return Err("worker reused a live kernel identity".into());
    }
    let data = initialized(inputs);
    let mut buffers = [0_u64; 6];
    for (index, storage) in data.iter().enumerate() {
        let allocation = request(
            transport,
            CommandV1::Allocate {
                bytes: storage.len() as u64,
            },
            vec![],
        )?;
        let ResponseV1::Allocated { buffer, bytes } = allocation.response else {
            return Err("worker did not return Qwen3 chain allocation".into());
        };
        if buffer == 0 || buffers.contains(&buffer) || bytes != storage.len() as u64 {
            return Err("Qwen3 chain allocation identity or extent mismatch".into());
        }
        buffers[index] = buffer;
        if !matches!(
            request(
                transport,
                CommandV1::Write {
                    buffer,
                    offset: 0,
                    payload_bytes: u32::try_from(storage.len()).expect("bounded allocation"),
                },
                storage.clone()
            )?
            .response,
            ResponseV1::Written
        ) {
            return Err("worker did not acknowledge Qwen3 chain initialization".into());
        }
    }
    let producer_elapsed = dispatch(transport, producer_id, producer_args, &buffers, false)?;
    // Capture the completed producer output without writing or reallocating it.
    // The next dispatch binds this exact allocation under a readonly contract.
    let projection = read_checked(transport, &buffers, 2)?;
    finite(&projection, true)?;
    let consumer_elapsed = dispatch(transport, consumer_id, consumer_args, &buffers, true)?;
    for (index, expected) in [
        (0, inputs.hidden),
        (1, inputs.weights),
        (3, inputs.norm_weights),
    ] {
        if read_checked(transport, &buffers, index)? != expected {
            return Err("Qwen3 chain readonly input changed".into());
        }
    }
    if read_checked(transport, &buffers, 2)? != projection {
        return Err("Qwen3 norm consumer modified its completed projection input".into());
    }
    let quantized = read_checked(transport, &buffers, 4)?;
    let output = read_checked(transport, &buffers, 5)?;
    finite(&quantized, false)?;
    finite(&output, false)?;
    for buffer in buffers.into_iter().rev() {
        if !matches!(
            request(transport, CommandV1::Free { buffer }, vec![])?.response,
            ResponseV1::Freed
        ) {
            return Err("worker did not acknowledge Qwen3 chain release".into());
        }
    }
    if !matches!(
        request(transport, CommandV1::Close, vec![])?.response,
        ResponseV1::Closed
    ) {
        return Err("worker did not acknowledge Qwen3 chain cleanup".into());
    }
    Ok(Observation {
        projection,
        quantized,
        output,
        elapsed_ns: [producer_elapsed, consumer_elapsed],
    })
}
