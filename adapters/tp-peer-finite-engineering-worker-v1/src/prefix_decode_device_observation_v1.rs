//! Raw completion-signal ticks for the closed four-forward route, not GPU ns.
use crate::finite_prefix_decode_wire_v1::{Bootstrap, Chain, Completion, Control};
use serde::{Deserialize, Serialize};
use std::io;

pub const SCHEMA: &str = "FerricPrefixDecodeDeviceObservationV1";
pub const MAX_BYTES: usize = 2 << 20;
pub const PER_FORWARD: usize = 293;
pub const MAX_ROWS: usize = 4 * PER_FORWARD;
pub const RANK_PACKETS: [u64; 2] = [592, 580];

pub(crate) fn require(ok: bool, why: &'static str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Embedding,
    Copy,
    Prefix,
    PostAttentionResidual,
    Mlp,
    PostMlpResidual,
    FinalNorm,
    Head,
    Argmax,
}
impl Stage {
    pub(crate) const fn entry(self) -> &'static str {
        match self {
            Self::Embedding => "ferric_qwen3_tp_batch_embedding_bf16_v2",
            Self::Copy => "ferric_qwen3_tp_peer_copy_bf16_v4",
            Self::Prefix => "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6",
            Self::PostAttentionResidual | Self::PostMlpResidual => {
                "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18"
            }
            Self::Mlp => "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2",
            Self::FinalNorm => "qwen3_rmsnorm_v1",
            Self::Head => "ferric_qwen3_tp_mfma_gemm_bf16_v3",
            Self::Argmax => "ferric_qwen3_tp_batch_argmax_bf16_v2",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    pub prefix: [u8; 32],
    pub mlp: [u8; 32],
    pub residual: [u8; 32],
    /// Loaded tail object from Begin.tail_image; copy uses Begin.residual_image.
    pub tail: [u8; 32],
    pub copy: [u8; 32],
}
impl Images {
    pub(crate) fn validate(&self, b: &Bootstrap) -> io::Result<()> {
        require(
            self.prefix == b.prefix_image.sha256
                && self.mlp == b.tiles_image.sha256
                && self.residual == b.begin.residual_image.sha256
                && b.begin
                    .tail_image
                    .is_some_and(|tail| self.tail == tail.sha256)
                && self.copy == b.begin.residual_image.sha256
                && [self.prefix, self.mlp, self.residual, self.tail, self.copy]
                    .iter()
                    .all(|v| *v != [0; 32]),
            "raw device image bindings",
        )
    }
    pub(crate) const fn image(self, stage: Stage) -> [u8; 32] {
        match stage {
            Stage::Prefix => self.prefix,
            Stage::Mlp => self.mlp,
            Stage::PostAttentionResidual | Stage::PostMlpResidual => self.residual,
            Stage::Copy => self.copy,
            _ => self.tail,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    pub rank: u32,
    pub unique_id: u64,
    pub queue_epoch: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub generation: u64,
    pub position: u32,
    pub stage: Stage,
    pub layer: Option<u32>,
    pub rank: u32,
    pub entry: String,
    pub image_sha256: [u8; 32],
    pub group_incarnation: u64,
    pub unique_id: u64,
    pub queue_epoch: u64,
    pub packet_id: u64,
    pub signal_generation: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    /// Native submission-to-observed-completion host interval, not GPU ns.
    pub host_elapsed_ns: u64,
}

/// Closed host-call order. Pair rank order is not device start-time order.
pub(crate) fn expected(index: usize) -> io::Result<(u64, u32, Stage, Option<u32>, u32)> {
    require(index < MAX_ROWS, "raw device row bound")?;
    let position = (index / PER_FORWARD) as u32;
    let slot = index % PER_FORWARD;
    let (stage, layer, rank) = match slot {
        0 => (Stage::Embedding, None, 0),
        1 => (Stage::Copy, None, 1),
        2..=289 => {
            let local = slot - 2;
            let stage = [
                Stage::Prefix,
                Stage::PostAttentionResidual,
                Stage::Mlp,
                Stage::PostMlpResidual,
            ][local % 8 / 2];
            (stage, Some((local / 8) as u32), (local % 2) as u32)
        }
        290 => (Stage::FinalNorm, None, 0),
        291 => (Stage::Head, None, 0),
        292 => (Stage::Argmax, None, 0),
        _ => return Err(io::Error::other("raw device slot")),
    };
    Ok((u64::from(position) + 1, position, stage, layer, rank))
}

pub(crate) fn validate_row(
    row: &Row,
    index: usize,
    group: u64,
    ranks: &[Rank; 2],
    images: Images,
    packets: &mut [u64; 2],
) -> io::Result<()> {
    let (generation, position, stage, layer, rank) = expected(index)?;
    let r = &ranks[rank as usize];
    let packet = packets[rank as usize];
    require(
        row.generation == generation
            && row.position == position
            && row.stage == stage
            && row.layer == layer
            && row.rank == rank
            && row.entry == stage.entry()
            && row.image_sha256 == images.image(stage)
            && row.group_incarnation == group
            && row.unique_id == r.unique_id
            && row.queue_epoch == r.queue_epoch
            && row.packet_id == packet
            && row.signal_generation == packet + 1
            && row.start_tick != 0
            && row.end_tick >= row.start_tick,
        "raw device row order/identity/ticks",
    )?;
    packets[rank as usize] += 1;
    Ok(())
}

pub(crate) fn join_control(rows: &[Row], control: &Control) -> io::Result<[u8; 32]> {
    control.validate()?;
    let times = control
        .embedding_ns
        .iter()
        .copied()
        .chain(
            control
                .layers
                .iter()
                .flat_map(|l| l.paired_ns.iter().flatten().copied()),
        )
        .chain(control.tail_ns.iter().copied());
    require(
        rows.len() == PER_FORWARD && rows.iter().map(|row| row.host_elapsed_ns).eq(times),
        "raw device host intervals differ from control",
    )?;
    Ok(crate::finite_forward_wire_v1::part(&control.encode()).sha256)
}

pub(crate) fn completions(
    b: &Bootstrap,
    profile: [u8; 32],
    values: &[Completion],
) -> io::Result<[u8; 32]> {
    require(values.len() <= 4, "raw device completion bound")?;
    let mut chain = Chain::new(b.registration, profile);
    let mut previous = None;
    for (position, c) in values.iter().enumerate() {
        require(
            c.generation == position as u64 + 1
                && c.position == position as u32
                && c.input_token == b.input(position as u32, previous)?
                && c.output_token < 151936
                && c.control.bytes == 241960
                && c.observation.bytes == 606976
                && c.control.sha256 != [0; 32]
                && c.observation.sha256 != [0; 32]
                && c.capture.total == c.observation
                && chain.advance(c) == c.chain,
            "raw device completion trajectory/capture/chain",
        )?;
        previous = Some(c.output_token);
    }
    Ok(chain.digest())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub bootstrap: Bootstrap,
    pub worker_sha256: [u8; 32],
    pub child_pid: u32,
    pub profile_sha256: [u8; 32],
    pub group_incarnation: u64,
    pub ranks: [Rank; 2],
    pub images: Images,
    pub rows: Vec<Row>,
    pub final_dispatches: [u64; 2],
    pub completions: Vec<Completion>,
    pub transcript_sha256: [u8; 32],
    pub raw_timestamp_queue: bool,
    pub shared_full_currentness: bool,
    pub cache_kernel_admission: bool,
    pub operational_currentness: bool,
    pub native_closed: bool,
    pub raw_completion_ticks: bool,
    pub calibrated_nanoseconds: bool,
    pub cross_device_clock_alignment: bool,
    pub overlap_claim: bool,
    pub numerical_acceptance: bool,
    pub performance_claim: bool,
    pub production_authority: bool,
    pub full_model_acceptance: bool,
}
impl Report {
    /// Join a retained ordinary wire control to this diagnostic after decoding.
    /// The parent must additionally authenticate the worker and complete transcript.
    pub fn validate_control(&self, position: u32, control: &Control) -> io::Result<()> {
        self.validate()?;
        require(position < 4, "raw device control position")?;
        let start = position as usize * PER_FORWARD;
        require(
            join_control(&self.rows[start..start + PER_FORWARD], control)?
                == self.completions[position as usize].control.sha256,
            "raw device control digest",
        )
    }
    pub fn decode(raw: &[u8]) -> io::Result<Self> {
        require(
            !raw.is_empty() && raw.len() <= MAX_BYTES,
            "raw device sidecar bound",
        )?;
        let value: Self = serde_json::from_slice(raw).map_err(io::Error::other)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> io::Result<()> {
        self.bootstrap.validate(
            self.bootstrap.device_ids,
            self.bootstrap.timeout_ms,
            self.child_pid,
            self.bootstrap.mode,
        )?;
        require(
            self.schema == SCHEMA
                && self.child_pid != 0
                && self.worker_sha256 != [0; 32]
                && self.profile_sha256 == self.bootstrap.sha256()?
                && self.group_incarnation != 0
                && self.rows.len() == MAX_ROWS
                && self.completions.len() == 4
                && self.final_dispatches == RANK_PACKETS
                && self.raw_timestamp_queue
                && self.shared_full_currentness
                && !self.cache_kernel_admission
                && !self.operational_currentness
                && self.native_closed
                && self.raw_completion_ticks
                && !self.calibrated_nanoseconds
                && !self.cross_device_clock_alignment
                && !self.overlap_claim
                && !self.numerical_acceptance
                && !self.performance_claim
                && !self.production_authority
                && !self.full_model_acceptance,
            "raw device report identity/census/policy/claims",
        )?;
        self.images.validate(&self.bootstrap)?;
        for (i, r) in self.ranks.iter().enumerate() {
            require(
                r.rank == i as u32 && r.unique_id == self.bootstrap.device_ids[i],
                "raw device rank identity",
            )?;
        }
        let mut packets = [0; 2];
        for (i, row) in self.rows.iter().enumerate() {
            validate_row(
                row,
                i,
                self.group_incarnation,
                &self.ranks,
                self.images,
                &mut packets,
            )?;
        }
        require(
            packets == RANK_PACKETS
                && completions(&self.bootstrap, self.profile_sha256, &self.completions)?
                    == self.transcript_sha256,
            "raw device final counts/transcript",
        )
    }
    pub(crate) fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut writer = Bounded(Vec::new());
        serde_json::to_writer(&mut writer, self).map_err(io::Error::other)?;
        Ok(writer.0)
    }
}
struct Bounded(Vec<u8>);
impl io::Write for Bounded {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        require(
            data.len() <= MAX_BYTES.saturating_sub(self.0.len()),
            "raw device sidecar bound",
        )?;
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "prefix_decode_device_observation_v1_tests.rs"]
pub(crate) mod tests;
