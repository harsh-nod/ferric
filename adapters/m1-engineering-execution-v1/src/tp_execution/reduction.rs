//! Explicit reduction ablations. Host workspace reuse does not imply peer memory.
//! These paths are Contracted; tests do not establish a device numerical proof.

use super::{
    EngineeringTpArgumentV1, EngineeringTpExecutionV1, EngineeringTpRankTransportV1,
    Qwen3TensorParallelCollectiveV1, Tensor, TpResult, allocate_tensor, decode_bf16, dispatch,
};

const DEVICE_RESIDUAL: &str = "ferric_qwen3_tp_batch_residual_bf16_v3";

/// Named, opt-in arithmetic/transport profiles with no implicit fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EngineeringTpReductionModeV3 {
    /// Original rank-ordered CPU reduction and host byte transport.
    #[default]
    HostStagedV1,
    /// Identical CPU arithmetic and transport with retained staging allocations.
    HostStagedReuseV3,
    /// TP1 only: GPU residual addition with no hidden/partial host transport.
    DeviceTp1V3,
    /// TP2/8: ordered device reduction in one serial shared-process peer owner.
    DevicePeerV4,
    /// TP2/8: all-rank publication before waiting, within one retained peer owner.
    DevicePeerConcurrentV1,
}

impl EngineeringTpReductionModeV3 {
    /// Stable evidence label; never identifies host staging as a device collective.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HostStagedV1 => "host-staged-v1",
            Self::HostStagedReuseV3 => "host-staged-reuse-v3",
            Self::DeviceTp1V3 => "device-tp1-v3",
            Self::DevicePeerV4 => "device-peer-serial-v4",
            Self::DevicePeerConcurrentV1 => "device-peer-concurrent-round-v1",
        }
    }

    /// Additional completed GPU dispatches per layer, relative to the v2 image.
    #[must_use]
    pub const fn extra_dispatches_per_layer(self) -> u64 {
        match self {
            Self::DeviceTp1V3 | Self::DevicePeerV4 | Self::DevicePeerConcurrentV1 => 2,
            Self::HostStagedV1 | Self::HostStagedReuseV3 => 0,
        }
    }

    /// Explicit embedding broadcast copy on every nonzero peer rank.
    #[must_use]
    pub const fn extra_dispatches_per_forward(self, rank: u32) -> u64 {
        if matches!(self, Self::DevicePeerV4 | Self::DevicePeerConcurrentV1) && rank != 0 {
            1
        } else {
            0
        }
    }

    /// Both peer profiles share arithmetic but have different execution contracts.
    #[must_use]
    pub const fn is_peer(self) -> bool {
        matches!(self, Self::DevicePeerV4 | Self::DevicePeerConcurrentV1)
    }
}

#[derive(Default)]
pub(super) enum ReductionWorkspace {
    #[default]
    Baseline,
    Host(HostWorkspace),
    DeviceTp1(Tensor),
    DevicePeer(Vec<Tensor>, EngineeringTpReductionModeV3),
}

impl ReductionWorkspace {
    pub(super) const fn mode(&self) -> EngineeringTpReductionModeV3 {
        match self {
            Self::Baseline => EngineeringTpReductionModeV3::HostStagedV1,
            Self::Host(_) => EngineeringTpReductionModeV3::HostStagedReuseV3,
            Self::DeviceTp1(_) => EngineeringTpReductionModeV3::DeviceTp1V3,
            Self::DevicePeer(_, mode) => *mode,
        }
    }
}

pub(super) struct HostWorkspace {
    bytes: Vec<u8>,
    partials: Vec<Vec<f32>>,
    output: Vec<u16>,
    broadcast: Vec<u8>,
}

