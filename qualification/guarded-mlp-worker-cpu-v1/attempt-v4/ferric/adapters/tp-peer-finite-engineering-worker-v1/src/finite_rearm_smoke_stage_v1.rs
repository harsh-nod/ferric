//! Strict decoding of the existing diagnostic, without source/launch authority.

use super::{STAGE_JSON_BYTES, check};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StagePart {
    boundary: String,
    rank: u32,
    role: String,
    scalar: String,
    elements: u32,
    offset: u32,
    bytes: u32,
    source_byte_offset: u64,
    sha256: [u8; 32],
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema: String,
    generation: u64,
    position: u32,
    layer: u32,
    parts: Vec<StagePart>,
    payload_bytes: u32,
    payload_sha256: [u8; 32],
    payload: Vec<u8>,
    full_cache_capture: bool,
    native_close_confirmed: bool,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}

type Spec = (&'static str, &'static str, usize);
const STAGES: [(&str, &[Spec]); 5] = [
    (
        "before_prefix",
        &[
            ("input", "bf16", 8192),
            ("cache_metadata", "u32", 580),
            ("rotary", "f32", 512),
        ],
    ),
    (
        "after_prefix",
        &[
            ("input_normalized", "bf16", 8192),
            ("raw_qkv", "bf16", 6144),
            ("query", "bf16", 4096),
            ("current_key", "bf16", 1024),
            ("current_value", "bf16", 1024),
            ("attention", "bf16", 4096),
            ("output_partial", "f32", 16384),
        ],
    ),
    ("after_first_residual", &[("first_residual", "bf16", 8192)]),
    (
        "after_mlp",
        &[
            ("post_normalized", "bf16", 8192),
            ("gate", "bf16", 12288),
            ("up", "bf16", 12288),
            ("activation", "bf16", 12288),
            ("down_partial", "f32", 16384),
        ],
    ),
    ("after_final_residual", &[("final_hidden", "bf16", 8192)]),
];

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn page_offset(bytes: &[u8]) -> io::Result<u64> {
    check(
        bytes.len() == 580 && bytes[..4] == [0; 4],
        "stage metadata position/extent",
    )?;
    let mut seen = [false; 144];
    for chunk in bytes[4..].chunks_exact(4) {
        let page = u32::from_le_bytes(chunk.try_into().map_err(io::Error::other)?) as usize;
        let slot = seen
            .get_mut(page)
            .ok_or_else(|| io::Error::other("stage page bounds"))?;
        check(!*slot, "stage page alias")?;
        *slot = true;
    }
    Ok(u64::from(u32::from_le_bytes(
        bytes[4..8].try_into().map_err(io::Error::other)?,
    )) * 16
        * 512
        * 2)
}

fn validated(bytes: &[u8]) -> io::Result<Capture> {
    check(
        !bytes.is_empty() && bytes.len() <= STAGE_JSON_BYTES,
        "stage JSON extent",
    )?;
    let capture: Capture = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    check(
        capture.schema == "FerricFiniteLayerZeroCaptureV1"
            && capture.generation == 1
            && capture.position == 0
            && capture.layer == 0
            && capture.parts.len() == 34
            && capture.payload_bytes == 256136
            && capture.payload.len() == 256136
            && digest(&capture.payload) == capture.payload_sha256
            && !capture.full_cache_capture
            && !capture.native_close_confirmed
            && !capture.numerical_acceptance
            && !capture.performance_claim
            && !capture.production_authority,
        "stage diagnostic shape/claims",
    )?;
    let mut index = 0;
    let mut offset = 0usize;
    let mut cache_offsets = [None; 2];
    for (boundary, specs) in STAGES {
        for rank in 0..2 {
            for &(role, scalar, size) in specs {
                let part = &capture.parts[index];
                let end = offset
                    .checked_add(size)
                    .ok_or_else(|| io::Error::other("stage part overflow"))?;
                let data = capture
                    .payload
                    .get(offset..end)
                    .ok_or_else(|| io::Error::other("stage part bounds"))?;
                let width = if scalar == "bf16" { 2 } else { 4 };
                let source_offset = if matches!(role, "current_key" | "current_value") {
                    cache_offsets[rank]
                        .ok_or_else(|| io::Error::other("stage cache offset unavailable"))?
                } else {
                    0
                };
                check(
                    part.boundary == boundary
                        && part.rank == rank as u32
                        && part.role == role
                        && part.scalar == scalar
                        && part.elements as usize == size / width
                        && part.offset as usize == offset
                        && part.bytes as usize == size
                        && part.source_byte_offset == source_offset
                        && part.sha256 == digest(data),
                    "stage exact part roster/hash",
                )?;
                match scalar {
                    "bf16" => check(
                        data.chunks_exact(2)
                            .all(|word| u16::from_le_bytes([word[0], word[1]]) & 0x7f80 != 0x7f80),
                        "stage nonfinite BF16",
                    )?,
                    "f32" => check(
                        data.chunks_exact(4).all(|word| {
                            f32::from_le_bytes([word[0], word[1], word[2], word[3]]).is_finite()
                        }),
                        "stage nonfinite F32",
                    )?,
                    "u32" => cache_offsets[rank] = Some(page_offset(data)?),
                    _ => unreachable!(),
                }
                index += 1;
                offset = end;
            }
        }
    }
    check(
        index == 34 && offset == capture.payload.len(),
        "stage complete coverage",
    )?;
    Ok(capture)
}

pub(super) fn validate(bytes: &[u8]) -> io::Result<()> {
    validated(bytes).map(|_| ())
}

pub(super) fn validate_observation(bytes: &[u8], observation: &[u8]) -> io::Result<()> {
    let capture = validated(bytes)?;
    let row = observation
        .get(..8192)
        .ok_or_else(|| io::Error::other("stage companion observation extent"))?;
    for part in &capture.parts[32..34] {
        let start = part.offset as usize;
        check(
            &capture.payload[start..start + 8192] == row,
            "stage final rank row differs from main layer-zero capture",
        )?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn fixture() -> Vec<u8> {
    let mut capture = Capture {
        schema: "FerricFiniteLayerZeroCaptureV1".into(),
        generation: 1,
        position: 0,
        layer: 0,
        parts: vec![],
        payload_bytes: 256136,
        payload_sha256: [0; 32],
        payload: vec![],
        full_cache_capture: false,
        native_close_confirmed: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    for (boundary, specs) in STAGES {
        for rank in 0..2 {
            for &(role, scalar, bytes) in specs {
                let mut data = vec![0; bytes];
                if role == "cache_metadata" {
                    for (page, word) in data[4..].chunks_exact_mut(4).enumerate() {
                        word.copy_from_slice(&(page as u32).to_le_bytes());
                    }
                }
                capture.parts.push(StagePart {
                    boundary: boundary.into(),
                    rank,
                    role: role.into(),
                    scalar: scalar.into(),
                    elements: (bytes / if scalar == "bf16" { 2 } else { 4 }) as u32,
                    offset: capture.payload.len() as u32,
                    bytes: bytes as u32,
                    source_byte_offset: 0,
                    sha256: digest(&data),
                });
                capture.payload.extend_from_slice(&data);
            }
        }
    }
    capture.payload_sha256 = digest(&capture.payload);
    serde_json::to_vec(&capture).unwrap()
}
