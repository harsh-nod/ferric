//! Parent-run qualification fixture, never an automatic GPU test or benchmark.

use fe2o3_kfd::engineering_wire::{BufferAccessV1 as Access, KernelMetadataV1};
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerGroupV1 as Group,
    Gfx950EngineeringPeerKernelV1 as Kernel,
};
use sha2::{Digest, Sha256};
use std::io::Read;

type Result<T> = std::result::Result<T, String>;
const WIDTH: usize = 4096;
const ROWS: usize = 16;
const GUARD: usize = 64;
const COPY: &str = "ferric_qwen3_tp_peer_copy_bf16_v4";
const REDUCE: &str = "ferric_qwen3_tp_peer_ordered_residual_bf16_v4";
const PARTIAL: &str = "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2";

fn artifact(path: &str, expected: &str) -> Result<(Vec<u8>, [u8; 32])> {
    if expected.len() != 64 || !expected.is_ascii() {
        return Err("invalid SHA256".into());
    }
    let mut hash = [0; 32];
    for (index, value) in hash.iter_mut().enumerate() {
        *value = u8::from_str_radix(&expected[index * 2..index * 2 + 2], 16)
            .map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("artifact is not regular".into());
    }
    let mut bytes = Vec::new();
    file.take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 || <[u8; 32]>::from(Sha256::digest(&bytes)) != hash {
        return Err("artifact size or SHA256 mismatch".into());
    }
    Ok((bytes, hash))
}

fn check_metadata(
    metadata: &KernelMetadataV1,
    name: &str,
    hash: [u8; 32],
    slices: usize,
    scalars: usize,
) -> Result<()> {
    let explicit = slices * 16 + scalars * 4;
    if metadata.symbol != name
        || metadata.object_sha256 != hash
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.implicit_argument_bytes != 256
        || metadata.explicit_arguments.len() != slices * 2 + scalars
        || !metadata.implicit_argument_offset.is_some_and(|offset| {
            offset as usize >= explicit && offset + 256 == metadata.kernarg_bytes
        })
        || metadata.kernarg_bytes > 65_536
    {
        return Err(format!("probe kernel metadata mismatch: {name}"));
    }
    for index in 0..slices {
        let pointer = &metadata.explicit_arguments[index * 2];
        let length = &metadata.explicit_arguments[index * 2 + 1];
        if pointer.offset as usize != index * 16
            || pointer.bytes != 8
            || !pointer.global_buffer
            || length.offset as usize != index * 16 + 8
            || length.bytes != 8
            || length.global_buffer
        {
            return Err("probe slice ABI mismatch".into());
        }
    }
    for index in 0..scalars {
        let scalar = &metadata.explicit_arguments[slices * 2 + index];
        if scalar.global_buffer
            || scalar.bytes != 4
            || scalar.offset as usize != slices * 16 + index * 4
        {
            return Err("probe scalar ABI mismatch".into());
        }
    }
    Ok(())
}

