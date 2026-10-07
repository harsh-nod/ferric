//! Source-only proposal for an opt-in TP1 model diagnostic, not serving code.
//! The caller owns ABI packing, process transport, numerical checks and teardown.

use std::collections::BTreeMap;
use std::io::{self, Write};

use fe2o3_kfd_current_wire::engineering_wire::{
    self as wire, CommandV1, OrderedBatchDispatchV1, ResponseV1,
};
use serde::Serialize;

pub const CORE_REVISION: &str = "5e2f668d0e91815f6c7aafe18dbdf283ab0394aa";
pub const MAX_GROUP_PACKETS: usize = 16;
pub const MAX_ORDERED64_GROUP_PACKETS: usize = 64;
pub const MAX_CAPTURE_PACKETS: usize = 100_000;
pub const MAX_CAPTURE_BATCHES: usize = 256;
pub const MAX_CAPTURE_KERNELS: usize = 256;
pub const MAX_CAPTURE_BYTES: usize = 12 * 1024 * 1024;
pub type Result<T> = std::result::Result<T, &'static str>;

/// Semantic roles must come from command construction, not kernel-name guessing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Operation {
    Embedding,
    InputNorm,
    QueryProjection,
    KeyProjection,
    ValueProjection,
    QueryNorm,
    KeyNorm,
    Rope,
    KvAppend,
    Attention,
    AttentionOutput,
    AttentionResidual,
    PostAttentionNorm,
    GateProjection,
    UpProjection,
    SwiGlu,
    DownProjection,
    FeedForwardResidual,
    FinalNorm,
    HeadProjection,
    Argmax,
}

const OPERATIONS: [&str; 21] = [
    "embedding",
    "input_norm",
    "query_projection",
    "key_projection",
    "value_projection",
    "query_norm",
    "key_norm",
    "rope",
    "kv_append",
    "attention",
    "attention_output",
    "attention_residual",
    "post_attention_norm",
    "gate_projection",
    "up_projection",
    "swiglu",
    "down_projection",
    "feed_forward_residual",
    "final_norm",
    "head_projection",
    "argmax",
];

impl Operation {
    fn is_layer(self) -> bool {
        !matches!(
            self,
            Self::Embedding | Self::FinalNorm | Self::HeadProjection | Self::Argmax
        )
    }
}

/// Captured alongside the command before it enters a deferred ordered group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketTag {
    pub batch_ordinal: u64,
    pub layer: Option<u32>,
    pub operation: Operation,
}

/// Original transport publication, before the diagnostic substitutes its command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Publication {
    Single,
    Ordered16,
    Ordered64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KernelIdentity {
    pub kernel: u64,
    pub symbol: String,
    pub object_sha256: [u8; 32],
}

/// Row kinds are supplied by the scheduler before numerical row permutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct BatchIdentity {
    pub ordinal: u64,
    pub scheduler_batch_id: u64,
    pub request: [u64; 2],
    pub prefill_rows: u16,
    pub decode_rows: u16,
    pub expected_packets: u64,
}

#[derive(Clone, Debug)]
struct Group {
    tags: Vec<PacketTag>,
    kernels: Vec<u64>,
    publication: Publication,
}

#[derive(Clone, Debug)]
enum Pending {
    Group(Group),
    Rollover,
}

/// Each row is [group, batch, `layer_plus_one`, operation, kernel, epoch, packet,
/// `start_tick`, `end_tick`]. Zero `layer_plus_one` means a non-layer operation.
type Record = [u64; 9];

#[derive(Debug)]
pub struct Collector {
    max_group_packets: usize,
    device: u64,
    layers: u32,
    epoch: u64,
    frontier: u64,
    kernels: BTreeMap<u64, KernelIdentity>,
    batches: Vec<BatchIdentity>,
    batch_packets: u64,
    groups: Vec<[u64; 3]>,
    records: Vec<Record>,
    pending: Option<Pending>,
    poisoned: bool,
}

