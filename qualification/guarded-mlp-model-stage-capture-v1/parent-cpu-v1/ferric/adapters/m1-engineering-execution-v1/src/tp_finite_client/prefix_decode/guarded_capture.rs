//! Closed post-Close diagnostic parser; no new wire or numerical authority.
use super::{Result, gwire, hash, old, require};
use serde::Deserialize;

const PAYLOAD_BYTES: usize = 256_136;
const LIMIT: usize = 1_100_000;

pub(super) fn worker_flag(capture: bool) -> &'static str {
    if capture {
        "--engineering-native-guarded-mlp-stage-capture-v1"
    } else {
        "--engineering-native-guarded-mlp-decode-v1"
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: String,
    profile_sha256: [u8; 32],
    registration_sha256: [u8; 32],
    session: [u8; 32],
    device_ids: [u64; 2],
    completed_forwards: u32,
    native_closed: bool,
    sampling: String,
    capture: Capture,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema: String,
    generation: u64,
    position: u32,
    layer: u32,
    parts: Vec<Part>,
    payload_bytes: u32,
    payload_sha256: [u8; 32],
    payload: Vec<u8>,
    full_cache_capture: bool,
    native_close_confirmed: bool,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
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

type Spec = (&'static str, &'static str, u32);
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

pub(super) fn validate(
    raw: &[u8],
    bootstrap: &gwire::Bootstrap,
    request: &gwire::Request,
    first_observation: &[u8],
) -> Result<()> {
    require(
        !raw.is_empty() && raw.len() <= LIMIT,
        "guarded capture stderr bound",
    )?;
    let envelope: Envelope = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
    let profile = bootstrap.sha256().map_err(|e| e.to_string())?;
    require(
        envelope.schema == "FerricFiniteGuardedMlpLayerZeroCaptureV1"
            && envelope.profile_sha256 == profile
            && envelope.registration_sha256 == bootstrap.decode.registration
            && envelope.session == bootstrap.decode.scope.session
            && envelope.device_ids == bootstrap.decode.device_ids
            && envelope.completed_forwards == 4
            && envelope.native_closed
            && envelope.sampling == "prefix-boundaries-and-post-paired-retained"
            && !envelope.numerical_acceptance
            && !envelope.performance_claim
            && !envelope.production_authority,
        "guarded capture namespace, identity or Close",
    )?;
    let gwire::Command::Forward {
        generation,
        token,
        cache_metadata,
        rotary_bits,
    } = &request.command
    else {
        return Err("guarded capture first request is not Forward".into());
    };
    require(
        request.protocol == gwire::PROTOCOL
            && request.id == 1
            && *generation == 1
            && *token == bootstrap.input(0, None).map_err(|e| e.to_string())?
            && request.profile_sha256 == profile
            && request.registration == bootstrap.decode.registration
            && request.session == bootstrap.decode.scope.session
            && request.device_ids == bootstrap.decode.device_ids
            && cache_metadata.len() == 145
            && cache_metadata[0] == 0
            && rotary_bits.len() == 128
            && first_observation.len() == old::OBSERVATION_BYTES,
        "guarded capture first-forward join",
    )?;
    let mut pages = cache_metadata[1..].to_vec();
    pages.sort_unstable();
    require(
        pages == (0..144).collect::<Vec<_>>(),
        "guarded capture page permutation",
    )?;
    let metadata = cache_metadata
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let rotary = rotary_bits
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let selected_offset = u64::from(cache_metadata[1]) * 16 * 512 * 2;
    let capture = envelope.capture;
    require(
        capture.schema == "FerricFiniteLayerZeroCaptureV1"
            && (capture.generation, capture.position, capture.layer) == (1, 0, 0)
            && capture.parts.len() == 34
            && capture.payload_bytes as usize == PAYLOAD_BYTES
            && capture.payload.len() == PAYLOAD_BYTES
            && hash(&capture.payload) == capture.payload_sha256
            && !capture.full_cache_capture
            && capture.native_close_confirmed
            && !capture.numerical_acceptance
            && !capture.performance_claim
            && !capture.production_authority,
        "guarded capture closed layout and scope",
    )?;
    let mut part_index = 0;
    let mut offset = 0usize;
    for (boundary, specs) in STAGES {
        for rank in 0..2 {
            for &(role, scalar, bytes) in specs {
                let part = &capture.parts[part_index];
                let width = if scalar == "bf16" { 2 } else { 4 };
                let source_offset = if matches!(role, "current_key" | "current_value") {
                    selected_offset
                } else {
                    0
                };
                require(
                    part.boundary == boundary
                        && part.rank == rank
                        && part.role == role
                        && part.scalar == scalar
                        && part.elements == bytes / width
                        && part.offset as usize == offset
                        && part.bytes == bytes
                        && part.source_byte_offset == source_offset,
                    "guarded capture stage/role/order/extent",
                )?;
                let data = capture
                    .payload
                    .get(offset..offset + bytes as usize)
                    .ok_or("guarded capture range")?;
                require(hash(data) == part.sha256, "guarded capture part digest")?;
                let finite = match scalar {
                    "bf16" => data
                        .chunks_exact(2)
                        .all(|v| u16::from_le_bytes([v[0], v[1]]) & 0x7f80 != 0x7f80),
                    "f32" => data
                        .chunks_exact(4)
                        .all(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]]).is_finite()),
                    _ => true,
                };
                require(finite, "guarded capture nonfinite value")?;
                match role {
                    "cache_metadata" => require(
                        data == metadata.as_slice(),
                        "guarded capture actual metadata",
                    )?,
                    "rotary" => {
                        require(data == rotary.as_slice(), "guarded capture actual rotary")?
                    }
                    "final_hidden" => require(
                        data == &first_observation[..8192],
                        "guarded capture actual first layer output",
                    )?,
                    _ => {}
                }
                offset += bytes as usize;
                part_index += 1;
            }
        }
    }
    require(
        part_index == 34 && offset == PAYLOAD_BYTES,
        "guarded capture exact coverage",
    )
}

#[cfg(test)]
#[path = "guarded_capture_tests.rs"]
mod tests;