impl HostWorkspace {
    fn new(world: usize, elements: usize) -> Self {
        Self {
            bytes: vec![0; elements * 4],
            partials: vec![vec![0.0; elements]; world],
            output: vec![0; elements],
            broadcast: vec![0; elements * 2],
        }
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    /// Configures one ablation before the first GPU dispatch. The caller must
    /// admit the separate v3 kernel roster before selecting `DeviceTp1V3`.
    /// No implicit host fallback is permitted for an unsupported world size.
    ///
    /// # Errors
    /// Rejects a running/closed stream, repeated configuration, unsupported
    /// device geometry, or scratch-allocation failure (which closes the group).
    #[cfg_attr(not(feature = "tp-batch-engineering"), allow(dead_code))]
    pub(super) fn configure_reduction(
        &mut self,
        mode: EngineeringTpReductionModeV3,
    ) -> TpResult<()> {
        if self.closed || self.ranks.iter().any(|rank| rank.dispatches != 0) {
            return Err("reduction mode requires a fresh, open execution".into());
        }
        if !matches!(self.reduction, ReductionWorkspace::Baseline) {
            return Err("reduction mode is already configured".into());
        }
        match mode {
            EngineeringTpReductionModeV3::DevicePeerV4
            | EngineeringTpReductionModeV3::DevicePeerConcurrentV1 => {
                let concurrent = mode == EngineeringTpReductionModeV3::DevicePeerConcurrentV1;
                if self
                    .transports
                    .iter()
                    .any(|transport| transport.supports_concurrent_rounds() != concurrent)
                {
                    return Err("peer execution profile does not match transport capability".into());
                }
                self.configure_device_peer(mode)?;
            }
            EngineeringTpReductionModeV3::HostStagedV1 => {}
            EngineeringTpReductionModeV3::HostStagedReuseV3 => {
                self.reduction = ReductionWorkspace::Host(HostWorkspace::new(
                    self.ranks.len(),
                    self.ranks[0].hidden.elements,
                ));
            }
            EngineeringTpReductionModeV3::DeviceTp1V3 => {
                if self.plan.world_size() != 1
                    || self.plan.model().role != ferric_spec::Qwen3ModelRole::Target8B
                    || self.plan.model().hidden_size != 4096
                {
                    return Err("device-tp1-v3 requires Qwen3-8B on exactly one rank; peer collectives are unsupported".into());
                }
                let scratch =
                    allocate_tensor(&mut self.transports[0], self.ranks[0].hidden.elements, 2);
                let scratch = match scratch {
                    Ok(value) => value,
                    Err(error) => {
                        return Err(match self.close() {
                            Ok(()) => error,
                            Err(close) => format!("{error}; close: {close}"),
                        });
                    }
                };
                self.reduction = ReductionWorkspace::DeviceTp1(scratch);
            }
        }
        Ok(())
    }

    pub(super) fn initialize_hidden_from_embedding(&mut self) -> TpResult<()> {
        if matches!(self.reduction, ReductionWorkspace::DevicePeer(..)) {
            return self.initialize_peer_hidden();
        }
        if matches!(self.reduction, ReductionWorkspace::DeviceTp1(_)) {
            return Ok(());
        }
        let mut hidden = vec![0; self.hidden.len() * 2];
        self.transports[0].read(self.ranks[0].hidden.id, 0, &mut hidden)?;
        self.hidden = decode_bf16(&hidden)?;
        self.broadcast_hidden(&hidden)
    }

    pub(super) fn reduce_device_tp1(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> TpResult<()> {
        let key = self.collective.expected();
        if key.layer != layer || key.operation != operation {
            return Err("device collective operation reordered".into());
        }
        let ReductionWorkspace::DeviceTp1(scratch) = self.reduction else {
            return Err("device residual workspace unavailable".into());
        };
        let ordered_tail = self.ordered_batches.is_some() && !self.draft_v10;
        if ordered_tail {
            let pending = self
                .ordered_batches
                .as_ref()
                .expect("ordered residual group");
            let expected = match operation {
                Qwen3TensorParallelCollectiveV1::AttentionOutputSum => 10,
                Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => 5,
            };
            let count = pending
                .len()
                .checked_add(1)
                .ok_or("residual batch overflow")?;
            if pending.len() != expected
                || !(1..=16).contains(&count)
                || self.ranks.len() != 1
                || self.transports.len() != 1
                || self.sequences.is_some()
                || self.ranks[0].dispatches.checked_add(count as u64).is_none()
            {
                return Err("ordered residual batch geometry or counter drifted".into());
            }
        }
        let elements = self.hidden.len();
        let width = if self.draft_v10 {
            if self.plan.model().role != ferric_spec::Qwen3ModelRole::Draft06B
                || self.plan.model().hidden_size != 1024
                || self.row_capacity != 32
            {
                return Err("draft v10 residual model binding drifted".into());
            }
            1024
        } else {
            4096
        };
        let rows = elements / width;
        let rank = &self.ranks[0];
        if self.plan.world_size() != 1
            || elements != rows * width
            || !(1..=self.row_capacity as usize).contains(&rows)
            || elements > scratch.elements
            || elements > rank.hidden.elements
            || elements > rank.partial.elements
            || scratch.id == rank.hidden.id
            || scratch.id == rank.partial.id
            || rank.hidden.id == rank.partial.id
        {
            return Err("device residual extent or ownership drifted".into());
        }
        let command = dispatch(
            DEVICE_RESIDUAL,
            u32::try_from(elements / 64).map_err(|_| "residual grid overflow")?,
            vec![
                Tensor {
                    elements,
                    ..rank.partial
                }
                .read(),
                Tensor {
                    elements,
                    ..rank.hidden
                }
                .read(),
                Tensor {
                    elements,
                    ..scratch
                }
                .write(),
                EngineeringTpArgumentV1::U32(
                    u32::try_from(rows).map_err(|_| "residual rows overflow")?,
                ),
            ],
        );
        if ordered_tail {
            // Keep the residual in the same completion frontier as its producer.
            self.dispatch_each(|_| command.clone())?;
            self.flush_dispatch_groups()?;
        } else {
            self.dispatch_zero(&command)?;
        }
        self.collective
            .arrive(0, key)
            .map_err(|error| format!("device collective arrival: {error:?}"))?;
        self.collective
            .advance()
            .map_err(|error| format!("device collective advance: {error:?}"))?;
        // Swap only after completion. On a GPU trap the caller poisons the
        // stream and cannot publish a partially written destination.
        let previous = self.ranks[0].hidden;
        self.ranks[0].hidden = scratch;
        self.reduction = ReductionWorkspace::DeviceTp1(previous);
        Ok(())
    }

    pub(super) fn reduce_host_reused(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> TpResult<()> {
        let key = self.collective.expected();
        if key.layer != layer || key.operation != operation {
            return Err("reused host collective operation reordered".into());
        }
        let ReductionWorkspace::Host(workspace) = &mut self.reduction else {
            return Err("host collective workspace unavailable".into());
        };
        let elements = self.hidden.len();
        if elements == 0
            || elements > workspace.output.len()
            || workspace.partials.len() != self.ranks.len()
            || !matches!(self.ranks.len(), 1 | 2 | 8)
        {
            return Err("host collective workspace geometry drifted".into());
        }
        for (index, (rank, transport)) in self.ranks.iter().zip(&mut self.transports).enumerate() {
            if rank.geometry.rank as usize != index {
                return Err("host collective rank order drifted".into());
            }
            transport.read(rank.partial.id, 0, &mut workspace.bytes[..elements * 4])?;
            for (value, bytes) in workspace.partials[index][..elements]
                .iter_mut()
                .zip(workspace.bytes[..elements * 4].chunks_exact(4))
            {
                *value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            }
            self.collective
                .arrive(rank.geometry.rank, key)
                .map_err(|error| format!("host collective arrival: {error:?}"))?;
        }
        for (index, bits) in self.hidden.iter().copied().enumerate() {
            let mut sum = 0.0_f32;
            for partial in &workspace.partials {
                let value = partial[index];
                if !value.is_finite() {
                    return Err("nonfinite row-parallel partial".into());
                }
                sum += value;
                if !sum.is_finite() {
                    return Err("row-parallel reduction overflow".into());
                }
            }
            let residual = f32::from_bits(u32::from(bits) << 16);
            let value = sum + residual;
            if !residual.is_finite() || !value.is_finite() {
                return Err("nonfinite residual sum".into());
            }
            let bits = value.to_bits();
            let rounding = 0x7fff + ((bits >> 16) & 1);
            let rounded = ((bits.wrapping_add(rounding)) >> 16) as u16;
            if !f32::from_bits(u32::from(rounded) << 16).is_finite() {
                return Err("BF16 residual sum overflow".into());
            }
            workspace.output[index] = rounded;
            workspace.broadcast[index * 2..index * 2 + 2].copy_from_slice(&rounded.to_le_bytes());
        }
        for (rank, transport) in self.ranks.iter().zip(&mut self.transports) {
            transport.write(rank.hidden.id, 0, &workspace.broadcast[..elements * 2])?;
        }
        self.collective
            .advance()
            .map_err(|error| format!("host collective advance: {error:?}"))?;
        self.hidden.copy_from_slice(&workspace.output[..elements]);
        Ok(())
    }
}