impl Collector {
    /// Only a fresh, separately owned worker is eligible; it starts at epoch 0.
    /// # Errors
    /// Rejects an empty device or model identity.
    pub fn new(device: u64, layers: u32) -> Result<Self> {
        Self::new_with_limit(device, layers, MAX_GROUP_PACKETS)
    }

    /// Separate diagnostic preserving original single and ordered64 groups.
    /// # Errors
    /// Rejects an empty device or model identity.
    pub fn new_ordered64(device: u64, layers: u32) -> Result<Self> {
        Self::new_with_limit(device, layers, MAX_ORDERED64_GROUP_PACKETS)
    }

    fn new_with_limit(device: u64, layers: u32, max_group_packets: usize) -> Result<Self> {
        if device == 0 || layers == 0 {
            return Err("empty diagnostic identity");
        }
        Ok(Self {
            max_group_packets,
            device,
            layers,
            epoch: 0,
            frontier: 0,
            kernels: BTreeMap::new(),
            batches: Vec::new(),
            batch_packets: 0,
            groups: Vec::new(),
            records: Vec::new(),
            pending: None,
            poisoned: false,
        })
    }

    /// Copy only identities already validated by the existing `LoadKernel` path.
    /// # Errors
    /// Rejects malformed, duplicated, late or over-budget kernel registrations.
    pub fn register_kernel(&mut self, identity: KernelIdentity) -> Result<()> {
        let valid = !self.poisoned
            && self.pending.is_none()
            && self.batches.is_empty()
            && identity.kernel != 0
            && !identity.symbol.is_empty()
            && identity.symbol.len() <= 256
            && identity
                .symbol
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            && self.kernels.len() < MAX_CAPTURE_KERNELS
            && !self.kernels.contains_key(&identity.kernel);
        if !valid {
            return self.reject("invalid or late admitted kernel identity");
        }
        self.kernels.insert(identity.kernel, identity);
        Ok(())
    }

    /// Independent expected packet counts come from the existing batch driver.
    /// # Errors
    /// Rejects stale or mixed-request rows, invalid bounds and incomplete prior work.
    pub fn begin_batch(&mut self, batch: BatchIdentity) -> Result<()> {
        let rows = u32::from(batch.prefill_rows) + u32::from(batch.decode_rows);
        let previous_complete = self
            .batches
            .last()
            .is_none_or(|previous| self.batch_packets == previous.expected_packets);
        let valid = !self.poisoned
            && self.pending.is_none()
            && !self.kernels.is_empty()
            && self.batches.len() < MAX_CAPTURE_BATCHES
            && batch.ordinal == self.batches.len() as u64 + 1
            && batch.request[0] < 32
            && batch.request[1] != 0
            && self
                .batches
                .first()
                .is_none_or(|first| first.request == batch.request)
            && self
                .batches
                .last()
                .is_none_or(|last| last.scheduler_batch_id < batch.scheduler_batch_id)
            && (1..=16).contains(&rows)
            && (batch.prefill_rows == 0 || batch.decode_rows == 0)
            && batch.expected_packets > 0
            && batch.expected_packets <= MAX_CAPTURE_PACKETS as u64
            && previous_complete;
        if !valid {
            return self.reject("invalid batch identity, envelope, or prior completion");
        }
        self.batches.push(batch);
        self.batch_packets = 0;
        Ok(())
    }

