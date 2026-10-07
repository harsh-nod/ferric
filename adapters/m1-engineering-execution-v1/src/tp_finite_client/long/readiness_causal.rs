//! Closed binary stderr sidecar, admitted only after the ordinary healthy Close.
use super::{Result, hash, long, ready, require, retained};
use serde::{Deserialize, Serialize};

const TOTAL: usize = 1_598_256;
const HEADER: usize = 128 << 10;
const MAGIC: &[u8; 8] = b"FCAP061\0";

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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    generation: u64,
    position: u32,
    layer: u32,
    parts: Vec<Part>,
    payload_bytes: u32,
    payload_sha256: [u8; 32],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: String,
    bootstrap: ready::Bootstrap,
    transcript_sha256: [u8; 32],
    captures: Vec<Snapshot>,
    payload_bytes: usize,
    payload_sha256: [u8; 32],
    cache_layout: String,
    native_close_confirmed: bool,
    numerical_acceptance: bool,
    performance_claim: bool,
    production_authority: bool,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub schema: &'static str,
    pub positions: [u32; 6],
    pub parts: usize,
    pub payload_bytes: usize,
    pub payload_sha256: [u8; 32],
    pub sidecar_bytes: usize,
    pub sidecar_sha256: [u8; 32],
    pub native_close_confirmed: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
}

