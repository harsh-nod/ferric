use fe2o3_kfd::engineering_wire::{
    BufferAccessV1, CommandV1, KernelMetadataV1, PointerFixupV1, ResponseV1, PROTOCOL_VERSION_V1,
    TARGET_V1,
};

use crate::artifact::{self, BYTES, GRID, GUARD, WORKGROUP};
use crate::session::{Packet, Transport};
use crate::Result;

pub struct Observation {
    pub output: Vec<u8>,
    pub elapsed_ns: u64,
}

pub fn ready(packet: &Packet, unique_id: u64) -> Result<()> {
    if !matches!(&packet.response, ResponseV1::Ready {
        protocol, target, device_unique_id, authority,
    } if *protocol == PROTOCOL_VERSION_V1 && target == TARGET_V1
        && *device_unique_id == unique_id && authority == "none")
        || !packet.payload.is_empty()
    {
        return Err("engineering worker identity, protocol, target or authority mismatch".into());
    }
    Ok(())
}

pub(crate) fn request(
    transport: &mut impl Transport,
    command: CommandV1,
    payload: Vec<u8>,
) -> Result<Packet> {
    if command
        .payload_bytes()
        .map_err(|_| "invalid protocol command")?
        != payload.len()
    {
        return Err("invalid protocol payload length".into());
    }
    let packet = transport.request(command, payload)?;
    if !matches!(packet.response, ResponseV1::Read { .. }) && !packet.payload.is_empty() {
        return Err("unexpected worker response payload".into());
    }
    Ok(packet)
}

pub(crate) fn read(transport: &mut impl Transport, buffer: u64, bytes: usize) -> Result<Vec<u8>> {
    let bytes_u32 = u32::try_from(bytes).map_err(|_| "read exceeds supported width")?;
    let packet = request(
        transport,
        CommandV1::Read {
            buffer,
            offset: 0,
            bytes: bytes_u32,
        },
        vec![],
    )?;
    if !matches!(packet.response, ResponseV1::Read { payload_bytes } if payload_bytes == bytes_u32)
        || packet.payload.len() != bytes
    {
        return Err("worker readback extent mismatch".into());
    }
    Ok(packet.payload)
}

/// One finite dispatch. Any error terminates the owning worker; no retry path.
pub fn run(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    inputs: &[u8],
    weights: &[u8],
) -> Result<Observation> {
    artifact::validate_abi(metadata)?;
    run_fixed(
        transport,
        object,
        metadata,
        inputs,
        weights,
        FixedDispatch {
            bytes: BYTES,
            grid: GRID,
            kernarg: artifact::kernarg(metadata)?,
            guard_inputs: false,
        },
    )
}

pub fn run_kproj(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    inputs: &[u8],
    weights: &[u8],
) -> Result<Observation> {
    crate::kproj_artifact::validate_abi(metadata)?;
    run_fixed(
        transport,
        object,
        metadata,
        inputs,
        weights,
        FixedDispatch {
            bytes: crate::kproj_artifact::BYTES,
            grid: crate::kproj_artifact::variant(metadata)?.grid(),
            kernarg: crate::kproj_artifact::kernarg(metadata)?,
            guard_inputs: true,
        },
    )
}

// Only the two validated, fixed contracts above can select a dispatch layout.
struct FixedDispatch {
    bytes: [usize; 3],
    grid: [u32; 3],
    kernarg: Vec<u8>,
    guard_inputs: bool,
}