    /// Return exactly one replacement command for exactly one existing group.
    /// Payload is the existing ABI-packed concatenation; never repack or regroup.
    /// # Errors
    /// Rejects unsupported original commands, attribution, framing and capacity drift.
    pub fn begin_group(
        &mut self,
        original: CommandV1,
        tags: Vec<PacketTag>,
        payload: &[u8],
    ) -> Result<CommandV1> {
        let (publication, dispatches, timeout_ms) = match original {
            CommandV1::Dispatch {
                kernel,
                payload_bytes,
                workgroup,
                grid,
                pointers,
                timeout_ms,
            } => (
                Publication::Single,
                vec![OrderedBatchDispatchV1 {
                    kernel,
                    payload_bytes,
                    workgroup,
                    grid,
                    pointers,
                }],
                timeout_ms,
            ),
            CommandV1::DispatchOrderedBatch {
                dispatches,
                timeout_ms,
            } if self.max_group_packets == MAX_GROUP_PACKETS => {
                (Publication::Ordered16, dispatches, timeout_ms)
            }
            CommandV1::DispatchOrderedBatch64 {
                dispatches,
                timeout_ms,
            } if self.max_group_packets == MAX_ORDERED64_GROUP_PACKETS => {
                (Publication::Ordered64, dispatches, timeout_ms)
            }
            _ => {
                return self
                    .reject("only original single or selected ordered publications are eligible");
            }
        };
        let count = dispatches.len();
        let Some(batch) = self.batches.last() else {
            return self.reject("dispatch without a bound scheduler batch");
        };
        let valid = !self.poisoned
            && self.pending.is_none()
            && (1..=self.max_group_packets).contains(&count)
            && (publication != Publication::Single || count == 1)
            && tags.len() == count
            && self.records.len() + count <= MAX_CAPTURE_PACKETS
            && self.batch_packets + count as u64 <= batch.expected_packets
            && self.frontier + count as u64 <= wire::MAX_UNRETIRED_RING_PACKETS_V1
            && dispatches
                .iter()
                .all(|dispatch| self.kernels.contains_key(&dispatch.kernel))
            && tags.iter().all(|tag| {
                tag.batch_ordinal == batch.ordinal
                    && tag.operation.is_layer() == tag.layer.is_some()
                    && tag.layer.is_none_or(|layer| layer < self.layers)
            });
        if !valid {
            return self
                .reject("profiled group changed the original packet envelope or attribution");
        }
        let kernels = dispatches.iter().map(|dispatch| dispatch.kernel).collect();
        let command = CommandV1::DispatchOrderedBatch64Profiled {
            dispatches,
            timeout_ms,
        };
        let mut header = Vec::new();
        if command.payload_bytes().ok() != Some(payload.len())
            || wire::write_header_v1(&mut header, &command).is_err()
        {
            return self.reject("profiled framing or payload mismatch");
        }
        self.pending = Some(Pending::Group(Group {
            tags,
            kernels,
            publication,
        }));
        Ok(command)
    }

    /// Validate the entire response before changing any packet or batch frontier.
    /// # Errors
    /// Rejects nonexact responses and poisons the capture without partial commitment.
    pub fn complete_group(&mut self, response: &ResponseV1, payload: &[u8]) -> Result<()> {
        if self.poisoned || !payload.is_empty() {
            return self.reject("poisoned collector or unexpected response payload");
        }
        let Some(Pending::Group(group)) = &self.pending else {
            return self.reject("profiled completion without a pending group");
        };
        let ResponseV1::DispatchOrderedBatch64ProfiledCompleted {
            device_unique_id,
            queue_epoch,
            elapsed_ns: _,
            timestamps,
        } = response
        else {
            return self.reject("wrong profiled completion variant");
        };
        let valid = *device_unique_id == self.device
            && *queue_epoch == self.epoch
            && timestamps.len() == group.tags.len()
            && timestamps.iter().enumerate().all(|(index, stamp)| {
                stamp.packet_id == self.frontier + index as u64
                    && stamp.kernel == group.kernels[index]
                    && stamp.start_tick > 0
                    && stamp.end_tick > stamp.start_tick
            });
        if !valid {
            return self
                .reject("profiled completion identity, frontier, kernel, or ticks mismatch");
        }
        let id = self.groups.len() as u64;
        for (tag, stamp) in group.tags.iter().zip(timestamps) {
            self.records.push([
                id,
                tag.batch_ordinal,
                tag.layer.map_or(0, |layer| u64::from(layer) + 1),
                tag.operation as u64,
                stamp.kernel,
                self.epoch,
                stamp.packet_id,
                stamp.start_tick,
                stamp.end_tick,
            ]);
        }
        self.groups
            .push([id, group.publication as u64, timestamps.len() as u64]);
        self.frontier += timestamps.len() as u64;
        self.batch_packets += timestamps.len() as u64;
        self.pending = None;
        Ok(())
    }