fn stages(position: u32) -> [(&'static str, Vec<(&'static str, &'static str, u32)>); 5] {
    [
        (
            "before_prefix",
            vec![
                ("input", "bf16", 8192),
                ("cache_metadata", "u32", 580),
                ("rotary", "f32", 512),
            ],
        ),
        (
            "after_prefix",
            vec![
                ("input_normalized", "bf16", 8192),
                ("raw_qkv", "bf16", 6144),
                ("query", "bf16", 4096),
                ("used_key", "bf16", 1024 * (position + 1)),
                ("used_value", "bf16", 1024 * (position + 1)),
                ("attention", "bf16", 4096),
                ("output_partial", "f32", 16384),
            ],
        ),
        (
            "after_first_residual",
            vec![("first_residual", "bf16", 8192)],
        ),
        (
            "after_mlp",
            vec![
                ("post_normalized", "bf16", 8192),
                ("gate", "bf16", 12288),
                ("up", "bf16", 12288),
                ("activation", "bf16", 12288),
                ("down_partial", "f32", 16384),
            ],
        ),
        ("after_final_residual", vec![("final_hidden", "bf16", 8192)]),
    ]
}

fn finite(scalar: &str, raw: &[u8]) -> bool {
    match scalar {
        "bf16" => raw
            .chunks_exact(2)
            .all(|v| u16::from_le_bytes([v[0], v[1]]) & 0x7f80 != 0x7f80),
        "f32" => raw
            .chunks_exact(4)
            .all(|v| f32::from_le_bytes(v.try_into().unwrap()).is_finite()),
        "u32" => true,
        _ => false,
    }
}

pub(super) fn validate_file(
    bootstrap: &ready::Bootstrap,
    transcript: [u8; 32],
    pages: &[u32],
    selected: &retained::Files,
) -> Result<Summary> {
    let raw = selected.child_stderr.read(2 << 20, true)?;
    validate(&raw, bootstrap, transcript, pages, selected)
}

pub(super) fn validate(
    raw: &[u8],
    bootstrap: &ready::Bootstrap,
    transcript: [u8; 32],
    pages: &[u32],
    selected: &retained::Files,
) -> Result<Summary> {
    require(
        raw.len() >= 16 && raw.len() <= 16 + HEADER + TOTAL && &raw[..8] == MAGIC,
        "causal sidecar magic/bound",
    )?;
    let head = u32::from_le_bytes(raw[8..12].try_into().unwrap()) as usize;
    let length = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
    require(
        head != 0 && head <= HEADER && length == TOTAL && raw.len() == 16 + head + length,
        "causal exact envelope lengths",
    )?;
    let h: Header = serde_json::from_slice(&raw[16..16 + head]).map_err(|e| e.to_string())?;
    let payload = &raw[16 + head..];
    require(
        h.schema == "FerricReadiness40CausalLayerZeroV1"
            && serde_json::to_value(&h.bootstrap).map_err(|e| e.to_string())?
                == serde_json::to_value(bootstrap).map_err(|e| e.to_string())?
            && bootstrap.sequence.profile == long::Profile::Readiness40Position5
            && h.transcript_sha256 == transcript
            && h.captures.len() == 6
            && h.payload_bytes == TOTAL
            && h.payload_sha256 == hash(payload)
            && h.cache_layout == "used-prefix-token-head-channel-bf16"
            && h.native_close_confirmed
            && !h.numerical_acceptance
            && !h.performance_claim
            && !h.production_authority
            && pages.len() == 144
            && pages
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                == (0..144).collect(),
        "causal same-run identity/closed scope",
    )?;
    let mut base = 0usize;
    let mut previous: [Option<(&[u8], &[u8])>; 2] = [None; 2];
    for (position, capture) in h.captures.iter().enumerate() {
        let bytes = 256136 + 4096 * position;
        require(
            capture.position as usize == position
                && capture.generation == position as u64 + 1
                && capture.layer == 0
                && capture.payload_bytes as usize == bytes
                && capture.parts.len() == 34,
            "causal exact snapshot census",
        )?;
        let data = payload
            .get(base..base + bytes)
            .ok_or("causal snapshot extent")?;
        require(hash(data) == capture.payload_sha256, "causal snapshot pin")?;
        let mut part_index = 0;
        let mut offset = 0usize;
        let mut keys = [None; 2];
        let mut values = [None; 2];
        let mut hidden = [None; 2];
        for (boundary, specs) in stages(position as u32) {
            for rank in 0..2 {
                for &(role, scalar, count) in &specs {
                    let p = &capture.parts[part_index];
                    let unit = if scalar == "bf16" { 2 } else { 4 };
                    let cache = matches!(role, "used_key" | "used_value");
                    let source_offset = if cache {
                        u64::from(pages[0]) * 16 * 1024
                    } else {
                        0
                    };
                    require(
                        p.boundary == boundary
                            && p.rank == rank as u32
                            && p.role == role
                            && p.scalar == scalar
                            && p.bytes == count
                            && p.elements == count / unit
                            && p.offset as usize == offset
                            && p.source_byte_offset == source_offset,
                        "causal ordered part ABI",
                    )?;
                    let item = data
                        .get(offset..offset + count as usize)
                        .ok_or("causal part extent")?;
                    require(
                        hash(item) == p.sha256 && finite(scalar, item),
                        "causal part pin/finite",
                    )?;
                    match role {
                        "cache_metadata" => {
                            let expected: Vec<u8> = std::iter::once(position as u32)
                                .chain(pages.iter().copied())
                                .flat_map(u32::to_le_bytes)
                                .collect();
                            require(item == expected, "causal actual metadata/pages")?;
                        }
                        "used_key" => keys[rank] = Some(item),
                        "used_value" => values[rank] = Some(item),
                        "final_hidden" => hidden[rank] = Some(item),
                        _ => (),
                    }
                    offset += count as usize;
                    part_index += 1;
                }
            }
        }
        require(
            offset == bytes && part_index == 34 && hidden[0] == hidden[1],
            "causal full snapshot/rank equality",
        )?;
        for rank in 0..2 {
            let k = keys[rank].ok_or("causal K absent")?;
            let v = values[rank].ok_or("causal V absent")?;
            if let Some((old_k, old_v)) = previous[rank] {
                require(
                    k.starts_with(old_k) && v.starts_with(old_v),
                    "causal completed KV persistence",
                )?;
            }
            previous[rank] = Some((k, v));
        }
        if position == 0 || position == 5 {
            let file = &selected
                .captures
                .iter()
                .find(|c| c.position == position as u32)
                .ok_or("causal original selected capture absent")?
                .file;
            let original = file.read((1 << 20) as u64, true)?;
            let start = crate::finite_guarded_mlp_decode_wire_v1::CONTROL_BYTES;
            require(
                original.get(start..start + 8192) == hidden[0],
                "causal ordinary layer-zero output join",
            )?;
        }
        base += bytes;
    }
    require(base == TOTAL, "causal complete concatenation")?;
    Ok(Summary {
        schema: "FerricReadiness40CausalLayerZeroCheckedV1",
        positions: [0, 1, 2, 3, 4, 5],
        parts: 204,
        payload_bytes: TOTAL,
        payload_sha256: h.payload_sha256,
        sidecar_bytes: raw.len(),
        sidecar_sha256: hash(raw),
        native_close_confirmed: true,
        numerical_acceptance: false,
        performance_claim: false,
    })
}

#[cfg(test)]
#[path = "readiness_causal_tests.rs"]
mod tests;