fn run_fixed(
    transport: &mut impl Transport,
    object: Vec<u8>,
    metadata: &KernelMetadataV1,
    inputs: &[u8],
    weights: &[u8],
    layout: FixedDispatch,
) -> Result<Observation> {
    if artifact::digest(&object) != metadata.object_sha256
        || inputs.len() != layout.bytes[0]
        || weights.len() != layout.bytes[1]
    {
        return Err("artifact digest or input extents mismatch".into());
    }
    let loaded = request(
        transport,
        CommandV1::LoadKernel {
            payload_bytes: u32::try_from(object.len()).map_err(|_| "code object is too large")?,
            object_sha256: metadata.object_sha256,
            symbol: metadata.symbol.clone(),
        },
        object,
    )?;
    let ResponseV1::LoadedKernel {
        kernel,
        metadata: observed,
    } = loaded.response
    else {
        return Err("worker did not return LoadedKernel".into());
    };
    if kernel == 0 || observed != *metadata {
        return Err("worker kernel metadata differs from offline exact artifact inspection".into());
    }

    let mut guarded = vec![0xa5; layout.bytes[2] + GUARD * 2];
    for word in guarded[GUARD..GUARD + layout.bytes[2]].chunks_exact_mut(4) {
        word.copy_from_slice(&0x7fc0_1234_u32.to_le_bytes());
    }
    let protect_input = |bytes: &[u8]| {
        if layout.guard_inputs {
            let mut protected = vec![0xa5; bytes.len() + GUARD * 2];
            protected[GUARD..GUARD + bytes.len()].copy_from_slice(bytes);
            protected
        } else {
            bytes.to_vec()
        }
    };
    let data = [protect_input(inputs), protect_input(weights), guarded];
    let mut buffers = [0_u64; 3];
    for (index, bytes) in data.iter().enumerate() {
        let allocated = request(
            transport,
            CommandV1::Allocate {
                bytes: bytes.len() as u64,
            },
            vec![],
        )?;
        let ResponseV1::Allocated {
            buffer,
            bytes: extent,
        } = allocated.response
        else {
            return Err("worker did not return Allocated".into());
        };
        if buffer == 0 || buffers.contains(&buffer) || extent != bytes.len() as u64 {
            return Err("worker returned invalid allocation identity or extent".into());
        }
        buffers[index] = buffer;
        let written = request(
            transport,
            CommandV1::Write {
                buffer,
                offset: 0,
                payload_bytes: u32::try_from(bytes.len()).expect("bounded fixture"),
            },
            bytes.clone(),
        )?;
        if !matches!(written.response, ResponseV1::Written) {
            return Err("worker did not acknowledge initialization".into());
        }
    }
    let pointers = buffers
        .iter()
        .enumerate()
        .map(|(index, buffer)| PointerFixupV1 {
            kernarg_offset: u32::try_from(index * 16).expect("three slices"),
            buffer: *buffer,
            buffer_offset: if index == 2 || layout.guard_inputs {
                GUARD as u64
            } else {
                0
            },
            extent_bytes: layout.bytes[index] as u64,
            access: if index == 2 {
                BufferAccessV1::Write
            } else {
                BufferAccessV1::Read
            },
        })
        .collect();
    let args = layout.kernarg;
    let dispatched = request(
        transport,
        CommandV1::Dispatch {
            kernel,
            payload_bytes: u32::try_from(args.len()).expect("bounded kernarg"),
            workgroup: WORKGROUP,
            grid: layout.grid,
            pointers,
            timeout_ms: 10_000,
        },
        args,
    )?;
    let ResponseV1::Dispatched { elapsed_ns } = dispatched.response else {
        return Err("worker did not observe dispatch completion".into());
    };
    for index in 0..2 {
        if read(transport, buffers[index], data[index].len())? != data[index] {
            return Err("read-only input changed during dispatch".into());
        }
    }
    let readback = read(transport, buffers[2], data[2].len())?;
    if readback[..GUARD]
        .iter()
        .chain(&readback[GUARD + layout.bytes[2]..])
        .any(|byte| *byte != 0xa5)
    {
        return Err("output guard corruption detected".into());
    }
    let output = readback[GUARD..GUARD + layout.bytes[2]].to_vec();
    if output
        .chunks_exact(4)
        .any(|word| !f32::from_le_bytes(word.try_into().expect("four bytes")).is_finite())
    {
        return Err("output contains an unwritten or nonfinite checkpoint".into());
    }
    for buffer in buffers.into_iter().rev() {
        if !matches!(
            request(transport, CommandV1::Free { buffer }, vec![])?.response,
            ResponseV1::Freed
        ) {
            return Err("worker did not acknowledge buffer release".into());
        }
    }
    if !matches!(
        request(transport, CommandV1::Close, vec![])?.response,
        ResponseV1::Closed
    ) {
        return Err("worker did not acknowledge queue and VM cleanup".into());
    }
    Ok(Observation { output, elapsed_ns })
}