    /// Called only where the existing driver already requires idle rollover.
    /// # Errors
    /// Rejects nonidle or unavailable queue state.
    pub fn begin_rollover(&mut self) -> Result<CommandV1> {
        if self.poisoned || self.pending.is_some() || self.frontier == 0 || self.epoch == u64::MAX {
            return self.reject("rollover without an idle completed frontier");
        }
        self.pending = Some(Pending::Rollover);
        Ok(CommandV1::RolloverQueue {
            expected_epoch: self.epoch,
            expected_completed_packets: self.frontier,
        })
    }

    /// Commit only an exact acknowledgement of the requested idle rollover.
    /// # Errors
    /// Rejects the wrong frontier, epoch, kind, payload or pending operation.
    pub fn complete_rollover(&mut self, response: &ResponseV1, payload: &[u8]) -> Result<()> {
        let valid = !self.poisoned
            && matches!(self.pending, Some(Pending::Rollover))
            && payload.is_empty()
            && matches!(response, ResponseV1::QueueRolledOver { retired_packets, queue_epoch }
                if *retired_packets == self.frontier && Some(*queue_epoch) == self.epoch.checked_add(1));
        if !valid {
            return self.reject("rollover acknowledgement mismatch");
        }
        self.epoch += 1;
        self.frontier = 0;
        self.pending = None;
        Ok(())
    }

    /// The worker must invoke this on every send/read/timeout/cleanup failure.
    pub fn poison(&mut self) {
        self.poisoned = true;
        self.pending = None;
    }

    fn reject<T>(&mut self, message: &'static str) -> Result<T> {
        self.poison();
        Err(message)
    }