fn dispatch(
    group: &mut Group,
    kernel: &Kernel,
    slices: &[(Buffer, usize, usize, Access)],
    scalars: &[u32],
    groups: u32,
) -> Result<()> {
    let mut payload = vec![0; kernel.metadata().kernarg_bytes as usize];
    let mut pointers = Vec::new();
    for (index, &(buffer, elements, width, access)) in slices.iter().enumerate() {
        payload[index * 16 + 8..index * 16 + 16].copy_from_slice(&(elements as u64).to_le_bytes());
        pointers.push(buffer.pointer(
            (index * 16) as u32,
            GUARD as u64,
            (elements * width) as u64,
            access,
        ));
    }
    for (index, value) in scalars.iter().enumerate() {
        let offset = slices.len() * 16 + index * 4;
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    // SAFETY: the operator opts into exact digest-bound fixture code. Every
    // binding uses a retained typed group token, declared extent and access;
    // this disposable process performs no concurrent or independent GPU work.
    unsafe {
        group.dispatch_unchecked(
            kernel,
            payload,
            [64, 1, 1],
            [groups * 64, 1, 1],
            &pointers,
            60_000,
        )
    }?;
    Ok(())
}

fn upload(group: &mut Group, buffer: Buffer, bytes: &[u8]) -> Result<()> {
    for (index, chunk) in bytes.chunks(1 << 20).enumerate() {
        group.write(buffer, (index * (1 << 20)) as u64, chunk)?;
    }
    Ok(())
}

fn allocation(
    group: &mut Group,
    owner: usize,
    world: usize,
    elements: usize,
    width: usize,
    shared: bool,
) -> Result<Buffer> {
    let peers = (0..world)
        .filter(|&peer| shared && peer != owner)
        .collect::<Vec<_>>();
    group.allocate(owner, &peers, (GUARD * 2 + elements * width) as u64)
}

fn guarded(elements: usize, width: usize) -> Vec<u8> {
    vec![0xa5; GUARD * 2 + elements * width]
}

fn write_bf16(bytes: &mut [u8], elements: usize, value: f32) {
    let bits = (value.to_bits() >> 16) as u16;
    for element in bytes[GUARD..GUARD + elements * 2].chunks_exact_mut(2) {
        element.copy_from_slice(&bits.to_le_bytes());
    }
}

fn check_buffer(
    group: &mut Group,
    buffer: Buffer,
    capacity: usize,
    active: usize,
    width: usize,
    value: &[u8],
) -> Result<()> {
    let bytes = group.read(
        buffer,
        0,
        u32::try_from(GUARD * 2 + capacity * width).map_err(|_| "probe read bound")?,
    )?;
    if bytes[..GUARD].iter().any(|&byte| byte != 0xa5)
        || bytes[GUARD + active * width..]
            .iter()
            .any(|&byte| byte != 0xa5)
        || bytes[GUARD..GUARD + active * width]
            .chunks_exact(width)
            .any(|element| element != value)
    {
        return Err(format!(
            "GPU-produced value or guard mismatch on owner {}",
            buffer.owner_rank()
        ));
    }
    Ok(())
}

fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 7 || args[0] != "--allow-unauthenticated-machine-code" {
        return Err("usage: probe --allow-unauthenticated-machine-code BASE_HSACO BASE_SHA PEER_HSACO PEER_SHA UID UID [six more]".into());
    }
    let ids = args[5..]
        .iter()
        .map(|id| id.parse::<u64>().map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>>>()?;
    if !matches!(ids.len(), 2 | 8) {
        return Err("exactly two or eight devices required".into());
    }
    let world = ids.len();
    let k = WIDTH / world;
    let (base, base_hash) = artifact(&args[1], &args[2])?;
    let (peer, peer_hash) = artifact(&args[3], &args[4])?;
    // SAFETY: this explicitly opted-in, single-threaded disposable fixture owns
    // all GPU activity in this process. Any error returns directly to exit(1).
    let mut group = unsafe { Group::open_unchecked(&ids) }?;
    let mut copies = Vec::new();
    let mut reductions = Vec::new();
    let mut projections = Vec::new();
    let mut inputs = Vec::new();
    let mut weights = Vec::new();
    let mut partials = Vec::new();
    let mut hidden = Vec::new();
    let mut output = Vec::new();
    for rank in 0..world {
        let copy = group.load_kernel(rank, peer.clone(), peer_hash, COPY.into())?;
        check_metadata(copy.metadata(), COPY, peer_hash, 2, 1)?;
        copies.push(copy);
        let reduce = group.load_kernel(rank, peer.clone(), peer_hash, REDUCE.into())?;
        check_metadata(reduce.metadata(), REDUCE, peer_hash, 10, 2)?;
        reductions.push(reduce);
        let projection = group.load_kernel(rank, base.clone(), base_hash, PARTIAL.into())?;
        check_metadata(projection.metadata(), PARTIAL, base_hash, 3, 5)?;
        projections.push(projection);
        inputs.push(allocation(&mut group, rank, world, ROWS * k, 2, false)?);
        let weight = allocation(&mut group, rank, world, WIDTH * k, 2, false)?;
        let mut values = guarded(WIDTH * k, 2);
        values[GUARD..GUARD + WIDTH * k * 2].fill(0);
        let value = (((rank + 1) as f32).to_bits() >> 16) as u16;
        for column in 0..WIDTH {
            let offset = GUARD + column * k * 2;
            values[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        upload(&mut group, weight, &values)?;
        weights.push(weight);
        partials.push(allocation(&mut group, rank, world, ROWS * WIDTH, 4, true)?);
        hidden.push(allocation(&mut group, rank, world, ROWS * WIDTH, 2, true)?);
        output.push(allocation(&mut group, rank, world, ROWS * WIDTH, 2, false)?);
    }
    let seed = allocation(&mut group, 0, world, ROWS * WIDTH, 2, false)?;
    let mut observations = Vec::new();
    for (phase, rows) in [1_usize, 3, 16].into_iter().enumerate() {
        eprintln!("peer producer fixture phase {phase} rows {rows} world {world}");
        let scale = (phase + 1) as f32;
        let mut source = guarded(ROWS * WIDTH, 2);
        write_bf16(&mut source, rows * WIDTH, scale);
        upload(&mut group, seed, &source)?;
        for rank in 0..world {
            let mut input = guarded(ROWS * k, 2);
            write_bf16(&mut input, rows * k, scale);
            upload(&mut group, inputs[rank], &input)?;
            upload(&mut group, partials[rank], &guarded(ROWS * WIDTH, 4))?;
            upload(&mut group, hidden[rank], &guarded(ROWS * WIDTH, 2))?;
            upload(&mut group, output[rank], &guarded(ROWS * WIDTH, 2))?;
        }
        // GPU-written BF16 source is consumed on every peer after its completion.
        dispatch(
            &mut group,
            &copies[0],
            &[
                (seed, rows * WIDTH, 2, Access::Read),
                (hidden[0], rows * WIDTH, 2, Access::Write),
            ],
            &[rows as u32],
            (rows * 64) as u32,
        )?;
        for rank in 1..world {
            dispatch(
                &mut group,
                &copies[rank],
                &[
                    (hidden[0], rows * WIDTH, 2, Access::Read),
                    (hidden[rank], rows * WIDTH, 2, Access::Write),
                ],
                &[rows as u32],
                (rows * 64) as u32,
            )?;
        }
        // Genuine FP32 output allocations are written by the baseline GEMM;
        // no BF16 allocation is relabeled as FP32 to manufacture a producer.
        for rank in 0..world {
            dispatch(
                &mut group,
                &projections[rank],
                &[
                    (inputs[rank], rows * k, 2, Access::Read),
                    (weights[rank], WIDTH * k, 2, Access::Read),
                    (partials[rank], rows * WIDTH, 4, Access::Write),
                ],
                &[rows as u32, WIDTH as u32, k as u32, world as u32, 1],
                (WIDTH / 16) as u32,
            )?;
        }
        for rank in 0..world {
            let mut arguments = (0..8)
                .map(|index| {
                    if index < world {
                        (partials[index], rows * WIDTH, 4, Access::Read)
                    } else {
                        (partials[0], 0, 4, Access::Read)
                    }
                })
                .collect::<Vec<_>>();
            arguments.extend([
                (hidden[rank], rows * WIDTH, 2, Access::Read),
                (output[rank], rows * WIDTH, 2, Access::Write),
            ]);
            dispatch(
                &mut group,
                &reductions[rank],
                &arguments,
                &[rows as u32, world as u32],
                (rows * 64) as u32,
            )?;
            let sum = (world * (world + 1) / 2 + 1) as f32 * scale;
            let output_bits = (sum.to_bits() >> 16) as u16;
            check_buffer(
                &mut group,
                output[rank],
                ROWS * WIDTH,
                rows * WIDTH,
                2,
                &output_bits.to_le_bytes(),
            )?;
            let input_bits = (scale.to_bits() >> 16) as u16;
            check_buffer(
                &mut group,
                hidden[rank],
                ROWS * WIDTH,
                rows * WIDTH,
                2,
                &input_bits.to_le_bytes(),
            )?;
            let partial = (rank + 1) as f32 * scale;
            check_buffer(
                &mut group,
                partials[rank],
                ROWS * WIDTH,
                rows * WIDTH,
                4,
                &partial.to_le_bytes(),
            )?;
            observations
                .push(serde_json::json!({"rank":rank,"rows":rows,"partial":partial,"reduced":sum}));
        }
        check_buffer(
            &mut group,
            seed,
            ROWS * WIDTH,
            rows * WIDTH,
            2,
            &((scale.to_bits() >> 16) as u16).to_le_bytes(),
        )?;
    }
    group.close()?;
    println!(
        "{}",
        serde_json::json!({"profile":"gpu-producer-peer-consumer-v4", "authority":"none",
        "pid":std::process::id(), "device_unique_ids":ids, "base_sha256":args[2], "peer_sha256":args[4],
        "observations":observations,"gpu_bf16_producer_to_peer_reader":true,
        "gpu_f32_projection_to_peer_reduction":true,"all_guards_and_active_outputs_exact":true,
        "all_peer_unmaps_before_owner_free":true,"all_closed":true})
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("peer producer fixture failed: {error}");
        std::process::exit(1);
    }
}