    /// Transport evidence only. Numerical parity and clean teardown are separate
    /// mandatory run-level gates, never asserted by this collector.
    /// # Errors
    /// Rejects pending, poisoned, incomplete or independently count-mismatched captures.
    pub fn finish(self, driver_completed_packets: u64) -> Result<Capture> {
        if self.poisoned
            || self.pending.is_some()
            || self.records.is_empty()
            || self.records.len() as u64 != driver_completed_packets
            || self
                .batches
                .last()
                .is_none_or(|batch| self.batch_packets != batch.expected_packets)
        {
            return Err("incomplete, poisoned, or packet-count-mismatched capture");
        }
        Ok(Capture {
            schema: if self.max_group_packets == MAX_ORDERED64_GROUP_PACKETS {
                "FerricOrdered64RawPacketIntervalsV1"
            } else {
                "FerricModelRawDispatchIntervalsV1"
            },
            evidence_scope: "transport_intervals_only_not_numerical_or_performance_qualification",
            core_revision: CORE_REVISION,
            tick_unit: "raw_device_ticks_frequency_unspecified",
            interval_scope: "end_tick_minus_start_tick_not_shader_only_not_wall_time",
            original_max_group_packets: self.max_group_packets,
            device_unique_id: self.device,
            operations: OPERATIONS,
            kernels: self.kernels.into_values().collect(),
            batches: self.batches,
            group_columns: [
                "group",
                if self.max_group_packets == MAX_ORDERED64_GROUP_PACKETS {
                    "original_publication_0_single_2_ordered64"
                } else {
                    "original_publication_0_single_1_ordered16"
                },
                "packets",
            ],
            groups: self.groups,
            record_columns: [
                "group",
                "batch",
                "layer_plus_one_zero_means_none",
                "operation",
                "kernel",
                "epoch",
                "packet",
                "start_tick",
                "end_tick",
            ],
            records: self.records,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Capture {
    schema: &'static str,
    evidence_scope: &'static str,
    core_revision: &'static str,
    tick_unit: &'static str,
    interval_scope: &'static str,
    original_max_group_packets: usize,
    device_unique_id: u64,
    operations: [&'static str; 21],
    kernels: Vec<KernelIdentity>,
    batches: Vec<BatchIdentity>,
    group_columns: [&'static str; 3],
    groups: Vec<[u64; 3]>,
    record_columns: [&'static str; 9],
    records: Vec<Record>,
}

/// Summaries are partitioned by device (one capture), epoch, phase, operation,
/// and actual loaded kernel ID. Their sums are NOT fractions of model wall time.
#[derive(Debug, Serialize)]
pub struct RawIntervalSummary {
    pub epoch: u64,
    pub phase: &'static str,
    pub operation: &'static str,
    pub kernel: u64,
    pub count: usize,
    pub min_raw_dispatch_interval_ticks: u64,
    pub p50_raw_dispatch_interval_ticks: u64,
    pub p95_raw_dispatch_interval_ticks: u64,
    pub max_raw_dispatch_interval_ticks: u64,
    pub sum_raw_dispatch_interval_ticks_nonadditive: String,
}

impl Capture {
    /// Descriptive raw intervals within a device/epoch, never normalized wall time.
    /// # Panics
    /// Only if internal collector invariants are violated: batch ordinals are
    /// one-based indices bounded by `MAX_CAPTURE_BATCHES`, and operation indices
    /// are discriminants of `Operation`. Both fit `usize` on supported targets.
    #[must_use]
    pub fn summarize(&self) -> Vec<RawIntervalSummary> {
        let mut partitions = BTreeMap::<(u64, bool, u64, u64), Vec<u64>>::new();
        for row in &self.records {
            let batch_index = usize::try_from(row[1]).expect("collector batch ordinal fits usize");
            let batch = &self.batches[batch_index - 1];
            partitions
                .entry((row[5], batch.prefill_rows != 0, row[3], row[4]))
                .or_default()
                .push(row[8] - row[7]);
        }
        partitions
            .into_iter()
            .map(|((epoch, prefill, operation, kernel), mut intervals)| {
                intervals.sort_unstable();
                let count = intervals.len();
                let operation_index =
                    usize::try_from(operation).expect("collector operation fits usize");
                RawIntervalSummary {
                    epoch,
                    phase: if prefill { "prefill" } else { "decode" },
                    operation: OPERATIONS[operation_index],
                    kernel,
                    count,
                    min_raw_dispatch_interval_ticks: intervals[0],
                    p50_raw_dispatch_interval_ticks: intervals[(count - 1) / 2],
                    p95_raw_dispatch_interval_ticks: intervals[(95 * count).div_ceil(100) - 1],
                    max_raw_dispatch_interval_ticks: intervals[count - 1],
                    sum_raw_dispatch_interval_ticks_nonadditive: intervals
                        .iter()
                        .map(|&ticks| u128::from(ticks))
                        .sum::<u128>()
                        .to_string(),
                }
            })
            .collect()
    }

    /// Encode in memory before opening a destination: fail, never truncate a
    /// seemingly successful capture at the shared-host file-size boundary.
    /// # Errors
    /// Rejects serialization that would exceed the fixed in-memory byte cap.
    pub fn to_bounded_json(&self) -> Result<Vec<u8>> {
        let mut writer = BoundedBytes(Vec::new());
        serde_json::to_writer(&mut writer, self).map_err(|_| "capture JSON exceeds byte cap")?;
        Ok(writer.0)
    }
}

struct BoundedBytes(Vec<u8>);

impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_CAPTURE_BYTES.saturating_sub(self.0.len()) {
            return Err(io::Error::other("capture byte cap"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
